//! Stream selection logic for choosing video quality, audio tracks, and subtitles.
//!
//! This module provides functionality to select the best matching streams
//! from a parsed MPD manifest based on user preferences.

use crate::api::CRSubtitle;
use crate::download::manifest::{AdaptationSet, ContentType, MpdManifest, Representation};
use crate::error::{DownloadError, Error, Result};
use crate::utils::locales_match;
use std::collections::HashMap;
use tracing::{debug, trace};

/// Selected streams for download.
#[derive(Debug, Clone)]
pub struct StreamSelection {
    /// Selected video stream.
    pub video: SelectedStream,
    /// Selected audio streams (one per language).
    pub audio: Vec<SelectedStream>,
    /// Selected subtitle tracks.
    pub subtitles: Vec<SubtitleTrack>,
    /// Widevine PSSH for DRM decryption.
    pub pssh: Option<String>,
    /// Default key ID.
    pub key_id: Option<String>,
}

/// A selected stream (video or audio).
#[derive(Debug, Clone)]
pub struct SelectedStream {
    /// Stream type description (e.g., "video", "audio").
    pub stream_type: String,
    /// Language code (e.g., "ja-JP", "en-US").
    pub lang: Option<String>,
    /// Quality label (e.g., "1080p", "128kbps").
    pub quality: String,
    /// Bandwidth in bits per second.
    pub bandwidth: u64,
    /// Video width (if video).
    pub width: Option<u32>,
    /// Video height (if video).
    pub height: Option<u32>,
    /// Codec string.
    pub codecs: Option<String>,
    /// Initialization segment URL.
    pub init_url: Option<String>,
    /// Media segment URLs.
    pub segment_urls: Vec<String>,
}

/// A subtitle track.
#[derive(Debug, Clone)]
pub struct SubtitleTrack {
    /// Language code (e.g., "en-US", "es-LA").
    pub lang: String,
    /// Human-readable language name.
    pub label: Option<String>,
    /// Subtitle URL.
    pub url: String,
    /// Subtitle format (e.g., "vtt", "ass").
    pub format: String,
    /// Whether this is a closed caption track (for hearing impaired).
    pub is_cc: bool,
    /// Whether this is a Signs & Songs track (subtitle lang matches audio lang).
    pub is_signs: bool,
}

/// Stream selector for choosing streams from a manifest.
pub struct StreamSelector;

impl StreamSelector {
    /// Select streams from a manifest based on preferences.
    ///
    /// # Arguments
    /// * `manifest` - Parsed MPD manifest.
    /// * `quality` - Desired video quality (height in pixels, or None for best).
    /// * `audio_langs` - Preferred audio languages in order of preference.
    /// * `sub_langs` - Preferred subtitle languages in order of preference.
    /// * `api_subtitles` - Subtitles from the Crunchyroll API (often better than MPD subtitles).
    /// * `api_captions` - Closed captions from the Crunchyroll API.
    /// * `skip_subs` - Whether to skip subtitle selection.
    pub fn select(
        manifest: &MpdManifest,
        quality: Option<u32>,
        audio_langs: &[String],
        sub_langs: &[String],
        api_subtitles: Option<&HashMap<String, CRSubtitle>>,
        api_captions: Option<&HashMap<String, CRSubtitle>>,
        skip_subs: bool,
    ) -> Result<StreamSelection> {
        debug!(
            "Selecting streams: quality={:?}, audio={:?}, subs={:?}",
            quality, audio_langs, sub_langs
        );

        // Select video
        let video = Self::select_video(manifest, quality)?;

        // Select audio
        let audio = Self::select_audio(manifest, audio_langs)?;

        // Collect selected audio locales for Signs & Songs detection
        let audio_locales: Vec<String> = audio.iter().filter_map(|a| a.lang.clone()).collect();

        // Select subtitles
        let subtitles = if skip_subs {
            Vec::new()
        } else {
            Self::select_subtitles(
                manifest,
                sub_langs,
                api_subtitles,
                api_captions,
                &audio_locales,
            )
        };

        // Extract DRM info
        let pssh = manifest.widevine_pssh().map(|s| s.to_string());
        let key_id = manifest.default_key_id().map(|s| s.to_string());

        Ok(StreamSelection {
            video,
            audio,
            subtitles,
            pssh,
            key_id,
        })
    }

