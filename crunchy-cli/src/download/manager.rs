//! Download manager - orchestrates the full download flow.
//!
//! This module handles the complete download process:
//! 1. Fetch episode metadata
//! 2. Get playback data and manifest
//! 3. Select streams based on user preferences
//! 4. Download video/audio segments
//! 5. Acquire DRM keys and decrypt
//! 6. Download and convert subtitles
//! 7. Mux everything into final output

use crate::api::types::{CREpisode, CRStreamData};
use crate::api::CrunchyrollClient;
use crate::config::Config;
use crate::error::{ConfigError, DownloadError, Error, MediaError, Result};
use crate::media::{
    acquire_keys, DecryptionKey, FfmpegBuilder, FilenameGenerator, FilenameVars,
    Mp4DecryptBuilder, SubtitleConverter,
};
use crate::utils::{format_bytes, format_elapsed, get_language, redact};
use reqwest::Client;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;
use tokio::sync::{Mutex, RwLock, Semaphore};
use tracing::{debug, info, trace, warn};

use super::cache::{DownloadCache, DownloadPhase, StreamCache};

use super::manifest::MpdManifest;
use super::progress::{null_reporter, ProgressReporter, ProgressTracker, StepProgress};
use super::segment::SegmentDownloader;
use super::selector::{SelectedStream, StreamSelector, SubtitleTrack};

/// Extract the playback media ID from a version GUID.
/// Handles GUIDs with ":" separator (e.g., "G0DUMX5E0:en-US" → "G0DUMX5E0")
fn extract_media_guid(guid: &str) -> &str {
    guid.split(':').next().unwrap_or(guid)
}

/// Information about an audio track from a specific version.
struct AudioTrackInfo {
    stream: SelectedStream,
    keys: Vec<DecryptionKey>,
    subtitles: Vec<SubtitleTrack>,
}

/// Cheaply-cloneable handle for tracking CR streaming tokens that need
/// releasing at the end of a download. The matching [`TokenReleaseGuard`]
/// drains the tracker on drop and fires DELETE-token requests as a detached
/// tokio task.
#[derive(Clone)]
struct TokenTracker {
    tokens: Arc<std::sync::Mutex<Vec<(String, String)>>>,
}

impl TokenTracker {
    fn track(&self, guid: String, token: String) {
        if let Ok(mut lock) = self.tokens.lock() {
            lock.push((guid, token));
        }
    }
}

/// RAII guard: releases every CR streaming token activated during a
/// download when the guard drops (success, error-via-?, panic). Without
/// this, CR keeps the slot counted for ~5 min after we stop fetching,
/// driving accounts into HTTP 420 TOO_MANY_ACTIVE_STREAMS on a backlog.
///
/// Drop can't be async, so we spawn a detached tokio task to issue the
/// DELETE calls. Failures are logged at debug and swallowed — the
/// worst-case outcome is identical to not having this guard at all
/// (tokens time out naturally on CR's side).
struct TokenReleaseGuard {
    client: Arc<CrunchyrollClient>,
    tracker: TokenTracker,
}

impl TokenReleaseGuard {
    fn new(client: Arc<CrunchyrollClient>) -> Self {
        Self {
            client,
            tracker: TokenTracker {
                tokens: Arc::new(std::sync::Mutex::new(Vec::new())),
            },
        }
    }

    fn tracker(&self) -> TokenTracker {
        self.tracker.clone()
    }
}

impl Drop for TokenReleaseGuard {
    fn drop(&mut self) {
        let to_release: Vec<(String, String)> = match self.tracker.tokens.lock() {
            Ok(mut lock) => std::mem::take(&mut *lock),
            Err(_) => return,
        };
        if to_release.is_empty() {
            return;
        }
        let client = self.client.clone();
        tokio::spawn(async move {
            for (guid, token) in to_release {
                if let Err(e) = client.deactivate_token(&guid, &token).await {
                    debug!(
                        "deactivate_token failed for {} (best-effort): {}",
                        guid, e
                    );
                }
            }
        });
    }
}

/// Intermediate result from parallel fetching of audio version data.
/// Contains all data needed to acquire decryption keys.
struct AudioVersionFetchResult {
    locale: String,
    media_guid: String,
    stream_data: CRStreamData,
    audio_stream: SelectedStream,
    pssh: Option<String>,
    subtitles: Vec<SubtitleTrack>,
}

/// Result of a successful download.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// Canonical URI of the published output (e.g. `file:///abs/path.mkv`,
    /// `s3://bucket/key.mkv`).
    pub output_uri: String,
    /// Episode title.
    pub title: String,
    /// Video quality downloaded.
    pub quality: String,
    /// Primary audio language (first in `audio_languages`).
    pub audio_language: String,
    /// Every audio locale actually muxed into the output. Used by the
    /// crunchy-api watchlist to detect when newly-available dub tracks make
    /// an existing download eligible for re-download.
    pub audio_languages: Vec<String>,
    /// Subtitle languages included.
    pub subtitle_languages: Vec<String>,
}

/// Specifies what content to download.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DownloadMode {
    /// Download all content (video, audio, subtitles).
    #[default]
    Full,
    /// Download only subtitles.
    OnlySubs,
    /// Download only audio tracks.
    OnlyAudio,
    /// Download only video track.
    OnlyVideo,
}

impl DownloadMode {
    /// Returns the filename suffix for this mode.
    pub fn filename_suffix(&self) -> Option<&'static str> {
        match self {
            DownloadMode::Full => None,
            DownloadMode::OnlySubs => Some("subtitles"),
            DownloadMode::OnlyAudio => Some("audios"),
            DownloadMode::OnlyVideo => Some("video"),
        }
    }

    /// Returns whether video should be downloaded in this mode.
    pub fn download_video(&self) -> bool {
        matches!(self, DownloadMode::Full | DownloadMode::OnlyVideo)
    }

    /// Returns whether audio should be downloaded in this mode.
    pub fn download_audio(&self) -> bool {
        matches!(self, DownloadMode::Full | DownloadMode::OnlyAudio)
    }

    /// Returns whether subtitles should be downloaded in this mode.
    pub fn download_subs(&self) -> bool {
        matches!(self, DownloadMode::Full | DownloadMode::OnlySubs)
    }
}

/// Options for downloading an episode.
#[derive(Debug, Clone, Default)]
pub struct DownloadOptions {
    /// Override video quality (e.g., "1080p", "720p", "best").
    pub video_quality: Option<String>,
    /// Override audio languages.
    pub audio_languages: Option<Vec<String>>,
    /// Override subtitle languages.
    pub subtitle_languages: Option<Vec<String>>,
    /// Override output directory.
    pub output_dir: Option<PathBuf>,
    /// Skip existing files.
    pub skip_existing: bool,
    /// What content to download (full, only-subs, only-audio, only-video).
    pub download_mode: DownloadMode,
    /// Enable experimental resumable download caching.
    pub resume_cache: bool,
    /// Include closed caption (CC) subtitles.
    pub include_cc: bool,
}

/// Manages the overall download process for episodes.
pub struct DownloadManager {
    client: Arc<CrunchyrollClient>,
    config: Arc<RwLock<Config>>,
    http_client: Client,
    progress: ProgressTracker,
    /// High-level progress reporter for external consumers (API, WebSocket).
    reporter: Arc<dyn ProgressReporter>,
    /// Optional sink for publishing finalized output. When `None`, a
    /// [`LocalFsSink`] rooted at the configured `output_dir` is used.
    sink: Option<Arc<dyn crate::storage::OutputSink>>,
}

impl DownloadManager {
    /// Create a new download manager.
    pub fn new(client: Arc<CrunchyrollClient>, config: Arc<RwLock<Config>>) -> Self {
        Self {
            client,
            config,
            http_client: Client::new(),
            progress: ProgressTracker::new(),
            reporter: null_reporter(),
            sink: None,
        }
    }

