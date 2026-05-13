//! HTTP client wrapper for Crunchyroll API.

use crate::api::throttle::{acquire_all, RequestRateLimiter};
use crate::api::token_store::{FileTokenStore, TokenStore, Tokens};
use crate::config::Config;
use crate::error::{ApiError, AuthError, Error, Result};
use crate::utils::{format_bytes, format_elapsed, redact};
use reqwest::{Client, Response, StatusCode};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

/// API endpoints.
pub mod endpoints {
    pub const API_BASE: &str = "https://www.crunchyroll.com";
    pub const AUTH_TOKEN: &str = "/auth/v1/token";
    pub const PROFILE: &str = "/accounts/v1/me/profile";
    pub const SEARCH: &str = "/content/v2/discover/search";
    pub const SERIES: &str = "/content/v2/cms/series";
    pub const SEASONS: &str = "/content/v2/cms/seasons";
    pub const EPISODES: &str = "/content/v2/cms/episodes";
}

/// User agents for different client types
#[allow(dead_code)]
pub mod user_agents {
    pub const ANDROID_TV: &str = "ANDROIDTV/3.59.0 Android/16";
    pub const MOBILE: &str = "Crunchyroll/3.95.2 Android/16 okhttp/4.12.0";
    pub const IOS: &str = "Crunchyroll/4.84.0 (bundle_identifier:com.crunchyroll.iphone; build_number:4275237.324007664) iOS/26.0.0 Gravity/4.84.0";
    pub const FIREFOX: &str =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:137.0) Gecko/20100101 Firefox/137.0";
}

/// Basic auth tokens (base64 encoded client_id:client_secret)
#[allow(dead_code)]
pub mod tokens {
    pub const ANDROID_TV: &str =
        "eTJhcnZqYjBoMHJndnRpemxvdnk6SlZMdndkSXBYdnhVLXFJQnZUMU04b1FUcjFxbFFKWDI=";
    pub const MOBILE: &str =
        "cGQ2dXczZGZ5aHpnaHMwd3hhZTM6NXJ5SjJFQXR3TFc0UklIOEozaWk1anVqbnZrRWRfTkY=";
    pub const IOS: &str =
        "anJncm53ZW12d2N0dDFrbDk5cWw6ekFXY1l2UEdXVnM0bWtUcEhBWXM5LURLU3dpblV5R24=";
    pub const FIREFOX: &str =
        "eHVuaWh2ZWRidDNtYmlzdWhldnQ6MWtJUzVkeVR2akUwX3JxYUEzWWVBaDBiVVhVbXhXMTE=";
}

/// Default user agent (Android TV works best to bypass Cloudflare)
pub const USER_AGENT: &str = user_agents::ANDROID_TV;

/// Default basic auth token
pub const BASIC_TOKEN: &str = tokens::ANDROID_TV;

/// Refresh token this many seconds before it expires (proactive refresh)
const TOKEN_REFRESH_BUFFER_SECS: u64 = 60;

/// The Crunchyroll API client.
pub struct CrunchyrollClient {
    http: Client,
    config: Arc<RwLock<Config>>,
    /// Persistence backend for refreshed tokens.
    token_store: Arc<dyn TokenStore>,
    /// Current access token (cached for fast access)
    access_token: RwLock<Option<String>>,
    /// Chain of token-bucket limiters acquired in order before every
    /// outbound request (per-user, then global, etc.). Empty = unlimited.
    /// Segment-CDN downloads use a separate `reqwest::Client` and do not
    /// pass through this limiter.
    rate_limiters: Vec<Arc<RequestRateLimiter>>,
}

impl CrunchyrollClient {
    /// Create a new API client with the given configuration.
    ///
    /// Refreshed tokens are persisted to the on-disk TOML config (the CLI's
    /// default). For alternative storage (e.g. a per-user DB row), use
    /// [`CrunchyrollClient::with_token_store`].
    pub async fn new(config: Arc<RwLock<Config>>) -> Result<Self> {
        let token_store: Arc<dyn TokenStore> = Arc::new(FileTokenStore::new(config.clone()));
        Self::with_token_store(config, token_store).await
    }

