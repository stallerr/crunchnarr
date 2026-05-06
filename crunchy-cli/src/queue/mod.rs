//! Persistent download queue module.

mod manager;
mod storage;
mod types;

pub use manager::QueueManager;
pub use storage::QueueStorage;
pub use types::{DownloadOptions, QueueItem, QueueProgress, QueueStatus};
