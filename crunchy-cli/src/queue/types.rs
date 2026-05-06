//! Queue data types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Status of a queue item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueueStatus {
    /// Waiting to be processed.
    Pending,
    /// Currently being downloaded.
    Active,
    /// Download paused.
    Paused,
    /// Download failed.
    Failed,
    /// Download completed.
    Completed,
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueStatus::Pending => write!(f, "pending"),
            QueueStatus::Active => write!(f, "active"),
            QueueStatus::Paused => write!(f, "paused"),
            QueueStatus::Failed => write!(f, "failed"),
            QueueStatus::Completed => write!(f, "completed"),
        }
    }
}

/// Progress information for a download.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueProgress {
    /// Current phase of the download.
    pub phase: String,
    /// Progress percentage (0-100).
    pub percent: f32,
    /// Downloaded bytes.
    pub downloaded_bytes: u64,
    /// Total bytes (if known).
    pub total_bytes: Option<u64>,
    /// Current download speed (bytes/sec).
    pub speed_bps: u64,
    /// Estimated time remaining (seconds).
    pub eta_secs: Option<u64>,
}

/// Download options for a queue item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadOptions {
    /// Output directory.
    pub output_dir: PathBuf,
    /// Audio languages to download.
    pub audio_langs: Vec<String>,
    /// Subtitle languages to download.
    pub sub_langs: Vec<String>,
    /// Video quality.
    pub video_quality: String,
    /// Whether to skip muxing.
    pub skip_mux: bool,
    /// Whether to skip subtitles.
    pub no_subs: bool,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("."),
            audio_langs: vec!["ja-JP".to_string()],
            sub_langs: vec!["en-US".to_string()],
            video_quality: "best".to_string(),
            skip_mux: false,
            no_subs: false,
        }
    }
}

/// A single item in the download queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    /// Unique identifier.
    pub uuid: Uuid,
    /// Episode ID.
    pub episode_id: String,
    /// Series title (for display).
    pub series_title: String,
    /// Episode title (for display).
    pub episode_title: String,
    /// Season number.
    pub season_number: u32,
    /// Episode number/string.
    pub episode_number: String,
    /// Current status.
    pub status: QueueStatus,
    /// Download progress.
    pub progress: QueueProgress,
    /// Download options.
    pub options: DownloadOptions,
    /// When the item was added.
    pub added_at: DateTime<Utc>,
    /// When the download started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the download completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Number of retry attempts.
    pub retry_count: u32,
}

impl QueueItem {
    /// Create a new queue item.
    pub fn new(
        episode_id: &str,
        series_title: &str,
        episode_title: &str,
        season_number: u32,
        episode_number: &str,
        options: DownloadOptions,
    ) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            episode_id: episode_id.to_string(),
            series_title: series_title.to_string(),
            episode_title: episode_title.to_string(),
            season_number,
            episode_number: episode_number.to_string(),
            status: QueueStatus::Pending,
            progress: QueueProgress::default(),
            options,
            added_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
            retry_count: 0,
        }
    }

    /// Get a display name for the item.
    pub fn display_name(&self) -> String {
        format!(
            "{} S{:02}E{} - {}",
            self.series_title, self.season_number, self.episode_number, self.episode_title
        )
    }

    /// Check if the item can be retried.
    pub fn can_retry(&self) -> bool {
        self.status == QueueStatus::Failed && self.retry_count < 3
    }

    /// Mark as started.
    pub fn start(&mut self) {
        self.status = QueueStatus::Active;
        self.started_at = Some(Utc::now());
        self.error = None;
    }

    /// Mark as completed.
    pub fn complete(&mut self) {
        self.status = QueueStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.progress.percent = 100.0;
    }

    /// Mark as failed.
    pub fn fail(&mut self, error: &str) {
        self.status = QueueStatus::Failed;
        self.error = Some(error.to_string());
        self.retry_count += 1;
    }

    /// Mark as paused.
    pub fn pause(&mut self) {
        if self.status == QueueStatus::Active {
            self.status = QueueStatus::Paused;
        }
    }

    /// Resume from paused state.
    pub fn resume(&mut self) {
        if self.status == QueueStatus::Paused {
            self.status = QueueStatus::Pending;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_item_creation() {
        let item = QueueItem::new(
            "EP123",
            "One Piece",
            "Romance Dawn",
            1,
            "1",
            DownloadOptions::default(),
        );

        assert_eq!(item.episode_id, "EP123");
        assert_eq!(item.status, QueueStatus::Pending);
        assert_eq!(item.retry_count, 0);
    }

    #[test]
    fn test_display_name() {
        let item = QueueItem::new(
            "EP123",
            "One Piece",
            "Romance Dawn",
            1,
            "1",
            DownloadOptions::default(),
        );

        assert_eq!(item.display_name(), "One Piece S01E1 - Romance Dawn");
    }

    #[test]
    fn test_status_transitions() {
        let mut item = QueueItem::new("EP123", "Test", "Test", 1, "1", DownloadOptions::default());

        item.start();
        assert_eq!(item.status, QueueStatus::Active);
        assert!(item.started_at.is_some());

        item.complete();
        assert_eq!(item.status, QueueStatus::Completed);
        assert!(item.completed_at.is_some());
    }

    #[test]
    fn test_retry_logic() {
        let mut item = QueueItem::new("EP123", "Test", "Test", 1, "1", DownloadOptions::default());

        item.fail("Error 1");
        assert!(item.can_retry());
        assert_eq!(item.retry_count, 1);

        item.fail("Error 2");
        item.fail("Error 3");
        assert!(!item.can_retry());
        assert_eq!(item.retry_count, 3);
    }
}
