//! Application state shared across handlers.

use crate::config::ServerConfig;
use crate::services::download::DownloadService;
use crate::services::tracking::TrackingService;
use crate::services::ws::WsBroadcaster;
use sqlx::SqlitePool;
use std::sync::Arc;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// SQLite connection pool.
    pub db: SqlitePool,
    /// Server configuration.
    pub config: Arc<ServerConfig>,
    /// Download service for managing background downloads.
    pub download_service: Arc<DownloadService>,
    /// WebSocket broadcaster for real-time events.
    pub ws_broadcaster: Arc<WsBroadcaster>,
    /// Watchlist polling service. Reachable via `POST /tracking/:id/check`.
    pub tracking_service: Arc<TrackingService>,
}