    /// Create a new download manager with a custom progress reporter.
    pub fn with_reporter(
        client: Arc<CrunchyrollClient>,
        config: Arc<RwLock<Config>>,
        reporter: Arc<dyn ProgressReporter>,
    ) -> Self {
        Self {
            client,
            config,
            http_client: Client::new(),
            progress: ProgressTracker::new(),
            reporter,
            sink: None,
        }
    }

    /// Create a new download manager with a custom progress reporter and a
    /// caller-provided output sink. Use this to direct finalized files to
    /// alternative backends (e.g. S3) instead of the local filesystem.
    pub fn with_reporter_and_sink(
        client: Arc<CrunchyrollClient>,
        config: Arc<RwLock<Config>>,
        reporter: Arc<dyn ProgressReporter>,
        sink: Arc<dyn crate::storage::OutputSink>,
    ) -> Self {
        Self {
            client,
            config,
            http_client: Client::new(),
            progress: ProgressTracker::new(),
            reporter,
            sink: Some(sink),
        }
    }

    /// Download an episode by ID with default options.
    pub async fn download_episode(&self, episode_id: &str) -> Result<DownloadResult> {
        self.download_episode_with_options(episode_id, DownloadOptions::default())
            .await
    }

    /// Download an episode by ID with custom options.
    pub async fn download_episode_with_options(
        &self,
        episode_id: &str,
        options: DownloadOptions,
    ) -> Result<DownloadResult> {
        let download_start = Instant::now();
        info!("Starting download for episode: {}", episode_id);
        trace!("Download options: {:?}", options);

        // RAII: every CR streaming token we activate below is released on
        // drop (success, error, or panic) so the active-stream slot frees
        // up immediately instead of CR's ~5-min server-side timeout.
        let token_guard = TokenReleaseGuard::new(self.client.clone());
        let token_tracker = token_guard.tracker();

        self.reporter
            .on_phase_change("metadata", &format!("Fetching episode {}", episode_id));

        // 1. Fetch episode metadata
        let episode = self.client.get_episode(episode_id).await?;
        let season_num = if episode.season_sequence_number > 0 { episode.season_sequence_number } else { episode.season_number };
        info!(
            "Downloading: {} - S{}E{} - {}",
            episode.series_title, season_num, episode.episode, episode.title
        );
        trace!(
            "Episode details: id={}, duration={}ms, audio={}, subs={:?}",
            episode.id,
            episode.duration_ms,
            episode.audio_locale,
            episode.subtitle_locales
        );

        // 2. Get playback data
        let stream_data = self.client.get_playback(&episode.id).await?;

        // 3. Get the manifest URL (prefer DRM DASH)
        let manifest_url = self.get_manifest_url(&stream_data)?;
        debug!("Manifest URL: {}", manifest_url);

        // 4. Fetch and parse the MPD manifest
        let manifest_content = self.client.get_manifest(&manifest_url).await?;

        // Extract base URL from manifest URL
        let base_url = manifest_url
            .rsplit_once('/')
            .map(|(base, _)| format!("{}/", base))
            .unwrap_or_default();

        let manifest = MpdManifest::parse(&manifest_content, &base_url)?;
        debug!("Parsed manifest with {} periods", manifest.periods.len());

        // 5. Read config for preferences
        let cfg = self.config.read().await;
        let video_quality = options
            .video_quality
            .clone()
            .unwrap_or_else(|| cfg.downloads.video_quality.clone());
        let audio_languages = options
            .audio_languages
            .clone()
            .unwrap_or_else(|| cfg.languages.audio.clone());
        let subtitle_languages = options
            .subtitle_languages
            .clone()
            .unwrap_or_else(|| cfg.languages.subtitles.clone());
        let output_dir = options
            .output_dir
            .clone()
            .unwrap_or_else(|| cfg.downloads.output_dir.clone());
        let temp_dir = cfg.downloads.temp_dir.clone();
        let format = cfg.muxing.format.clone();
        let filename_template = cfg.muxing.filename_template.clone();
        let embed_subs = cfg.muxing.embed_subs;
        let default_audio = cfg.muxing.default_audio.clone();
        let default_sub = cfg.muxing.default_sub.clone();
        let prefer_signs_songs = cfg.muxing.prefer_signs_songs;
        let max_speed_kbps = cfg.downloads.max_speed_kbps;
        let parts = cfg.downloads.parts as usize;
        let max_concurrent_keys = cfg.downloads.max_concurrent_keys as usize;

        // Tool paths
        let ffmpeg_path = crate::media::ensure_ffmpeg().await?;
        let mp4decrypt_path = cfg.tools.mp4decrypt.clone();
        let widevine_client = cfg.tools.widevine_client.clone();
        let widevine_private_key = cfg.tools.widevine_private_key.clone();
        drop(cfg);

        // 6. Select streams based on preferences
        // Parse quality preference
        let quality_height = parse_quality(&video_quality);

        let api_captions = if options.include_cc {
            Some(&stream_data.closed_captions)
        } else {
            None
        };
        let selection = StreamSelector::select(
            &manifest,
            quality_height,
            &audio_languages,
            &subtitle_languages,
            Some(&stream_data.subtitles),
            api_captions,
            false, // don't skip subs
        )?;

        info!(
            "Selected: {}p video from primary manifest",
            selection.video.height.unwrap_or(0),
        );
        trace!(
            "Video stream: bandwidth={}bps, codecs={:?}, {} segments",
            selection.video.bandwidth,
            selection.video.codecs,
            selection.video.segment_urls.len()
        );
        if let Some(ref pssh) = selection.pssh {
            trace!("PSSH found: {}...", &pssh[..pssh.len().min(32)]);
        }

        // Build map: audio_locale -> version info for quick lookup
        let version_map: HashMap<&str, &crate::api::types::CREpisodeVersion> = episode
            .versions
            .iter()
            .map(|v| (v.audio_locale.as_str(), v))
            .collect();

        // Determine which audio locales we need and in what order (from config)
        // This preserves config order for final muxing
        let mut audio_locales_ordered: Vec<String> = Vec::new();
        for lang in &audio_languages {
            if version_map.contains_key(lang.as_str()) || lang == &stream_data.audio_locale {
                if !audio_locales_ordered.contains(lang) {
                    audio_locales_ordered.push(lang.clone());
                }
            }
            // Skip silently if not available
        }

        // If no languages from config are available, fall back to primary
        if audio_locales_ordered.is_empty() {
            audio_locales_ordered.push(stream_data.audio_locale.clone());
        }

        info!(
            "Audio tracks to download (in order): {:?}, {} subtitle(s) from primary",
            audio_locales_ordered,
            selection.subtitles.len()
        );

        // 7. Generate output filename
        let filename_gen = FilenameGenerator::new(&filename_template, output_dir.clone());
        let filename_vars = FilenameVars {
            series: episode.series_title.clone(),
            // Prefer season_sequence_number (user-visible), fall back to season_number
            season: if episode.season_sequence_number > 0 { episode.season_sequence_number } else { episode.season_number },
            season_title: episode.season_title.clone(),
            episode: episode.episode.clone(),
            episode_number: episode.episode_number,
            title: episode.title.clone(),
            quality: format!("{}p", selection.video.height.unwrap_or(0)),
            audio: selection
                .audio
                .first()
                .and_then(|a| a.lang.clone())
                .unwrap_or_default(),
            year: episode
                .episode_air_date
                .map(|d| d.format("%Y").to_string())
                .unwrap_or_default(),
        };
        let output_path = filename_gen.generate_with_suffix(
            &filename_vars,
            options.download_mode.filename_suffix(),
            &format,
        );

        // Check if output already exists
        if options.skip_existing && output_path.exists() {
            info!("File already exists, skipping: {:?}", output_path);
            let canonical = output_path.canonicalize().unwrap_or_else(|_| output_path.clone());
            let selected_audio: Vec<String> = selection
                .audio
                .iter()
                .filter_map(|a| a.lang.clone())
                .collect();
            return Ok(DownloadResult {
                output_uri: format!("file://{}", canonical.display()),
                title: episode.title,
                quality: video_quality,
                audio_language: selected_audio.first().cloned().unwrap_or_default(),
                audio_languages: selected_audio,
                subtitle_languages: selection.subtitles.iter().map(|s| s.lang.clone()).collect(),
            });
        }

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                Error::Download(DownloadError::OutputDirNotFound(format!(
                    "Failed to create output directory: {}",
                    e
                )))
            })?;
        }

        // Create temp directory for this download (using episode_id for resume support)
        let work_dir = temp_dir.join(&episode.id);
        fs::create_dir_all(&work_dir).await?;
        debug!("Working directory: {:?}", work_dir);

        // Load or create download cache
        let cache_path = work_dir.join("cache.json");
        let mut cache = if options.resume_cache {
            match DownloadCache::load(&cache_path).await? {
                Some(mut existing_cache) => {
                    if existing_cache.is_manifest_valid(&manifest_content) {
                        let summary = existing_cache.get_resume_summary();
                        info!("Resuming download from cache ({})", summary);
                    } else {
                        // Manifest URLs contain volatile CDN tokens that change on
                        // every fetch, but the underlying segment content is the same.
                        // Keep verified segments and just update the manifest hash.
                        let summary = existing_cache.get_resume_summary();
                        info!(
                            "Manifest URLs changed (token rotation), resuming with existing segments ({})",
                            summary
                        );
                        existing_cache.update_manifest_hash(&manifest_content);
                    }
                    existing_cache
                }
                None => {
                    debug!("No existing cache, starting fresh download");
                    DownloadCache::new(&episode.id, &manifest_content)
                }
            }
        } else {
            debug!("Resume cache disabled, starting fresh download");
            DownloadCache::new(&episode.id, &manifest_content)
        };

        // Save initial cache state
        cache.save(&cache_path).await?;

        // 8. Get DRM keys if content is encrypted (use cache if available)
        self.reporter
            .on_phase_change("drm_keys", "Acquiring DRM keys");
        let keys = if let Some(ref pssh) = selection.pssh {
            // Check if we have cached keys
            if cache.has_drm_keys() {
                debug!("Using cached DRM keys");
                cache
                    .get_drm_keys()
                    .into_iter()
                    .map(|(kid, key)| DecryptionKey { kid, key })
                    .collect()
            } else {
                debug!("Content is encrypted, fetching keys...");
                let license_url = self.get_license_url(&stream_data)?;

                // Get bearer token (access token) from client
                let bearer_token = self.client.get_access_token().await.ok_or_else(|| {
                    Error::Config(ConfigError::Invalid(
                        "No access token available. Please login first.".to_string(),
                    ))
                })?;

                // Get video token from playback response
                let video_token = stream_data.token.as_ref().ok_or_else(|| {
                    Error::Media(MediaError::DecryptionFailed(
                        "No video token in playback response".to_string(),
                    ))
                })?;

                // Activate the video token before requesting license keys
                // This is required by Crunchyroll's DRM system
                debug!("Activating video token for episode: {}", episode.id);
                self.client
                    .activate_token(&episode.id, video_token)
                    .await?;
                token_tracker.track(episode.id.clone(), video_token.to_string());

                // Content ID is the media_id from stream_data
                let content_id = if !stream_data.media_id.is_empty() {
                    &stream_data.media_id
                } else {
                    &episode.id
                };

                let fetched_keys = self
                    .get_decryption_keys(
                        widevine_client.as_ref(),
                        widevine_private_key.as_ref(),
                        &license_url,
                        pssh,
                        &bearer_token,
                        video_token,
                        content_id,
                        "Video & Audio",
                    )
                    .await?;

                // Cache the keys
                cache.set_drm_keys(
                    &fetched_keys
                        .iter()
                        .map(|k| (k.kid.clone(), k.key.clone()))
                        .collect::<Vec<_>>(),
                );
                cache.save(&cache_path).await?;

                fetched_keys
            }
        } else {
            debug!("Content is not encrypted");
            Vec::new()
        };

        // Store all audio track info, keyed by locale for ordering later
        let mut audio_tracks_by_locale: HashMap<String, AudioTrackInfo> = HashMap::new();

        // Primary audio from main manifest
        if let Some(primary_audio) = selection.audio.first().cloned() {
            audio_tracks_by_locale.insert(
                stream_data.audio_locale.clone(),
                AudioTrackInfo {
                    stream: primary_audio,
                    keys: keys.clone(),
                    subtitles: Vec::new(), // Primary subs handled separately (full subs)
                },
            );
        }

        // Fetch additional versions for requested audio languages (PARALLEL)
        // Phase 1: Parallel fetch of playback data, manifests, and stream selection
        let additional_locales: Vec<_> = audio_locales_ordered
            .iter()
            .filter(|locale| *locale != &stream_data.audio_locale)
            .filter_map(|locale| {
                version_map.get(locale.as_str()).map(|v| (locale.clone(), *v))
            })
            .collect();

        let fetch_futures: Vec<_> = additional_locales
            .into_iter()
            .map(|(locale, version)| {
                let client = self.client.clone();
                let locale = locale.clone();
                let media_guid = extract_media_guid(&version.guid).to_string();
                let subtitle_languages = subtitle_languages.clone();
                let include_cc = options.include_cc;

                async move {
                    debug!(
                        "Fetching playback for audio version: {} ({})",
                        media_guid, locale
                    );

                    // 1. Get playback data for this version
                    let version_stream_data = match client.get_playback(&media_guid).await {
                        Ok(data) => data,
                        Err(e) => {
                            debug!("Failed to get playback for {}: {}, skipping", locale, e);
                            return None;
                        }
                    };

                    // 2. Get and parse manifest
                    let version_manifest_url = match DownloadManager::get_manifest_url_static(&version_stream_data) {
                        Ok(url) => url,
                        Err(e) => {
                            debug!("Failed to get manifest URL for {}: {}, skipping", locale, e);
                            return None;
                        }
                    };
                    let version_manifest_content = match client.get_manifest(&version_manifest_url).await {
                        Ok(content) => content,
                        Err(e) => {
                            debug!("Failed to fetch manifest for {}: {}, skipping", locale, e);
                            return None;
                        }
                    };
                    let version_base_url = version_manifest_url
                        .rsplit_once('/')
                        .map(|(base, _)| format!("{}/", base))
                        .unwrap_or_default();
                    let version_manifest = match MpdManifest::parse(&version_manifest_content, &version_base_url) {
                        Ok(m) => m,
                        Err(e) => {
                            debug!("Failed to parse manifest for {}: {}, skipping", locale, e);
                            return None;
                        }
                    };

                    // 3. Select audio stream from this manifest
                    let version_selection = match StreamSelector::select(
                        &version_manifest,
                        None, // No video quality needed
                        &[locale.clone()],
                        &[], // Handle subs separately
                        None,
                        None,
                        true, // skip_subs from manifest
                    ) {
                        Ok(s) => s,
                        Err(e) => {
                            debug!("Failed to select streams for {}: {}, skipping", locale, e);
                            return None;
                        }
                    };

                    let audio_stream = match version_selection.audio.into_iter().next() {
                        Some(s) => s,
                        None => {
                            debug!("No audio stream found in manifest for {}", locale);
                            return None;
                        }
                    };

                    // Verify the picked stream actually carries the locale we asked
                    // for. `StreamSelector::select_audio` has a "fall back to the
                    // first adaptation set" branch for the primary-manifest case;
                    // we don't want it here. If a version-specific manifest doesn't
                    // contain the requested language (CR sometimes returns a JA-only
                    // manifest under what's supposed to be an en-US version GUID),
                    // skip the version rather than embed a mislabeled audio track
                    // (e.g. JA audio tagged as `en` — observed on Jujutsu Kaisen
                    // S01E05).
                    let stream_lang = audio_stream.lang.clone().unwrap_or_default();
                    let lang_matches = stream_lang.eq_ignore_ascii_case(&locale)
                        || stream_lang
                            .to_lowercase()
                            .starts_with(&locale.to_lowercase())
                        || locale
                            .to_lowercase()
                            .starts_with(&stream_lang.to_lowercase());
                    if !lang_matches {
                        warn!(
                            "Skipping audio version {}: manifest fallback returned {:?}. \
                             CR's version-specific manifest didn't contain the requested \
                             language; embedding it would mislabel the audio.",
                            locale, audio_stream.lang
                        );
                        return None;
                    }

                    // 4. Collect CC and Signs & Songs subtitles from this version
                    let mut version_subs: Vec<SubtitleTrack> = Vec::new();

                    // Helper to check if locale matches user's subtitle preferences
                    let locale_matches_prefs = |loc: &str| -> bool {
                        subtitle_languages.iter().any(|l| {
                            l == loc
                                || loc.starts_with(l.as_str())
                                || l.starts_with(loc)
                        })
                    };

                    // Signs & Songs subtitles (from subtitles HashMap, marked as signs)
                    for (_, sub) in &version_stream_data.subtitles {
                        if sub.url.is_empty() {
                            continue;
                        }
                        if !locale_matches_prefs(&sub.locale) {
                            continue;
                        }
                        version_subs.push(SubtitleTrack {
                            lang: sub.locale.clone(),
                            label: None,
                            url: sub.url.clone(),
                            format: sub.format.clone(),
                            is_cc: false,
                            is_signs: true,
                        });
                    }

                    // CC subtitles (from closed_captions HashMap)
                    if include_cc {
                        for (_, sub) in &version_stream_data.closed_captions {
                            if sub.url.is_empty() {
                                continue;
                            }
                            if !locale_matches_prefs(&sub.locale) {
                                continue;
                            }
                            version_subs.push(SubtitleTrack {
                                lang: sub.locale.clone(),
                                label: None,
                                url: sub.url.clone(),
                                format: sub.format.clone(),
                                is_cc: true,
                                is_signs: false,
                            });
                        }
                    }

                    Some(AudioVersionFetchResult {
                        locale,
                        media_guid,
                        stream_data: version_stream_data,
                        audio_stream,
                        pssh: version_selection.pssh,
                        subtitles: version_subs,
                    })
                }
            })
            .collect();

        // Execute Phase 1 in parallel
        let fetch_results: Vec<Option<AudioVersionFetchResult>> =
            futures::future::join_all(fetch_futures).await;
        let fetched_versions: Vec<AudioVersionFetchResult> =
            fetch_results.into_iter().flatten().collect();

        // Phase 2: Parallel key acquisition for tracks that need it
        let tracks_needing_keys: Vec<&AudioVersionFetchResult> = fetched_versions
            .iter()
            .filter(|v| v.pssh.is_some())
            .collect();

        let acquired_keys: HashMap<String, Vec<DecryptionKey>> = if !tracks_needing_keys.is_empty() {
            // Show single spinner for all key acquisitions
            let spinner = self.progress.add_spinner(&format!(
                "Acquiring decryption keys for {} audio track(s)...",
                tracks_needing_keys.len()
            ));

            // Create semaphore for concurrency control (0 = unlimited)
            let semaphore = if max_concurrent_keys > 0 {
                Some(Arc::new(Semaphore::new(max_concurrent_keys)))
            } else {
                None
            };

            let key_futures: Vec<_> = tracks_needing_keys
                .iter()
                .map(|data| {
                    let client = self.client.clone();
                    let widevine_client = widevine_client.clone();
                    let widevine_private_key = widevine_private_key.clone();
                    let http_client = self.http_client.clone();
                    let locale = data.locale.clone();
                    let media_guid = data.media_guid.clone();
                    let pssh = data.pssh.clone().unwrap(); // Safe: filtered above
                    let stream_data_ref = &data.stream_data;
                    let license_url = DownloadManager::get_license_url_static(stream_data_ref)
                        .unwrap_or_else(|_| "https://www.crunchyroll.com/license/v1/license/widevine".to_string());
                    let video_token = stream_data_ref.token.clone();
                    let content_id = if !stream_data_ref.media_id.is_empty() {
                        stream_data_ref.media_id.clone()
                    } else {
                        media_guid.clone()
                    };
                    let semaphore = semaphore.clone();
                    let token_tracker = token_tracker.clone();

                    async move {
                        // Acquire semaphore permit if concurrency is limited
                        let _permit = if let Some(ref sem) = semaphore {
                            Some(sem.acquire().await.ok()?)
                        } else {
                            None
                        };

                        // Activate token for this version
                        if let Some(ref token) = video_token {
                            if let Err(e) = client.activate_token(&media_guid, token).await {
                                debug!("Failed to activate token for {}: {}, skipping", locale, e);
                                return None;
                            }
                            token_tracker.track(media_guid.clone(), token.clone());
                        }

                        let bearer_token = match client.get_access_token().await {
                            Some(t) => t,
                            None => {
                                debug!("No access token for {}, skipping", locale);
                                return None;
                            }
                        };

                        let video_token = match video_token {
                            Some(ref t) => t.clone(),
                            None => {
                                debug!("No video token for {}, skipping", locale);
                                return None;
                            }
                        };

                        // Acquire keys in-process
                        let wv_client = match widevine_client.as_ref() {
                            Some(p) => p.as_path(),
                            None => {
                                debug!("No widevine_client configured for {}, skipping", locale);
                                return None;
                            }
                        };
                        let wv_key = match widevine_private_key.as_ref() {
                            Some(p) => p.as_path(),
                            None => {
                                debug!("No widevine_private_key configured for {}, skipping", locale);
                                return None;
                            }
                        };

                        let result = acquire_keys(
                            &http_client,
                            wv_client,
                            wv_key,
                            &license_url,
                            &pssh,
                            &bearer_token,
                            &video_token,
                            &content_id,
                        )
                        .await;

                        match result {
                            Ok(k) => {
                                debug!("Acquired {} key(s) for {}", k.len(), locale);
                                Some((locale, k))
                            }
                            Err(e) => {
                                debug!("Failed to get keys for {}: {}, skipping", locale, e);
                                None
                            }
                        }
                    }
                })
                .collect();

            // Execute Phase 2 in parallel
            let key_results: Vec<Option<(String, Vec<DecryptionKey>)>> =
                futures::future::join_all(key_futures).await;

            let successful_keys: usize = key_results.iter().filter(|r| r.is_some()).count();
            spinner.finish_with_message(format!(
                "Acquired decryption keys for {} audio track(s)",
                successful_keys
            ));

            key_results.into_iter().flatten().collect()
        } else {
            HashMap::new()
        };

        // Merge fetched data with keys into audio_tracks_by_locale
        for version_data in fetched_versions {
            let version_keys = if version_data.pssh.is_some() {
                acquired_keys
                    .get(&version_data.locale)
                    .cloned()
                    .unwrap_or_else(|| keys.clone())
            } else {
                keys.clone() // Use primary keys if no separate PSSH
            };

            // Skip if we needed keys but didn't get them
            if version_data.pssh.is_some() && !acquired_keys.contains_key(&version_data.locale) {
                debug!("Skipping {} - failed to acquire keys", version_data.locale);
                continue;
            }

            info!(
                "Added audio version: {} ({} subtitles)",
                version_data.locale,
                version_data.subtitles.len()
            );

            audio_tracks_by_locale.insert(
                version_data.locale.clone(),
                AudioTrackInfo {
                    stream: version_data.audio_stream,
                    keys: version_keys,
                    subtitles: version_data.subtitles,
                },
            );
        }

        // 9. Download and process streams
        self.reporter
            .on_phase_change("downloading", "Downloading streams");

        // Calculate total steps for progress display
        let mut total_steps = 0;
        if options.download_mode.download_video() {
            total_steps += 1;
        }
        if options.download_mode.download_audio() {
            total_steps += audio_tracks_by_locale.len();
        }
        if options.download_mode.download_subs() && embed_subs {
            total_steps += 1;
        }
        total_steps += 1; // muxing
        let mut current_step = 0;

        let segment_downloader =
            SegmentDownloader::new(self.http_client.clone(), parts, max_speed_kbps);

        // Wrap cache in Arc<Mutex<>> for shared access
        let cache = Arc::new(Mutex::new(cache));

        // Update phase to segments
        {
            let mut cache_guard = cache.lock().await;
            cache_guard.phase = DownloadPhase::Segments;
            cache_guard.save(&cache_path).await?;
        }

        // Download video with caching (skip if only-subs or only-audio mode)
        let video_path = if options.download_mode.download_video() {
            current_step += 1;
            let path = self
                .download_stream_cached(
                    &segment_downloader,
                    &selection.video,
                    "video",
                    None,
                    &work_dir,
                    &format!("[{}/{}] Video", current_step, total_steps),
                    cache.clone(),
                    cache_path.clone(),
                    &keys,
                    &mp4decrypt_path,
                    Some((current_step, total_steps, "Video")),
                )
                .await?;
            Some(path)
        } else {
            debug!("Skipping video download (mode: {:?})", options.download_mode);
            None
        };

        // Download audio tracks in config-specified order with caching (skip if only-subs or only-video mode)
        let audio_paths: Vec<(PathBuf, String)> = if options.download_mode.download_audio() {
            let mut paths = Vec::new();
            for (i, locale) in audio_locales_ordered.iter().enumerate() {
                let track_info = match audio_tracks_by_locale.get(locale) {
                    Some(t) => t,
                    None => continue,
                };

                current_step += 1;
                let audio_label = format!("Audio ({})", locale);
                let stream_id = format!("audio_{}", i);
                let path = self
                    .download_stream_cached(
                        &segment_downloader,
                        &track_info.stream,
                        &stream_id,
                        Some(locale),
                        &work_dir,
                        &format!("[{}/{}] {}", current_step, total_steps, audio_label),
                        cache.clone(),
                        cache_path.clone(),
                        &track_info.keys,
                        &mp4decrypt_path,
                        Some((current_step, total_steps, &audio_label)),
                    )
                    .await?;

                paths.push((path, locale.clone()));
            }
            paths
        } else {
            debug!("Skipping audio download (mode: {:?})", options.download_mode);
            Vec::new()
        };

        // Update phase to subtitles
        {
            let mut cache_guard = cache.lock().await;
            cache_guard.phase = DownloadPhase::Subtitles;
            cache_guard.save(&cache_path).await?;
        }

        // 10. Download and convert subtitles (skip if only-audio or only-video mode)
        self.reporter
            .on_phase_change("subtitles", "Downloading subtitles");
        let subtitle_paths = if options.download_mode.download_subs() && embed_subs {
            // Collect all subtitles: primary (full subs) + CC/Signs from additional versions
            let mut all_subtitle_tracks: Vec<SubtitleTrack> = selection.subtitles.clone();

            // Add CC and Signs subtitles from additional audio versions
            for locale in &audio_locales_ordered {
                if locale == &stream_data.audio_locale {
                    continue; // Primary subs already in selection.subtitles
                }
                if let Some(track_info) = audio_tracks_by_locale.get(locale) {
                    all_subtitle_tracks.extend(track_info.subtitles.clone());
                }
            }

            // Download and deduplicate subtitles
            current_step += 1;
            let step_prefix = format!("[{}/{}]", current_step, total_steps);
            let all_subs = self
                .download_subtitles(
                    &all_subtitle_tracks,
                    &work_dir,
                    &step_prefix,
                    Some((current_step, total_steps)),
                )
                .await?;

            // Deduplicate by (locale, is_cc, is_signs) - keep last
            let mut seen: HashMap<(String, bool, bool), (PathBuf, SubtitleTrack)> = HashMap::new();
            for (path, track) in all_subs {
                seen.insert((track.lang.clone(), track.is_cc, track.is_signs), (path, track));
            }
            let mut deduped: Vec<(PathBuf, SubtitleTrack)> = seen.into_values().collect();

            // Sort subtitle tracks to match the user's configured language order.
            // Within the same language: full subs first, then Signs & Songs, then CC.
            deduped.sort_by(|(_, a), (_, b)| {
                let lang_pos = |lang: &str| -> usize {
                    subtitle_languages
                        .iter()
                        .position(|l| l.eq_ignore_ascii_case(lang)
                            || l.to_lowercase().starts_with(&lang.to_lowercase())
                            || lang.to_lowercase().starts_with(&l.to_lowercase()))
                        .unwrap_or(usize::MAX)
                };
                let pos_a = lang_pos(&a.lang);
                let pos_b = lang_pos(&b.lang);
                pos_a.cmp(&pos_b)
                    .then(a.is_signs.cmp(&b.is_signs))   // full subs (false) before signs (true)
                    .then(a.is_cc.cmp(&b.is_cc))         // regular (false) before CC (true)
            });
            deduped
        } else {
            if !options.download_mode.download_subs() {
                debug!("Skipping subtitle download (mode: {:?})", options.download_mode);
            }
            Vec::new()
        };

        // Update phase to muxing
        {
            let mut cache_guard = cache.lock().await;
            cache_guard.phase = DownloadPhase::Muxing;
            cache_guard.save(&cache_path).await?;
        }

        // 11. Mux everything together
        self.reporter.on_phase_change("muxing", "Muxing final output");
        current_step += 1;
        let mux_prefix = format!("[{}/{}]", current_step, total_steps);
        self.mux_streams(
            &ffmpeg_path,
            video_path.as_ref(),
            &audio_paths,
            &subtitle_paths,
            &output_path,
            &format,
            &episode,
            &default_audio,
            &default_sub,
            prefer_signs_songs,
            &mux_prefix,
            Some((current_step, total_steps)),
        )
        .await?;

        // Mark as complete
        {
            let mut cache_guard = cache.lock().await;
            cache_guard.phase = DownloadPhase::Complete;
            cache_guard.save(&cache_path).await?;
        }

        // NOTE: Don't clean up `work_dir` here — the publish step below can
        // still fail (e.g. EXDEV / EPERM on cross-FS NAS mounts). If we wiped
        // the cache before publish, a retry would have to redownload every
        // segment instead of reusing the verified ones. Cleanup happens only
        // after the publish succeeds.

        let total_elapsed = download_start.elapsed();
        info!(
            "Download complete: {:?} (total time: {})",
            output_path,
            format_elapsed(total_elapsed)
        );

        // Log file size if possible
        if let Ok(metadata) = std::fs::metadata(&output_path) {
            trace!(
                "Output file size: {}, avg speed: {}/s",
                format_bytes(metadata.len()),
                format_bytes((metadata.len() as f64 / total_elapsed.as_secs_f64()) as u64)
            );
        }

        // 13. Publish via the configured OutputSink (defaults to LocalFsSink
        //     rooted at output_dir, in which case this is a no-op rename).
        let target = crate::storage::OutputTarget::from_relative_path(
            output_path
                .strip_prefix(&output_dir)
                .unwrap_or(output_path.as_path()),
        )
        .ok_or_else(|| {
            Error::Download(DownloadError::OutputDirNotFound(format!(
                "Cannot derive output target from {:?}",
                output_path
            )))
        })?;

        let sink: Arc<dyn crate::storage::OutputSink> = match self.sink.clone() {
            Some(s) => s,
            None => Arc::new(crate::storage::LocalFsSink::new(output_dir.clone())),
        };
        let output_uri = sink.publish(&output_path, &target).await?;

        // Cleanup temp directory only after both mux AND publish succeeded.
        // Failures above this line leave `work_dir` intact so the next run
        // can resume from the cache.
        fs::remove_dir_all(&work_dir).await.ok();

        let result = DownloadResult {
            output_uri,
            title: episode.title,
            quality: format!("{}p", selection.video.height.unwrap_or(0)),
            audio_language: audio_locales_ordered
                .first()
                .cloned()
                .unwrap_or_default(),
            audio_languages: audio_locales_ordered.clone(),
            subtitle_languages: subtitle_paths.iter().map(|(_, t)| t.lang.clone()).collect(),
        };
        self.reporter.on_complete(&result);
        Ok(result)
    }

    /// Get the manifest URL from stream data (prefers DRM DASH).
    fn get_manifest_url(&self, stream_data: &CRStreamData) -> Result<String> {
        Self::get_manifest_url_static(stream_data)
    }

    /// Get the Widevine license URL from stream data.
    fn get_license_url(&self, stream_data: &CRStreamData) -> Result<String> {
        Self::get_license_url_static(stream_data)
    }

    /// Get the manifest URL from stream data (static version for async contexts).
    fn get_manifest_url_static(stream_data: &CRStreamData) -> Result<String> {
        // Prefer DRM DASH manifest
        if let Some(url) = stream_data.urls.drm_dash.values().next() {
            return Ok(url.clone());
        }

        // Fall back to regular DASH
        if !stream_data.urls.dash.is_empty() {
            return Ok(stream_data.urls.dash.clone());
        }

        // Fall back to generic URL
        if !stream_data.urls.url.is_empty() {
            return Ok(stream_data.urls.url.clone());
        }

        Err(Error::Download(DownloadError::NoStreams))
    }

    /// Get the Widevine license URL from stream data (static version for async contexts).
    fn get_license_url_static(stream_data: &CRStreamData) -> Result<String> {
        // The license URL is typically in the drm_dash keys
        // Crunchyroll uses "license_url" or similar key
        for (key, value) in &stream_data.urls.drm_dash {
            if key.contains("license") || key == "license_url" {
                return Ok(value.clone());
            }
        }

        // Default Crunchyroll license URL
        Ok("https://www.crunchyroll.com/license/v1/license/widevine".to_string())
    }

    /// Get decryption keys using in-process Widevine CDM.
    ///
    /// The `label` parameter describes what the keys are for (e.g., "Video & Audio", "Audio (ja-JP)").
    async fn get_decryption_keys(
        &self,
        client_id_path: Option<&PathBuf>,
        private_key_path: Option<&PathBuf>,
        license_url: &str,
        pssh: &str,
        bearer_token: &str,
        video_token: &str,
        content_id: &str,
        label: &str,
    ) -> Result<Vec<DecryptionKey>> {
        let client_id = client_id_path.ok_or_else(|| {
            Error::Config(ConfigError::Invalid(
                "DRM-protected content requires Widevine credentials.\n\
                 Please set 'tools.widevine_client' in your config to the path of your client_id.bin file.\n\
                 You can edit your config with: crunchy-cli config edit".to_string(),
            ))
        })?;

        let private_key = private_key_path.ok_or_else(|| {
            Error::Config(ConfigError::Invalid(
                "DRM-protected content requires Widevine credentials.\n\
                 Please set 'tools.widevine_private_key' in your config to the path of your private_key.pem file.\n\
                 You can edit your config with: crunchy-cli config edit".to_string(),
            ))
        })?;

        let spinner = self
            .progress
            .add_spinner(&format!("Acquiring decryption keys for {}...", label));

        let keys = acquire_keys(
            &self.http_client,
            client_id,
            private_key,
            license_url,
            pssh,
            bearer_token,
            video_token,
            content_id,
        )
        .await?;

        spinner.finish_with_message(format!(
            "Acquired {} decryption key(s) for {}",
            keys.len(),
            label
        ));

        debug!("Got {} decryption keys for {}", keys.len(), label);
        for key in &keys {
            trace!("Key: kid={} key={}", redact(&key.kid), redact(&key.key));
        }
        Ok(keys)
    }

    /// Download a stream with caching support for resume.
    ///
    /// Returns the path to the downloaded (and optionally decrypted) stream.
    async fn download_stream_cached(
        &self,
        downloader: &SegmentDownloader,
        stream: &SelectedStream,
        stream_id: &str,
        locale: Option<&str>,
        work_dir: &Path,
        label: &str,
        cache: Arc<Mutex<DownloadCache>>,
        cache_path: PathBuf,
        keys: &[DecryptionKey],
        mp4decrypt_path: &str,
        step_context: Option<(usize, usize, &str)>,
    ) -> Result<PathBuf> {
        // Build list of all segment URLs (init + media segments)
        let mut all_segments = Vec::new();
        if let Some(ref init_url) = stream.init_url {
            all_segments.push(init_url.clone());
        }
        all_segments.extend(stream.segment_urls.clone());

        let segment_count = all_segments.len() as u64;
        let pb = self.progress.add_download(segment_count, label);

        // Report step start to external reporter
        if let Some((current, total, step_label)) = step_context {
            self.reporter.on_step_progress(&StepProgress {
                current_step: current,
                total_steps: total,
                label: step_label.to_string(),
                completed: 0,
                total: segment_count,
                speed_bps: 0,
                eta_secs: None,
            });
        }

        // Initialize or get stream cache
        let stream_cache = {
            let mut cache_guard = cache.lock().await;

            // Check if we already have a stream cache for this stream
            let existing = if stream_id == "video" {
                cache_guard.video.clone()
            } else {
                cache_guard.get_audio(locale.unwrap_or("")).cloned()
            };

            match existing {
                Some(sc) if sc.total_segments == all_segments.len() => {
                    // Use existing cache
                    Arc::new(Mutex::new(sc))
                }
                _ => {
                    // Create new stream cache
                    let sc = StreamCache::new(stream_id, locale, &all_segments);
                    if stream_id == "video" {
                        cache_guard.video = Some(sc.clone());
                    } else if let Some(loc) = locale {
                        cache_guard.init_audio(stream_id, loc, &all_segments);
                    }
                    Arc::new(Mutex::new(sc))
                }
            }
        };

        // Create save callback
        let cache_for_save = cache.clone();
        let cache_path_for_save = cache_path.clone();
        let stream_cache_for_save = stream_cache.clone();
        let stream_id_owned = stream_id.to_string();
        let locale_owned = locale.map(String::from);

        let save_cache = move || {
            let cache = cache_for_save.clone();
            let cache_path = cache_path_for_save.clone();
            let stream_cache = stream_cache_for_save.clone();
            let stream_id = stream_id_owned.clone();
            let locale = locale_owned.clone();

            Box::pin(async move {
                let sc = stream_cache.lock().await.clone();
                let mut cache_guard = cache.lock().await;

                // Update the stream cache in the main cache
                if stream_id == "video" {
                    cache_guard.video = Some(sc);
                } else if let Some(ref loc) = locale {
                    if let Some(audio_cache) = cache_guard.get_audio_mut(loc) {
                        *audio_cache = sc;
                    }
                }

                cache_guard.save(&cache_path).await
            }) as futures::future::BoxFuture<'static, Result<()>>
        };

        // Skip everything if already fully processed
        {
            let sc = stream_cache.lock().await;
            if !keys.is_empty() {
                // DRM: check for decrypted output
                let decrypted_path = work_dir.join(format!("{}_decrypted.mp4", stream_id));
                if sc.decrypted && decrypted_path.exists() {
                    debug!("{} already decrypted, skipping download", label);
                    pb.finish_with_message(format!("{} complete (cached)", label));
                    return Ok(decrypted_path);
                }
            } else {
                // Non-DRM: check for concatenated output
                let concat_path = work_dir.join(format!("{}.mp4", stream_id));
                if sc.concatenated && concat_path.exists() {
                    debug!("{} already concatenated, skipping download", label);
                    pb.finish_with_message(format!("{} complete (cached)", label));
                    return Ok(concat_path);
                }
            }
        }

        debug!(
            "Downloading {} segments for {} to {:?} (cached)",
            segment_count, label, work_dir
        );

        // Spawn a background task that periodically reports progress via the reporter
        let progress_report_handle = if let Some((current, total, step_label)) = step_context {
            let reporter = self.reporter.clone();
            let pb_clone = pb.clone();
            let step_label = step_label.to_string();
            let total_segs = segment_count;
            let bandwidth = stream.bandwidth; // bits per second from manifest
            Some(tokio::spawn(async move {
                let start = std::time::Instant::now();
                let mut prev_pos: u64 = 0;
                let mut prev_time = start;
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let pos = pb_clone.position();
                    let now = std::time::Instant::now();

                    // Calculate speed: segments completed in interval × estimated bytes per segment
                    // bandwidth is bits/s from manifest; bytes per segment ≈ bandwidth * segment_duration / 8
                    // Typical segment duration is ~2-6s; use segment rate instead
                    let elapsed_secs = now.duration_since(start).as_secs_f64();
                    let interval_secs = now.duration_since(prev_time).as_secs_f64();
                    let segments_in_interval = pos.saturating_sub(prev_pos) as f64;

                    // Speed based on segment rate and manifest bandwidth
                    // Each segment ≈ (bandwidth / 8) * segment_duration
                    // segment_duration ≈ total_duration / total_segments (unknown)
                    // Simpler: use overall completion rate × total estimated size
                    let total_bytes_estimate = if total_segs > 0 && bandwidth > 0 {
                        // Assume ~4s per segment (common for DASH)
                        (bandwidth / 8) * 4 * total_segs
                    } else {
                        0
                    };

                    let speed_bps = if interval_secs > 0.1 && total_segs > 0 && total_bytes_estimate > 0 {
                        let bytes_per_segment = total_bytes_estimate / total_segs;
                        (segments_in_interval * bytes_per_segment as f64 / interval_secs) as u64
                    } else {
                        0
                    };

                    let eta_secs = if pos > 0 && pos < total_segs && elapsed_secs > 0.0 {
                        let rate = pos as f64 / elapsed_secs;
                        Some((total_segs - pos) as f64 / rate)
                    } else {
                        None
                    };

                    reporter.on_step_progress(&StepProgress {
                        current_step: current,
                        total_steps: total,
                        label: step_label.clone(),
                        completed: pos,
                        total: total_segs,
                        speed_bps,
                        eta_secs,
                    });

                    prev_pos = pos;
                    prev_time = now;

                    if pos >= total_segs {
                        break;
                    }
                }
            }))
        } else {
            None
        };

        // Download with caching
        let encrypted_path = downloader
            .download_segments_cached(
                &all_segments,
                work_dir,
                stream_id,
                stream_cache.clone(),
                Some(&pb),
                save_cache,
            )
            .await?;

        // Stop the progress reporting task
        if let Some(handle) = progress_report_handle {
            handle.abort();
        }

        pb.finish_with_message(format!("{} complete", label));

        // Report step completion to external reporter
        if let Some((current, total, step_label)) = step_context {
            self.reporter.on_step_progress(&StepProgress {
                current_step: current,
                total_steps: total,
                label: step_label.to_string(),
                completed: segment_count,
                total: segment_count,
                speed_bps: 0,
                eta_secs: Some(0.0),
            });
        }

        // Decrypt if needed
        let final_path = if !keys.is_empty() {
            let decrypted_path = work_dir.join(format!("{}_decrypted.mp4", stream_id));

            // Check if already decrypted
            let already_decrypted = {
                let sc = stream_cache.lock().await;
                sc.decrypted && decrypted_path.exists()
            };

            if already_decrypted {
                debug!("{} already decrypted, skipping", label);
                decrypted_path
            } else {
                self.decrypt_stream(mp4decrypt_path, &encrypted_path, &decrypted_path, keys)
                    .await?;

                // Mark as decrypted
                {
                    let mut sc = stream_cache.lock().await;
                    sc.decrypted = true;
                    sc.output_path = Some(decrypted_path.clone());
                }

                // Save cache
                {
                    let sc = stream_cache.lock().await.clone();
                    let mut cache_guard = cache.lock().await;
                    if stream_id == "video" {
                        cache_guard.video = Some(sc);
                    } else if let Some(loc) = locale {
                        if let Some(audio_cache) = cache_guard.get_audio_mut(loc) {
                            *audio_cache = stream_cache.lock().await.clone();
                        }
                    }
                    cache_guard.save(&cache_path).await?;
                }

                decrypted_path
            }
        } else {
            encrypted_path
        };

        Ok(final_path)
    }

    /// Decrypt an encrypted stream using mp4decrypt.
    async fn decrypt_stream(
        &self,
        mp4decrypt_path: &str,
        input_path: &Path,
        output_path: &Path,
        keys: &[DecryptionKey],
    ) -> Result<()> {
        debug!(
            "Decrypting {:?} -> {:?} with {} keys",
            input_path,
            output_path,
            keys.len()
        );

        let builder = Mp4DecryptBuilder::new(
            mp4decrypt_path,
            input_path.to_str().unwrap_or_default(),
            output_path.to_str().unwrap_or_default(),
        )
        .keys(keys.to_vec());

        builder.execute().await?;

        Ok(())
    }

    /// Download and convert subtitles.
    ///
    /// Returns a vector of (path, SubtitleTrack) tuples, preserving all track
    /// metadata for use in muxing (language, is_cc, is_signs).
    ///
    /// This function is resilient - it will skip subtitles that fail to download
    /// and warn the user instead of aborting the entire download.
    async fn download_subtitles(
        &self,
        subtitles: &[SubtitleTrack],
        work_dir: &Path,
        step_prefix: &str,
        step_context: Option<(usize, usize)>,
    ) -> Result<Vec<(PathBuf, SubtitleTrack)>> {
        let mut results = Vec::new();

        // Early return if no subtitles
        if subtitles.is_empty() {
            return Ok(results);
        }

        let total_subs = subtitles.len() as u64;

        // Create a single progress bar for all subtitles
        let pb = self
            .progress
            .add_download(total_subs, &format!("{} Subtitles", step_prefix));

        // Report step start
        if let Some((current, total)) = step_context {
            self.reporter.on_step_progress(&StepProgress {
                current_step: current,
                total_steps: total,
                label: "Subtitles".to_string(),
                completed: 0,
                total: total_subs,
                speed_bps: 0,
                eta_secs: None,
            });
        }
        let mut completed_labels: Vec<String> = Vec::new();

        for (i, sub) in subtitles.iter().enumerate() {
            // Build label for progress display (e.g., "(en-US)", "(en-US, CC)")
            let label = if sub.is_cc {
                format!("({}, CC)", sub.lang)
            } else if sub.is_signs {
                format!("({}, Signs)", sub.lang)
            } else {
                format!("({})", sub.lang)
            };

            // Update progress bar message with current subtitle
            pb.set_message(format!("Subtitle {}", label));

            // Skip empty URLs
            if sub.url.is_empty() {
                warn!("Skipping subtitle {} - empty URL", sub.lang);
                pb.inc(1);
                continue;
            }

            let ass_path = work_dir.join(format!("sub_{}.ass", i));

            debug!(
                "Downloading subtitle: {} from '{}' (cc={}, signs={})",
                sub.lang, sub.url, sub.is_cc, sub.is_signs
            );

            // Download VTT file
            let response = match self.http_client.get(&sub.url).send().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Skipping subtitle {} - download failed: {}", sub.lang, e);
                    pb.inc(1);
                    continue;
                }
            };

            let vtt_content = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("Skipping subtitle {} - read failed: {}", sub.lang, e);
                    pb.inc(1);
                    continue;
                }
            };

            // Convert to ASS (handles both VTT and already-ASS content)
            let ass_content = match SubtitleConverter::to_ass(&vtt_content) {
                Ok(c) => c,
                Err(e) => {
                    warn!("Skipping subtitle {} - conversion failed: {}", sub.lang, e);
                    pb.inc(1);
                    continue;
                }
            };

            // Write ASS file
            if let Err(e) = fs::write(&ass_path, ass_content).await {
                warn!("Skipping subtitle {} - write failed: {}", sub.lang, e);
                pb.inc(1);
                continue;
            }

            // Success - track this label and increment progress
            completed_labels.push(label);
            pb.inc(1);

            // Clone the track with updated format (now ASS)
            let mut track = sub.clone();
            track.format = "ass".to_string();
            results.push((ass_path, track));
        }

        // Build final message with all completed labels
        let final_message = if completed_labels.is_empty() {
            format!("{} Subtitles (none downloaded)", step_prefix)
        } else {
            format!("{} Subtitles {} complete", step_prefix, completed_labels.join(" "))
        };
        pb.finish_with_message(final_message);

        // Report step completion
        if let Some((current, total)) = step_context {
            self.reporter.on_step_progress(&StepProgress {
                current_step: current,
                total_steps: total,
                label: "Subtitles".to_string(),
                completed: total_subs,
                total: total_subs,
                speed_bps: 0,
                eta_secs: Some(0.0),
            });
        }

        debug!("Downloaded {} subtitle tracks", results.len());
        Ok(results)
    }

    /// Mux video, audio, and subtitles into final output.
    ///
    /// Sets proper track metadata including:
    /// - Language codes (ISO 639-2/3 for FFmpeg)
    /// - Track titles (human-readable names with suffixes like "(CC)" or "(Signs & Songs)")
    /// - Disposition flags (default, forced, hearing_impaired)
    async fn mux_streams(
        &self,
        ffmpeg_path: &str,
        video_path: Option<&PathBuf>,
        audio_paths: &[(PathBuf, String)],
        subtitle_paths: &[(PathBuf, SubtitleTrack)],
        output_path: &Path,
        format: &str,
        episode: &CREpisode,
        default_audio_locale: &str,
        default_sub_locale: &str,
        prefer_signs_songs: bool,
        step_prefix: &str,
        step_context: Option<(usize, usize)>,
    ) -> Result<()> {
        let spinner = self
            .progress
            .add_spinner(&format!("{} Muxing final output...", step_prefix));

        // Report muxing step start
        if let Some((current, total)) = step_context {
            self.reporter.on_step_progress(&StepProgress {
                current_step: current,
                total_steps: total,
                label: "Muxing".to_string(),
                completed: 0,
                total: 1,
                speed_bps: 0,
                eta_secs: None,
            });
        }

        let mut builder = FfmpegBuilder::new(ffmpeg_path, output_path.to_str().unwrap_or_default())
            .format(format);

        let mut input_index = 0;
        let mut audio_stream_index = 0;
        let mut sub_stream_index = 0;

        // Add video input
        if let Some(video) = video_path {
            builder = builder.input(video.to_str().unwrap_or_default());
            builder = builder.map_all(input_index);
            input_index += 1;
        }

        // Set default disposition for audio to first track by default - we'll override it
        // for the preferred track below. This ensures that if the user doesn't specify a
        // default audio locale, the first track will still be marked as default.
        builder = builder.disposition(&"a", &["0"]);

        // Add audio inputs with metadata
        let mut default_audio_set = false;
        for (audio_path, cr_locale) in audio_paths {
            builder = builder.input(audio_path.to_str().unwrap_or_default());
            builder = builder.map_all(input_index);

            // Set language metadata
            if let Some(lang) = get_language(cr_locale) {
                builder = builder
                    .stream_metadata(
                        &format!("s:a:{}", audio_stream_index),
                        "language",
                        lang.code,
                    )
                    .stream_metadata(&format!("s:a:{}", audio_stream_index), "title", lang.name);
            }

            // Set default disposition for preferred audio track.
            let is_default = cr_locale.eq_ignore_ascii_case(default_audio_locale);
            if is_default && !default_audio_set {
                builder = builder.disposition(&format!("a:{}", audio_stream_index), &["default"]);
                default_audio_set = true;
            }

            input_index += 1;
            audio_stream_index += 1;
        }

        // Add subtitle inputs with metadata
        let mut default_sub_set = false;
        let mut forced_sub_set = false;
        for (sub_path, track) in subtitle_paths {
            builder = builder.input(sub_path.to_str().unwrap_or_default());
            builder = builder.map_all(input_index);

            // Default: prefer matching default_sub_locale, or first non-signs track.
            // When prefer_signs_songs is enabled prefer the Signs & Songs track instead
            // of the full sub.
            let want_signs = prefer_signs_songs;
            let is_default = track.lang.eq_ignore_ascii_case(default_sub_locale);
            let is_forced = is_default && track.is_signs;


            // Set language metadata
            if let Some(lang) = get_language(&track.lang) {
                builder = builder.stream_metadata(
                    &format!("s:s:{}", sub_stream_index),
                    "language",
                    lang.code,
                );

                // Build title with suffixes (matching reference implementation)
                let mut title = lang.name.to_string();
                if track.is_signs {
                    title.push_str(" (Signs & Songs)");
                }
                if track.is_cc {
                    title.push_str(" (CC)");
                }
                if !(is_default && !track.is_signs && !default_sub_set) && is_forced && want_signs && !forced_sub_set {
                    title.push_str(" [Forced]");
                }
                builder = builder.stream_metadata(
                    &format!("s:s:{}", sub_stream_index),
                    "title",
                    &title,
                );
            }

            // Set subtitle codec (ASS format after conversion)
            let codec = if track.format == "srt" { "srt" } else { "ass" };
            builder = builder.subtitle_codec(sub_stream_index, codec);

            // Set disposition flags
            let mut flags = Vec::new();

            if is_default && !track.is_signs && !default_sub_set {
                flags.push("default");
                default_sub_set = true;
            } else if is_forced && want_signs && !forced_sub_set {
                flags.push("forced");
                forced_sub_set = true;
            }

            // Closed captions are marked as hearing_impaired
            if track.is_cc {
                flags.push("hearing_impaired");
            }

            // If this track is both forced and default, FFmpeg will treat it as just default,
            // so we need to set both flags explicitly.
            if !flags.is_empty() {
                builder = builder.disposition(&format!("s:{}", sub_stream_index), &flags);
            }

            input_index += 1;
            sub_stream_index += 1;
        }

        // Add global metadata
        let season_num = if episode.season_sequence_number > 0 { episode.season_sequence_number } else { episode.season_number };
        builder = builder
            .metadata("title", &episode.title)
            .metadata("show", &episode.series_title)
            .metadata("season_number", &season_num.to_string())
            .metadata("episode_id", &episode.episode);

        if let Some(ep_num) = episode.episode_number {
            builder = builder.metadata("episode_sort", &(ep_num as i32).to_string());
        }

        builder.execute().await?;

        spinner.finish_with_message(format!("{} Muxing complete", step_prefix));

        // Report muxing step completion
        if let Some((current, total)) = step_context {
            self.reporter.on_step_progress(&StepProgress {
                current_step: current,
                total_steps: total,
                label: "Muxing".to_string(),
                completed: 1,
                total: 1,
                speed_bps: 0,
                eta_secs: Some(0.0),
            });
        }

        Ok(())
    }
}

