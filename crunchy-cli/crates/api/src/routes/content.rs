//! Content endpoints: series, seasons, episodes.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::auth::middleware::AuthUser;
use crate::error::{ApiError, ErrorBody};
use crate::services::crunchyroll::CrunchyrollService;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/series/{id}", get(get_series))
        .route("/series/{id}/seasons", get(get_seasons))
        .route("/seasons/{season_id}/episodes", get(get_episodes))
        .route("/episodes/{id}", get(get_episode))
}

#[utoipa::path(
    get,
    path = "/series/{id}",
    params(("id" = String, Path, description = "Series ID")),
    responses(
        (status = 200, description = "Series details", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Series not found", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Content"
)]
async fn get_series(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = CrunchyrollService::new(state.db.clone());
    let client = service.get_client(&auth.user_id).await?;
    let series = client
        .get_series(&id)
        .await
        ?;
    Ok(Json(serde_json::to_value(&series).unwrap()))
}

#[utoipa::path(
    get,
    path = "/series/{id}/seasons",
    params(("id" = String, Path, description = "Series ID")),
    responses(
        (status = 200, description = "List of seasons for the series", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Series not found", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Content"
)]
async fn get_seasons(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = CrunchyrollService::new(state.db.clone());
    let client = service.get_client(&auth.user_id).await?;
    let seasons = client
        .get_seasons(&id)
        .await
        ?;
    Ok(Json(serde_json::to_value(&seasons).unwrap()))
}

#[utoipa::path(
    get,
    path = "/seasons/{season_id}/episodes",
    params(("season_id" = String, Path, description = "Season ID")),
    responses(
        (status = 200, description = "List of episodes for the season", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Season not found", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Content"
)]
async fn get_episodes(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(season_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = CrunchyrollService::new(state.db.clone());
    let client = service.get_client(&auth.user_id).await?;
    let episodes = client
        .get_episodes(&season_id)
        .await
        ?;
    Ok(Json(serde_json::to_value(&episodes).unwrap()))
}

#[utoipa::path(
    get,
    path = "/episodes/{id}",
    params(("id" = String, Path, description = "Episode ID")),
    responses(
        (status = 200, description = "Episode details", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Episode not found", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Content"
)]
async fn get_episode(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = CrunchyrollService::new(state.db.clone());
    let client = service.get_client(&auth.user_id).await?;
    let episode = client
        .get_episode(&id)
        .await
        ?;
    Ok(Json(serde_json::to_value(&episode).unwrap()))
}
