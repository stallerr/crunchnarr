//! Tracked series CRUD operations.

use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TrackedSeriesRow {
    pub id: String,
    pub user_id: String,
    pub series_id: String,
    pub series_title: String,
    pub series_thumbnail: Option<String>,
    pub download_mode: String, // "new_only" | "all"
    pub baseline_episode_ids: String, // JSON array of episode IDs
    pub enabled: bool,
    pub added_at: String,
    pub last_checked_at: Option<String>,
}

pub async fn insert_tracked_series(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    series_id: &str,
    series_title: &str,
    series_thumbnail: Option<&str>,
    download_mode: &str,
    baseline_episode_ids: &str,
    added_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tracked_series \
         (id, user_id, series_id, series_title, series_thumbnail, \
          download_mode, baseline_episode_ids, enabled, added_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(series_id)
    .bind(series_title)
    .bind(series_thumbnail)
    .bind(download_mode)
    .bind(baseline_episode_ids)
    .bind(added_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_tracked_series(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<TrackedSeriesRow>, sqlx::Error> {
    sqlx::query_as::<_, TrackedSeriesRow>(
        "SELECT * FROM tracked_series WHERE user_id = ? ORDER BY added_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn get_tracked_series(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<TrackedSeriesRow>, sqlx::Error> {
    sqlx::query_as::<_, TrackedSeriesRow>(
        "SELECT * FROM tracked_series WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_by_series_id(
    pool: &SqlitePool,
    user_id: &str,
    series_id: &str,
) -> Result<Option<TrackedSeriesRow>, sqlx::Error> {
    sqlx::query_as::<_, TrackedSeriesRow>(
        "SELECT * FROM tracked_series WHERE user_id = ? AND series_id = ?",
    )
    .bind(user_id)
    .bind(series_id)
    .fetch_optional(pool)
    .await
}

/// Update mutable fields. `Ok(false)` when no row matched.
pub async fn update_tracked_series(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    download_mode: &str,
    baseline_episode_ids: &str,
    enabled: bool,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE tracked_series \
         SET download_mode = ?, baseline_episode_ids = ?, enabled = ? \
         WHERE id = ? AND user_id = ?",
    )
    .bind(download_mode)
    .bind(baseline_episode_ids)
    .bind(enabled)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_tracked_series(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM tracked_series WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// All `enabled = 1` rows across all users — used by the polling loop.
pub async fn list_all_enabled(
    pool: &SqlitePool,
) -> Result<Vec<TrackedSeriesRow>, sqlx::Error> {
    sqlx::query_as::<_, TrackedSeriesRow>(
        "SELECT * FROM tracked_series WHERE enabled = 1 ORDER BY added_at",
    )
    .fetch_all(pool)
    .await
}

pub async fn touch_last_checked(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE tracked_series SET last_checked_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
