//! Crunchyroll credential storage.

use chrono::Utc;
use sqlx::SqlitePool;

/// A Crunchyroll credentials row from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CrunchyrollCredentialRow {
    pub user_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub account_id: Option<String>,
    pub profile_id: Option<String>,
    pub device_id: String,
    pub updated_at: String,
}

/// Upsert Crunchyroll credentials for a user.
pub async fn upsert_credentials(
    pool: &SqlitePool,
    user_id: &str,
    access_token: &str,
    refresh_token: &str,
    expires_at: &str,
    account_id: Option<&str>,
    profile_id: Option<&str>,
    device_id: &str,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO crunchyroll_credentials (user_id, access_token, refresh_token, expires_at, account_id, profile_id, device_id, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET
           access_token = excluded.access_token,
           refresh_token = excluded.refresh_token,
           expires_at = excluded.expires_at,
           account_id = excluded.account_id,
           profile_id = excluded.profile_id,
           device_id = excluded.device_id,
           updated_at = excluded.updated_at"
    )
    .bind(user_id)
    .bind(access_token)
    .bind(refresh_token)
    .bind(expires_at)
    .bind(account_id)
    .bind(profile_id)
    .bind(device_id)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get Crunchyroll credentials for a user.
pub async fn get_credentials(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Option<CrunchyrollCredentialRow>, sqlx::Error> {
    sqlx::query_as::<_, CrunchyrollCredentialRow>(
        "SELECT * FROM crunchyroll_credentials WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// Delete Crunchyroll credentials for a user.
pub async fn delete_credentials(pool: &SqlitePool, user_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM crunchyroll_credentials WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
