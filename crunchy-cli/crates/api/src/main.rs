//! Crunchy-API server entrypoint.

use crunchy_api::config::ServerConfig;
use crunchy_api::db;
use crunchy_api::routes;
use crunchy_api::services::download::DownloadService;
use crunchy_api::services::tracking::TrackingService;
use crunchy_api::services::ws::WsBroadcaster;
use crunchy_api::state::AppState;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Load config
    let config = ServerConfig::from_env();
    info!("Starting crunchy-api on {}:{}", config.host, config.port);

    // Initialize database
    let db_pool = db::init_pool(&config.database_url).await?;

    // Create shared services
    let ws_broadcaster = Arc::new(WsBroadcaster::new());
    let download_service = Arc::new(DownloadService::new(ws_broadcaster.clone()));
    let tracking_service = TrackingService::new(
        db_pool.clone(),
        download_service.clone(),
        config.tracking_interval_secs,
    );
    tracking_service.clone().spawn();

    // Build app state
    let state = AppState {
        db: db_pool,
        config: Arc::new(config.clone()),
        download_service,
        ws_broadcaster,
        tracking_service,
    };

    // Build router
    let app = routes::build_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