    /// Select the best video stream based on quality preference.
    fn select_video(manifest: &MpdManifest, target_height: Option<u32>) -> Result<SelectedStream> {
        let video_sets = manifest.video_adaptation_sets();
        if video_sets.is_empty() {
            return Err(Error::Download(DownloadError::NoStreams));
        }

        // Get all video representations
        let mut representations: Vec<(&AdaptationSet, &Representation)> = video_sets
            .iter()
            .flat_map(|as_| as_.representations.iter().map(move |rep| (*as_, rep)))
            .collect();

        if representations.is_empty() {
            return Err(Error::Download(DownloadError::NoStreams));
        }

        // Sort by bandwidth (highest first)
        representations.sort_by(|a, b| b.1.bandwidth.cmp(&a.1.bandwidth));

        // Log available qualities
        trace!(
            "Available video qualities: {:?}",
            representations
                .iter()
                .map(|(_, r)| format!("{}p ({}bps)", r.height.unwrap_or(0), r.bandwidth))
                .collect::<Vec<_>>()
        );

        // Select representation based on quality preference
        let (adaptation_set, rep) = match target_height {
            None => {
                // Best quality: pick highest bandwidth
                debug!("Selecting best quality (highest bandwidth)");
                representations[0]
            }
            Some(height) => {
                // Find exact match or closest lower quality
                debug!("Selecting quality closest to {}p", height);

                // First try exact match
                if let Some(found) = representations
                    .iter()
                    .find(|(_, r)| r.height == Some(height))
                {
                    *found
                } else {
                    // Find closest height that doesn't exceed target
                    let candidates: Vec<_> = representations
                        .iter()
                        .filter(|(_, r)| r.height.map(|h| h <= height).unwrap_or(false))
                        .collect();

                    if let Some(found) = candidates.first() {
                        **found
                    } else {
                        // All available qualities are higher than requested, pick lowest
                        *representations.last().unwrap()
                    }
                }
            }
        };

        debug!(
            "Selected video: {} ({}x{}, {} bps)",
            rep.id,
            rep.width.unwrap_or(0),
            rep.height.unwrap_or(0),
            rep.bandwidth
        );

        Ok(SelectedStream {
            stream_type: "video".to_string(),
            lang: None,
            quality: rep.quality_label(),
            bandwidth: rep.bandwidth,
            width: rep.width,
            height: rep.height,
            codecs: rep.get_codecs(adaptation_set).map(|s| s.to_string()),
            init_url: rep.init_url(adaptation_set, &manifest.base_url),
            segment_urls: rep.segment_urls(adaptation_set, &manifest.base_url),
        })
    }

