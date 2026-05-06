//! Queue manager for download operations.

use super::storage::QueueStorage;
use super::types::{QueueItem, QueueStatus};
use crate::error::{Error, QueueError, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Manages the download queue.
pub struct QueueManager {
    storage: QueueStorage,
    items: Arc<RwLock<Vec<QueueItem>>>,
    max_simultaneous: usize,
}

impl QueueManager {
    /// Create a new queue manager.
    pub fn new(max_simultaneous: usize) -> Result<Self> {
        let storage = QueueStorage::new()?;
        let items = storage.load()?;

        Ok(Self {
            storage,
            items: Arc::new(RwLock::new(items)),
            max_simultaneous,
        })
    }

    /// Create a queue manager with custom storage.
    pub fn with_storage(storage: QueueStorage, max_simultaneous: usize) -> Result<Self> {
        let items = storage.load()?;

        Ok(Self {
            storage,
            items: Arc::new(RwLock::new(items)),
            max_simultaneous,
        })
    }

    /// Add an item to the queue.
    pub async fn add(&self, item: QueueItem) -> Result<Uuid> {
        let uuid = item.uuid;

        let mut items = self.items.write().await;

        // Check for duplicates
        if items.iter().any(|i| i.episode_id == item.episode_id) {
            return Err(Error::Queue(QueueError::AlreadyExists(item.episode_id)));
        }

        items.push(item);
        self.storage.save(&items)?;

        info!("Added item to queue: {}", uuid);
        Ok(uuid)
    }

    /// Remove an item from the queue.
    pub async fn remove(&self, id: &str) -> Result<()> {
        let mut items = self.items.write().await;

        // Try to parse as UUID first, then try as index
        let index = if let Ok(uuid) = Uuid::parse_str(id) {
            items.iter().position(|i| i.uuid == uuid)
        } else if let Ok(idx) = id.parse::<usize>() {
            if idx < items.len() {
                Some(idx)
            } else {
                None
            }
        } else {
            None
        };

        match index {
            Some(idx) => {
                let removed = items.remove(idx);
                self.storage.save(&items)?;
                info!("Removed item from queue: {}", removed.uuid);
                Ok(())
            }
            None => Err(Error::Queue(QueueError::ItemNotFound(id.to_string()))),
        }
    }

    /// Clear all items from the queue.
    pub async fn clear(&self) -> Result<usize> {
        let mut items = self.items.write().await;
        let count = items.len();
        items.clear();
        self.storage.save(&items)?;
        info!("Cleared {} items from queue", count);
        Ok(count)
    }

    /// Get all queue items.
    pub async fn list(&self) -> Vec<QueueItem> {
        self.items.read().await.clone()
    }

    /// Get a single queue item by UUID.
    pub async fn get(&self, uuid: &Uuid) -> Option<QueueItem> {
        self.items
            .read()
            .await
            .iter()
            .find(|i| &i.uuid == uuid)
            .cloned()
    }

    /// Get pending items up to the simultaneous limit.
    pub async fn get_pending(&self) -> Vec<QueueItem> {
        let items = self.items.read().await;
        let active_count = items
            .iter()
            .filter(|i| i.status == QueueStatus::Active)
            .count();

        let available_slots = self.max_simultaneous.saturating_sub(active_count);

        items
            .iter()
            .filter(|i| i.status == QueueStatus::Pending)
            .take(available_slots)
            .cloned()
            .collect()
    }

    /// Update an item's status.
    pub async fn update_status(&self, uuid: &Uuid, status: QueueStatus) -> Result<()> {
        let mut items = self.items.write().await;

        if let Some(item) = items.iter_mut().find(|i| &i.uuid == uuid) {
            item.status = status;
            self.storage.save(&items)?;
            debug!("Updated status of {} to {}", uuid, status);
            Ok(())
        } else {
            Err(Error::Queue(QueueError::ItemNotFound(uuid.to_string())))
        }
    }

    /// Update an item with a closure.
    pub async fn update<F>(&self, uuid: &Uuid, f: F) -> Result<()>
    where
        F: FnOnce(&mut QueueItem),
    {
        let mut items = self.items.write().await;

        if let Some(item) = items.iter_mut().find(|i| &i.uuid == uuid) {
            f(item);
            self.storage.save(&items)?;
            Ok(())
        } else {
            Err(Error::Queue(QueueError::ItemNotFound(uuid.to_string())))
        }
    }

    /// Get queue statistics.
    pub async fn stats(&self) -> QueueStats {
        let items = self.items.read().await;

        QueueStats {
            total: items.len(),
            pending: items
                .iter()
                .filter(|i| i.status == QueueStatus::Pending)
                .count(),
            active: items
                .iter()
                .filter(|i| i.status == QueueStatus::Active)
                .count(),
            paused: items
                .iter()
                .filter(|i| i.status == QueueStatus::Paused)
                .count(),
            completed: items
                .iter()
                .filter(|i| i.status == QueueStatus::Completed)
                .count(),
            failed: items
                .iter()
                .filter(|i| i.status == QueueStatus::Failed)
                .count(),
        }
    }

    /// Pause all active downloads.
    pub async fn pause_all(&self) -> Result<usize> {
        let mut items = self.items.write().await;
        let mut count = 0;

        for item in items.iter_mut() {
            if item.status == QueueStatus::Active {
                item.pause();
                count += 1;
            }
        }

        self.storage.save(&items)?;
        info!("Paused {} items", count);
        Ok(count)
    }

    /// Resume all paused downloads.
    pub async fn resume_all(&self) -> Result<usize> {
        let mut items = self.items.write().await;
        let mut count = 0;

        for item in items.iter_mut() {
            if item.status == QueueStatus::Paused {
                item.resume();
                count += 1;
            }
        }

        self.storage.save(&items)?;
        info!("Resumed {} items", count);
        Ok(count)
    }
}

/// Queue statistics.
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total: usize,
    pub pending: usize,
    pub active: usize,
    pub paused: usize,
    pub completed: usize,
    pub failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::DownloadOptions;
    use tempfile::tempdir;

    fn create_test_manager() -> QueueManager {
        let dir = tempdir().unwrap();
        let path = dir.path().join("queue.json");
        let storage = QueueStorage::with_path(path);
        QueueManager::with_storage(storage, 2).unwrap()
    }

    #[tokio::test]
    async fn test_add_and_list() {
        let manager = create_test_manager();

        let item = QueueItem::new("EP123", "Test", "Test", 1, "1", DownloadOptions::default());

        manager.add(item).await.unwrap();

        let items = manager.list().await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].episode_id, "EP123");
    }

    #[tokio::test]
    async fn test_duplicate_prevention() {
        let manager = create_test_manager();

        let item1 = QueueItem::new("EP123", "Test", "Test", 1, "1", DownloadOptions::default());
        let item2 = QueueItem::new("EP123", "Test", "Test", 1, "1", DownloadOptions::default());

        manager.add(item1).await.unwrap();
        let result = manager.add(item2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stats() {
        let manager = create_test_manager();

        let item = QueueItem::new("EP123", "Test", "Test", 1, "1", DownloadOptions::default());

        manager.add(item).await.unwrap();

        let stats = manager.stats().await;
        assert_eq!(stats.total, 1);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.active, 0);
    }
}
