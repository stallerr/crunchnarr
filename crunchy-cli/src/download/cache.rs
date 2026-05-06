//! Download cache for resumable downloads.
//!
//! This module provides persistent caching of download state to enable
//! resuming failed or interrupted downloads.

use crate::error::{DownloadError, Error, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, BufReader};
use tracing::{debug, info, warn};

/// Current cache schema version.
const CACHE_VERSION: u32 = 1;

/// Download cache for an episode.
///
/// Tracks the state of all downloaded segments, DRM keys, and subtitles
/// to enable resuming downloads after failures or restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadCache {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Episode ID this cache is for.
    pub episode_id: String,
    /// When this cache was created.
    pub created_at: DateTime<Utc>,
    /// Last update time.
    pub updated_at: DateTime<Utc>,
    /// SHA-256 hash of the manifest content (to detect changes).
    pub manifest_hash: String,
    /// Video stream cache.
    pub video: Option<StreamCache>,
    /// Audio stream caches (one per language).
    pub audio: Vec<StreamCache>,
    /// Subtitle caches.
    pub subtitles: Vec<SubtitleCache>,
    /// Cached DRM decryption keys (key_id -> key).
    #[serde(default)]
    pub drm_keys: HashMap<String, String>,
    /// Current download phase.
    pub phase: DownloadPhase,
}

/// Cache state for a single stream (video or audio).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamCache {
    /// Unique identifier for this stream.
    pub stream_id: String,
    /// Language locale (for audio streams).
    pub locale: Option<String>,
    /// Total number of segments.
    pub total_segments: usize,
    /// State of each segment.
    pub segments: Vec<SegmentState>,
    /// Whether all segments have been concatenated.
    pub concatenated: bool,
    /// Whether decryption has completed (if needed).
    pub decrypted: bool,
    /// Path to the final stream file (after concat/decrypt).
    pub output_path: Option<PathBuf>,
}

/// State of a single segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentState {
    /// Segment index (0-based).
    pub index: usize,
    /// URL this segment was downloaded from.
    pub url: String,
    /// File size in bytes (for validation).
    pub size: Option<u64>,
    /// SHA-256 checksum (computed after download).
    pub checksum: Option<String>,
    /// Current status.
    pub status: SegmentStatus,
}

/// Status of a segment download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentStatus {
    /// Not yet downloaded.
    Pending,
    /// Currently downloading.
    Downloading,
    /// Downloaded but not verified.
    Downloaded,
    /// Downloaded and checksum verified.
    Verified,
    /// Download failed.
    Failed,
}

/// Cache state for a subtitle track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleCache {
    /// Language locale.
    pub lang: String,
    /// Whether this is a closed caption track.
    pub is_cc: bool,
    /// Whether this is a signs & songs track.
    pub is_signs: bool,
    /// Whether the subtitle has been downloaded.
    pub downloaded: bool,
    /// Whether the subtitle has been converted to ASS.
    pub converted: bool,
    /// Path to the converted file.
    pub output_path: Option<PathBuf>,
}

/// Current phase of the download process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DownloadPhase {
    /// Initial state, fetching metadata.
    #[default]
    Metadata,
    /// Downloading segments.
    Segments,
    /// Decrypting streams.
    Decryption,
    /// Downloading subtitles.
    Subtitles,
    /// Muxing final output.
    Muxing,
    /// Download complete.
    Complete,
}

impl DownloadCache {
    /// Create a new cache for an episode.
    pub fn new(episode_id: &str, manifest_content: &str) -> Self {
        let now = Utc::now();
        Self {
            version: CACHE_VERSION,
            episode_id: episode_id.to_string(),
            created_at: now,
            updated_at: now,
            manifest_hash: compute_hash(manifest_content.as_bytes()),
            video: None,
            audio: Vec::new(),
            subtitles: Vec::new(),
            drm_keys: HashMap::new(),
            phase: DownloadPhase::Metadata,
        }
    }

