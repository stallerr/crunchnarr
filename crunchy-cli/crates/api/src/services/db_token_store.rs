//! Per-user `TokenStore` that persists refreshed Crunchyroll tokens to the
//! `crunchyroll_credentials` SQLite row.

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use sqlx::SqlitePool;

use crunchy_cli::api::token_store::{TokenStore, Tokens};
use crunchy_cli::error::{ConfigError, Error, Result};

use crate::db::auth as db_auth;

/// Persists refreshed tokens into `crunchyroll_credentials` for a specific user.
pub struct DbTokenStore {
    db: SqlitePool,
    user_id: String,
    device_id: String,
}

impl DbTokenStore {
    pub fn new(db: SqlitePool, user_id: String, device_id: String) -> Self {
        Self {
            db,
            user_id,
            device_id,
        }
    }
}

#[async_trait]
impl TokenStore for DbTokenStore {
    async fn save_tokens(&self, tokens: &Tokens) -> Result<()> {
        let expires_at_rfc = Utc
            .timestamp_opt(tokens.expires_at as i64, 0)
            .single()
            .unwrap_or_else(Utc::now)
            .to_rfc3339();

        db_auth::upsert_credentials(
            &self.db,
            &self.user_id,
            &tokens.access_token,
            &tokens.refresh_token,
            &expires_at_rfc,
            tokens.account_id.as_deref(),
            tokens.profile_id.as_deref(),
            &self.device_id,
        )
        .await
        .map_err(|e| Error::Config(ConfigError::WriteError(e.to_string())))?;

        Ok(())
    }
}
