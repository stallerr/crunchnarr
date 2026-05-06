//! Search endpoints.

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::auth::middleware::AuthUser;
use crate::error::{ApiError, ErrorBody};
use crate::services::crunchyroll::CrunchyrollService;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/search", get(search))
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct SearchParams {
    /// Search query string
    q: String,
    /// Max number of results (default: 10)
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    10
}

#[utoipa::path(
    get,
    path = "/search",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results from Crunchyroll", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Search"
)]
async fn search(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<SearchParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let service = CrunchyrollService::new(state.db.clone());
    let client = service.get_client(&auth.user_id).await?;

    let results = client.search(&params.q, params.limit).await?;

    Ok(Json(serde_json::to_value(&results).unwrap()))
}