    /// Select audio streams based on language preference.
    fn select_audio(
        manifest: &MpdManifest,
        preferred_langs: &[String],
    ) -> Result<Vec<SelectedStream>> {
        let audio_sets = manifest.audio_adaptation_sets();
        if audio_sets.is_empty() {
            return Err(Error::Download(DownloadError::NoStreams));
        }

        let mut selected = Vec::new();

        // If no language preference, select the first (usually default) audio
        if preferred_langs.is_empty() {
            if let Some(as_) = audio_sets.first() {
                if let Some(rep) = Self::best_audio_representation(as_) {
                    selected.push(Self::audio_stream_from_rep(manifest, as_, rep));
                }
            }
            return Ok(selected);
        }

        // Select audio for each preferred language
        for lang in preferred_langs {
            // Try exact match first
            let matching_set = audio_sets
                .iter()
                .find(|as_| {
                    as_.lang
                        .as_ref()
                        .map(|l| l.eq_ignore_ascii_case(lang))
                        .unwrap_or(false)
                })
                .or_else(|| {
                    // Try prefix match (e.g., "ja" matches "ja-JP")
                    audio_sets.iter().find(|as_| {
                        as_.lang
                            .as_ref()
                            .map(|l| {
                                l.to_lowercase().starts_with(&lang.to_lowercase())
                                    || lang.to_lowercase().starts_with(&l.to_lowercase())
                            })
                            .unwrap_or(false)
                    })
                });

            if let Some(as_) = matching_set {
                if let Some(rep) = Self::best_audio_representation(as_) {
                    let stream = Self::audio_stream_from_rep(manifest, as_, rep);
                    // Avoid duplicates
                    if !selected.iter().any(|s| s.lang == stream.lang) {
                        debug!(
                            "Selected audio: {} ({:?}, {} bps)",
                            rep.id, as_.lang, rep.bandwidth
                        );
                        selected.push(stream);
                    }
                }
            } else {
                debug!("No audio track found for language: {}", lang);
            }
        }

        // If no matches found, fall back to first available
        if selected.is_empty() {
            if let Some(as_) = audio_sets.first() {
                if let Some(rep) = Self::best_audio_representation(as_) {
                    debug!("Falling back to first audio: {} ({:?})", rep.id, as_.lang);
                    selected.push(Self::audio_stream_from_rep(manifest, as_, rep));
                }
            }
        }

        Ok(selected)
    }

    /// Get the best audio representation from an adaptation set (highest bandwidth).
    fn best_audio_representation(adaptation_set: &AdaptationSet) -> Option<&Representation> {
        adaptation_set
            .representations
            .iter()
            .max_by_key(|r| r.bandwidth)
    }

    /// Create a SelectedStream from an audio representation.
    fn audio_stream_from_rep(
        manifest: &MpdManifest,
        adaptation_set: &AdaptationSet,
        rep: &Representation,
    ) -> SelectedStream {
        SelectedStream {
            stream_type: "audio".to_string(),
            lang: adaptation_set.lang.clone(),
            quality: rep.quality_label(),
            bandwidth: rep.bandwidth,
            width: None,
            height: None,
            codecs: rep.get_codecs(adaptation_set).map(|s| s.to_string()),
            init_url: rep.init_url(adaptation_set, &manifest.base_url),
            segment_urls: rep.segment_urls(adaptation_set, &manifest.base_url),
        }
    }

