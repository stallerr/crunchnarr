//! Server-wide settings (single tenant; any authenticated user can edit).

use axum::extract::State;
use axum::routing::{get, patch};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::middleware::AuthUser;
use crate::db::app_settings::{
    get_settings_json, get_tracking_interval_secs, merge_settings,
    MAX_TRACKING_INTERVAL_SECS, MIN_TRACKING_INTERVAL_SECS,
};
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/app-settings", get(get_app_settings))
        .route("/app-settings", patch(update_app_settings))
}

#[derive(Serialize, ToSchema)]
pub struct AppSettings {
    /// How often the watchlist worker polls. Falls back to the
    /// `TRACKING_INTERVAL_SECS` env var when unset.
    pub tracking_interval_secs: u64,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateAppSettingsRequest {
    pub tracking_interval_secs: Option<u64>,
}

async fn current_settings(state: &AppState) -> Result<AppSettings, ApiError> {
    let interval = get_tracking_interval_secs(&state.db)
        .await
        .unwrap_or(state.config.tracking_interval_secs);
    Ok(AppSettings {
        tracking_interval_secs: interval,
    })
}

#[utoipa::path(
    get,
    path = "/app-settings",
    responses(
        (status = 200, description = "Server-wide settings", body = AppSettings),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "App Settings"
)]
pub async fn get_app_settings(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<AppSettings>, ApiError> {
    Ok(Json(current_settings(&state).await?))
}

#[utoipa::path(
    patch,
    path = "/app-settings",
    request_body = UpdateAppSettingsRequest,
    responses(
        (status = 200, description = "Updated", body = AppSettings),
        (status = 400, description = "Invalid value", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "App Settings"
)]
pub async fn update_app_settings(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<UpdateAppSettingsRequest>,
) -> Result<Json<AppSettings>, ApiError> {
    let mut updates = serde_json::Map::new();
    if let Some(secs) = req.tracking_interval_secs {
        if !(MIN_TRACKING_INTERVAL_SECS..=MAX_TRACKING_INTERVAL_SECS).contains(&secs) {
            return Err(ApiError::BadRequest(format!(
                "tracking_interval_secs must be between {} and {} seconds",
                MIN_TRACKING_INTERVAL_SECS, MAX_TRACKING_INTERVAL_SECS,
            )));
        }
        updates.insert(
            "tracking_interval_secs".to_string(),
            serde_json::Value::from(secs),
        );
    }

    if !updates.is_empty() {
        let _ = merge_settings(&state.db, &serde_json::Value::Object(updates))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    } else {
        // Touch nothing. Still returning 200 with current state — no_op semantics.
        let _ = get_settings_json(&state.db).await.ok();
    }

    Ok(Json(current_settings(&state).await?))
}
