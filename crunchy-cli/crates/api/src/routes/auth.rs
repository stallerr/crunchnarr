//! Authentication routes: register, login, refresh, me.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::middleware::AuthUser;
use crate::auth::{create_access_token, create_refresh_token, decode_token};
use crate::db::users;
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/me", get(me))
}

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RefreshRequest {
    refresh_token: String,
}

#[derive(Serialize, ToSchema)]
pub struct AuthResponse {
    access_token: String,
    refresh_token: String,
    token_type: String,
    expires_in: u64,
}

#[derive(Serialize, ToSchema)]
pub struct UserResponse {
    id: String,
    username: String,
    email: String,
    created_at: String,
}

#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registration successful", body = AuthResponse),
        (status = 400, description = "Invalid input", body = ErrorBody),
        (status = 409, description = "Email or username already taken", body = ErrorBody),
    ),
    tag = "Auth"
)]
async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Validate input
    if req.username.is_empty() || req.email.is_empty() || req.password.is_empty() {
        return Err(ApiError::BadRequest(
            "Username, email, and password are required".to_string(),
        ));
    }

    if req.password.len() < 8 {
        return Err(ApiError::BadRequest(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Check if user already exists
    if users::find_by_email(&state.db, &req.email)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict("Email already registered".to_string()));
    }

    if users::find_by_username(&state.db, &req.username)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict("Username already taken".to_string()));
    }

    // Hash password
    let password_hash = hash_password(&req.password)?;

    // Create user
    let user_id = users::create_user(&state.db, &req.username, &req.email, &password_hash).await?;

    // Generate tokens
    let access_token =
        create_access_token(&user_id, &state.config.jwt_secret, state.config.access_token_ttl)
            .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))?;
    let refresh_token =
        create_refresh_token(&user_id, &state.config.jwt_secret, state.config.refresh_token_ttl)
            .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.access_token_ttl,
    }))
}

#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials", body = ErrorBody),
    ),
    tag = "Auth"
)]
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let user = users::find_by_email(&state.db, &req.email)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid email or password".to_string()))?;

    // Verify password
    if !verify_password(&req.password, &user.password_hash)? {
        return Err(ApiError::Unauthorized(
            "Invalid email or password".to_string(),
        ));
    }

    // Generate tokens
    let access_token =
        create_access_token(&user.id, &state.config.jwt_secret, state.config.access_token_ttl)
            .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))?;
    let refresh_token =
        create_refresh_token(&user.id, &state.config.jwt_secret, state.config.refresh_token_ttl)
            .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.access_token_ttl,
    }))
}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Tokens refreshed", body = AuthResponse),
        (status = 401, description = "Invalid or expired refresh token", body = ErrorBody),
    ),
    tag = "Auth"
)]
async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let claims = decode_token(&req.refresh_token, &state.config.jwt_secret)?;

    if claims.token_type != "refresh" {
        return Err(ApiError::Unauthorized("Invalid token type".to_string()));
    }

    // Verify user still exists
    users::find_by_id(&state.db, &claims.sub)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    // Generate new tokens
    let access_token =
        create_access_token(&claims.sub, &state.config.jwt_secret, state.config.access_token_ttl)
            .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))?;
    let new_refresh_token = create_refresh_token(
        &claims.sub,
        &state.config.jwt_secret,
        state.config.refresh_token_ttl,
    )
    .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token: new_refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.access_token_ttl,
    }))
}

#[utoipa::path(
    get,
    path = "/auth/me",
    responses(
        (status = 200, description = "Current user profile", body = UserResponse),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
    tag = "Auth"
)]
async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<UserResponse>, ApiError> {
    let user = users::find_by_id(&state.db, &auth.user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok(Json(UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        created_at: user.created_at,
    }))
}

/// Hash a password using Argon2.
fn hash_password(password: &str) -> Result<String, ApiError> {
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2, PasswordHasher,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(format!("Failed to hash password: {}", e)))
}

/// Verify a password against a hash.
fn verify_password(password: &str, hash: &str) -> Result<bool, ApiError> {
    use argon2::{
        password_hash::PasswordHash, Argon2, PasswordVerifier,
    };

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| ApiError::Internal(format!("Invalid password hash: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
