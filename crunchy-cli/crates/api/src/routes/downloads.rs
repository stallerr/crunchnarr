//! Download endpoints.

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;
use utoipa::{IntoParams, ToSchema};

use crate::auth::middleware::AuthUser;
use crate::error::{ApiError, ErrorBody};
use crate::services::download::DownloadRow;
use crate::services::storage_secrets::decrypt_storage_secrets;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/downloads", post(start_download))
        .route("/downloads", get(list_downloads))
        .route("/downloads/counts", get(download_counts))
        .route("/downloads/episode-ids", get(downloaded_episode_ids))
        .route("/downloads/active", delete(cancel_active))
        .route("/downloads/manual", post(mark_manual))
        .route("/downloads/manual/bulk", post(mark_manual_bulk))
        .route("/downloads/manual/{episode_id}", delete(unmark_manual))
        .route("/downloads/{id}", get(get_download))
        .route("/downloads/{id}", delete(cancel_download))
        .route("/downloads/{id}/pause", patch(pause_download))
        .route("/downloads/{id}/resume", patch(resume_download))
        .route("/downloads/{id}/file", get(serve_file))
}

#[derive(Deserialize, ToSchema)]
pub struct StartDownloadRequest {
    /// Crunchyroll URL to download
    url: String,
    /// Optional download options (format, quality, etc.)
    #[serde(default)]
    options: serde_json::Value,
}

#[derive(Serialize, ToSchema)]
pub struct DownloadResponse {
    /// New download UUID when started, or the prior row's UUID when skipped
    /// because the DB already had an entry for this episode. None when
    /// skipped purely because the file existed at the templated output path.
    id: Option<String>,
    /// "pending" for a newly-queued download, "skipped" otherwise.
    status: String,
    episode_id: String,
    episode_title: Option<String>,
    /// Set when `status == "skipped"`. One of `"already_downloaded"`,
    /// `"in_progress"`, or `"file_exists"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_reason: Option<String>,
    /// The UUID of the prior `downloads` row that triggered the skip, if
    /// any. Always None for `skip_reason == "file_exists"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    existing_download_id: Option<String>,
    /// The on-disk path that already exists. Set only when
    /// `skip_reason == "file_exists"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    existing_path: Option<String>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct ListDownloadsParams {
    /// Filter by status tab (e.g. "active", "completed", "failed", "cancelled")
    status: Option<String>,
    /// Cursor for pagination — the `created_at` value of the last item from previous page
    cursor: Option<String>,
    /// Page size (default: 20)
    #[serde(default = "default_page_size")]
    limit: u32,
    /// When true, also include rows superseded by an in-flight watchlist upgrade.
    /// Defaults to false — most callers want one row per episode.
    #[serde(default)]
    include_superseded: bool,
}

fn default_page_size() -> u32 {
    20
}

#[derive(Serialize, ToSchema)]
pub struct PaginatedDownloads {
    items: Vec<DownloadRow>,
    next_cursor: Option<String>,
    has_more: bool,
}

#[derive(Serialize, ToSchema)]
pub struct DownloadCounts {
    pub all: i64,
    pub active: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
}

