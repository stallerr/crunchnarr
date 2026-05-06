//! Watchlist routes — series tracking + auto-download.

use axum::extract::{Path, State};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::db::tracking;
use crate::error::{ApiError, ErrorBody};
use crate::services::crunchyroll::CrunchyrollService;
use crate::services::tracking::{fetch_all_episodes, pick_series_thumbnail, CheckSummary};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tracking", get(list_tracked))
        .route("/tracking", post(add_tracked))
        .route("/tracking/{id}", patch(update_tracked))
        .route("/tracking/{id}", delete(delete_tracked))
        .route("/tracking/{id}/check", post(check_tracked))
}

#[derive(Serialize, ToSchema)]
pub struct TrackedSeriesItem {
    pub id: String,
    pub series_id: String,
    pub series_title: String,
    pub series_thumbnail: Option<String>,
    pub download_mode: String,
    pub enabled: bool,
    pub added_at: String,
    pub last_checked_at: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct AddTrackingRequest {
    pub series_id: String,
    /// `"new_only"` (default) or `"all"`.
    #[serde(default = "default_mode")]
    pub download_mode: String,
}

fn default_mode() -> String {
    "new_only".to_string()
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateTrackingRequest {
    pub download_mode: Option<String>,
    pub enabled: Option<bool>,
}

fn validate_mode(mode: &str) -> Result<(), ApiError> {
    if mode != "new_only" && mode != "all" {
        return Err(ApiError::BadRequest(format!(
            "download_mode must be 'new_only' or 'all', got: {}",
            mode
        )));
    }
    Ok(())
}

fn row_to_item(row: &crate::db::tracking::TrackedSeriesRow) -> TrackedSeriesItem {
    TrackedSeriesItem {
        id: row.id.clone(),
        series_id: row.series_id.clone(),
        series_title: row.series_title.clone(),
        series_thumbnail: row.series_thumbnail.clone(),
        download_mode: row.download_mode.clone(),
        enabled: row.enabled,
        added_at: row.added_at.clone(),
        last_checked_at: row.last_checked_at.clone(),
    }
}

#[utoipa::path(
    get,
    path = "/tracking",
    responses(
        (status = 200, description = "Tracked series for the current user", body = [TrackedSeriesItem]),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Watchlist"
)]
pub async fn list_tracked(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<TrackedSeriesItem>>, ApiError> {
    let rows = tracking::list_tracked_series(&state.db, &auth.user_id).await?;
    Ok(Json(rows.iter().map(row_to_item).collect()))
}

#[utoipa::path(
    post,
    path = "/tracking",
    request_body = AddTrackingRequest,
    responses(
        (status = 201, description = "Series added to watchlist", body = TrackedSeriesItem),
        (status = 400, description = "Invalid input", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 409, description = "Already tracking this series", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Watchlist"
)]
pub async fn add_tracked(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AddTrackingRequest>,
) -> Result<(axum::http::StatusCode, Json<TrackedSeriesItem>), ApiError> {
    let series_id = req.series_id.trim();
    if series_id.is_empty() {
        return Err(ApiError::BadRequest("series_id is required".to_string()));
    }
    validate_mode(&req.download_mode)?;

    if tracking::get_by_series_id(&state.db, &auth.user_id, series_id)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(
            "Already tracking this series".to_string(),
        ));
    }

    // Fetch series metadata + (for new_only) the baseline episode list.
    let cr_service = CrunchyrollService::new(state.db.clone());
    let client = cr_service.get_client(&auth.user_id).await?;
    let series = client
        .get_series(series_id)
        .await
        .map_err(crunchy_cli::Error::from)?;

    let baseline_json = if req.download_mode == "new_only" {
        let episodes = fetch_all_episodes(&client, series_id).await?;
        let ids: Vec<String> = episodes.iter().map(|e| e.id.clone()).collect();
        serde_json::to_string(&ids).unwrap_or_else(|_| "[]".to_string())
    } else {
        "[]".to_string()
    };

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let thumbnail = pick_series_thumbnail(&series.images);

    tracking::insert_tracked_series(
        &state.db,
        &id,
        &auth.user_id,
        series_id,
        &series.title,
        thumbnail.as_deref(),
        &req.download_mode,
        &baseline_json,
        &now,
    )
    .await?;

    let row = tracking::get_tracked_series(&state.db, &id, &auth.user_id)
        .await?
        .ok_or_else(|| ApiError::Internal("Inserted row vanished".to_string()))?;
    Ok((axum::http::StatusCode::CREATED, Json(row_to_item(&row))))
}

#[utoipa::path(
    patch,
    path = "/tracking/{id}",
    params(("id" = String, Path, description = "Tracking ID")),
    request_body = UpdateTrackingRequest,
    responses(
        (status = 200, description = "Updated", body = TrackedSeriesItem),
        (status = 400, description = "Invalid input", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Tracking entry not found", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Watchlist"
)]
pub async fn update_tracked(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateTrackingRequest>,
) -> Result<Json<TrackedSeriesItem>, ApiError> {
    let row = tracking::get_tracked_series(&state.db, &id, &auth.user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Tracking entry not found".to_string()))?;

    let new_mode = req.download_mode.as_deref().unwrap_or(&row.download_mode);
    validate_mode(new_mode)?;
    let new_enabled = req.enabled.unwrap_or(row.enabled);

    // `all → new_only` requires snapshotting the current episode list as the
    // new baseline, otherwise the next poll treats every episode as "new" and
    // re-downloads everything.
    let new_baseline = if row.download_mode == "all" && new_mode == "new_only" {
        let cr_service = CrunchyrollService::new(state.db.clone());
        let client = cr_service.get_client(&auth.user_id).await?;
        let episodes = fetch_all_episodes(&client, &row.series_id).await?;
        let ids: Vec<String> = episodes.iter().map(|e| e.id.clone()).collect();
        serde_json::to_string(&ids).unwrap_or_else(|_| "[]".to_string())
    } else {
        row.baseline_episode_ids.clone()
    };

    let updated = tracking::update_tracked_series(
        &state.db,
        &id,
        &auth.user_id,
        new_mode,
        &new_baseline,
        new_enabled,
    )
    .await?;
    if !updated {
        return Err(ApiError::NotFound("Tracking entry not found".to_string()));
    }

    let row = tracking::get_tracked_series(&state.db, &id, &auth.user_id)
        .await?
        .ok_or_else(|| ApiError::Internal("Updated row vanished".to_string()))?;
    Ok(Json(row_to_item(&row)))
}

#[utoipa::path(
    delete,
    path = "/tracking/{id}",
    params(("id" = String, Path, description = "Tracking ID")),
    responses(
        (status = 204, description = "Removed from watchlist"),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Tracking entry not found", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Watchlist"
)]
pub async fn delete_tracked(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let removed = tracking::delete_tracked_series(&state.db, &id, &auth.user_id).await?;
    if !removed {
        return Err(ApiError::NotFound("Tracking entry not found".to_string()));
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/tracking/{id}/check",
    params(("id" = String, Path, description = "Tracking ID")),
    responses(
        (status = 200, description = "Check completed", body = CheckSummary),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Tracking entry not found", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Watchlist"
)]
pub async fn check_tracked(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<CheckSummary>, ApiError> {
    let row = tracking::get_tracked_series(&state.db, &id, &auth.user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Tracking entry not found".to_string()))?;

    let summary = state.tracking_service.check_series(&row).await?;
    let _ = tracking::touch_last_checked(&state.db, &row.id).await;
    Ok(Json(summary))
}
