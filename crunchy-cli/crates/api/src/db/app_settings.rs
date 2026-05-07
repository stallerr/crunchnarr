//! Server-wide settings — single row keyed at id=1.

use sqlx::SqlitePool;

/// Lower bound for `tracking_interval_secs`. 5 minutes — anything smaller is
/// almost certainly an accident and would hammer Crunchyroll.
pub const MIN_TRACKING_INTERVAL_SECS: u64 = 300;
/// Upper bound for `tracking_interval_secs`. 24 hours.
pub const MAX_TRACKING_INTERVAL_SECS: u64 = 86_400;

/// Read the raw settings JSON. `{}` on a fresh row.
pub async fn get_settings_json(pool: &SqlitePool) -> Result<serde_json::Value, sqlx::Error> {
    let row: Option<String> =
        sqlx::query_scalar("SELECT settings_json FROM app_settings WHERE id = 1")
            .fetch_optional(pool)
            .await?;
    let raw = row.unwrap_or_else(|| "{}".to_string());
    Ok(serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({})))
}

/// Merge `updates` over the existing settings and persist.
pub async fn merge_settings(
    pool: &SqlitePool,
    updates: &serde_json::Value,
) -> Result<serde_json::Value, sqlx::Error> {
    let mut current = get_settings_json(pool).await?;
    if let (Some(base), Some(overlay)) = (current.as_object_mut(), updates.as_object()) {
        for (k, v) in overlay {
            base.insert(k.clone(), v.clone());
        }
    }
    let now = chrono::Utc::now().to_rfc3339();
    let serialized = serde_json::to_string(&current).unwrap_or_else(|_| "{}".to_string());
    sqlx::query(
        "UPDATE app_settings SET settings_json = ?, updated_at = ? WHERE id = 1",
    )
    .bind(&serialized)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(current)
}

/// Read `tracking_interval_secs`, clamped to `[MIN, MAX]`. Returns `None`
/// when not set so callers can fall back to the env-var default.
pub async fn get_tracking_interval_secs(pool: &SqlitePool) -> Option<u64> {
    let json = get_settings_json(pool).await.ok()?;
    let raw = json.get("tracking_interval_secs")?.as_u64()?;
    Some(raw.clamp(MIN_TRACKING_INTERVAL_SECS, MAX_TRACKING_INTERVAL_SECS))
}