/// Parse a quality string like "1080p", "720p", "best" into an optional height.
fn parse_quality(quality: &str) -> Option<u32> {
    let quality = quality.to_lowercase();
    if quality == "best" || quality.is_empty() {
        return None;
    }

    // Try to parse "1080p" -> 1080, "720" -> 720, etc.
    let num_str = quality.trim_end_matches('p');
    num_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_options_default() {
        let opts = DownloadOptions::default();
        assert!(opts.video_quality.is_none());
        assert!(opts.audio_languages.is_none());
        assert!(opts.subtitle_languages.is_none());
        assert!(!opts.skip_existing);
    }

    #[test]
    fn test_download_result() {
        let result = DownloadResult {
            output_uri: "file:///downloads/test.mkv".to_string(),
            title: "Test Episode".to_string(),
            quality: "1080p".to_string(),
            audio_language: "ja-JP".to_string(),
            audio_languages: vec!["ja-JP".to_string()],
            subtitle_languages: vec!["en-US".to_string()],
        };
        assert_eq!(result.quality, "1080p");
    }

    #[test]
    fn test_parse_quality() {
        assert_eq!(parse_quality("best"), None);
        assert_eq!(parse_quality(""), None);
        assert_eq!(parse_quality("1080p"), Some(1080));
        assert_eq!(parse_quality("720p"), Some(720));
        assert_eq!(parse_quality("480"), Some(480));
        assert_eq!(parse_quality("1080P"), Some(1080)); // case insensitive
    }
}
