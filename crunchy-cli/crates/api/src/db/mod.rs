//! Database module - SQLite pool setup and migrations.

pub mod api_keys;
pub mod auth;
pub mod bookmarks;
pub mod tracking;
pub mod users;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use tracing::info;

/// Initialize the database pool and run migrations.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .pragma("foreign_keys", "ON");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    run_migrations(&pool).await?;

    info!("Database initialized: {}", database_url);
    Ok(pool)
}

/// Run SQL migrations.
async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let migration_sql = include_str!("migrations/001_initial.sql");
    sqlx::raw_sql(migration_sql).execute(pool).await?;
    let migration_002 = include_str!("migrations/002_user_settings.sql");
    sqlx::raw_sql(migration_002).execute(pool).await?;
    // ALTER TABLE ADD COLUMN is not idempotent in SQLite, so ignore "duplicate column" errors
    let migration_003 = include_str!("migrations/003_download_thumbnail.sql");
    if let Err(e) = sqlx::raw_sql(migration_003).execute(pool).await {
        let msg = e.to_string();
        if !msg.contains("duplicate column") {
            return Err(e);
        }
    }
    let migration_004 = include_str!("migrations/004_download_publish_failed.sql");
    if let Err(e) = sqlx::raw_sql(migration_004).execute(pool).await {
        let msg = e.to_string();
        if !msg.contains("duplicate column") {
            return Err(e);
        }
    }
    let migration_005 = include_str!("migrations/005_api_keys.sql");
    sqlx::raw_sql(migration_005).execute(pool).await?;
    let migration_006 = include_str!("migrations/006_drop_download_queue.sql");
    sqlx::raw_sql(migration_006).execute(pool).await?;
    let migration_007 = include_str!("migrations/007_bookmarks.sql");
    sqlx::raw_sql(migration_007).execute(pool).await?;
    let migration_008 = include_str!("migrations/008_tracking.sql");
    sqlx::raw_sql(migration_008).execute(pool).await?;
    let migration_009 = include_str!("migrations/009_downloads_tracking_columns.sql");
    if let Err(e) = sqlx::raw_sql(migration_009).execute(pool).await {
        let msg = e.to_string();
        if !msg.contains("duplicate column") {
            return Err(e);
        }
    }
    let migration_010 = include_str!("migrations/010_downloads_manual.sql");
    if let Err(e) = sqlx::raw_sql(migration_010).execute(pool).await {
        let msg = e.to_string();
        if !msg.contains("duplicate column") {
            return Err(e);
        }
    }
    info!("Database migrations applied");
    Ok(())
}
