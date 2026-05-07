//! Server configuration.

use std::path::PathBuf;

/// Configuration for the API server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Port to listen on.
    pub port: u16,
    /// Host to bind to.
    pub host: String,
    /// Path to SQLite database file.
    pub database_url: String,
    /// JWT signing secret.
    pub jwt_secret: String,
    /// Access token TTL in seconds (default: 1 hour).
    pub access_token_ttl: u64,
    /// Refresh token TTL in seconds (default: 30 days).
    pub refresh_token_ttl: u64,
    /// Downloads output directory.
    pub downloads_dir: PathBuf,
    /// Watchlist polling interval in seconds (default: 1 hour).
    pub tracking_interval_secs: u64,
    /// Crunchyroll API request rate cap, per linked CR account (default: 3 RPS).
    pub cr_rate_limit_rps: u32,
    /// Crunchyroll API per-account burst capacity (default: 6).
    pub cr_rate_limit_burst: u32,
    /// Crunchyroll API request rate cap aggregated across all users in this
    /// process (default: 15 RPS). Protects against multi-tenant deployments
    /// breaching CR's per-IP throttle.
    pub cr_rate_limit_global_rps: u32,
    /// Global burst capacity (default: 30).
    pub cr_rate_limit_global_burst: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "0.0.0.0".to_string(),
            database_url: "sqlite:crunchy-api.db?mode=rwc".to_string(),
            jwt_secret: "change-me-in-production".to_string(),
            access_token_ttl: 3600,
            refresh_token_ttl: 30 * 24 * 3600,
            downloads_dir: dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("./downloads"))
                .join("Crunchyroll"),
            tracking_interval_secs: 3600,
            cr_rate_limit_rps: 3,
            cr_rate_limit_burst: 6,
            cr_rate_limit_global_rps: 15,
            cr_rate_limit_global_burst: 30,
        }
    }
}

impl ServerConfig {
    /// Load config from environment variables, falling back to defaults.
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.port),
            host: std::env::var("HOST").unwrap_or(default.host),
            database_url: std::env::var("DATABASE_URL").unwrap_or(default.database_url),
            jwt_secret: std::env::var("JWT_SECRET").unwrap_or(default.jwt_secret),
            access_token_ttl: std::env::var("ACCESS_TOKEN_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.access_token_ttl),
            refresh_token_ttl: std::env::var("REFRESH_TOKEN_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.refresh_token_ttl),
            downloads_dir: std::env::var("DOWNLOADS_DIR")
                .map(PathBuf::from)
                .unwrap_or(default.downloads_dir),
            tracking_interval_secs: std::env::var("TRACKING_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.tracking_interval_secs),
            cr_rate_limit_rps: std::env::var("CRUNCHYROLL_RATE_LIMIT_RPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.cr_rate_limit_rps),
            cr_rate_limit_burst: std::env::var("CRUNCHYROLL_RATE_LIMIT_BURST")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.cr_rate_limit_burst),
            cr_rate_limit_global_rps: std::env::var("CRUNCHYROLL_RATE_LIMIT_GLOBAL_RPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.cr_rate_limit_global_rps),
            cr_rate_limit_global_burst: std::env::var("CRUNCHYROLL_RATE_LIMIT_GLOBAL_BURST")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default.cr_rate_limit_global_burst),
        }
    }
}