    /// Create a new API client with a caller-provided token-persistence
    /// backend.
    pub async fn with_token_store(
        config: Arc<RwLock<Config>>,
        token_store: Arc<dyn TokenStore>,
    ) -> Result<Self> {
        Self::with_token_store_and_rate_limiters(config, token_store, Vec::new()).await
    }

    /// Like [`with_token_store`] but installs a chain of [`RequestRateLimiter`]s
    /// that are acquired in order before every outbound request. Used by the
    /// API server crate to layer per-user + global caps.
    ///
    /// [`with_token_store`]: Self::with_token_store
    pub async fn with_token_store_and_rate_limiters(
        config: Arc<RwLock<Config>>,
        token_store: Arc<dyn TokenStore>,
        rate_limiters: Vec<Arc<RequestRateLimiter>>,
    ) -> Result<Self> {
        let config_read = config.read().await;

        let mut builder = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .cookie_store(true);

        // Configure proxy if enabled
        if config_read.proxy.enabled {
            if let Some(ref proxy_url) = config_read.proxy.url {
                debug!("Using proxy: {}", proxy_url);
                let proxy = reqwest::Proxy::all(proxy_url)
                    .map_err(|e| Error::other(format!("Invalid proxy URL: {}", e)))?;
                builder = builder.proxy(proxy);
            }
        }

        let http = builder
            .build()
            .map_err(|e| Error::other(format!("Failed to create HTTP client: {}", e)))?;

        // Get cached access token if available
        let access_token = config_read.auth.access_token.clone();
        let has_token = access_token.is_some();
        drop(config_read);

        trace!(
            "API client initialized (has_token: {}, timeout: 30s, connect_timeout: 10s)",
            has_token
        );

        Ok(Self {
            http,
            config,
            token_store,
            access_token: RwLock::new(access_token),
            rate_limiters,
        })
    }

    /// Acquire one token from each rate limiter in the chain. Inlined ahead
    /// of every outbound request — segment CDN traffic uses a separate
    /// reqwest client and bypasses this code path.
    async fn throttle(&self) {
        if !self.rate_limiters.is_empty() {
            acquire_all(&self.rate_limiters).await;
        }
    }

    /// Get the base URL for API requests.
    fn base_url(&self) -> &str {
        endpoints::API_BASE
    }

    /// Build a URL for an API endpoint.
    pub fn url(&self, endpoint: &str) -> String {
        format!("{}{}", self.base_url(), endpoint)
    }

    /// Check if the user is authenticated.
    pub async fn is_authenticated(&self) -> bool {
        self.access_token.read().await.is_some()
    }

    /// Get the current access token.
    pub async fn get_access_token(&self) -> Option<String> {
        self.access_token.read().await.clone()
    }

    /// Set a new access token.
    pub async fn set_access_token(&self, token: Option<String>) {
        *self.access_token.write().await = token;
    }

    /// Ensure we have a valid (non-expired) access token.
    ///
    /// This performs proactive token refresh if the token is expired or will
    /// expire within `TOKEN_REFRESH_BUFFER_SECS` seconds.
    ///
    /// Returns `Ok(true)` if the token was refreshed, `Ok(false)` if no refresh was needed.
    async fn ensure_valid_token(&self) -> Result<bool> {
        // Check if token is expired or expiring soon
        let (needs_refresh, refresh_token) = {
            let config = self.config.read().await;
            let needs_refresh = config.is_token_expired(TOKEN_REFRESH_BUFFER_SECS);
            let refresh_token = config.auth.refresh_token.clone();
            (needs_refresh, refresh_token)
        };

        if !needs_refresh {
            return Ok(false);
        }

        // Get the refresh token or return error if not available
        let refresh_token = refresh_token.ok_or(Error::Auth(AuthError::NotLoggedIn))?;

        info!("Access token expired or expiring soon, refreshing...");
        trace!("Using refresh token: {}", redact(&refresh_token));
        self.perform_token_refresh(&refresh_token).await?;

        Ok(true)
    }

