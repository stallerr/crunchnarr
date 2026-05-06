//! API key CRUD operations.

use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKeyRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

pub async fn insert_api_key(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    name: &str,
    key_hash: &str,
    key_prefix: &str,
    created_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO api_keys (id, user_id, name, key_hash, key_prefix, created_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(key_hash)
    .bind(key_prefix)
    .bind(created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_api_keys(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<ApiKeyRow>, sqlx::Error> {
    sqlx::query_as::<_, ApiKeyRow>(
        "SELECT * FROM api_keys WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn get_api_key_by_hash(
    pool: &SqlitePool,
    key_hash: &str,
) -> Result<Option<ApiKeyRow>, sqlx::Error> {
    sqlx::query_as::<_, ApiKeyRow>("SELECT * FROM api_keys WHERE key_hash = ?")
        .bind(key_hash)
        .fetch_optional(pool)
        .await
}

/// Returns `Ok(false)` when no row matched (either wrong id or wrong user).
pub async fn delete_api_key(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM api_keys WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn touch_api_key(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
