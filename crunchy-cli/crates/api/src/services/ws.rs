//! WebSocket broadcaster for real-time events.

use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use futures::SinkExt;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Manages WebSocket connections per user and broadcasts events.
pub struct WsBroadcaster {
    connections: RwLock<HashMap<String, Vec<SplitSink<WebSocket, Message>>>>,
}

impl WsBroadcaster {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }

    /// Add a WebSocket connection for a user.
    pub async fn add_connection(&self, user_id: &str, sender: SplitSink<WebSocket, Message>) {
        let mut conns = self.connections.write().await;
        conns.entry(user_id.to_string()).or_default().push(sender);
        debug!("WebSocket connected for user {}", user_id);
    }

    /// Remove all connections for a user.
    pub async fn remove_connection(&self, user_id: &str) {
        let mut conns = self.connections.write().await;
        conns.remove(user_id);
        debug!("WebSocket disconnected for user {}", user_id);
    }

    /// Send a JSON message to all connections for a user.
    pub async fn send_to_user(&self, user_id: &str, msg: &serde_json::Value) {
        let text = serde_json::to_string(msg).unwrap_or_default();
        let mut conns = self.connections.write().await;

        if let Some(senders) = conns.get_mut(user_id) {
            let mut failed_indices = Vec::new();

            for (i, sender) in senders.iter_mut().enumerate() {
                if sender.send(Message::Text(text.clone().into())).await.is_err() {
                    failed_indices.push(i);
                }
            }

            // Remove failed connections (in reverse to preserve indices)
            for i in failed_indices.into_iter().rev() {
                warn!("Removing failed WebSocket connection for user {}", user_id);
                let _ = senders.remove(i);
            }

            // Clean up empty entries
            if senders.is_empty() {
                conns.remove(user_id);
            }
        }
    }

    /// Broadcast a message to all connected users.
    pub async fn broadcast(&self, msg: &serde_json::Value) {
        let user_ids: Vec<String> = {
            let conns = self.connections.read().await;
            conns.keys().cloned().collect()
        };

        for user_id in user_ids {
            self.send_to_user(&user_id, msg).await;
        }
    }
}

impl Default for WsBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
