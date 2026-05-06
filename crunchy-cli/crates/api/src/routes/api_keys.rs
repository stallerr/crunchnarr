//! API key management routes.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::api_key::{generate_api_key, hash_api_key, key_prefix};
use crate::auth::middleware::AuthUser;
use crate::db::api_keys;
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api-keys", get(list_keys))
        .route("/api-keys", post(create_key))
        .route("/api-keys/{id}", delete(revoke_key))
}

const MAX_NAME_LEN: usize = 100;

#[derive(Serialize, ToSchema)]
pub struct ApiKeyItem {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub name: String,
    /// Full key. Returned exactly once at creation — store it now.
    pub key: String,
    pub key_prefix: String,
    pub created_at: String,
}

#[utoipa::path(
    get,
    path = "/api-keys",
    responses(
        (status = 200, description = "List of the caller's API keys", body = [ApiKeyItem]),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "API Keys"
)]
async fn list_keys(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ApiKeyItem>>, ApiError> {
    let rows = api_keys::list_api_keys(&state.db, &auth.user_id).await?;
    let items = rows
        .into_iter()
        .map(|r| ApiKeyItem {
            id: r.id,
            name: r.name,
            key_prefix: r.key_prefix,
            created_at: r.created_at,
            last_used_at: r.last_used_at,
        })
        .collect();
    Ok(Json(items))
}

#[utoipa::path(
    post,
    path = "/api-keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "Key created — full key returned once", body = CreateApiKeyResponse),
        (status = 400, description = "Invalid name", body = ErrorBody),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "API Keys"
)]
async fn create_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), ApiError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("Name is required".to_string()));
    }
    if name.len() > MAX_NAME_LEN {
        return Err(ApiError::BadRequest(format!(
            "Name must be at most {} characters",
            MAX_NAME_LEN
        )));
    }

    let key = generate_api_key();
    let prefix = key_prefix(&key)
        .ok_or_else(|| ApiError::Internal("Generated key has invalid shape".to_string()))?
        .to_string();
    let hash = hash_api_key(&key);
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    api_keys::insert_api_key(
        &state.db,
        &id,
        &auth.user_id,
        name,
        &hash,
        &prefix,
        &created_at,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateApiKeyResponse {
            id,
            name: name.to_string(),
            key,
            key_prefix: prefix,
            created_at,
        }),
    ))
}

#[utoipa::path(
    delete,
    path = "/api-keys/{id}",
    params(("id" = String, Path, description = "API key id")),
    responses(
        (status = 204, description = "Key revoked"),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Key not found", body = ErrorBody),
    ),
    security(("bearer_auth" = []), ("api_key" = [])),
    tag = "API Keys"
)]
async fn revoke_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let removed = api_keys::delete_api_key(&state.db, &id, &auth.user_id).await?;
    if !removed {
        return Err(ApiError::NotFound("API key not found".to_string()));
    }
    Ok(StatusCode::NO_CONTENT)
}