    /// Attempt to refresh the token and update both in-memory cache and config.
    ///
    /// This is used both for proactive refresh and reactive refresh (on 401).
    async fn perform_token_refresh(&self, refresh_token: &str) -> Result<()> {
        // Use login_with_token to refresh (this also updates in-memory cache)
        let new_token = self.login_with_token(refresh_token).await?;

        // Update in-memory config so subsequent reads see fresh tokens, then
        // hand the bundle to the persistence backend.
        let tokens = {
            let mut config = self.config.write().await;
            config.set_tokens(
                new_token.access_token.clone(),
                new_token.refresh_token.clone(),
                new_token.expires_in,
                new_token.account_id.clone(),
                new_token.profile_id.clone(),
            );
            Tokens {
                access_token: new_token.access_token,
                refresh_token: new_token.refresh_token,
                expires_at: config.auth.expires_at.unwrap_or(0),
                account_id: new_token.account_id,
                profile_id: new_token.profile_id,
            }
        };

        self.token_store.save_tokens(&tokens).await.map_err(|e| {
            warn!("Failed to persist refreshed tokens: {}", e);
            e
        })?;

        info!("Token refreshed and saved successfully");
        Ok(())
    }

    /// Make an authenticated GET request.
    ///
    /// This method automatically handles token refresh:
    /// - Proactively refreshes if the token is expired or expiring soon
    /// - Reactively refreshes and retries on 401 Unauthorized responses
    pub async fn get(&self, url: &str) -> Result<Response> {
        self.throttle().await;

        // Proactive refresh: ensure token is valid before making request
        self.ensure_valid_token().await?;

        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(Error::Auth(AuthError::NotLoggedIn))?;

        debug!("GET {}", url);
        trace!("Authorization: Bearer {}", redact(&token));

        let start = Instant::now();
        let response = self
            .http
            .get(url)
            .bearer_auth(&token)
            .header("Accept", "*/*")
            .header("Accept-Encoding", "gzip, deflate, br")
            .send()
            .await
            .map_err(ApiError::Request)?;

        let elapsed = start.elapsed();
        let status = response.status();
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        trace!(
            "GET {} -> {} ({}) {}",
            url,
            status.as_u16(),
            format_elapsed(elapsed),
            content_length.map_or("".to_string(), |len| format_bytes(len))
        );

        // Reactive refresh: if we get 401, try to refresh and retry once
        if response.status() == StatusCode::UNAUTHORIZED {
            debug!("Got 401 Unauthorized, attempting token refresh and retry");

            // Get refresh token from config
            let refresh_token = {
                let config = self.config.read().await;
                config.auth.refresh_token.clone()
            };

            if let Some(rt) = refresh_token {
                // Try to refresh - if this fails, we'll return the original 401 error
                if self.perform_token_refresh(&rt).await.is_ok() {
                    // Get the new token and retry the request
                    let new_token = self
                        .access_token
                        .read()
                        .await
                        .clone()
                        .ok_or(Error::Auth(AuthError::NotLoggedIn))?;

                    debug!("Retrying GET {} with refreshed token", url);

                    let retry_response = self
                        .http
                        .get(url)
                        .bearer_auth(&new_token)
                        .send()
                        .await
                        .map_err(ApiError::Request)?;

                    return self.handle_response(retry_response).await;
                }
            }

            // If refresh failed or no refresh token, handle the original 401 response
            return self.handle_response(response).await;
        }

        self.handle_response(response).await
    }

    /// Make an unauthenticated GET request.
    pub async fn get_anonymous(&self, url: &str) -> Result<Response> {
        self.throttle().await;
        debug!("GET (anonymous) {}", url);

        let start = Instant::now();
        let response = self.http.get(url).send().await.map_err(ApiError::Request)?;
        let elapsed = start.elapsed();
        let status = response.status();

        trace!(
            "GET (anonymous) {} -> {} ({})",
            url,
            status.as_u16(),
            format_elapsed(elapsed)
        );

        self.handle_response(response).await
    }