    /// Select subtitle tracks based on language preference.
    ///
    /// # Arguments
    /// * `manifest` - Parsed MPD manifest.
    /// * `preferred_langs` - Preferred subtitle languages in order of preference.
    /// * `api_subtitles` - Subtitles from the Crunchyroll API.
    /// * `api_captions` - Closed captions from the Crunchyroll API.
    /// * `audio_locales` - Selected audio track locales for Signs & Songs detection.
    fn select_subtitles(
        manifest: &MpdManifest,
        preferred_langs: &[String],
        api_subtitles: Option<&HashMap<String, CRSubtitle>>,
        api_captions: Option<&HashMap<String, CRSubtitle>>,
        audio_locales: &[String],
    ) -> Vec<SubtitleTrack> {
        let mut selected = Vec::new();

        // Helper to check if subtitle is Signs & Songs (matches any audio locale)
        let is_signs_and_songs = |sub_locale: &str| -> bool {
            audio_locales
                .iter()
                .any(|audio_locale| locales_match(sub_locale, audio_locale))
        };

        // Prefer API subtitles over MPD subtitles (usually better quality)
        if let Some(subs) = api_subtitles {
            for lang in preferred_langs {
                // Try exact match
                if let Some(sub) = subs.get(lang) {
                    let is_signs = is_signs_and_songs(&sub.locale);
                    selected.push(SubtitleTrack {
                        lang: sub.locale.clone(),
                        label: None,
                        url: sub.url.clone(),
                        format: if sub.format.is_empty() {
                            "vtt".to_string()
                        } else {
                            sub.format.clone()
                        },
                        is_cc: false,
                        is_signs,
                    });
                    continue;
                }

                // Try prefix match
                for (key, sub) in subs.iter() {
                    if key.to_lowercase().starts_with(&lang.to_lowercase())
                        || lang.to_lowercase().starts_with(&key.to_lowercase())
                    {
                        if !selected.iter().any(|s| s.lang == sub.locale && !s.is_cc) {
                            let is_signs = is_signs_and_songs(&sub.locale);
                            selected.push(SubtitleTrack {
                                lang: sub.locale.clone(),
                                label: None,
                                url: sub.url.clone(),
                                format: if sub.format.is_empty() {
                                    "vtt".to_string()
                                } else {
                                    sub.format.clone()
                                },
                                is_cc: false,
                                is_signs,
                            });
                        }
                        break;
                    }
                }
            }
        }

        // Add closed captions (separate from regular subtitles)
        if let Some(captions) = api_captions {
            for lang in preferred_langs {
                // Try exact match
                if let Some(cap) = captions.get(lang) {
                    if !selected.iter().any(|s| s.lang == cap.locale && s.is_cc) {
                        selected.push(SubtitleTrack {
                            lang: cap.locale.clone(),
                            label: None,
                            url: cap.url.clone(),
                            format: if cap.format.is_empty() {
                                "vtt".to_string()
                            } else {
                                cap.format.clone()
                            },
                            is_cc: true,
                            is_signs: false, // CCs are never Signs & Songs
                        });
                    }
                    continue;
                }

                // Try prefix match
                for (key, cap) in captions.iter() {
                    if key.to_lowercase().starts_with(&lang.to_lowercase())
                        || lang.to_lowercase().starts_with(&key.to_lowercase())
                    {
                        if !selected.iter().any(|s| s.lang == cap.locale && s.is_cc) {
                            selected.push(SubtitleTrack {
                                lang: cap.locale.clone(),
                                label: None,
                                url: cap.url.clone(),
                                format: if cap.format.is_empty() {
                                    "vtt".to_string()
                                } else {
                                    cap.format.clone()
                                },
                                is_cc: true,
                                is_signs: false,
                            });
                        }
                        break;
                    }
                }
            }
        }

        // If we found API subtitles/captions, return them
        if !selected.is_empty() {
            debug!(
                "Selected {} subtitle tracks from API ({} regular, {} CC)",
                selected.len(),
                selected.iter().filter(|s| !s.is_cc).count(),
                selected.iter().filter(|s| s.is_cc).count()
            );
            return selected;
        }

        // Fall back to MPD subtitles
        let text_sets: Vec<&AdaptationSet> = manifest
            .periods
            .iter()
            .flat_map(|p| &p.adaptation_sets)
            .filter(|as_| as_.content_type == ContentType::Text)
            .collect();

        for lang in preferred_langs {
            for as_ in &text_sets {
                let matches = as_
                    .lang
                    .as_ref()
                    .map(|l| {
                        l.eq_ignore_ascii_case(lang)
                            || l.to_lowercase().starts_with(&lang.to_lowercase())
                            || lang.to_lowercase().starts_with(&l.to_lowercase())
                    })
                    .unwrap_or(false);

                if matches {
                    // Get URL from first representation's base URL
                    if let Some(rep) = as_.representations.first() {
                        if let Some(ref url) = rep.base_url {
                            let sub_lang = as_.lang.clone().unwrap_or_default();
                            if !selected.iter().any(|s| s.lang == sub_lang) {
                                let is_signs = is_signs_and_songs(&sub_lang);
                                selected.push(SubtitleTrack {
                                    lang: sub_lang,
                                    label: None,
                                    url: url.clone(),
                                    format: as_
                                        .mime_type
                                        .as_ref()
                                        .map(|m| {
                                            if m.contains("vtt") {
                                                "vtt"
                                            } else if m.contains("ttml") {
                                                "ttml"
                                            } else {
                                                "vtt"
                                            }
                                        })
                                        .unwrap_or("vtt")
                                        .to_string(),
                                    is_cc: false,
                                    is_signs,
                                });
                            }
                        }
                    }
                }
            }
        }

        debug!("Selected {} subtitle tracks from MPD", selected.len());
        selected
    }