#[utoipa::path(
    post,
    path = "/downloads",
    request_body = StartDownloadRequest,
    responses(
        (status = 200, description = "Downloads started", body = Vec<DownloadResponse>),
        (status = 400, description = "Invalid URL", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Downloads"
)]
async fn start_download(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<StartDownloadRequest>,
) -> Result<Json<Vec<DownloadResponse>>, ApiError> {
    let outcomes = state
        .download_service
        .start_download(&auth.user_id, &req.url, req.options, &state.db)
        .await?;

    Ok(Json(
        outcomes
            .into_iter()
            .map(|o| DownloadResponse {
                id: o.download_id,
                status: o.status.to_string(),
                episode_id: o.episode_id,
                episode_title: Some(o.episode_title),
                skip_reason: o.skip_reason.map(String::from),
                existing_download_id: o.existing_download_id,
                existing_path: o.existing_path,
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/downloads",
    params(ListDownloadsParams),
    responses(
        (status = 200, description = "Paginated list of downloads", body = PaginatedDownloads),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Downloads"
)]
async fn list_downloads(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListDownloadsParams>,
) -> Result<Json<PaginatedDownloads>, ApiError> {
    let limit = params.limit.min(100).max(1);
    let mut rows = state
        .download_service
        .list_downloads(
            &auth.user_id,
            params.status.as_deref(),
            params.cursor.as_deref(),
            limit,
            params.include_superseded,
            &state.db,
        )
        .await?;

    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.pop();
    }
    // Compound cursor `<created_at>|<id>` — needed because a season-level
    // download inserts N episodes in one tight loop and many rows share the
    // exact same created_at. A scalar `created_at` cursor with strict `<`
    // would skip the siblings.
    let next_cursor = if has_more {
        rows.last().map(|r| format!("{}|{}", r.created_at, r.id))
    } else {
        None
    };

    Ok(Json(PaginatedDownloads {
        items: rows,
        next_cursor,
        has_more,
    }))
}

#[utoipa::path(
    get,
    path = "/downloads/counts",
    responses(
        (status = 200, description = "Download counts by status", body = DownloadCounts),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Downloads"
)]
async fn download_counts(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<DownloadCounts>, ApiError> {
    let counts = state
        .download_service
        .download_counts(&auth.user_id, &state.db)
        .await?;
    Ok(Json(counts))
}

#[derive(Serialize, ToSchema)]
pub struct DownloadedEpisodeIds {
    /// Episode IDs with a real completed download (manual = 0).
    pub completed: Vec<String>,
    /// Episode IDs the user manually marked as already downloaded.
    pub manual: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/downloads/episode-ids",
    responses(
        (status = 200, description = "Episode IDs the user has downloaded", body = DownloadedEpisodeIds),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Downloads"
)]
async fn downloaded_episode_ids(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<DownloadedEpisodeIds>, ApiError> {
    let (completed, manual) = state
        .download_service
        .episode_id_buckets(&auth.user_id, &state.db)
        .await?;
    Ok(Json(DownloadedEpisodeIds { completed, manual }))
}

#[derive(Deserialize, ToSchema)]
pub struct MarkManualRequest {
    pub episode_id: String,
    pub series_title: Option<String>,
    pub episode_title: Option<String>,
    pub season_number: Option<i64>,
    pub episode_number: Option<f64>,
    pub thumbnail_url: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct MarkManualBulkRequest {
    pub items: Vec<MarkManualRequest>,
}

#[derive(Serialize, ToSchema)]
pub struct MarkManualResponse {
    /// Number of episodes that were newly marked. `0` means everything in the
    /// request was either already manually marked or already truly downloaded.
    pub marked: u32,
    /// Number of items that were no-ops because a real (non-manual) completed
    /// row already exists.
    pub skipped: u32,
}

#[utoipa::path(
    post,
    path = "/downloads/manual",
    request_body = MarkManualRequest,
    responses(
        (status = 200, description = "Episode marked", body = MarkManualResponse),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Downloads"
)]
async fn mark_manual(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<MarkManualRequest>,
) -> Result<Json<MarkManualResponse>, ApiError> {
    let (marked, skipped) = state
        .download_service
        .mark_manual(&auth.user_id, &[req], &state.db)
        .await?;
    Ok(Json(MarkManualResponse { marked, skipped }))
}

#[utoipa::path(
    post,
    path = "/downloads/manual/bulk",
    request_body = MarkManualBulkRequest,
    responses(
        (status = 200, description = "Episodes marked", body = MarkManualResponse),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Downloads"
)]
async fn mark_manual_bulk(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<MarkManualBulkRequest>,
) -> Result<Json<MarkManualResponse>, ApiError> {
    let (marked, skipped) = state
        .download_service
        .mark_manual(&auth.user_id, &req.items, &state.db)
        .await?;
    Ok(Json(MarkManualResponse { marked, skipped }))
}

#[utoipa::path(
    delete,
    path = "/downloads/manual/{episode_id}",
    params(("episode_id" = String, Path, description = "Crunchyroll episode ID")),
    responses(
        (status = 204, description = "Manual mark removed"),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "No manual mark for this episode", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Downloads"
)]
async fn unmark_manual(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(episode_id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let removed = state
        .download_service
        .unmark_manual(&auth.user_id, &episode_id, &state.db)
        .await?;
    if !removed {
        return Err(ApiError::NotFound("No manual mark for this episode".to_string()));
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/downloads/{id}",
    params(("id" = String, Path, description = "Download ID")),
    responses(
        (status = 200, description = "Download details", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Download not found", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Downloads"
)]
async fn get_download(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = state
        .download_service
        .get_download(&auth.user_id, &id, &state.db)
        .await?;
    Ok(Json(serde_json::to_value(&row).unwrap()))
}

#[utoipa::path(
    delete,
    path = "/downloads/{id}",
    params(("id" = String, Path, description = "Download ID")),
    responses(
        (status = 200, description = "Download cancelled", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Download not found", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Downloads"
)]
async fn cancel_download(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .download_service
        .cancel_download(&auth.user_id, &id, &state.db)
        .await?;
    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}

#[utoipa::path(
    delete,
    path = "/downloads/active",
    responses(
        (status = 200, description = "All active/pending/paused downloads cancelled", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Downloads"
)]
async fn cancel_active(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cancelled = state
        .download_service
        .cancel_active_for_user(&auth.user_id, &state.db)
        .await?;
    Ok(Json(serde_json::json!({ "cancelled": cancelled })))
}

#[utoipa::path(
    patch,
    path = "/downloads/{id}/pause",
    params(("id" = String, Path, description = "Download ID")),
    responses(
        (status = 200, description = "Download paused", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Download not found", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Downloads"
)]
async fn pause_download(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .download_service
        .pause_download(&auth.user_id, &id, &state.db)
        .await?;
    Ok(Json(serde_json::json!({ "status": "paused" })))
}

#[utoipa::path(
    patch,
    path = "/downloads/{id}/resume",
    params(("id" = String, Path, description = "Download ID")),
    responses(
        (status = 200, description = "Download resumed", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Download not found", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Downloads"
)]
async fn resume_download(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .download_service
        .resume_download(&auth.user_id, &id, &state.db)
        .await?;
    Ok(Json(serde_json::json!({ "status": "active" })))
}

#[utoipa::path(
    get,
    path = "/downloads/{id}/file",
    params(("id" = String, Path, description = "Download ID")),
    responses(
        (status = 200, description = "File stream"),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "File not found", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Downloads"
)]
async fn serve_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let row = state
        .download_service
        .get_download(&auth.user_id, &id, &state.db)
        .await?;

    let output_path = row
        .output_path
        .as_deref()
        .ok_or_else(|| ApiError::NotFound("Download has no output file".to_string()))?;

    // S3-backed outputs: redirect the client straight to a short-lived
    // presigned URL using the user's current storage credentials. Local
    // outputs (`file://...`, or bare paths in legacy rows) are streamed from
    // disk below.
    if let Some(rest) = output_path.strip_prefix("s3://") {
        let (bucket, key) = rest.split_once('/').ok_or_else(|| {
            ApiError::Internal(format!("Malformed s3 URI: {output_path}"))
        })?;
        let s3 = build_user_s3_sink(&state, &auth.user_id).await?;
        let presigned = s3.presigned_get(bucket, key).await?;
        return Ok((
            axum::http::StatusCode::FOUND,
            [(header::LOCATION, presigned)],
        )
            .into_response());
    }

    let local_path: std::path::PathBuf = if let Some(rest) = output_path.strip_prefix("file://") {
        std::path::PathBuf::from(rest)
    } else if output_path.contains("://") {
        return Err(ApiError::NotFound(format!(
            "Output stored on unsupported sink ({}); cannot serve",
            output_path.split("://").next().unwrap_or("?")
        )));
    } else {
        std::path::PathBuf::from(output_path)
    };

    let path = local_path.as_path();
    if !path.exists() {
        return Err(ApiError::NotFound(
            "File no longer exists at the download location".to_string(),
        ));
    }

    let file = tokio::fs::File::open(path)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to open file: {e}")))?;

    let metadata = file
        .metadata()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to read metadata: {e}")))?;

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download");

    let content_type = match path.extension().and_then(|e| e.to_str()) {
        Some("mkv") => "video/x-matroska",
        Some("mp4") => "video/mp4",
        _ => "application/octet-stream",
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::CONTENT_LENGTH, metadata.len().to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        body,
    )
        .into_response())
}

/// Build an `S3Sink` from the user's stored settings (regardless of whether
/// `storage.kind` is currently set to `s3`) so we can presign GETs against
/// existing s3:// rows.
async fn build_user_s3_sink(
    state: &AppState,
    user_id: &str,
) -> Result<crate::services::s3_sink::S3Sink, ApiError> {
    let raw: Option<String> = sqlx::query_scalar::<_, Option<String>>(
        "SELECT settings_json FROM user_settings WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .flatten();

    let mut settings: serde_json::Value = raw
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::Value::Null);
    decrypt_storage_secrets(&mut settings)?;

    let storage = settings
        .get("storage")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ApiError::NotFound("No storage settings configured for this user".into()))?;

    let bucket = storage
        .get("bucket")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::NotFound("storage.bucket missing".into()))?
        .to_string();

    let region = storage
        .get("region")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let endpoint = storage
        .get("endpoint")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let prefix = storage
        .get("prefix")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let access_key_id = storage
        .get("access_key_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let secret_access_key = storage
        .get("secret_access_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let force_path_style = storage
        .get("force_path_style")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    crate::services::s3_sink::S3Sink::new(
        bucket,
        region,
        endpoint,
        prefix,
        access_key_id,
        secret_access_key,
        force_path_style,
    )
    .await
}
