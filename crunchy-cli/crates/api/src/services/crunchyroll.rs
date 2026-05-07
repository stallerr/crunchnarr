//! Per-user Crunchyroll client factory.
//!
//! Creates `CrunchyrollClient` instances from DB-stored credentials,
//! handles token refresh writeback, and chains a per-user + global
//! request-rate limiter onto every outbound CR-API call.

use crate::db::auth;
use crate::error::ApiError;
use crate::services::db_token_store::DbTokenStore;
use chrono::Utc;
use crunchy_cli::api::token_store::TokenStore;
use crunchy_cli::api::{CrunchyrollClient, RequestRateLimiter};
use crunchy_cli::config::Config;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex, RwLock};

/// Process-wide rate-limiter pool. Initialized once from environment at
/// API server startup; afterwards, every `get_client` call layers the
/// caller's per-user bucket on top of the global bucket.
static RATE_LIMITER_POOL: OnceLock<RateLimiterPool> = OnceLock::new();

/// Per-user + global request-rate caps for outbound Crunchyroll API calls.
///
/// CR throttles per-account, so per-user caps prevent one user's bulk add
/// from starving another. CR (or the Cloudflare WAF in front of it) also
/// throttles per-IP, so a global cap protects multi-tenant deployments
/// that share a single egress IP.
pub struct RateLimiterPool {
    global: Arc<RequestRateLimiter>,
    per_user: Mutex<HashMap<String, Arc<RequestRateLimiter>>>,
    per_user_rps: u32,
    per_user_burst: u32,
}

impl RateLimiterPool {
    /// Initialize the process-wide pool. Must be called once at startup
    /// before any `get_client` invocation. Subsequent calls are no-ops.
    pub fn init(
        per_user_rps: u32,
        per_user_burst: u32,
        global_rps: u32,
        global_burst: u32,
    ) {
        let _ = RATE_LIMITER_POOL.set(RateLimiterPool {
            global: RequestRateLimiter::new(global_rps, global_burst),
            per_user: Mutex::new(HashMap::new()),
            per_user_rps,
            per_user_burst,
        });
    }

    /// Build the limiter chain (per-user, then global) for `user_id`. The
    /// per-user bucket is created lazily on first use and reused on
    /// subsequent calls for the same user.
    async fn chain_for(&self, user_id: &str) -> Vec<Arc<RequestRateLimiter>> {
        let user_limiter = {
            let mut map = self.per_user.lock().await;
            map.entry(user_id.to_string())
                .or_insert_with(|| {
                    RequestRateLimiter::new(self.per_user_rps, self.per_user_burst)
                })
                .clone()
        };
        vec![user_limiter, self.global.clone()]
    }
}

/// Build the limiter chain for `user_id` if the pool has been initialized.
/// Falls back to an empty (unlimited) chain when running in test/CLI
/// contexts that didn't call `RateLimiterPool::init`.
async fn limiter_chain_for(user_id: &str) -> Vec<Arc<RequestRateLimiter>> {
    match RATE_LIMITER_POOL.get() {
        Some(pool) => pool.chain_for(user_id).await,
        None => Vec::new(),
    }
}

/// Service for managing per-user Crunchyroll clients.
pub struct CrunchyrollService {
    db: SqlitePool,
}

/// Result of a Crunchyroll login.
pub struct CrLoginResult {
    pub account_id: Option<String>,
}

impl CrunchyrollService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Get a `CrunchyrollClient` for the given user, using stored credentials.
    pub async fn get_client(&self, user_id: &str) -> Result<CrunchyrollClient, ApiError> {
        let creds = auth::get_credentials(&self.db, user_id)
            .await?
            .ok_or_else(|| {
                ApiError::Unauthorized(
                    "Crunchyroll account not linked. POST /crunchyroll/login first.".to_string(),
                )
            })?;

        // Build a config with the user's stored tokens
        let mut config = Config::default();
        let device_id = creds.device_id.clone();
        config.auth.device_id = Some(creds.device_id);
        config.auth.access_token = creds.access_token;
        config.auth.refresh_token = creds.refresh_token;
        if let Some(ref expires_at) = creds.expires_at {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                config.auth.expires_at = Some(dt.timestamp() as u64);
            }
        }
        config.auth.account_id = creds.account_id;
        config.auth.profile_id = creds.profile_id;

        let config = Arc::new(RwLock::new(config));
        let token_store: Arc<dyn TokenStore> = Arc::new(DbTokenStore::new(
            self.db.clone(),
            user_id.to_string(),
            device_id,
        ));
        let limiters = limiter_chain_for(user_id).await;
        let client = CrunchyrollClient::with_token_store_and_rate_limiters(
            config,
            token_store,
            limiters,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create CR client: {}", e)))?;

        Ok(client)
    }

    /// Login to Crunchyroll and store credentials.
    ///
    /// Login uses an unlimited client — the user has no bucket yet, and
    /// rate-limiting the auth endpoint here doesn't help (one-shot per user).
    /// A login-route-level throttle in `routes/crunchyroll.rs` is the right
    /// tool against malicious frontends; out of scope for this layer.
    pub async fn login(
        &self,
        user_id: &str,
        username: Option<&str>,
        password: Option<&str>,
        refresh_token: Option<&str>,
    ) -> Result<CrLoginResult, ApiError> {
        // Create a temporary config for login
        let config = Arc::new(RwLock::new(Config::default()));
        let client = CrunchyrollClient::new(config.clone())
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to create CR client: {}", e)))?;

        let token = if let Some(rt) = refresh_token {
            client
                .login_with_token(rt)
                .await
                .map_err(|e| ApiError::Unauthorized(format!("CR login failed: {}", e)))?
        } else {
            let username = username
                .ok_or_else(|| ApiError::BadRequest("Username required".to_string()))?;
            let password = password
                .ok_or_else(|| ApiError::BadRequest("Password required".to_string()))?;
            client
                .login(username, password)
                .await
                .map_err(|e| ApiError::Unauthorized(format!("CR login failed: {}", e)))?
        };

        // Compute expiration time
        let expires_at = Utc::now() + chrono::Duration::seconds(token.expires_in as i64);

        // Get device ID from config
        let device_id = {
            let cfg = config.read().await;
            cfg.auth
                .device_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        };

        // Store credentials
        auth::upsert_credentials(
            &self.db,
            user_id,
            &token.access_token,
            &token.refresh_token,
            &expires_at.to_rfc3339(),
            token.account_id.as_deref(),
            token.profile_id.as_deref(),
            &device_id,
        )
        .await?;

        Ok(CrLoginResult {
            account_id: token.account_id,
        })
    }
}
