//! Queue persistence storage.

use super::types::QueueItem;
use crate::config::Config;
use crate::error::{Error, QueueError, Result};
use std::path::PathBuf;
use tracing::{debug, info};

/// Handles queue persistence to disk.
pub struct QueueStorage {
    path: PathBuf,
}

impl QueueStorage {
    /// Create a new queue storage with default path.
    pub fn new() -> Result<Self> {
        let path = Config::queue_path()?;
        Ok(Self { path })
    }

    /// Create a queue storage with a custom path.
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Load queue from disk.
    pub fn load(&self) -> Result<Vec<QueueItem>> {
        if !self.path.exists() {
            debug!("Queue file not found, returning empty queue");
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&self.path).map_err(|e| {
            Error::Queue(QueueError::Corrupted(format!(
                "Failed to read queue file: {}",
                e
            )))
        })?;

        let items: Vec<QueueItem> = serde_json::from_str(&content).map_err(|e| {
            Error::Queue(QueueError::Corrupted(format!(
                "Failed to parse queue file: {}",
                e
            )))
        })?;

        debug!("Loaded {} items from queue", items.len());
        Ok(items)
    }

    /// Save queue to disk.
    pub fn save(&self, items: &[QueueItem]) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Queue(QueueError::OperationFailed(format!(
                    "Failed to create queue directory: {}",
                    e
                )))
            })?;
        }

        let content = serde_json::to_string_pretty(items)?;

        // Atomic write
        let temp_path = self.path.with_extension("json.tmp");
        std::fs::write(&temp_path, &content).map_err(|e| {
            Error::Queue(QueueError::OperationFailed(format!(
                "Failed to write queue file: {}",
                e
            )))
        })?;

        std::fs::rename(&temp_path, &self.path).map_err(|e| {
            Error::Queue(QueueError::OperationFailed(format!(
                "Failed to save queue file: {}",
                e
            )))
        })?;

        debug!("Saved {} items to queue", items.len());
        Ok(())
    }

    /// Get the queue file path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Delete the queue file.
    pub fn delete(&self) -> Result<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|e| {
                Error::Queue(QueueError::OperationFailed(format!(
                    "Failed to delete queue file: {}",
                    e
                )))
            })?;
            info!("Deleted queue file");
        }
        Ok(())
    }
}

impl Default for QueueStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create queue storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::types::{DownloadOptions, QueueStatus};
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("queue.json");
        let storage = QueueStorage::with_path(path);

        let items = vec![QueueItem::new(
            "EP123",
            "Test Series",
            "Test Episode",
            1,
            "1",
            DownloadOptions::default(),
        )];

        storage.save(&items).unwrap();
        let loaded = storage.load().unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].episode_id, "EP123");
    }

    #[test]
    fn test_load_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let storage = QueueStorage::with_path(path);

        let loaded = storage.load().unwrap();
        assert!(loaded.is_empty());
    }
}