    /// Load cache from a file.
    pub async fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path).await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to read cache file: {}",
                e
            )))
        })?;

        let cache: Self = serde_json::from_str(&content).map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to parse cache file: {}",
                e
            )))
        })?;

        // Check version compatibility
        if cache.version > CACHE_VERSION {
            warn!(
                "Cache version {} is newer than supported version {}, ignoring",
                cache.version, CACHE_VERSION
            );
            return Ok(None);
        }

        Ok(Some(cache))
    }

    /// Save cache to a file (atomic write).
    pub async fn save(&mut self, path: &Path) -> Result<()> {
        self.updated_at = Utc::now();

        let content = serde_json::to_string_pretty(self).map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to serialize cache: {}",
                e
            )))
        })?;

        // Atomic write: write to temp file, then rename
        let temp_path = path.with_extension("json.tmp");

        fs::write(&temp_path, &content).await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to write cache file: {}",
                e
            )))
        })?;

        fs::rename(&temp_path, path).await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to save cache file: {}",
                e
            )))
        })?;

        debug!("Saved cache to {:?}", path);
        Ok(())
    }

    /// Check if the manifest has changed since cache was created.
    pub fn is_manifest_valid(&self, manifest_content: &str) -> bool {
        let current_hash = compute_hash(manifest_content.as_bytes());
        self.manifest_hash == current_hash
    }

    /// Update the stored manifest hash without discarding cached segment data.
    ///
    /// This is used when the manifest content changes (e.g., due to CDN token
    /// rotation) but the underlying stream structure is the same. Existing
    /// verified segments on disk remain valid since segment content doesn't
    /// change — only the URLs do.
    pub fn update_manifest_hash(&mut self, manifest_content: &str) {
        self.manifest_hash = compute_hash(manifest_content.as_bytes());
        self.updated_at = Utc::now();
    }

    /// Initialize video stream cache.
    pub fn init_video(&mut self, stream_id: &str, segment_urls: &[String]) {
        self.video = Some(StreamCache::new(stream_id, None, segment_urls));
    }

    /// Initialize audio stream cache.
    pub fn init_audio(&mut self, stream_id: &str, locale: &str, segment_urls: &[String]) {
        self.audio
            .push(StreamCache::new(stream_id, Some(locale), segment_urls));
    }

    /// Get audio cache by locale.
    pub fn get_audio(&self, locale: &str) -> Option<&StreamCache> {
        self.audio.iter().find(|a| a.locale.as_deref() == Some(locale))
    }

    /// Get mutable audio cache by locale.
    pub fn get_audio_mut(&mut self, locale: &str) -> Option<&mut StreamCache> {
        self.audio
            .iter_mut()
            .find(|a| a.locale.as_deref() == Some(locale))
    }

    /// Initialize subtitle cache.
    pub fn init_subtitle(&mut self, lang: &str, is_cc: bool, is_signs: bool) {
        self.subtitles.push(SubtitleCache {
            lang: lang.to_string(),
            is_cc,
            is_signs,
            downloaded: false,
            converted: false,
            output_path: None,
        });
    }

    /// Get subtitle cache by lang and type.
    pub fn get_subtitle_mut(
        &mut self,
        lang: &str,
        is_cc: bool,
        is_signs: bool,
    ) -> Option<&mut SubtitleCache> {
        self.subtitles
            .iter_mut()
            .find(|s| s.lang == lang && s.is_cc == is_cc && s.is_signs == is_signs)
    }

    /// Cache DRM keys.
    pub fn set_drm_keys(&mut self, keys: &[(String, String)]) {
        for (key_id, key) in keys {
            self.drm_keys.insert(key_id.clone(), key.clone());
        }
    }

    /// Get cached DRM keys.
    pub fn get_drm_keys(&self) -> Vec<(String, String)> {
        self.drm_keys
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Check if cache has valid DRM keys.
    pub fn has_drm_keys(&self) -> bool {
        !self.drm_keys.is_empty()
    }

    /// Get resume summary for display.
    pub fn get_resume_summary(&self) -> ResumeSummary {
        let video_progress = self
            .video
            .as_ref()
            .map(|v| (v.verified_count(), v.total_segments))
            .unwrap_or((0, 0));

        let audio_progress: Vec<_> = self
            .audio
            .iter()
            .map(|a| {
                (
                    a.locale.clone().unwrap_or_default(),
                    a.verified_count(),
                    a.total_segments,
                )
            })
            .collect();

        ResumeSummary {
            video_verified: video_progress.0,
            video_total: video_progress.1,
            audio_progress,
            phase: self.phase,
        }
    }
}

impl StreamCache {
    /// Create a new stream cache.
    pub fn new(stream_id: &str, locale: Option<&str>, segment_urls: &[String]) -> Self {
        let segments = segment_urls
            .iter()
            .enumerate()
            .map(|(i, url)| SegmentState {
                index: i,
                url: url.clone(),
                size: None,
                checksum: None,
                status: SegmentStatus::Pending,
            })
            .collect();

        Self {
            stream_id: stream_id.to_string(),
            locale: locale.map(String::from),
            total_segments: segment_urls.len(),
            segments,
            concatenated: false,
            decrypted: false,
            output_path: None,
        }
    }

