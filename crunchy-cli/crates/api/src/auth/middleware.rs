//! Axum extractor for authenticated users.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::auth::api_key::hash_api_key;
use crate::auth::jwt::decode_token;
use crate::db::api_keys;
use crate::state::AppState;

/// Authenticated user extracted from JWT or API key.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
}

/// Error returned when authentication fails.
pub struct AuthError(String);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "unauthorized",
                "detail": self.0
            })),
        )
            .into_response()
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 1. Try JWT via Authorization: Bearer <token>
        if let Some(auth_header) = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
        {
            let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
                AuthError("Invalid Authorization header format".to_string())
            })?;

            let claims = decode_token(token, &state.config.jwt_secret)
                .map_err(|e| AuthError(format!("Invalid token: {}", e)))?;

            if claims.token_type != "access" {
                return Err(AuthError("Invalid token type".to_string()));
            }

            return Ok(AuthUser {
                user_id: claims.sub,
            });
        }

        // 2. Try API key via X-Api-Key
        if let Some(raw_key) = parts.headers.get("X-Api-Key").and_then(|v| v.to_str().ok()) {
            let hash = hash_api_key(raw_key);
            let row = api_keys::get_api_key_by_hash(&state.db, &hash)
                .await
                .map_err(|e| AuthError(format!("DB error: {}", e)))?
                .ok_or_else(|| AuthError("Invalid API key".to_string()))?;

            // Fire-and-forget last_used_at update — don't block the request.
            let pool = state.db.clone();
            let id = row.id.clone();
            tokio::spawn(async move {
                let _ = api_keys::touch_api_key(&pool, &id).await;
            });

            return Ok(AuthUser {
                user_id: row.user_id,
            });
        }

        Err(AuthError("Missing Authorization or X-Api-Key header".to_string()))
    }
}