    /// Get all available video qualities from the manifest.
    pub fn available_video_qualities(manifest: &MpdManifest) -> Vec<(u32, u64)> {
        let mut qualities: Vec<(u32, u64)> = manifest
            .video_adaptation_sets()
            .iter()
            .flat_map(|as_| &as_.representations)
            .filter_map(|rep| rep.height.map(|h| (h, rep.bandwidth)))
            .collect();

        qualities.sort_by(|a, b| b.0.cmp(&a.0)); // Sort by height descending
        qualities.dedup_by(|a, b| a.0 == b.0); // Remove duplicates
        qualities
    }

    /// Get all available audio languages from the manifest.
    pub fn available_audio_languages(manifest: &MpdManifest) -> Vec<String> {
        let mut langs: Vec<String> = manifest
            .audio_adaptation_sets()
            .iter()
            .filter_map(|as_| as_.lang.clone())
            .collect();

        langs.sort();
        langs.dedup();
        langs
    }

    /// Get all available subtitle languages.
    pub fn available_subtitle_languages(
        manifest: &MpdManifest,
        api_subtitles: Option<&HashMap<String, CRSubtitle>>,
    ) -> Vec<String> {
        let mut langs: Vec<String> = Vec::new();

        // From API
        if let Some(subs) = api_subtitles {
            langs.extend(subs.values().map(|s| s.locale.clone()));
        }

        // From MPD
        langs.extend(
            manifest
                .periods
                .iter()
                .flat_map(|p| &p.adaptation_sets)
                .filter(|as_| as_.content_type == ContentType::Text)
                .filter_map(|as_| as_.lang.clone()),
        );

        langs.sort();
        langs.dedup();
        langs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manifest() -> MpdManifest {
        use crate::download::manifest::{Period, SegmentEntry, SegmentTemplate};

        let video_as = AdaptationSet {
            id: Some("1".to_string()),
            content_type: ContentType::Video,
            lang: None,
            mime_type: Some("video/mp4".to_string()),
            codecs: Some("avc1.640028".to_string()),
            base_url: None,
            segment_template: Some(SegmentTemplate {
                initialization: Some("init-$RepresentationID$.mp4".to_string()),
                media: Some("seg-$RepresentationID$-$Number$.m4s".to_string()),
                timescale: 1000,
                start_number: 1,
                timeline: vec![SegmentEntry {
                    t: None,
                    d: 2000,
                    r: 4,
                }],
            }),
            content_protection: Vec::new(),
            representations: vec![
                Representation {
                    id: "v1080".to_string(),
                    bandwidth: 8000000,
                    width: Some(1920),
                    height: Some(1080),
                    frame_rate: None,
                    codecs: None,
                    mime_type: None,
                    base_url: Some("https://cdn.example.com/".to_string()),
                    segment_template: None,
                    content_protection: Vec::new(),
                },
                Representation {
                    id: "v720".to_string(),
                    bandwidth: 4000000,
                    width: Some(1280),
                    height: Some(720),
                    frame_rate: None,
                    codecs: None,
                    mime_type: None,
                    base_url: Some("https://cdn.example.com/".to_string()),
                    segment_template: None,
                    content_protection: Vec::new(),
                },
                Representation {
                    id: "v480".to_string(),
                    bandwidth: 2000000,
                    width: Some(854),
                    height: Some(480),
                    frame_rate: None,
                    codecs: None,
                    mime_type: None,
                    base_url: Some("https://cdn.example.com/".to_string()),
                    segment_template: None,
                    content_protection: Vec::new(),
                },
            ],
        };

        let audio_ja = AdaptationSet {
            id: Some("2".to_string()),
            content_type: ContentType::Audio,
            lang: Some("ja-JP".to_string()),
            mime_type: Some("audio/mp4".to_string()),
            codecs: Some("mp4a.40.2".to_string()),
            base_url: None,
            segment_template: Some(SegmentTemplate {
                initialization: Some("init-$RepresentationID$.mp4".to_string()),
                media: Some("seg-$RepresentationID$-$Number$.m4s".to_string()),
                timescale: 1000,
                start_number: 1,
                timeline: vec![SegmentEntry {
                    t: None,
                    d: 2000,
                    r: 4,
                }],
            }),
            content_protection: Vec::new(),
            representations: vec![Representation {
                id: "a-ja".to_string(),
                bandwidth: 128000,
                width: None,
                height: None,
                frame_rate: None,
                codecs: None,
                mime_type: None,
                base_url: Some("https://cdn.example.com/".to_string()),
                segment_template: None,
                content_protection: Vec::new(),
            }],
        };

        let audio_en = AdaptationSet {
            id: Some("3".to_string()),
            content_type: ContentType::Audio,
            lang: Some("en-US".to_string()),
            mime_type: Some("audio/mp4".to_string()),
            codecs: Some("mp4a.40.2".to_string()),
            base_url: None,
            segment_template: Some(SegmentTemplate {
                initialization: Some("init-$RepresentationID$.mp4".to_string()),
                media: Some("seg-$RepresentationID$-$Number$.m4s".to_string()),
                timescale: 1000,
                start_number: 1,
                timeline: vec![SegmentEntry {
                    t: None,
                    d: 2000,
                    r: 4,
                }],
            }),
            content_protection: Vec::new(),
            representations: vec![Representation {
                id: "a-en".to_string(),
                bandwidth: 128000,
                width: None,
                height: None,
                frame_rate: None,
                codecs: None,
                mime_type: None,
                base_url: Some("https://cdn.example.com/".to_string()),
                segment_template: None,
                content_protection: Vec::new(),
            }],
        };

        let text_en = AdaptationSet {
            id: Some("4".to_string()),
            content_type: ContentType::Text,
            lang: Some("en-US".to_string()),
            mime_type: Some("text/vtt".to_string()),
            codecs: None,
            base_url: None,
            segment_template: None,
            content_protection: Vec::new(),
            representations: vec![Representation {
                id: "s-en".to_string(),
                bandwidth: 0,
                width: None,
                height: None,
                frame_rate: None,
                codecs: None,
                mime_type: None,
                base_url: Some("https://cdn.example.com/subs/en.vtt".to_string()),
                segment_template: None,
                content_protection: Vec::new(),
            }],
        };

        MpdManifest {
            base_url: "https://example.com/".to_string(),
            duration: Some(600.0),
            min_buffer_time: Some(2.0),
            periods: vec![Period {
                id: Some("1".to_string()),
                start: None,
                duration: None,
                base_url: None,
                adaptation_sets: vec![video_as, audio_ja, audio_en, text_en],
            }],
        }
    }

    #[test]
    fn test_select_video_best_quality() {
        let manifest = create_test_manifest();
        let video = StreamSelector::select_video(&manifest, None).unwrap();

        assert_eq!(video.height, Some(1080));
        assert_eq!(video.bandwidth, 8000000);
        assert_eq!(video.quality, "1080p");
        assert!(video.init_url.is_some());
        assert_eq!(video.segment_urls.len(), 5);
    }

    #[test]
    fn test_select_video_specific_quality() {
        let manifest = create_test_manifest();

        // Exact match
        let video = StreamSelector::select_video(&manifest, Some(720)).unwrap();
        assert_eq!(video.height, Some(720));

        // Closest match (600p should select 480p)
        let video = StreamSelector::select_video(&manifest, Some(600)).unwrap();
        assert_eq!(video.height, Some(480));

        // Lower than all (should get lowest available)
        let video = StreamSelector::select_video(&manifest, Some(240)).unwrap();
        assert_eq!(video.height, Some(480)); // Lowest available is 480p
    }

    #[test]
    fn test_select_audio_preferred_language() {
        let manifest = create_test_manifest();

        // Prefer Japanese
        let audio = StreamSelector::select_audio(&manifest, &["ja-JP".to_string()]).unwrap();
        assert_eq!(audio.len(), 1);
        assert_eq!(audio[0].lang, Some("ja-JP".to_string()));

        // Prefer English
        let audio = StreamSelector::select_audio(&manifest, &["en-US".to_string()]).unwrap();
        assert_eq!(audio.len(), 1);
        assert_eq!(audio[0].lang, Some("en-US".to_string()));

        // Prefer both
        let audio =
            StreamSelector::select_audio(&manifest, &["ja-JP".to_string(), "en-US".to_string()])
                .unwrap();
        assert_eq!(audio.len(), 2);
    }

    #[test]
    fn test_select_audio_prefix_match() {
        let manifest = create_test_manifest();

        // "ja" should match "ja-JP"
        let audio = StreamSelector::select_audio(&manifest, &["ja".to_string()]).unwrap();
        assert_eq!(audio.len(), 1);
        assert_eq!(audio[0].lang, Some("ja-JP".to_string()));

        // "en" should match "en-US"
        let audio = StreamSelector::select_audio(&manifest, &["en".to_string()]).unwrap();
        assert_eq!(audio.len(), 1);
        assert_eq!(audio[0].lang, Some("en-US".to_string()));
    }

    #[test]
    fn test_select_audio_fallback() {
        let manifest = create_test_manifest();

        // Unknown language should fall back to first available
        let audio = StreamSelector::select_audio(&manifest, &["fr-FR".to_string()]).unwrap();
        assert_eq!(audio.len(), 1);
        // Falls back to first (Japanese)
        assert_eq!(audio[0].lang, Some("ja-JP".to_string()));
    }

    #[test]
    fn test_select_audio_empty_preference() {
        let manifest = create_test_manifest();

        // No preference should select first available
        let audio = StreamSelector::select_audio(&manifest, &[]).unwrap();
        assert_eq!(audio.len(), 1);
    }

    #[test]
    fn test_select_subtitles() {
        let manifest = create_test_manifest();
        let audio_locales = vec!["ja-JP".to_string()];

        let subs = StreamSelector::select_subtitles(
            &manifest,
            &["en-US".to_string()],
            None,
            None,
            &audio_locales,
        );
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].lang, "en-US");
        assert_eq!(subs[0].format, "vtt");
        assert!(!subs[0].is_signs); // en-US != ja-JP
        assert!(!subs[0].is_cc);
    }

    #[test]
    fn test_select_subtitles_from_api() {
        let manifest = create_test_manifest();
        let audio_locales = vec!["ja-JP".to_string()];

        let mut api_subs = HashMap::new();
        api_subs.insert(
            "en-US".to_string(),
            CRSubtitle {
                locale: "en-US".to_string(),
                url: "https://api.example.com/subs/en.vtt".to_string(),
                format: "vtt".to_string(),
            },
        );
        api_subs.insert(
            "es-LA".to_string(),
            CRSubtitle {
                locale: "es-LA".to_string(),
                url: "https://api.example.com/subs/es.vtt".to_string(),
                format: "vtt".to_string(),
            },
        );

        let subs = StreamSelector::select_subtitles(
            &manifest,
            &["en-US".to_string(), "es-LA".to_string()],
            Some(&api_subs),
            None,
            &audio_locales,
        );
        assert_eq!(subs.len(), 2);
        assert!(subs[0].url.contains("api.example.com")); // From API, not MPD
    }

    #[test]
    fn test_select_subtitles_signs_and_songs() {
        let manifest = create_test_manifest();
        // Audio is Japanese, so Japanese subtitles should be Signs & Songs
        let audio_locales = vec!["ja-JP".to_string()];

        let mut api_subs = HashMap::new();
        api_subs.insert(
            "ja-JP".to_string(),
            CRSubtitle {
                locale: "ja-JP".to_string(),
                url: "https://api.example.com/subs/ja.vtt".to_string(),
                format: "vtt".to_string(),
            },
        );
        api_subs.insert(
            "en-US".to_string(),
            CRSubtitle {
                locale: "en-US".to_string(),
                url: "https://api.example.com/subs/en.vtt".to_string(),
                format: "vtt".to_string(),
            },
        );

        let subs = StreamSelector::select_subtitles(
            &manifest,
            &["ja-JP".to_string(), "en-US".to_string()],
            Some(&api_subs),
            None,
            &audio_locales,
        );

        assert_eq!(subs.len(), 2);
        // Japanese subs with Japanese audio = Signs & Songs
        let ja_sub = subs.iter().find(|s| s.lang == "ja-JP").unwrap();
        assert!(ja_sub.is_signs);
        // English subs with Japanese audio = Full subs
        let en_sub = subs.iter().find(|s| s.lang == "en-US").unwrap();
        assert!(!en_sub.is_signs);
    }

    #[test]
    fn test_select_subtitles_with_closed_captions() {
        let manifest = create_test_manifest();
        let audio_locales = vec!["ja-JP".to_string()];

        let mut api_subs = HashMap::new();
        api_subs.insert(
            "en-US".to_string(),
            CRSubtitle {
                locale: "en-US".to_string(),
                url: "https://api.example.com/subs/en.vtt".to_string(),
                format: "vtt".to_string(),
            },
        );

        let mut api_captions = HashMap::new();
        api_captions.insert(
            "en-US".to_string(),
            CRSubtitle {
                locale: "en-US".to_string(),
                url: "https://api.example.com/captions/en.vtt".to_string(),
                format: "vtt".to_string(),
            },
        );

        let subs = StreamSelector::select_subtitles(
            &manifest,
            &["en-US".to_string()],
            Some(&api_subs),
            Some(&api_captions),
            &audio_locales,
        );

        // Should have both regular sub and CC
        assert_eq!(subs.len(), 2);
        let regular = subs.iter().find(|s| !s.is_cc).unwrap();
        assert_eq!(regular.lang, "en-US");
        assert!(!regular.is_cc);
        let cc = subs.iter().find(|s| s.is_cc).unwrap();
        assert_eq!(cc.lang, "en-US");
        assert!(cc.is_cc);
    }

    #[test]
    fn test_full_selection() {
        let manifest = create_test_manifest();

        let selection = StreamSelector::select(
            &manifest,
            Some(720),
            &["ja-JP".to_string()],
            &["en-US".to_string()],
            None,
            None,
            false,
        )
        .unwrap();

        assert_eq!(selection.video.height, Some(720));
        assert_eq!(selection.audio.len(), 1);
        assert_eq!(selection.audio[0].lang, Some("ja-JP".to_string()));
        assert_eq!(selection.subtitles.len(), 1);
        assert_eq!(selection.subtitles[0].lang, "en-US");
    }

    #[test]
    fn test_available_qualities() {
        let manifest = create_test_manifest();
        let qualities = StreamSelector::available_video_qualities(&manifest);

        assert_eq!(qualities.len(), 3);
        assert_eq!(qualities[0].0, 1080); // Highest first
        assert_eq!(qualities[1].0, 720);
        assert_eq!(qualities[2].0, 480);
    }

    #[test]
    fn test_available_languages() {
        let manifest = create_test_manifest();

        let audio_langs = StreamSelector::available_audio_languages(&manifest);
        assert_eq!(audio_langs.len(), 2);
        assert!(audio_langs.contains(&"ja-JP".to_string()));
        assert!(audio_langs.contains(&"en-US".to_string()));

        let sub_langs = StreamSelector::available_subtitle_languages(&manifest, None);
        assert_eq!(sub_langs.len(), 1);
        assert!(sub_langs.contains(&"en-US".to_string()));
    }
}
