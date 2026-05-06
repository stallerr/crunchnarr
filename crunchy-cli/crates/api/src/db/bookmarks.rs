//! Bookmark CRUD operations.

use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BookmarkRow {
    pub user_id: String,
    pub series_id: String,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list_bookmarks(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<BookmarkRow>, sqlx::Error> {
    sqlx::query_as::<_, BookmarkRow>(
        "SELECT * FROM bookmarks WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Insert or update a bookmark. `created_at` is preserved on conflict;
/// `updated_at` always advances. Used for both new bookmarks and note edits
/// triggered through `POST /bookmarks` (idempotent).
pub async fn upsert_bookmark(
    pool: &SqlitePool,
    user_id: &str,
    series_id: &str,
    note: &str,
    now: &str,
) -> Result<BookmarkRow, sqlx::Error> {
    sqlx::query(
        "INSERT INTO bookmarks (user_id, series_id, note, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(user_id, series_id) DO UPDATE SET \
            note = excluded.note, \
            updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind(series_id)
    .bind(note)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, BookmarkRow>(
        "SELECT * FROM bookmarks WHERE user_id = ? AND series_id = ?",
    )
    .bind(user_id)
    .bind(series_id)
    .fetch_one(pool)
    .await
}

pub async fn delete_bookmark(
    pool: &SqlitePool,
    user_id: &str,
    series_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM bookmarks WHERE user_id = ? AND series_id = ?")
        .bind(user_id)
        .bind(series_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Update only the note. Returns `Ok(None)` when no bookmark matched.
pub async fn update_note(
    pool: &SqlitePool,
    user_id: &str,
    series_id: &str,
    note: &str,
    now: &str,
) -> Result<Option<BookmarkRow>, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE bookmarks SET note = ?, updated_at = ? \
         WHERE user_id = ? AND series_id = ?",
    )
    .bind(note)
    .bind(now)
    .bind(user_id)
    .bind(series_id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    sqlx::query_as::<_, BookmarkRow>(
        "SELECT * FROM bookmarks WHERE user_id = ? AND series_id = ?",
    )
    .bind(user_id)
    .bind(series_id)
    .fetch_optional(pool)
    .await
}
