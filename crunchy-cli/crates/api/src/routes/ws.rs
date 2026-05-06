//! WebSocket endpoint for real-time events.

use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures::StreamExt;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::auth::jwt::decode_token;
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

#[derive(Deserialize, IntoParams)]
pub struct WsParams {
    /// JWT access token (browsers can't set WS headers)
    token: String,
}

#[utoipa::path(
    get,
    path = "/ws",
    params(WsParams),
    responses(
        (status = 101, description = "WebSocket upgrade"),
        (status = 401, description = "Invalid token", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "WebSocket"
)]
async fn ws_handler(
    State(state): State<AppState>,
    Query(params): Query<WsParams>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ApiError> {
    // Authenticate via query param (browsers can't set WS headers)
    let claims = decode_token(&params.token, &state.config.jwt_secret)?;

    if claims.token_type != "access" {
        return Err(ApiError::Unauthorized("Invalid token type".to_string()));
    }

    let user_id = claims.sub;
    let broadcaster = state.ws_broadcaster.clone();

    Ok(ws.on_upgrade(move |socket| async move {
        let (sender, mut receiver) = socket.split();

        // Register this connection
        broadcaster.add_connection(&user_id, sender).await;

        // Handle incoming messages
        while let Some(Ok(msg)) = receiver.next().await {
            if let axum::extract::ws::Message::Text(text) = msg {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some("ping") = parsed.get("type").and_then(|t| t.as_str()) {
                        broadcaster
                            .send_to_user(
                                &user_id,
                                &serde_json::json!({"type": "pong"}),
                            )
                            .await;
                    }
                }
            }
        }

        // Connection closed - remove it
        broadcaster.remove_connection(&user_id).await;
    }))
}
