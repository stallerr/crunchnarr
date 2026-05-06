//! DASH MPD manifest parser for Crunchyroll streams.
//!
//! Parses MPEG-DASH Media Presentation Description (MPD) manifests
//! to extract video/audio representations, segment URLs, and DRM information.

use crate::error::{DownloadError, Error, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use tracing::debug;

/// Content type for adaptation sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Video,
    Audio,
    Text,
    Unknown,
}

impl ContentType {
    /// Parse content type from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "video" => ContentType::Video,
            "audio" => ContentType::Audio,
            "text" | "subtitle" => ContentType::Text,
            _ => ContentType::Unknown,
        }
    }

    /// Infer content type from mime type.
    pub fn from_mime(mime: &str) -> Self {
        let mime_lower = mime.to_lowercase();
        if mime_lower.starts_with("video/") {
            ContentType::Video
        } else if mime_lower.starts_with("audio/") {
            ContentType::Audio
        } else if mime_lower.starts_with("text/") || mime_lower.contains("ttml") {
            ContentType::Text
        } else {
            ContentType::Unknown
        }
    }
}

/// Parsed DASH MPD manifest.
#[derive(Debug, Clone)]
pub struct MpdManifest {
    /// Base URL for resolving relative URLs.
    pub base_url: String,
    /// Media presentation duration in seconds.
    pub duration: Option<f64>,
    /// Minimum buffer time in seconds.
    pub min_buffer_time: Option<f64>,
    /// Periods in the manifest.
    pub periods: Vec<Period>,
}

/// A period in the MPD manifest.
#[derive(Debug, Clone)]
pub struct Period {
    /// Period ID.
    pub id: Option<String>,
    /// Period start time.
    pub start: Option<f64>,
    /// Period duration.
    pub duration: Option<f64>,
    /// Base URL for this period.
    pub base_url: Option<String>,
    /// Adaptation sets in this period.
    pub adaptation_sets: Vec<AdaptationSet>,
}

/// An adaptation set containing representations of the same content type.
#[derive(Debug, Clone)]
pub struct AdaptationSet {
    /// Adaptation set ID.
    pub id: Option<String>,
    /// Content type (video, audio, text).
    pub content_type: ContentType,
    /// Language code (e.g., "en", "ja-JP").
    pub lang: Option<String>,
    /// MIME type.
    pub mime_type: Option<String>,
    /// Codec string.
    pub codecs: Option<String>,
    /// Base URL for this adaptation set.
    pub base_url: Option<String>,
    /// Segment template (if defined at adaptation set level).
    pub segment_template: Option<SegmentTemplate>,
    /// Content protection (DRM) information.
    pub content_protection: Vec<ContentProtection>,
    /// Available representations.
    pub representations: Vec<Representation>,
}

/// A specific representation (quality level) of content.
#[derive(Debug, Clone)]
pub struct Representation {
    /// Representation ID.
    pub id: String,
    /// Bandwidth in bits per second.
    pub bandwidth: u64,
    /// Video width in pixels.
    pub width: Option<u32>,
    /// Video height in pixels.
    pub height: Option<u32>,
    /// Frame rate.
    pub frame_rate: Option<String>,
    /// Codec string.
    pub codecs: Option<String>,
    /// MIME type.
    pub mime_type: Option<String>,
    /// Base URL for this representation.
    pub base_url: Option<String>,
    /// Segment template (if defined at representation level).
    pub segment_template: Option<SegmentTemplate>,
    /// Content protection (DRM) information.
    pub content_protection: Vec<ContentProtection>,
}

/// Segment template for generating segment URLs.
#[derive(Debug, Clone)]
pub struct SegmentTemplate {
    /// Initialization segment URL template.
    pub initialization: Option<String>,
    /// Media segment URL template.
    pub media: Option<String>,
    /// Timescale (units per second).
    pub timescale: u64,
    /// Start number for segment numbering.
    pub start_number: u64,
    /// Segment timeline entries.
    pub timeline: Vec<SegmentEntry>,
}

/// A segment timeline entry.
#[derive(Debug, Clone)]
pub struct SegmentEntry {
    /// Start time (in timescale units).
    pub t: Option<u64>,
    /// Duration (in timescale units).
    pub d: u64,
    /// Repeat count (-1 means repeat until end).
    pub r: i32,
}

/// Content protection (DRM) information.
#[derive(Debug, Clone)]
pub struct ContentProtection {
    /// Scheme ID URI (e.g., Widevine, PlayReady).
    pub scheme_id_uri: String,
    /// Key ID.
    pub key_id: Option<String>,
    /// PSSH box (base64 encoded).
    pub pssh: Option<String>,
    /// License URL.
    pub license_url: Option<String>,
}

impl ContentProtection {
    /// Check if this is Widevine DRM.
    pub fn is_widevine(&self) -> bool {
        self.scheme_id_uri
            .to_lowercase()
            .contains("edef8ba9-79d6-4ace-a3c8-27dcd51d21ed")
    }

    /// Check if this is PlayReady DRM.
    pub fn is_playready(&self) -> bool {
        self.scheme_id_uri
            .to_lowercase()
            .contains("9a04f079-9840-4286-ab92-e65be0885f95")
    }
}

/// Parser state tracking what element we're currently in.
#[derive(Debug, Clone, PartialEq)]
enum ParserContext {
    Root,
    Mpd,
    Period,
    AdaptationSet,
    Representation,
    SegmentTemplate,
    SegmentTimeline,
    ContentProtection,
    BaseUrl,
    Pssh,
}