    /// Make an authenticated POST request with form data.
    pub async fn post_form<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        form: &T,
    ) -> Result<Response> {
        self.throttle().await;
        debug!("POST {}", url);

        let start = Instant::now();
        let response = self
            .http
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(form)
            .send()
            .await
            .map_err(ApiError::Request)?;
        let elapsed = start.elapsed();

        trace!("POST {} -> {} ({})", url, response.status().as_u16(), format_elapsed(elapsed));

        self.handle_response(response).await
    }

    /// Make an authenticated POST request with form data and basic auth.
    pub async fn post_form_with_basic_auth<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        form: &T,
        client_id: &str,
        client_secret: &str,
    ) -> Result<Response> {
        self.throttle().await;
        debug!("POST {} (with basic auth)", url);
        trace!("Basic auth client_id: {}", redact(client_id));

        let start = Instant::now();
        let response = self
            .http
            .post(url)
            .basic_auth(client_id, Some(client_secret))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(form)
            .send()
            .await
            .map_err(ApiError::Request)?;
        let elapsed = start.elapsed();

        trace!("POST {} -> {} ({})", url, response.status().as_u16(), format_elapsed(elapsed));

        self.handle_response(response).await
    }

    /// Make a POST request with form data and a pre-encoded basic auth token.
    /// This is used for Crunchyroll's API which expects `Basic <base64_token>`.
    pub async fn post_form_with_basic_token<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        form: &T,
        basic_token: &str,
    ) -> Result<Response> {
        self.throttle().await;
        debug!("POST {} (with basic token)", url);
        trace!("Basic token: {}", redact(basic_token));

        let start = Instant::now();
        let response = self
            .http
            .post(url)
            .header("Authorization", format!("Basic {}", basic_token))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(form)
            .send()
            .await
            .map_err(ApiError::Request)?;
        let elapsed = start.elapsed();

        trace!("POST {} -> {} ({})", url, response.status().as_u16(), format_elapsed(elapsed));

        self.handle_response(response).await
    }

    /// Make a POST request with form data and a pre-encoded basic auth token and user agent.
    /// This is used for Crunchyroll's API which expects `Basic <base64_token>`.
    pub async fn post_form_with_basic_token_and_user_agent<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        form: &T,
        basic_token: &str,
        user_agent: &str,
    ) -> Result<Response> {
        self.throttle().await;
        debug!("POST {} (with basic token)", url);
        trace!("User-Agent: {}", user_agent);

        let start = Instant::now();
        let response = self
            .http
            .post(url)
            .header("Authorization", format!("Basic {}", basic_token))
            .header("User-Agent", user_agent)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(form)
            .send()
            .await
            .map_err(ApiError::Request)?;
        let elapsed = start.elapsed();

        trace!("POST {} -> {} ({})", url, response.status().as_u16(), format_elapsed(elapsed));

        self.handle_response(response).await
    }

    /// Make an authenticated PATCH request.
    ///
    /// This method automatically handles token refresh:
    /// - Proactively refreshes if the token is expired or expiring soon
    /// - Reactively refreshes and retries on 401 Unauthorized responses
    pub async fn patch(&self, url: &str) -> Result<Response> {
        self.throttle().await;

        // Proactive refresh: ensure token is valid before making request
        self.ensure_valid_token().await?;

        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(Error::Auth(AuthError::NotLoggedIn))?;

        debug!("PATCH {}", url);

        let start = Instant::now();
        let response = self
            .http
            .patch(url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(ApiError::Request)?;
        let elapsed = start.elapsed();

        trace!("PATCH {} -> {} ({})", url, response.status().as_u16(), format_elapsed(elapsed));

        // Reactive refresh: if we get 401, try to refresh and retry once
        if response.status() == StatusCode::UNAUTHORIZED {
            debug!("Got 401 Unauthorized, attempting token refresh and retry");

            // Get refresh token from config
            let refresh_token = {
                let config = self.config.read().await;
                config.auth.refresh_token.clone()
            };

            if let Some(rt) = refresh_token {
                // Try to refresh - if this fails, we'll return the original 401 error
                if self.perform_token_refresh(&rt).await.is_ok() {
                    // Get the new token and retry the request
                    let new_token = self
                        .access_token
                        .read()
                        .await
                        .clone()
                        .ok_or(Error::Auth(AuthError::NotLoggedIn))?;

                    debug!("Retrying PATCH {} with refreshed token", url);

                    let retry_response = self
                        .http
                        .patch(url)
                        .bearer_auth(&new_token)
                        .send()
                        .await
                        .map_err(ApiError::Request)?;

                    return self.handle_response(retry_response).await;
                }
            }

            // If refresh failed or no refresh token, handle the original 401 response
            return self.handle_response(response).await;
        }

        self.handle_response(response).await
    }

    /// Make an authenticated DELETE request.
    ///
    /// Same shape as [`patch`](Self::patch): proactive refresh, reactive
    /// refresh + retry on 401, and a request-rate-limiter acquire up front.
    pub async fn delete(&self, url: &str) -> Result<Response> {
        self.throttle().await;
        self.ensure_valid_token().await?;

        let token = self
            .access_token
            .read()
            .await
            .clone()
            .ok_or(Error::Auth(AuthError::NotLoggedIn))?;

        debug!("DELETE {}", url);

        let start = Instant::now();
        let response = self
            .http
            .delete(url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(ApiError::Request)?;
        let elapsed = start.elapsed();

        trace!("DELETE {} -> {} ({})", url, response.status().as_u16(), format_elapsed(elapsed));

        if response.status() == StatusCode::UNAUTHORIZED {
            debug!("Got 401 Unauthorized, attempting token refresh and retry");
            let refresh_token = {
                let config = self.config.read().await;
                config.auth.refresh_token.clone()
            };
            if let Some(rt) = refresh_token {
                if self.perform_token_refresh(&rt).await.is_ok() {
                    let new_token = self
                        .access_token
                        .read()
                        .await
                        .clone()
                        .ok_or(Error::Auth(AuthError::NotLoggedIn))?;
                    debug!("Retrying DELETE {} with refreshed token", url);
                    let retry_response = self
                        .http
                        .delete(url)
                        .bearer_auth(&new_token)
                        .send()
                        .await
                        .map_err(ApiError::Request)?;
                    return self.handle_response(retry_response).await;
                }
            }
            return self.handle_response(response).await;
        }

        self.handle_response(response).await
    }

    /// Handle API response, checking for errors.
    async fn handle_response(&self, response: Response) -> Result<Response> {
        let status = response.status();

        // Check for Cloudflare/DDoS protection
        if let Some(server) = response.headers().get("server") {
            if let Ok(server_str) = server.to_str() {
                if server_str.contains("cloudflare") && status == StatusCode::FORBIDDEN {
                    return Err(Error::Api(ApiError::DdosProtection(
                        "Cloudflare protection detected".to_string(),
                    )));
                }
            }
        }

        // Check for DDoS-Guard
        if response.headers().contains_key("x-ddos-protection") {
            warn!("DDoS protection header detected");
        }

        match status {
            StatusCode::OK | StatusCode::CREATED | StatusCode::NO_CONTENT => Ok(response),
            StatusCode::UNAUTHORIZED => {
                let text = response.text().await.unwrap_or_default();
                debug!("401 response body: {}", text);
                if text.contains("invalid_grant") || text.contains("invalid_client") {
                    Err(Error::Auth(AuthError::InvalidCredentials))
                } else {
                    Err(Error::Auth(AuthError::SessionExpired))
                }
            }
            StatusCode::FORBIDDEN => {
                let text = response.text().await.unwrap_or_default();
                if text.contains("premium") || text.contains("subscription") {
                    Err(Error::Auth(AuthError::PremiumRequired))
                } else {
                    Err(Error::Api(ApiError::Response {
                        status: status.as_u16(),
                        message: text,
                    }))
                }
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok());
                warn!(
                    "Rate limited (429). Retry-After: {:?}",
                    retry_after
                );
                Err(Error::Api(ApiError::RateLimited { retry_after }))
            }
            StatusCode::NOT_FOUND => {
                let text = response.text().await.unwrap_or_default();
                Err(Error::Api(ApiError::Response {
                    status: 404,
                    message: if text.is_empty() {
                        "Not found".to_string()
                    } else {
                        text
                    },
                }))
            }
            _ => {
                let text = response.text().await.unwrap_or_default();
                Err(Error::Api(ApiError::Response {
                    status: status.as_u16(),
                    message: text,
                }))
            }
        }
    }

    /// Get the raw HTTP client for special cases.
    pub fn http(&self) -> &Client {
        &self.http
    }

    /// Get the basic auth token for authentication.
    pub fn basic_token(&self) -> &str {
        BASIC_TOKEN
    }
}