    /// Get segment by index.
    pub fn get_segment(&self, index: usize) -> Option<&SegmentState> {
        self.segments.get(index)
    }

    /// Get mutable segment by index.
    pub fn get_segment_mut(&mut self, index: usize) -> Option<&mut SegmentState> {
        self.segments.get_mut(index)
    }

    /// Mark segment as downloading.
    pub fn mark_downloading(&mut self, index: usize) {
        if let Some(seg) = self.segments.get_mut(index) {
            seg.status = SegmentStatus::Downloading;
        }
    }

    /// Mark segment as downloaded with size.
    pub fn mark_downloaded(&mut self, index: usize, size: u64) {
        if let Some(seg) = self.segments.get_mut(index) {
            seg.status = SegmentStatus::Downloaded;
            seg.size = Some(size);
        }
    }

    /// Mark segment as verified with checksum.
    pub fn mark_verified(&mut self, index: usize, checksum: String) {
        if let Some(seg) = self.segments.get_mut(index) {
            seg.status = SegmentStatus::Verified;
            seg.checksum = Some(checksum);
        }
    }

    /// Mark segment as failed.
    pub fn mark_failed(&mut self, index: usize) {
        if let Some(seg) = self.segments.get_mut(index) {
            seg.status = SegmentStatus::Failed;
        }
    }

    /// Check if a segment is verified.
    pub fn is_verified(&self, index: usize) -> bool {
        self.segments
            .get(index)
            .map(|s| s.status == SegmentStatus::Verified)
            .unwrap_or(false)
    }

    /// Count verified segments.
    pub fn verified_count(&self) -> usize {
        self.segments
            .iter()
            .filter(|s| s.status == SegmentStatus::Verified)
            .count()
    }

    /// Check if all segments are verified.
    pub fn all_verified(&self) -> bool {
        self.verified_count() == self.total_segments
    }

    /// Get indices of segments that need downloading.
    pub fn pending_indices(&self) -> Vec<usize> {
        self.segments
            .iter()
            .filter(|s| s.status != SegmentStatus::Verified)
            .map(|s| s.index)
            .collect()
    }
}

/// Summary of resume state for display.
#[derive(Debug)]
pub struct ResumeSummary {
    pub video_verified: usize,
    pub video_total: usize,
    pub audio_progress: Vec<(String, usize, usize)>,
    pub phase: DownloadPhase,
}

impl std::fmt::Display for ResumeSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();

        if self.video_total > 0 {
            parts.push(format!(
                "video: {}/{}",
                self.video_verified, self.video_total
            ));
        }

        for (locale, verified, total) in &self.audio_progress {
            if *total > 0 {
                parts.push(format!("audio[{}]: {}/{}", locale, verified, total));
            }
        }

        write!(f, "{} segments verified", parts.join(", "))
    }
}

/// Compute SHA-256 hash of data.
pub fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Compute SHA-256 hash of a file (streaming).
pub async fn compute_file_hash(path: &Path) -> Result<String> {
    let file = File::open(path).await.map_err(|e| {
        Error::Download(DownloadError::SegmentFailed(format!(
            "Failed to open file for hashing: {}",
            e
        )))
    })?;

    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

    loop {
        let bytes_read = reader.read(&mut buffer).await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to read file for hashing: {}",
                e
            )))
        })?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Verify a file's checksum.
pub async fn verify_checksum(path: &Path, expected: &str) -> Result<bool> {
    let actual = compute_file_hash(path).await?;
    Ok(actual == expected)
}

/// Statistics from cache cleanup.
#[derive(Debug, Default)]
pub struct CleanupStats {
    /// Number of cache directories scanned.
    pub scanned: usize,
    /// Number of cache directories removed.
    pub removed: usize,
    /// Total bytes freed.
    pub bytes_freed: u64,
}

/// Clean up stale caches older than the specified duration.
pub async fn cleanup_stale_caches(cache_dir: &Path, max_age: Duration) -> Result<CleanupStats> {
    let mut stats = CleanupStats::default();

    if !cache_dir.exists() {
        return Ok(stats);
    }

    let mut entries = fs::read_dir(cache_dir).await.map_err(|e| {
        Error::Download(DownloadError::SegmentFailed(format!(
            "Failed to read cache directory: {}",
            e
        )))
    })?;

    let cutoff = Utc::now() - max_age;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        Error::Download(DownloadError::SegmentFailed(format!(
            "Failed to read cache entry: {}",
            e
        )))
    })? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        stats.scanned += 1;

        let cache_file = path.join("cache.json");
        if !cache_file.exists() {
            continue;
        }

        // Try to load and check cache age
        if let Ok(Some(cache)) = DownloadCache::load(&cache_file).await {
            if cache.created_at < cutoff {
                // Calculate size before removing
                if let Ok(size) = dir_size(&path).await {
                    stats.bytes_freed += size;
                }

                if let Err(e) = fs::remove_dir_all(&path).await {
                    warn!("Failed to remove stale cache {:?}: {}", path, e);
                } else {
                    info!("Removed stale cache: {:?}", path);
                    stats.removed += 1;
                }
            }
        }
    }

    Ok(stats)
}

