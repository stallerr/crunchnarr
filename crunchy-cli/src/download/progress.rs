//! Progress tracking for downloads.

use std::sync::Arc;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::manager::DownloadResult;

/// Progress context for step-based reporting.
#[derive(Debug, Clone)]
pub struct StepProgress {
    /// Current step number (1-based).
    pub current_step: usize,
    /// Total number of steps.
    pub total_steps: usize,
    /// Human-readable label (e.g., "Video", "Audio (ja-JP)").
    pub label: String,
    /// Completed segments in this step.
    pub completed: u64,
    /// Total segments in this step.
    pub total: u64,
    /// Estimated download speed in bytes per second.
    pub speed_bps: u64,
    /// Estimated time remaining in seconds.
    pub eta_secs: Option<f64>,
}

/// High-level progress reporter trait for download lifecycle events.
///
/// Implementations receive notifications at key points during downloads.
/// The CLI uses `TerminalProgress` (indicatif), while the API can use
/// a WebSocket-based implementation to push real-time events to clients.
pub trait ProgressReporter: Send + Sync {
    /// A segment has been downloaded for a stream.
    fn on_segment_complete(&self, stream_id: &str, completed: u64, total: u64);

    /// Step-level progress update with segment counts and step context.
    fn on_step_progress(&self, _progress: &StepProgress) {}

    /// The download has entered a new phase (e.g., "downloading_video", "decrypting", "muxing").
    fn on_phase_change(&self, phase: &str, detail: &str);

    /// The download completed successfully.
    fn on_complete(&self, result: &DownloadResult);

    /// The download encountered an error.
    fn on_error(&self, error: &str);
}

/// A no-op progress reporter that discards all events.
///
/// Used as the default when no external reporting is needed (e.g., CLI mode
/// where indicatif handles display directly).
pub struct NullReporter;

impl ProgressReporter for NullReporter {
    fn on_segment_complete(&self, _stream_id: &str, _completed: u64, _total: u64) {}
    fn on_phase_change(&self, _phase: &str, _detail: &str) {}
    fn on_complete(&self, _result: &DownloadResult) {}
    fn on_error(&self, _error: &str) {}
}

/// Tracks progress for multiple concurrent downloads using terminal progress bars.
pub struct ProgressTracker {
    multi: MultiProgress,
}

impl ProgressTracker {
    /// Create a new progress tracker.
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
        }
    }

    /// Create a new progress bar for a download.
    pub fn add_download(&self, total_segments: u64, title: &str) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new(total_segments));
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{msg:.bold.cyan} [{bar:30.cyan/blue}] {pos}/{len} segments ({percent}%) [{elapsed_precise}]",
                )
                .unwrap()
                .progress_chars("━╸─"),
        );
        pb.set_message(title.to_string());
        pb
    }

    /// Create a spinner for indeterminate progress.
    pub fn add_spinner(&self, message: &str) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg:.bold}")
                .unwrap(),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns a default no-op reporter wrapped in an Arc.
pub fn null_reporter() -> Arc<dyn ProgressReporter> {
    Arc::new(NullReporter)
}
