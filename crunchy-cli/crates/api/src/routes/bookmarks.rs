//! Bookmark routes — saved series per user.

use axum::extract::{Path, State};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use chrono::Utc;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tracing::warn;
use utoipa::ToSchema;

use crate::auth::middleware::AuthUser;
use crate::db::bookmarks;
use crate::error::{ApiError, ErrorBody};
use crate::services::crunchyroll::CrunchyrollService;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/bookmarks", get(list_bookmarks))
        .route("/bookmarks", post(create_bookmark))
        .route("/bookmarks/{series_id}", delete(remove_bookmark))
        .route("/bookmarks/{series_id}", patch(update_bookmark_note))
}

const MAX_NOTE_LEN: usize = 1000;

#[derive(Serialize, ToSchema)]
pub struct BookmarkItem {
    pub series_id: String,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
    /// `null` if the CR fetch failed (deleted/region-locked series).
    pub series: Option<serde_json::Value>,
}

#[derive(Serialize, ToSchema)]
pub struct Bookmark {
    pub user_id: String,
    pub series_id: String,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateBookmarkRequest {
    pub series_id: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateBookmarkRequest {
    pub note: String,
}

fn validate_note(note: &str) -> Result<(), ApiError> {
    if note.len() > MAX_NOTE_LEN {
        return Err(ApiError::BadRequest(format!(
            "Note must be at most {} characters",
            MAX_NOTE_LEN
        )));
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/bookmarks",
    responses(
        (status = 200, description = "Bookmarked series with hydrated CR metadata", body = [BookmarkItem]),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Bookmarks"
)]
async fn list_bookmarks(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<BookmarkItem>>, ApiError> {
    let rows = bookmarks::list_bookmarks(&state.db, &auth.user_id).await?;
    if rows.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let cr_service = CrunchyrollService::new(state.db.clone());
    let client = cr_service.get_client(&auth.user_id).await?;

    let fetches = rows.iter().map(|row| {
        let client = &client;
        async move {
            match client.get_series(&row.series_id).await {
                Ok(series) => Some(serde_json::to_value(&series).unwrap_or(serde_json::Value::Null)),
                Err(e) => {
                    warn!(
                        "Failed to hydrate bookmark {}: {}",
                        row.series_id, e
                    );
                    None
                }
            }
        }
    });
    let hydrated = join_all(fetches).await;

    let items = rows
        .into_iter()
        .zip(hydrated)
        .map(|(row, series)| BookmarkItem {
            series_id: row.series_id,
            note: row.note,
            created_at: row.created_at,
            updated_at: row.updated_at,
            series,
        })
        .collect();

    Ok(Json(items))
}

#[utoipa::path(
    post,
    path = "/bookmarks",
    request_body = CreateBookmarkRequest,
    responses(
        (status = 200, description = "Bookmark created or updated", body = Bookmark),
        (status = 400, description = "Invalid input", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Bookmarks"
)]
async fn create_bookmark(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateBookmarkRequest>,
) -> Result<Json<Bookmark>, ApiError> {
    let series_id = req.series_id.trim();
    if series_id.is_empty() {
        return Err(ApiError::BadRequest("series_id is required".to_string()));
    }
    validate_note(&req.note)?;

    let now = Utc::now().to_rfc3339();
    let row = bookmarks::upsert_bookmark(&state.db, &auth.user_id, series_id, &req.note, &now)
        .await?;

    Ok(Json(Bookmark {
        user_id: row.user_id,
        series_id: row.series_id,
        note: row.note,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

#[utoipa::path(
    delete,
    path = "/bookmarks/{series_id}",
    params(("series_id" = String, Path, description = "Series ID")),
    responses(
        (status = 204, description = "Bookmark removed"),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Bookmark not found", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Bookmarks"
)]
async fn remove_bookmark(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(series_id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let removed = bookmarks::delete_bookmark(&state.db, &auth.user_id, &series_id).await?;
    if !removed {
        return Err(ApiError::NotFound("Bookmark not found".to_string()));
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    patch,
    path = "/bookmarks/{series_id}",
    params(("series_id" = String, Path, description = "Series ID")),
    request_body = UpdateBookmarkRequest,
    responses(
        (status = 200, description = "Note updated", body = Bookmark),
        (status = 400, description = "Invalid input", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Bookmark not found", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "Bookmarks"
)]
async fn update_bookmark_note(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(series_id): Path<String>,
    Json(req): Json<UpdateBookmarkRequest>,
) -> Result<Json<Bookmark>, ApiError> {
    validate_note(&req.note)?;

    let now = Utc::now().to_rfc3339();
    let row = bookmarks::update_note(&state.db, &auth.user_id, &series_id, &req.note, &now)
        .await?
        .ok_or_else(|| ApiError::NotFound("Bookmark not found".to_string()))?;

    Ok(Json(Bookmark {
        user_id: row.user_id,
        series_id: row.series_id,
        note: row.note,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}