/// List all cached downloads.
pub async fn list_caches(cache_dir: &Path) -> Result<Vec<CacheInfo>> {
    let mut caches = Vec::new();

    if !cache_dir.exists() {
        return Ok(caches);
    }

    let mut entries = fs::read_dir(cache_dir).await.map_err(|e| {
        Error::Download(DownloadError::SegmentFailed(format!(
            "Failed to read cache directory: {}",
            e
        )))
    })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        Error::Download(DownloadError::SegmentFailed(format!(
            "Failed to read cache entry: {}",
            e
        )))
    })? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let cache_file = path.join("cache.json");
        if !cache_file.exists() {
            continue;
        }

        if let Ok(Some(cache)) = DownloadCache::load(&cache_file).await {
            let size = dir_size(&path).await.unwrap_or(0);
            let summary = cache.get_resume_summary();
            caches.push(CacheInfo {
                path,
                episode_id: cache.episode_id,
                created_at: cache.created_at,
                phase: cache.phase,
                size,
                summary,
            });
        }
    }

    // Sort by creation time (newest first)
    caches.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(caches)
}

/// Information about a cached download.
#[derive(Debug)]
pub struct CacheInfo {
    /// Path to the cache directory.
    pub path: PathBuf,
    /// Episode ID.
    pub episode_id: String,
    /// When the cache was created.
    pub created_at: DateTime<Utc>,
    /// Current download phase.
    pub phase: DownloadPhase,
    /// Total size in bytes.
    pub size: u64,
    /// Resume summary.
    pub summary: ResumeSummary,
}

/// Calculate the total size of a directory.
async fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;

    let mut entries = fs::read_dir(path).await.map_err(|e| {
        Error::Download(DownloadError::SegmentFailed(format!(
            "Failed to read directory: {}",
            e
        )))
    })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        Error::Download(DownloadError::SegmentFailed(format!(
            "Failed to read entry: {}",
            e
        )))
    })? {
        let metadata = entry.metadata().await.map_err(|e| {
            Error::Download(DownloadError::SegmentFailed(format!(
                "Failed to read metadata: {}",
                e
            )))
        })?;

        if metadata.is_file() {
            total += metadata.len();
        } else if metadata.is_dir() {
            total += Box::pin(dir_size(&entry.path())).await?;
        }
    }

    Ok(total)
}

/// Format bytes as human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_download_cache_new() {
        let cache = DownloadCache::new("episode123", "<MPD>content</MPD>");
        assert_eq!(cache.episode_id, "episode123");
        assert_eq!(cache.version, CACHE_VERSION);
        assert!(cache.video.is_none());
        assert!(cache.audio.is_empty());
    }

    #[test]
    fn test_stream_cache_new() {
        let urls = vec![
            "http://example.com/seg0.m4s".to_string(),
            "http://example.com/seg1.m4s".to_string(),
        ];
        let cache = StreamCache::new("video1", None, &urls);

        assert_eq!(cache.total_segments, 2);
        assert_eq!(cache.verified_count(), 0);
        assert!(!cache.all_verified());
    }

    #[test]
    fn test_stream_cache_mark_verified() {
        let urls = vec!["http://example.com/seg0.m4s".to_string()];
        let mut cache = StreamCache::new("video1", None, &urls);

        cache.mark_verified(0, "abc123".to_string());
        assert!(cache.is_verified(0));
        assert!(cache.all_verified());
    }

    #[test]
    fn test_manifest_validation() {
        let manifest = "<MPD>content</MPD>";
        let cache = DownloadCache::new("ep1", manifest);

        assert!(cache.is_manifest_valid(manifest));
        assert!(!cache.is_manifest_valid("<MPD>different</MPD>"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_segment_status_serialization() {
        let state = SegmentState {
            index: 0,
            url: "http://example.com/seg.m4s".to_string(),
            size: Some(1024),
            checksum: Some("abc".to_string()),
            status: SegmentStatus::Verified,
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: SegmentState = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.status, SegmentStatus::Verified);
        assert_eq!(parsed.size, Some(1024));
    }
}
