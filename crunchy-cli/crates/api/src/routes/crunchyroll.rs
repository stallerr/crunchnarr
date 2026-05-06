//! Crunchyroll account linking routes.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::middleware::AuthUser;
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/crunchyroll/login", post(cr_login))
        .route("/crunchyroll/logout", post(cr_logout))
        .route("/crunchyroll/whoami", get(cr_whoami))
}

#[derive(Deserialize, ToSchema)]
pub struct CrLoginRequest {
    /// Crunchyroll username (optional if using refresh_token)
    #[serde(default)]
    username: Option<String>,
    /// Crunchyroll password (optional if using refresh_token)
    #[serde(default)]
    password: Option<String>,
    /// Crunchyroll refresh token (alternative to username/password)
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CrLoginResponse {
    status: String,
    account_id: Option<String>,
}

#[utoipa::path(
    post,
    path = "/crunchyroll/login",
    request_body = CrLoginRequest,
    responses(
        (status = 200, description = "Crunchyroll login successful", body = CrLoginResponse),
        (status = 401, description = "Invalid Crunchyroll credentials", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Crunchyroll"
)]
async fn cr_login(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrLoginRequest>,
) -> Result<Json<CrLoginResponse>, ApiError> {
    use crate::services::crunchyroll::CrunchyrollService;

    let service = CrunchyrollService::new(state.db.clone());
    let result = service.login(&auth.user_id, req.username.as_deref(), req.password.as_deref(), req.refresh_token.as_deref()).await?;

    Ok(Json(CrLoginResponse {
        status: "ok".to_string(),
        account_id: result.account_id,
    }))
}

#[utoipa::path(
    post,
    path = "/crunchyroll/logout",
    responses(
        (status = 200, description = "Crunchyroll session cleared", body = Object),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Crunchyroll"
)]
async fn cr_logout(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    crate::db::auth::delete_credentials(&state.db, &auth.user_id).await?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

#[utoipa::path(
    get,
    path = "/crunchyroll/whoami",
    responses(
        (status = 200, description = "Current Crunchyroll profile", body = Object),
        (status = 401, description = "Not authenticated or not linked", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Crunchyroll"
)]
async fn cr_whoami(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    use crate::services::crunchyroll::CrunchyrollService;

    let service = CrunchyrollService::new(state.db.clone());
    let client = service.get_client(&auth.user_id).await?;
    let profile = client.get_profile().await?;
    Ok(Json(serde_json::to_value(&profile).unwrap()))
}
