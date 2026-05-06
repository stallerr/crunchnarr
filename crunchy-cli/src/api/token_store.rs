//! Persistence interface for refreshed Crunchyroll tokens.
//!
//! `CrunchyrollClient` calls into a `TokenStore` after refreshing tokens, so
//! the persistence backend (TOML config file vs API server's per-user DB row)
//! is a strategy rather than hard-coded.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::error::Result;

/// A refreshed token bundle to persist.
#[derive(Debug, Clone)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix-epoch seconds at which the access token expires.
    pub expires_at: u64,
    pub account_id: Option<String>,
    pub profile_id: Option<String>,
}

/// Strategy for persisting refreshed Crunchyroll tokens.
#[async_trait]
pub trait TokenStore: Send + Sync {
    async fn save_tokens(&self, tokens: &Tokens) -> Result<()>;
}

/// Persists tokens to the on-disk TOML config (CLI default).
///
/// Assumes the in-memory `Config` has already been updated by the caller.
pub struct FileTokenStore {
    config: Arc<RwLock<Config>>,
}

impl FileTokenStore {
    pub fn new(config: Arc<RwLock<Config>>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl TokenStore for FileTokenStore {
    async fn save_tokens(&self, _tokens: &Tokens) -> Result<()> {
        let cfg = self.config.read().await;
        cfg.save()
    }
}
