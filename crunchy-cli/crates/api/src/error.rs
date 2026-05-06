//! API error types and HTTP status mapping.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// API error type that maps to HTTP responses.
#[derive(Debug)]
pub enum ApiError {
    /// 400 Bad Request
    BadRequest(String),
    /// 401 Unauthorized
    Unauthorized(String),
    /// 403 Forbidden
    Forbidden(String),
    /// 404 Not Found
    NotFound(String),
    /// 409 Conflict
    Conflict(String),
    /// 422 Unprocessable Entity
    UnprocessableEntity(String),
    /// 429 Too Many Requests
    RateLimited { retry_after: Option<u64> },
    /// 500 Internal Server Error
    Internal(String),
}

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, detail) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", Some(msg)),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", Some(msg)),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", Some(msg)),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", Some(msg)),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", Some(msg)),
            ApiError::UnprocessableEntity(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "unprocessable_entity", Some(msg))
            }
            ApiError::RateLimited { retry_after } => {
                let mut resp = (
                    StatusCode::TOO_MANY_REQUESTS,
                    axum::Json(ErrorBody {
                        error: "rate_limited".to_string(),
                        detail: retry_after.map(|s| format!("Retry after {} seconds", s)),
                    }),
                )
                    .into_response();
                if let Some(secs) = retry_after {
                    resp.headers_mut().insert(
                        "Retry-After",
                        secs.to_string().parse().unwrap(),
                    );
                }
                return resp;
            }
            ApiError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error", Some(msg))
            }
        };

        (
            status,
            axum::Json(ErrorBody {
                error: error.to_string(),
                detail,
            }),
        )
            .into_response()
    }
}

/// Map crunchy-cli library errors to API errors.
impl From<crunchy_cli::Error> for ApiError {
    fn from(err: crunchy_cli::Error) -> Self {
        use crunchy_cli::error as cli_err;
        match &err {
            crunchy_cli::Error::Auth(cli_err::AuthError::NotLoggedIn) => {
                ApiError::Unauthorized("Not logged in".to_string())
            }
            crunchy_cli::Error::Auth(cli_err::AuthError::InvalidCredentials) => {
                ApiError::Unauthorized("Invalid credentials".to_string())
            }
            crunchy_cli::Error::Auth(cli_err::AuthError::PremiumRequired) => {
                ApiError::Forbidden("Premium subscription required".to_string())
            }
            crunchy_cli::Error::Api(cli_err::ApiError::RateLimited { retry_after }) => {
                ApiError::RateLimited {
                    retry_after: *retry_after,
                }
            }
            crunchy_cli::Error::Download(cli_err::DownloadError::NotFound(msg)) => {
                ApiError::NotFound(msg.clone())
            }
            crunchy_cli::Error::Download(cli_err::DownloadError::RegionLocked) => {
                ApiError::Forbidden("Content is not available in your region".to_string())
            }
            crunchy_cli::Error::Config(cli_err::ConfigError::InvalidValue { key, message }) => {
                ApiError::UnprocessableEntity(format!("{}: {}", key, message))
            }
            crunchy_cli::Error::Queue(cli_err::QueueError::AlreadyExists(msg)) => {
                ApiError::Conflict(msg.clone())
            }
            crunchy_cli::Error::Queue(cli_err::QueueError::ItemNotFound(msg)) => {
                ApiError::NotFound(msg.clone())
            }
            crunchy_cli::Error::Api(cli_err::ApiError::DdosProtection(msg)) => {
                ApiError::BadRequest(format!("DDoS protection detected: {}", msg))
            }
            _ => {
                tracing::error!("Unmapped crunchy-cli error: {:?}", err);
                ApiError::Internal(err.to_string())
            }
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Database error: {}", err);
        ApiError::Internal("Database error".to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        ApiError::Unauthorized(format!("Invalid token: {}", err))
    }
}
