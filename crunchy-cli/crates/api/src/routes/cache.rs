//! Cache management endpoints.

use axum::extract::{Query, State};
use axum::routing::{delete, get};
use axum::{Json, Router};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::auth::middleware::AuthUser;
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

/// Where the resume-cache lives on disk. Matches the write site in
/// `crunchy_cli::download::SegmentDownloader`, which uses
/// `cfg.get_cache_dir()` → falls back to `temp_dir` = this path.
fn cache_root() -> std::path::PathBuf {
    std::env::temp_dir().join("crunchy-cli")
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/cache", get(list_caches))
        .route("/cache", delete(clean_caches))
        .route("/cache/stats", get(cache_stats))
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct CleanParams {
    /// If true, remove all caches regardless of age
    #[serde(default)]
    all: bool,
}

#[utoipa::path(
    get,
    path = "/cache",
    responses(
        (status = 200, description = "List of cached items", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Cache"
)]
async fn list_caches(
    _state: State<AppState>,
    _auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cache_dir = cache_root();
    let caches = crunchy_cli::download::list_caches(&cache_dir)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list caches: {}", e)))?;

    let total_size_bytes: u64 = caches.iter().map(|c| c.size).sum();
    let entry_count = caches.len();

    let entries: Vec<serde_json::Value> = caches
        .iter()
        .map(|c| {
            serde_json::json!({
                "episode_id": c.episode_id,
                "created_at": c.created_at.to_rfc3339(),
                "size_bytes": c.size,
                "phase": format!("{:?}", c.phase),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "total_size_bytes": total_size_bytes,
        "entry_count": entry_count,
        "retention_days": 1,
        "entries": entries,
    })))
}

#[utoipa::path(
    delete,
    path = "/cache",
    params(CleanParams),
    responses(
        (status = 200, description = "Caches cleaned", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Cache"
)]
async fn clean_caches(
    _state: State<AppState>,
    _auth: AuthUser,
    Query(params): Query<CleanParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cache_dir = cache_root();
    let max_age = if params.all {
        chrono::Duration::zero()
    } else {
        chrono::Duration::hours(24)
    };

    let stats = crunchy_cli::download::cleanup_stale_caches(&cache_dir, max_age)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to clean caches: {}", e)))?;

    Ok(Json(serde_json::json!({
        "deleted": stats.removed,
        "bytes_freed": stats.bytes_freed,
    })))
}

#[utoipa::path(
    get,
    path = "/cache/stats",
    responses(
        (status = 200, description = "Cache statistics", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Cache"
)]
async fn cache_stats(
    _state: State<AppState>,
    _auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cache_dir = cache_root();
    let caches = crunchy_cli::download::list_caches(&cache_dir)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list caches: {}", e)))?;

    let total_size: u64 = caches.iter().map(|c| c.size).sum();

    Ok(Json(serde_json::json!({
        "count": caches.len(),
        "total_size_bytes": total_size,
    })))
}
