//! Download engine module.
//!
//! Handles segment downloading, progress tracking, and download orchestration.

pub mod cache;
pub mod manager;
pub mod manifest;
pub mod progress;
pub mod resolve;
mod segment;
mod selector;
mod throttle;

pub use cache::{
    cleanup_stale_caches, compute_file_hash, list_caches, verify_checksum, CacheInfo,
    CleanupStats, DownloadCache, DownloadPhase, ResumeSummary, SegmentState, SegmentStatus,
    StreamCache, SubtitleCache,
};
pub use manager::{DownloadManager, DownloadMode, DownloadOptions, DownloadResult};
pub use manifest::{
    AdaptationSet, ContentProtection, ContentType, MpdManifest, Period, Representation,
    SegmentEntry, SegmentTemplate,
};
pub use progress::{null_reporter, NullReporter, ProgressReporter, ProgressTracker, StepProgress};
pub use resolve::{resolve_episodes, resolve_series_episodes};
pub use segment::SegmentDownloader;
pub use selector::{SelectedStream, StreamSelection, StreamSelector, SubtitleTrack};
