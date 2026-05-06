//! Authentication module for Crunchyroll API.

use super::client::{endpoints, CrunchyrollClient, BASIC_TOKEN, USER_AGENT};
use super::types::{CRProfile, CRToken};
use crate::error::{AuthError, Error, Result};
use crate::utils::redact;
use serde::Deserialize;
use std::time::Instant;
use tracing::{debug, info, trace};
use uuid::Uuid;

/// Device type for Android TV client
const DEVICE_TYPE: &str = "Android TV";
/// Device name for Android TV client
const DEVICE_NAME: &str = "Android TV";

/// Authentication response from the token endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    profile_id: Option<String>,
}

impl TokenResponse {
    fn into_cr_token(self) -> Result<CRToken> {
        let access_token = self
            .access_token
            .ok_or_else(|| Error::Auth(AuthError::InvalidCredentials))?;
        let refresh_token = self
            .refresh_token
            .ok_or_else(|| Error::Auth(AuthError::InvalidCredentials))?;

        Ok(CRToken {
            access_token,
            refresh_token,
            expires_in: self.expires_in.unwrap_or(0),
            token_type: self.token_type.unwrap_or_else(|| "Bearer".to_string()),
            scope: self.scope.unwrap_or_default(),
            country: self.country.unwrap_or_default(),
            account_id: self.account_id,
            profile_id: self.profile_id,
        })
    }
}

/// Authentication operations.
impl CrunchyrollClient {
    /// Generate a new device ID for authentication.
    pub fn generate_device_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Login with username and password.
    pub async fn login(&self, username: &str, password: &str) -> Result<CRToken> {
        info!("Logging in as {}", username);

        let device_id = Self::generate_device_id();
        trace!("Generated device_id: {}", device_id);
        trace!("Device type: {}, Device name: {}", DEVICE_TYPE, DEVICE_NAME);

        let form = [
            ("username", username),
            ("password", password),
            ("grant_type", "password"),
            ("scope", "offline_access"),
            ("device_id", &device_id),
            ("device_name", DEVICE_NAME),
            ("device_type", DEVICE_TYPE),
        ];

        let url = self.url(endpoints::AUTH_TOKEN);
        let start = Instant::now();
        let response = self
            .post_form_with_basic_token_and_user_agent(&url, &form, BASIC_TOKEN, USER_AGENT)
            .await?;
        let elapsed = start.elapsed();

        // Get response text first for debugging
        let response_text = response.text().await.map_err(|e| {
            Error::Api(crate::error::ApiError::InvalidResponse(format!(
                "Failed to read response body: {}",
                e
            )))
        })?;

        trace!("Token response received in {:?}", elapsed);
        trace!("Token response body: {}", redact(&response_text));

        let token_response: TokenResponse = serde_json::from_str(&response_text).map_err(|e| {
            Error::Api(crate::error::ApiError::InvalidResponse(format!(
                "Failed to parse token response: {} - Response: {}",
                e, response_text
            )))
        })?;

        let cr_token = token_response.into_cr_token()?;

        // Update the cached access token
        self.set_access_token(Some(cr_token.access_token.clone()))
            .await;

        debug!(
            "Login successful, token expires in {} seconds",
            cr_token.expires_in
        );
        trace!(
            "Access token: {}, Refresh token: {}",
            redact(&cr_token.access_token),
            redact(&cr_token.refresh_token)
        );
        if let Some(ref account_id) = cr_token.account_id {
            trace!("Account ID: {}", account_id);
        }

        Ok(cr_token)
    }

    /// Login with an existing refresh token.
    pub async fn login_with_token(&self, refresh_token: &str) -> Result<CRToken> {
        info!("Logging in with refresh token");
        trace!("Refresh token: {}", redact(refresh_token));

        let device_id = Self::generate_device_id();
        trace!("Generated device_id: {}", device_id);

        let form = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", "offline_access"),
            ("device_id", &device_id),
            ("device_name", DEVICE_NAME),
            ("device_type", DEVICE_TYPE),
        ];

        let url = self.url(endpoints::AUTH_TOKEN);
        let start = Instant::now();
        let response = self
            .post_form_with_basic_token(&url, &form, BASIC_TOKEN)
            .await
            .map_err(|e| match e {
                Error::Auth(AuthError::SessionExpired) => {
                    Error::Auth(AuthError::RefreshFailed("Token expired".to_string()))
                }
                _ => e,
            })?;
        let elapsed = start.elapsed();

        // Get response text first for debugging
        let response_text = response.text().await.map_err(|e| {
            Error::Auth(AuthError::RefreshFailed(format!(
                "Failed to read response body: {}",
                e
            )))
        })?;

        trace!("Token refresh response received in {:?}", elapsed);
        trace!("Token response body: {}", redact(&response_text));

        let token_response: TokenResponse = serde_json::from_str(&response_text).map_err(|e| {
            Error::Auth(AuthError::RefreshFailed(format!(
                "Failed to parse token response: {} - Response: {}",
                e, response_text
            )))
        })?;

        let cr_token = token_response.into_cr_token().map_err(|e| {
            Error::Auth(AuthError::RefreshFailed(format!(
                "Invalid token response: {}",
                e
            )))
        })?;

        // Update the cached access token
        self.set_access_token(Some(cr_token.access_token.clone()))
            .await;

        debug!(
            "Token refresh successful, new token expires in {} seconds",
            cr_token.expires_in
        );
        trace!(
            "New access token: {}, New refresh token: {}",
            redact(&cr_token.access_token),
            redact(&cr_token.refresh_token)
        );

        Ok(cr_token)
    }

    /// Refresh the current access token.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<CRToken> {
        self.login_with_token(refresh_token).await
    }

    /// Get the current user's profile.
    pub async fn get_profile(&self) -> Result<CRProfile> {
        debug!("Fetching user profile");
        let url = self.url(endpoints::PROFILE);
        let response = self.get(&url).await?;

        let profile: CRProfile = response.json().await.map_err(|e| {
            Error::Api(crate::error::ApiError::InvalidResponse(format!(
                "Failed to parse profile response: {}",
                e
            )))
        })?;

        trace!(
            "Profile fetched: username={}, email={}",
            profile.username,
            redact(&profile.email)
        );

        Ok(profile)
    }

    /// Clear authentication state (logout).
    pub async fn logout(&self) {
        self.set_access_token(None).await;
        info!("Logged out");
    }
}
