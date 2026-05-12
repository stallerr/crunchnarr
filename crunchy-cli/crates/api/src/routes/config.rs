//! User config endpoints.

use axum::extract::State;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::Utc;

use crate::auth::middleware::AuthUser;
use crate::error::{ApiError, ErrorBody};
use crate::services::storage_secrets::{
    mask_storage_secrets, mask_widevine_blobs, maybe_encrypt_storage_secrets,
    maybe_encrypt_widevine_blobs, restore_placeholder_secret,
    restore_placeholder_widevine_blobs,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config", get(get_config))
        .route("/config", patch(update_config))
        .route("/config/reset", post(reset_config))
}

/// Build default config as flat JSON matching the frontend's AppConfig type.
fn default_config() -> serde_json::Value {
    let config = crunchy_cli::config::Config::default();
    serde_json::json!({
        "video_quality": config.downloads.video_quality,
        "simultaneous_downloads": config.downloads.simultaneous,
        "parallel_segments": 4,
        "max_speed_kbps": null,
        "retry_count": config.downloads.retry_count,
        "audio_languages": config.languages.audio,
        "subtitle_languages": config.languages.subtitles,
        "closed_captions": config.languages.include_cc,
        "output_format": config.muxing.format,
        "embed_subtitles": config.muxing.embed_subs,
        "default_audio_track": "",
        "default_subtitle_track": "",
        "prefer_signs_songs": false,
        "filename_template": config.muxing.filename_template,
        "output_dir": config.downloads.output_dir.display().to_string(),
        "cache_retention_days": 7,
        "concurrent_key_acquisitions": 2,
        "on_existing_download": "skip",
        "proxy_enabled": false,
        "proxy_url": "",
        "widevine_client": config.tools.widevine_client.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
        "widevine_private_key": config.tools.widevine_private_key.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
        "storage": {
            "kind": "local",
            "output_dir": config.downloads.output_dir.display().to_string(),
            "bucket": "",
            "region": "",
            "endpoint": "",
            "prefix": "",
            "access_key_id": "",
            "secret_access_key": "",
            "force_path_style": false,
        }
    })
}

/// Read the saved settings JSON for a user, or empty object if none.
async fn read_user_settings(
    db: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<serde_json::Value, ApiError> {
    let row: Option<String> = sqlx::query_scalar(
        "SELECT settings_json FROM user_settings WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .flatten();

    match row {
        Some(json_str) => serde_json::from_str(&json_str)
            .map_err(|e| ApiError::Internal(e.to_string())),
        None => Ok(serde_json::json!({})),
    }
}

/// Merge saved settings on top of defaults and return the full config.
fn merge_config(saved: &serde_json::Value) -> serde_json::Value {
    let mut config = default_config();
    if let (Some(base), Some(overlay)) = (config.as_object_mut(), saved.as_object()) {
        for (key, value) in overlay {
            base.insert(key.clone(), value.clone());
        }
    }
    config
}

#[utoipa::path(
    get,
    path = "/config",
    responses(
        (status = 200, description = "Current user configuration", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Config"
)]
async fn get_config(
    state: State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let saved = read_user_settings(&state.db, &auth.user_id).await?;
    let merged = merge_config(&saved);
    Ok(Json(mask_widevine_blobs(mask_storage_secrets(merged))))
}

#[utoipa::path(
    patch,
    path = "/config",
    request_body = Object,
    responses(
        (status = 200, description = "Configuration updated", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 422, description = "Invalid configuration value", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Config"
)]
async fn update_config(
    state: State<AppState>,
    auth: AuthUser,
    Json(mut updates): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut saved = read_user_settings(&state.db, &auth.user_id).await?;

    restore_placeholder_secret(&mut updates, &saved);
    restore_placeholder_widevine_blobs(&mut updates, &saved);

    if let (Some(base), Some(overlay)) = (saved.as_object_mut(), updates.as_object()) {
        for (key, value) in overlay {
            base.insert(key.clone(), value.clone());
        }
    }

    maybe_encrypt_storage_secrets(&mut saved)?;
    maybe_encrypt_widevine_blobs(&mut saved)?;

    let now = Utc::now().to_rfc3339();
    let settings_str = serde_json::to_string(&saved)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    sqlx::query(
        "INSERT INTO user_settings (user_id, settings_json, updated_at)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET settings_json = excluded.settings_json, updated_at = excluded.updated_at",
    )
    .bind(&auth.user_id)
    .bind(&settings_str)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(mask_widevine_blobs(mask_storage_secrets(merge_config(&saved)))))
}

#[utoipa::path(
    post,
    path = "/config/reset",
    responses(
        (status = 200, description = "Configuration reset to defaults", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Config"
)]
async fn reset_config(
    state: State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("DELETE FROM user_settings WHERE user_id = ?")
        .bind(&auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(default_config()))
}