impl MpdManifest {
    /// Parse an MPD manifest from XML.
    ///
    /// # Arguments
    /// * `xml` - The MPD XML content.
    /// * `base_url` - The base URL for resolving relative URLs.
    pub fn parse(xml: &str, base_url: &str) -> Result<Self> {
        debug!("Parsing MPD manifest, base_url: {}", base_url);

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut manifest = MpdManifest {
            base_url: base_url.to_string(),
            duration: None,
            min_buffer_time: None,
            periods: Vec::new(),
        };

        let mut buf = Vec::new();
        let mut context_stack: Vec<ParserContext> = vec![ParserContext::Root];

        // Current elements being built
        let mut current_period: Option<Period> = None;
        let mut current_adaptation_set: Option<AdaptationSet> = None;
        let mut current_representation: Option<Representation> = None;
        let mut current_segment_template: Option<SegmentTemplate> = None;
        let mut current_content_protection: Option<ContentProtection> = None;
        let mut timeline_entries: Vec<SegmentEntry> = Vec::new();
        let mut text_content = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    text_content.clear();

                    match tag_name.as_str() {
                        "MPD" => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);
                                match key.as_ref() {
                                    "mediaPresentationDuration" => {
                                        manifest.duration = parse_duration(&value);
                                    }
                                    "minBufferTime" => {
                                        manifest.min_buffer_time = parse_duration(&value);
                                    }
                                    _ => {}
                                }
                            }
                            context_stack.push(ParserContext::Mpd);
                        }
                        "Period" => {
                            let mut period = Period {
                                id: None,
                                start: None,
                                duration: None,
                                base_url: None,
                                adaptation_sets: Vec::new(),
                            };
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);
                                match key.as_ref() {
                                    "id" => period.id = Some(value.to_string()),
                                    "start" => period.start = parse_duration(&value),
                                    "duration" => period.duration = parse_duration(&value),
                                    _ => {}
                                }
                            }
                            current_period = Some(period);
                            context_stack.push(ParserContext::Period);
                        }
                        "AdaptationSet" => {
                            let mut adaptation_set = AdaptationSet {
                                id: None,
                                content_type: ContentType::Unknown,
                                lang: None,
                                mime_type: None,
                                codecs: None,
                                base_url: None,
                                segment_template: None,
                                content_protection: Vec::new(),
                                representations: Vec::new(),
                            };
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);
                                match key.as_ref() {
                                    "id" => adaptation_set.id = Some(value.to_string()),
                                    "contentType" => {
                                        adaptation_set.content_type = ContentType::from_str(&value);
                                    }
                                    "lang" => adaptation_set.lang = Some(value.to_string()),
                                    "mimeType" => {
                                        adaptation_set.mime_type = Some(value.to_string());
                                        if adaptation_set.content_type == ContentType::Unknown {
                                            adaptation_set.content_type =
                                                ContentType::from_mime(&value);
                                        }
                                    }
                                    "codecs" => adaptation_set.codecs = Some(value.to_string()),
                                    _ => {}
                                }
                            }
                            current_adaptation_set = Some(adaptation_set);
                            context_stack.push(ParserContext::AdaptationSet);
                        }
                        "Representation" => {
                            let mut representation = Representation {
                                id: String::new(),
                                bandwidth: 0,
                                width: None,
                                height: None,
                                frame_rate: None,
                                codecs: None,
                                mime_type: None,
                                base_url: None,
                                segment_template: None,
                                content_protection: Vec::new(),
                            };
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);
                                match key.as_ref() {
                                    "id" => representation.id = value.to_string(),
                                    "bandwidth" => {
                                        representation.bandwidth = value.parse().unwrap_or(0);
                                    }
                                    "width" => representation.width = value.parse().ok(),
                                    "height" => representation.height = value.parse().ok(),
                                    "frameRate" => {
                                        representation.frame_rate = Some(value.to_string())
                                    }
                                    "codecs" => representation.codecs = Some(value.to_string()),
                                    "mimeType" => {
                                        representation.mime_type = Some(value.to_string())
                                    }
                                    _ => {}
                                }
                            }
                            current_representation = Some(representation);
                            context_stack.push(ParserContext::Representation);
                        }
                        "SegmentTemplate" => {
                            let mut template = SegmentTemplate {
                                initialization: None,
                                media: None,
                                timescale: 1,
                                start_number: 1,
                                timeline: Vec::new(),
                            };
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);
                                match key.as_ref() {
                                    "initialization" => {
                                        template.initialization = Some(value.to_string())
                                    }
                                    "media" => template.media = Some(value.to_string()),
                                    "timescale" => template.timescale = value.parse().unwrap_or(1),
                                    "startNumber" => {
                                        template.start_number = value.parse().unwrap_or(1)
                                    }
                                    _ => {}
                                }
                            }
                            current_segment_template = Some(template);
                            timeline_entries.clear();
                            context_stack.push(ParserContext::SegmentTemplate);
                        }
                        "SegmentTimeline" => {
                            context_stack.push(ParserContext::SegmentTimeline);
                        }
                        "ContentProtection" => {
                            let mut cp = ContentProtection {
                                scheme_id_uri: String::new(),
                                key_id: None,
                                pssh: None,
                                license_url: None,
                            };
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);
                                match key.as_ref() {
                                    "schemeIdUri" => cp.scheme_id_uri = value.to_string(),
                                    "cenc:default_KID" | "default_KID" => {
                                        cp.key_id = Some(value.replace('-', ""));
                                    }
                                    _ => {}
                                }
                            }
                            current_content_protection = Some(cp);
                            context_stack.push(ParserContext::ContentProtection);
                        }
                        "cenc:pssh" | "pssh" => {
                            context_stack.push(ParserContext::Pssh);
                        }
                        "BaseURL" => {
                            context_stack.push(ParserContext::BaseUrl);
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "S" => {
                            // Segment timeline entry (self-closing)
                            if context_stack.last() == Some(&ParserContext::SegmentTimeline) {
                                let mut entry = SegmentEntry {
                                    t: None,
                                    d: 0,
                                    r: 0,
                                };
                                for attr in e.attributes().flatten() {
                                    let key = String::from_utf8_lossy(attr.key.as_ref());
                                    let value = String::from_utf8_lossy(&attr.value);
                                    match key.as_ref() {
                                        "t" => entry.t = value.parse().ok(),
                                        "d" => entry.d = value.parse().unwrap_or(0),
                                        "r" => entry.r = value.parse().unwrap_or(0),
                                        _ => {}
                                    }
                                }
                                timeline_entries.push(entry);
                            }
                        }
                        "Representation" => {
                            // Self-closing Representation (no SegmentTemplate inside)
                            let mut representation = Representation {
                                id: String::new(),
                                bandwidth: 0,
                                width: None,
                                height: None,
                                frame_rate: None,
                                codecs: None,
                                mime_type: None,
                                base_url: None,
                                segment_template: None,
                                content_protection: Vec::new(),
                            };
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);
                                match key.as_ref() {
                                    "id" => representation.id = value.to_string(),
                                    "bandwidth" => {
                                        representation.bandwidth = value.parse().unwrap_or(0);
                                    }
                                    "width" => representation.width = value.parse().ok(),
                                    "height" => representation.height = value.parse().ok(),
                                    "frameRate" => {
                                        representation.frame_rate = Some(value.to_string())
                                    }
                                    "codecs" => representation.codecs = Some(value.to_string()),
                                    "mimeType" => {
                                        representation.mime_type = Some(value.to_string())
                                    }
                                    _ => {}
                                }
                            }
                            // Add directly to adaptation set
                            if let Some(ref mut adaptation_set) = current_adaptation_set {
                                adaptation_set.representations.push(representation);
                            }
                        }
                        "ContentProtection" => {
                            // Self-closing ContentProtection (usually for cenc:default_KID)
                            let mut cp = ContentProtection {
                                scheme_id_uri: String::new(),
                                key_id: None,
                                pssh: None,
                                license_url: None,
                            };
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let value = String::from_utf8_lossy(&attr.value);
                                match key.as_ref() {
                                    "schemeIdUri" => cp.scheme_id_uri = value.to_string(),
                                    "cenc:default_KID" | "default_KID" => {
                                        cp.key_id = Some(value.replace('-', ""));
                                    }
                                    _ => {}
                                }
                            }
                            // Add to appropriate parent
                            if let Some(ref mut rep) = current_representation {
                                rep.content_protection.push(cp);
                            } else if let Some(ref mut as_) = current_adaptation_set {
                                as_.content_protection.push(cp);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if let Ok(text) = e.unescape() {
                        text_content.push_str(&text);
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "MPD" => {
                            context_stack.pop();
                        }
                        "Period" => {
                            if let Some(period) = current_period.take() {
                                manifest.periods.push(period);
                            }
                            context_stack.pop();
                        }
                        "AdaptationSet" => {
                            if let Some(mut adaptation_set) = current_adaptation_set.take() {
                                // Apply segment template if defined at adaptation set level
                                if let Some(template) = current_segment_template.take() {
                                    adaptation_set.segment_template = Some(template);
                                }
                                // Infer content type from representations if not already set
                                if adaptation_set.content_type == ContentType::Unknown {
                                    if let Some(rep) = adaptation_set.representations.first() {
                                        if let Some(ref mime) = rep.mime_type {
                                            adaptation_set.content_type =
                                                ContentType::from_mime(mime);
                                        }
                                    }
                                }
                                if let Some(ref mut period) = current_period {
                                    period.adaptation_sets.push(adaptation_set);
                                }
                            }
                            context_stack.pop();
                        }
                        "Representation" => {
                            if let Some(representation) = current_representation.take() {
                                // Note: SegmentTemplate is NOT applied here.
                                // It's applied when SegmentTemplate ends, based on context.
                                if let Some(ref mut adaptation_set) = current_adaptation_set {
                                    adaptation_set.representations.push(representation);
                                }
                            }
                            context_stack.pop();
                        }
                        "SegmentTemplate" => {
                            // Apply timeline entries to template
                            if let Some(ref mut template) = current_segment_template {
                                template.timeline = std::mem::take(&mut timeline_entries);
                            }

                            // Apply template to the correct parent based on context
                            // If we're inside a Representation, apply to representation
                            // Otherwise apply to AdaptationSet (will be done when AdaptationSet ends)
                            if current_representation.is_some() {
                                if let Some(template) = current_segment_template.take() {
                                    if let Some(ref mut rep) = current_representation {
                                        rep.segment_template = Some(template);
                                    }
                                }
                            }
                            // If not in a Representation, leave current_segment_template for AdaptationSet

                            context_stack.pop();
                        }
                        "SegmentTimeline" => {
                            context_stack.pop();
                        }
                        "ContentProtection" => {
                            if let Some(cp) = current_content_protection.take() {
                                // Add to appropriate parent
                                if let Some(ref mut rep) = current_representation {
                                    rep.content_protection.push(cp);
                                } else if let Some(ref mut as_) = current_adaptation_set {
                                    as_.content_protection.push(cp);
                                }
                            }
                            context_stack.pop();
                        }
                        "cenc:pssh" | "pssh" => {
                            let pssh = text_content.trim().to_string();
                            if !pssh.is_empty() {
                                if let Some(ref mut cp) = current_content_protection {
                                    cp.pssh = Some(pssh);
                                }
                            }
                            text_content.clear();
                            context_stack.pop();
                        }
                        "BaseURL" => {
                            let base = text_content.trim().to_string();
                            if !base.is_empty() {
                                // Apply BaseURL to the correct level based on context
                                if current_representation.is_some() {
                                    if let Some(ref mut rep) = current_representation {
                                        rep.base_url = Some(base);
                                    }
                                } else if current_adaptation_set.is_some() {
                                    if let Some(ref mut as_) = current_adaptation_set {
                                        as_.base_url = Some(base);
                                    }
                                } else if current_period.is_some() {
                                    if let Some(ref mut p) = current_period {
                                        p.base_url = Some(base);
                                    }
                                } else {
                                    manifest.base_url = resolve_url(&manifest.base_url, &base);
                                }
                            }
                            text_content.clear();
                            context_stack.pop();
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(Error::Download(DownloadError::ManifestError(format!(
                        "XML parse error: {}",
                        e
                    ))));
                }
                _ => {}
            }
            buf.clear();
        }

        debug!(
            "Parsed MPD: {} periods, duration: {:?}",
            manifest.periods.len(),
            manifest.duration
        );

        Ok(manifest)
    }

    /// Get all video adaptation sets from the manifest.
    pub fn video_adaptation_sets(&self) -> Vec<&AdaptationSet> {
        self.periods
            .iter()
            .flat_map(|p| &p.adaptation_sets)
            .filter(|as_| as_.content_type == ContentType::Video)
            .collect()
    }

    /// Get all audio adaptation sets from the manifest.
    pub fn audio_adaptation_sets(&self) -> Vec<&AdaptationSet> {
        self.periods
            .iter()
            .flat_map(|p| &p.adaptation_sets)
            .filter(|as_| as_.content_type == ContentType::Audio)
            .collect()
    }

    /// Get Widevine PSSH from the manifest.
    pub fn widevine_pssh(&self) -> Option<&str> {
        for period in &self.periods {
            for as_ in &period.adaptation_sets {
                for cp in &as_.content_protection {
                    if cp.is_widevine() {
                        if let Some(ref pssh) = cp.pssh {
                            return Some(pssh);
                        }
                    }
                }
                for rep in &as_.representations {
                    for cp in &rep.content_protection {
                        if cp.is_widevine() {
                            if let Some(ref pssh) = cp.pssh {
                                return Some(pssh);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Get the default key ID from the manifest.
    pub fn default_key_id(&self) -> Option<&str> {
        for period in &self.periods {
            for as_ in &period.adaptation_sets {
                for cp in &as_.content_protection {
                    if let Some(ref kid) = cp.key_id {
                        return Some(kid);
                    }
                }
            }
        }
        None
    }
}

impl Representation {
    /// Get the segment template, falling back to adaptation set level.
    pub fn get_segment_template<'a>(
        &'a self,
        adaptation_set: &'a AdaptationSet,
    ) -> Option<&'a SegmentTemplate> {
        self.segment_template
            .as_ref()
            .or(adaptation_set.segment_template.as_ref())
    }

    /// Get the codec string, falling back to adaptation set level.
    pub fn get_codecs<'a>(&'a self, adaptation_set: &'a AdaptationSet) -> Option<&'a str> {
        self.codecs.as_deref().or(adaptation_set.codecs.as_deref())
    }

    /// Get the mime type, falling back to adaptation set level.
    pub fn get_mime_type<'a>(&'a self, adaptation_set: &'a AdaptationSet) -> Option<&'a str> {
        self.mime_type
            .as_deref()
            .or(adaptation_set.mime_type.as_deref())
    }

    /// Generate the initialization segment URL.
    pub fn init_url(&self, adaptation_set: &AdaptationSet, base_url: &str) -> Option<String> {
        let template = self.get_segment_template(adaptation_set)?;
        let init = template.initialization.as_ref()?;

        let url = init
            .replace("$RepresentationID$", &self.id)
            .replace("$Bandwidth$", &self.bandwidth.to_string());

        Some(resolve_url(
            &self.resolve_base_url(adaptation_set, base_url),
            &url,
        ))
    }

    /// Generate all media segment URLs.
    pub fn segment_urls(&self, adaptation_set: &AdaptationSet, base_url: &str) -> Vec<String> {
        let Some(template) = self.get_segment_template(adaptation_set) else {
            return Vec::new();
        };
        let Some(ref media) = template.media else {
            return Vec::new();
        };

        let resolved_base = self.resolve_base_url(adaptation_set, base_url);
        let mut urls = Vec::new();
        let mut segment_number = template.start_number;
        let mut time = 0u64;

        for entry in &template.timeline {
            // Set start time if specified
            if let Some(t) = entry.t {
                time = t;
            }

            // Generate URL for this segment and any repeats
            let repeat_count = if entry.r < 0 { 0 } else { entry.r as u32 };
            for _ in 0..=repeat_count {
                let url = media
                    .replace("$RepresentationID$", &self.id)
                    .replace("$Bandwidth$", &self.bandwidth.to_string())
                    .replace("$Number$", &segment_number.to_string())
                    .replace(
                        &format!("$Number%0{}d$", 5),
                        &format!("{:05}", segment_number),
                    )
                    .replace(
                        &format!("$Number%0{}d$", 6),
                        &format!("{:06}", segment_number),
                    )
                    .replace("$Time$", &time.to_string());

                urls.push(resolve_url(&resolved_base, &url));

                segment_number += 1;
                time += entry.d;
            }
        }

        urls
    }

    /// Resolve the base URL for this representation.
    fn resolve_base_url(&self, adaptation_set: &AdaptationSet, manifest_base: &str) -> String {
        // Representation base URL takes precedence
        if let Some(ref base) = self.base_url {
            if base.starts_with("http://") || base.starts_with("https://") {
                return base.clone();
            }
            return resolve_url(manifest_base, base);
        }

        // Then adaptation set base URL
        if let Some(ref base) = adaptation_set.base_url {
            if base.starts_with("http://") || base.starts_with("https://") {
                return base.clone();
            }
            return resolve_url(manifest_base, base);
        }

        manifest_base.to_string()
    }

    /// Get a human-readable quality label.
    pub fn quality_label(&self) -> String {
        if let Some(height) = self.height {
            format!("{}p", height)
        } else {
            format!("{}kbps", self.bandwidth / 1000)
        }
    }
}

/// Parse ISO 8601 duration (PT...H...M...S) to seconds.
fn parse_duration(s: &str) -> Option<f64> {
    let s = s.trim();
    if !s.starts_with("PT") && !s.starts_with("P") {
        return None;
    }

    let s = s.trim_start_matches('P').trim_start_matches('T');
    let mut total = 0.0;
    let mut current_num = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            current_num.push(c);
        } else {
            let num: f64 = current_num.parse().unwrap_or(0.0);
            match c {
                'H' => total += num * 3600.0,
                'M' => total += num * 60.0,
                'S' => total += num,
                _ => {}
            }
            current_num.clear();
        }
    }

    if total > 0.0 {
        Some(total)
    } else {
        None
    }
}

/// Resolve a relative URL against a base URL.
fn resolve_url(base: &str, relative: &str) -> String {
    // If relative is absolute, return it
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }

    // If relative starts with /, use origin + relative
    if relative.starts_with('/') {
        if let Some(origin_end) = base.find("://").map(|i| {
            base[i + 3..]
                .find('/')
                .map(|j| i + 3 + j)
                .unwrap_or(base.len())
        }) {
            return format!("{}{}", &base[..origin_end], relative);
        }
        return relative.to_string();
    }

    // Otherwise, resolve relative to base path
    let base_path_end = base.rfind('/').unwrap_or(0);
    format!("{}/{}", &base[..base_path_end], relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("PT1H30M45S"), Some(5445.0));
        assert_eq!(parse_duration("PT30M"), Some(1800.0));
        assert_eq!(parse_duration("PT45.5S"), Some(45.5));
        assert_eq!(parse_duration("PT0S"), None); // Zero returns None
        assert_eq!(parse_duration("invalid"), None);
    }

    #[test]
    fn test_resolve_url() {
        assert_eq!(
            resolve_url("https://example.com/path/", "segment.mp4"),
            "https://example.com/path/segment.mp4"
        );
        assert_eq!(
            resolve_url("https://example.com/path/manifest.mpd", "segment.mp4"),
            "https://example.com/path/segment.mp4"
        );
        assert_eq!(
            resolve_url("https://example.com/path/", "/absolute/path.mp4"),
            "https://example.com/absolute/path.mp4"
        );
        assert_eq!(
            resolve_url("https://example.com/path/", "https://other.com/file.mp4"),
            "https://other.com/file.mp4"
        );
    }

    #[test]
    fn test_content_type_from_str() {
        assert_eq!(ContentType::from_str("video"), ContentType::Video);
        assert_eq!(ContentType::from_str("Video"), ContentType::Video);
        assert_eq!(ContentType::from_str("audio"), ContentType::Audio);
        assert_eq!(ContentType::from_str("text"), ContentType::Text);
        assert_eq!(ContentType::from_str("unknown"), ContentType::Unknown);
    }

    #[test]
    fn test_content_type_from_mime() {
        assert_eq!(ContentType::from_mime("video/mp4"), ContentType::Video);
        assert_eq!(ContentType::from_mime("audio/mp4"), ContentType::Audio);
        assert_eq!(ContentType::from_mime("text/vtt"), ContentType::Text);
    }

    #[test]
    fn test_parse_simple_mpd() {
        let mpd = r#"<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" mediaPresentationDuration="PT24M30S">
  <Period id="1">
    <AdaptationSet id="1" contentType="video" mimeType="video/mp4">
      <Representation id="1" bandwidth="5000000" width="1920" height="1080">
        <SegmentTemplate initialization="init-$RepresentationID$.mp4" media="seg-$RepresentationID$-$Number$.m4s" timescale="90000" startNumber="1">
          <SegmentTimeline>
            <S d="180000" r="9"/>
          </SegmentTimeline>
        </SegmentTemplate>
      </Representation>
      <Representation id="2" bandwidth="2000000" width="1280" height="720">
        <SegmentTemplate initialization="init-$RepresentationID$.mp4" media="seg-$RepresentationID$-$Number$.m4s" timescale="90000" startNumber="1">
          <SegmentTimeline>
            <S d="180000" r="9"/>
          </SegmentTimeline>
        </SegmentTemplate>
      </Representation>
    </AdaptationSet>
    <AdaptationSet id="2" contentType="audio" lang="ja" mimeType="audio/mp4">
      <Representation id="audio-ja" bandwidth="128000">
        <SegmentTemplate initialization="init-$RepresentationID$.mp4" media="seg-$RepresentationID$-$Number$.m4s" timescale="48000" startNumber="1">
          <SegmentTimeline>
            <S d="96000" r="9"/>
          </SegmentTimeline>
        </SegmentTemplate>
      </Representation>
    </AdaptationSet>
  </Period>
</MPD>"#;

        let manifest = MpdManifest::parse(mpd, "https://example.com/dash/").unwrap();

        assert_eq!(manifest.duration, Some(1470.0)); // 24*60 + 30
        assert_eq!(manifest.periods.len(), 1);

        let period = &manifest.periods[0];
        assert_eq!(period.adaptation_sets.len(), 2);

        // Video adaptation set
        let video_as = &period.adaptation_sets[0];
        assert_eq!(video_as.content_type, ContentType::Video);
        assert_eq!(video_as.representations.len(), 2);

        // Check 1080p representation
        let rep_1080 = &video_as.representations[0];
        assert_eq!(rep_1080.height, Some(1080));
        assert_eq!(rep_1080.bandwidth, 5000000);

        // Generate URLs
        let init_url = rep_1080.init_url(video_as, &manifest.base_url);
        assert_eq!(
            init_url,
            Some("https://example.com/dash/init-1.mp4".to_string())
        );

        let segment_urls = rep_1080.segment_urls(video_as, &manifest.base_url);
        assert_eq!(segment_urls.len(), 10); // r="9" means 10 segments total
        assert_eq!(segment_urls[0], "https://example.com/dash/seg-1-1.m4s");

        // Audio adaptation set
        let audio_as = &period.adaptation_sets[1];
        assert_eq!(audio_as.content_type, ContentType::Audio);
        assert_eq!(audio_as.lang, Some("ja".to_string()));
    }

    #[test]
    fn test_parse_mpd_with_drm() {
        let mpd = r#"<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" xmlns:cenc="urn:mpeg:cenc:2013">
  <Period>
    <AdaptationSet contentType="video">
      <ContentProtection schemeIdUri="urn:mpeg:dash:mp4protection:2011" cenc:default_KID="12345678-1234-1234-1234-123456789012"/>
      <ContentProtection schemeIdUri="urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed">
        <cenc:pssh>AAAAH3Bzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAAAEi</cenc:pssh>
      </ContentProtection>
      <Representation id="v1" bandwidth="3000000" width="1280" height="720">
        <SegmentTemplate initialization="init.mp4" media="seg-$Number$.m4s" timescale="1000" startNumber="1">
          <SegmentTimeline>
            <S d="2000" r="4"/>
          </SegmentTimeline>
        </SegmentTemplate>
      </Representation>
    </AdaptationSet>
  </Period>
</MPD>"#;

        let manifest = MpdManifest::parse(mpd, "https://drm.example.com/").unwrap();

        // Check Widevine PSSH extraction
        let pssh = manifest.widevine_pssh();
        assert!(pssh.is_some());
        assert_eq!(
            pssh.unwrap(),
            "AAAAH3Bzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAAAEi"
        );

        // Check key ID extraction
        let key_id = manifest.default_key_id();
        assert!(key_id.is_some());
        assert_eq!(key_id.unwrap(), "12345678123412341234123456789012");
    }

    #[test]
    fn test_representation_quality_label() {
        let rep = Representation {
            id: "1".to_string(),
            bandwidth: 5000000,
            width: Some(1920),
            height: Some(1080),
            frame_rate: None,
            codecs: None,
            mime_type: None,
            base_url: None,
            segment_template: None,
            content_protection: Vec::new(),
        };
        assert_eq!(rep.quality_label(), "1080p");

        let rep_no_height = Representation {
            id: "2".to_string(),
            bandwidth: 128000,
            width: None,
            height: None,
            frame_rate: None,
            codecs: None,
            mime_type: None,
            base_url: None,
            segment_template: None,
            content_protection: Vec::new(),
        };
        assert_eq!(rep_no_height.quality_label(), "128kbps");
    }

    #[test]
    fn test_content_protection_is_widevine() {
        let widevine = ContentProtection {
            scheme_id_uri: "urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed".to_string(),
            key_id: None,
            pssh: Some("test".to_string()),
            license_url: None,
        };
        assert!(widevine.is_widevine());
        assert!(!widevine.is_playready());

        let playready = ContentProtection {
            scheme_id_uri: "urn:uuid:9a04f079-9840-4286-ab92-e65be0885f95".to_string(),
            key_id: None,
            pssh: None,
            license_url: None,
        };
        assert!(!playready.is_widevine());
        assert!(playready.is_playready());
    }

    #[test]
    fn test_adaptation_set_level_segment_template() {
        // Test when SegmentTemplate is at AdaptationSet level instead of Representation level
        let mpd = r#"<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" mediaPresentationDuration="PT10S">
  <Period>
    <AdaptationSet contentType="video" mimeType="video/mp4">
      <SegmentTemplate initialization="init-$RepresentationID$.mp4" media="seg-$RepresentationID$-$Number$.m4s" timescale="1000" startNumber="1">
        <SegmentTimeline>
          <S d="2000" r="4"/>
        </SegmentTimeline>
      </SegmentTemplate>
      <Representation id="v1" bandwidth="1000000" width="1280" height="720"/>
      <Representation id="v2" bandwidth="500000" width="854" height="480"/>
    </AdaptationSet>
  </Period>
</MPD>"#;

        let manifest = MpdManifest::parse(mpd, "https://example.com/").unwrap();
        let video_as = &manifest.periods[0].adaptation_sets[0];

        // SegmentTemplate should be at AdaptationSet level
        assert!(video_as.segment_template.is_some());

        // Representations should not have their own template
        assert!(video_as.representations[0].segment_template.is_none());
        assert!(video_as.representations[1].segment_template.is_none());

        // But segment URLs should still work via get_segment_template fallback
        let rep = &video_as.representations[0];
        let urls = rep.segment_urls(video_as, &manifest.base_url);
        assert_eq!(urls.len(), 5);
        assert_eq!(urls[0], "https://example.com/seg-v1-1.m4s");
    }

    #[test]
    fn test_parse_real_crunchyroll_manifest() {
        // This is a real Crunchyroll manifest structure (truncated segment timeline)
        let mpd = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<MPD xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:mpeg:dash:schema:mpd:2011" xsi:schemaLocation="urn:mpeg:dash:schema:mpd:2011 http://standards.iso.org/ittf/PubliclyAvailableStandards/MPEG-DASH_schema_files/DASH-MPD.xsd" type="static" mediaPresentationDuration="PT1420.003S" minBufferTime="PT4S" profiles="urn:mpeg:dash:profile:full:2011">
    <Period>
        <AdaptationSet id="1" segmentAlignment="true" maxWidth="1920" maxHeight="1080" maxFrameRate="45000/1877">
            <ContentProtection xmlns:cenc="urn:mpeg:cenc:2013" schemeIdUri="urn:mpeg:dash:mp4protection:2011" value="cenc"/>
            <ContentProtection xmlns:cenc="urn:mpeg:cenc:2013" schemeIdUri="urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed" cenc:default_KID="eb022758-9778-3131-a722-2375be5ea406">
                <cenc:pssh>AAAAjnBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAAG4IARIQ6wInWJd4MTGnIiN1vl6kBiJYZXlKaGMzTmxkRWxrSWpvaVkySTJOR1V6TVRCak1EbGtaakprT1dGbFl6Y3dPRFZqWkRNeFkyUTRNbU1pTENKMllYSnBZVzUwU1dRaU9pSmhkbXRsZVNKOQ==</cenc:pssh>
            </ContentProtection>
            <ContentProtection xmlns:mspr="urn:microsoft:playready" schemeIdUri="urn:uuid:9a04f079-9840-4286-ab92-e65be0885f95">
                <mspr:pro>KAMAAAEAAQAeAzw...</mspr:pro>
            </ContentProtection>
            <SegmentTemplate timescale="1000" media="fragment-$Number$-$RepresentationID$.m4s?t=token" initialization="init-$RepresentationID$.mp4?t=token" startNumber="1">
                <SegmentTimeline>
                    <S d="4004"/>
                    <S d="4004"/>
                    <S d="4004" r="1"/>
                    <S d="4004"/>
                    <S d="2586"/>
                </SegmentTimeline>
            </SegmentTemplate>
            <Representation id="f1-v1-x3" mimeType="video/mp4" codecs="avc1.640028" width="1280" height="720" frameRate="45000/1877" sar="1:1" startWithSAP="1" bandwidth="3986038">
                <BaseURL>https://a-vrv.akamaized.net/path/to/video/</BaseURL>
            </Representation>
            <Representation id="f2-v1-x3" mimeType="video/mp4" codecs="avc1.640028" width="1920" height="1080" frameRate="45000/1877" sar="1:1" startWithSAP="1" bandwidth="7982997">
                <BaseURL>https://a-vrv.akamaized.net/path/to/video/</BaseURL>
            </Representation>
            <Representation id="f5-v1-x3" mimeType="video/mp4" codecs="avc1.42c015" width="428" height="240" frameRate="45000/1877" sar="1:1" startWithSAP="1" bandwidth="498241">
                <BaseURL>https://a-vrv.akamaized.net/path/to/video/</BaseURL>
            </Representation>
        </AdaptationSet>
        <AdaptationSet id="2" segmentAlignment="true">
            <AudioChannelConfiguration schemeIdUri="urn:mpeg:dash:23003:3:audio_channel_configuration:2011" value="1"/>
            <ContentProtection xmlns:cenc="urn:mpeg:cenc:2013" schemeIdUri="urn:mpeg:dash:mp4protection:2011" value="cenc"/>
            <ContentProtection xmlns:cenc="urn:mpeg:cenc:2013" schemeIdUri="urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed" cenc:default_KID="eb022758-9778-3131-a722-2375be5ea406">
                <cenc:pssh>AAAAjnBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAAG4IARIQ6wInWJd4MTGnIiN1vl6kBiJYZXlKaGMzTmxkRWxrSWpvaVkySTJOR1V6TVRCak1EbGtaakprT1dGbFl6Y3dPRFZqWkRNeFkyUTRNbU1pTENKMllYSnBZVzUwU1dRaU9pSmhkbXRsZVNKOQ==</cenc:pssh>
            </ContentProtection>
            <SegmentTemplate timescale="1000" media="fragment-$Number$-$RepresentationID$.m4s?t=token" initialization="init-$RepresentationID$.mp4?t=token" startNumber="1">
                <SegmentTimeline>
                    <S d="4017"/>
                    <S d="3994"/>
                    <S d="3994" r="2"/>
                </SegmentTimeline>
            </SegmentTemplate>
            <Representation id="f1-a1-x3" mimeType="audio/mp4" codecs="mp4a.40.2" audioSamplingRate="44100" startWithSAP="1" bandwidth="128000">
                <BaseURL>https://a-vrv.akamaized.net/path/to/audio/</BaseURL>
            </Representation>
        </AdaptationSet>
        <AdaptationSet contentType="text" lang="fr-FR" label="Français" mimeType="text/vtt">
            <Representation id="textstream_en_0" bandwidth="0">
                <BaseURL>https://v.vrv.co/path/to/sub-f6.vtt?Policy=xxx</BaseURL>
            </Representation>
        </AdaptationSet>
    </Period>
</MPD>"#;

        let manifest = MpdManifest::parse(mpd, "https://example.com/").unwrap();

        // Check basic parsing
        assert_eq!(manifest.duration, Some(1420.003));
        assert_eq!(manifest.min_buffer_time, Some(4.0));
        assert_eq!(manifest.periods.len(), 1);

        let period = &manifest.periods[0];
        assert_eq!(period.adaptation_sets.len(), 3);

        // Video AdaptationSet (id=1) - content type inferred from mimeType
        let video_as = &period.adaptation_sets[0];
        assert_eq!(video_as.content_type, ContentType::Video);
        assert_eq!(video_as.representations.len(), 3);

        // Check DRM info
        assert_eq!(video_as.content_protection.len(), 3);

        // Find Widevine protection
        let widevine = video_as
            .content_protection
            .iter()
            .find(|cp| cp.is_widevine())
            .expect("Should have Widevine protection");
        assert!(widevine.pssh.is_some());
        assert_eq!(
            widevine.key_id.as_deref(),
            Some("eb02275897783131a7222375be5ea406") // Dashes removed from eb022758-9778-3131-a722-2375be5ea406
        );

        // Check video representations
        let rep_1080 = video_as
            .representations
            .iter()
            .find(|r| r.height == Some(1080))
            .expect("Should have 1080p");
        assert_eq!(rep_1080.id, "f2-v1-x3");
        assert_eq!(rep_1080.bandwidth, 7982997);
        assert_eq!(rep_1080.codecs.as_deref(), Some("avc1.640028"));

        // Check segment URL generation (uses AdaptationSet's SegmentTemplate + Representation's BaseURL)
        let init_url = rep_1080.init_url(video_as, &manifest.base_url);
        assert!(init_url.is_some());
        assert!(init_url.as_ref().unwrap().contains("init-f2-v1-x3.mp4"));
        assert!(init_url
            .as_ref()
            .unwrap()
            .starts_with("https://a-vrv.akamaized.net/"));

        let segment_urls = rep_1080.segment_urls(video_as, &manifest.base_url);
        assert_eq!(segment_urls.len(), 6); // 1 + 1 + 2 + 1 + 1 = 6 segments
        assert!(segment_urls[0].contains("fragment-1-f2-v1-x3.m4s"));

        // Audio AdaptationSet (id=2) - content type inferred from mimeType
        let audio_as = &period.adaptation_sets[1];
        assert_eq!(audio_as.content_type, ContentType::Audio);
        assert_eq!(audio_as.representations.len(), 1);

        let audio_rep = &audio_as.representations[0];
        assert_eq!(audio_rep.id, "f1-a1-x3");
        assert_eq!(audio_rep.bandwidth, 128000);

        // Text/Subtitle AdaptationSet
        let text_as = &period.adaptation_sets[2];
        assert_eq!(text_as.content_type, ContentType::Text);
        assert_eq!(text_as.lang.as_deref(), Some("fr-FR"));
        assert_eq!(text_as.representations.len(), 1);

        // Subtitle has direct URL in BaseURL
        let sub_rep = &text_as.representations[0];
        assert!(sub_rep.base_url.as_ref().unwrap().contains("sub-f6.vtt"));

        // Test manifest-level helpers
        let pssh = manifest.widevine_pssh();
        assert!(pssh.is_some());
        assert!(pssh.unwrap().starts_with("AAAAjnBzc2g"));

        let key_id = manifest.default_key_id();
        assert!(key_id.is_some());
    }

    #[test]
    fn test_parse_full_crunchyroll_manifest_file() {
        // Test parsing the actual manifest file from the test fixtures
        let manifest_path =
            std::path::Path::new("/Users/rico/Documents/apps/crunchyroll-labs/crunchy-cli/manifest.xml");
        if !manifest_path.exists() {
            // Skip test if file doesn't exist
            return;
        }

        let xml = std::fs::read_to_string(manifest_path).expect("Failed to read manifest file");
        let manifest =
            MpdManifest::parse(&xml, "https://example.com/").expect("Failed to parse manifest");

        // Verify basic structure
        assert_eq!(manifest.periods.len(), 1);
        let period = &manifest.periods[0];
        assert_eq!(period.adaptation_sets.len(), 3); // video, audio, subtitles

        // Video
        let video_sets = manifest.video_adaptation_sets();
        assert_eq!(video_sets.len(), 1);
        let video_as = video_sets[0];
        assert_eq!(video_as.representations.len(), 5); // 1080p, 720p, 480p, 360p, 240p

        // Check we have all quality levels
        let heights: Vec<_> = video_as
            .representations
            .iter()
            .filter_map(|r| r.height)
            .collect();
        assert!(heights.contains(&1080));
        assert!(heights.contains(&720));
        assert!(heights.contains(&480));
        assert!(heights.contains(&360));
        assert!(heights.contains(&240));

        // Audio
        let audio_sets = manifest.audio_adaptation_sets();
        assert_eq!(audio_sets.len(), 1);
        let audio_as = audio_sets[0];
        assert_eq!(audio_as.representations.len(), 3); // 128k, 96k, 64k

        // Verify segment URL generation works with the real base URLs
        let best_video = video_as
            .representations
            .iter()
            .max_by_key(|r| r.bandwidth)
            .unwrap();

        // Init URL should use representation's BaseURL
        let init_url = best_video.init_url(video_as, &manifest.base_url);
        assert!(init_url.is_some());
        let init = init_url.unwrap();
        assert!(init.starts_with("https://a-vrv.akamaized.net/"));
        assert!(init.contains(&best_video.id));

        // Segment URLs
        let segment_urls = best_video.segment_urls(video_as, &manifest.base_url);
        assert!(segment_urls.len() > 300); // Should be many segments for ~24 min video

        // First segment
        assert!(segment_urls[0].starts_with("https://a-vrv.akamaized.net/"));
        assert!(segment_urls[0].contains("fragment-1-"));
        assert!(segment_urls[0].contains(&best_video.id));

        // Widevine PSSH
        let pssh = manifest.widevine_pssh();
        assert!(pssh.is_some());
        assert!(pssh.unwrap().len() > 50); // PSSH should be substantial

        // Key ID
        let key_id = manifest.default_key_id();
        assert!(key_id.is_some());
        assert_eq!(key_id.unwrap().len(), 32); // 32 hex chars (16 bytes)

        println!("Parsed manifest successfully:");
        println!("  Duration: {:?} seconds", manifest.duration);
        println!(
            "  Video representations: {}",
            video_as.representations.len()
        );
        println!(
            "  Audio representations: {}",
            audio_as.representations.len()
        );
        println!("  Total video segments: {}", segment_urls.len());
        println!("  PSSH present: {}", pssh.is_some());
    }
}
