//! Error types for the crunchy-cli application.
//!
//! This module defines all error types used throughout the application,
//! using `thiserror` for ergonomic error definitions.

use thiserror::Error;

/// A specialized Result type for crunchy-cli operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The main error type for the crunchy-cli application.
#[derive(Error, Debug)]
pub enum Error {
    /// API-related errors (HTTP, network, etc.)
    #[error("API error: {0}")]
    Api(#[from] ApiError),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Download errors
    #[error("Download error: {0}")]
    Download(#[from] DownloadError),

    /// Media processing errors (muxing, subtitles, etc.)
    #[error("Media error: {0}")]
    Media(#[from] MediaError),

    /// Queue management errors
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),

    /// Generic I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// TOML serialization/deserialization errors
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOML serialization errors
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// URL parsing errors
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    /// Other errors with context
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a new error with a custom message.
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self::Other(msg.into())
    }
}

/// API-related errors (HTTP, network, rate limiting, etc.)
#[derive(Error, Debug)]
pub enum ApiError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// API returned an error response
    #[error("API error {status}: {message}")]
    Response { status: u16, message: String },

    /// Rate limited by the API
    #[error("Rate limited, retry after {retry_after:?} seconds")]
    RateLimited { retry_after: Option<u64> },

    /// Cloudflare or DDoS protection detected
    #[error("DDoS protection detected: {0}")]
    DdosProtection(String),

    /// Invalid API response format
    #[error("Invalid API response: {0}")]
    InvalidResponse(String),

    /// Request timeout
    #[error("Request timed out after {0} seconds")]
    Timeout(u64),

    /// Network connectivity issue
    #[error("Network error: {0}")]
    Network(String),
}

/// Authentication-related errors
#[derive(Error, Debug)]
pub enum AuthError {
    /// Invalid credentials
    #[error("Invalid username or password")]
    InvalidCredentials,

    /// Token expired and refresh failed
    #[error("Session expired, please login again")]
    SessionExpired,

    /// Token refresh failed
    #[error("Failed to refresh token: {0}")]
    RefreshFailed(String),

    /// No credentials stored
    #[error("Not logged in. Run 'crunchy-cli login' first")]
    NotLoggedIn,

    /// Account-related issues (banned, region locked, etc.)
    #[error("Account error: {0}")]
    AccountError(String),

    /// Premium required for this content
    #[error("Premium subscription required")]
    PremiumRequired,
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Config file not found
    #[error("Config file not found at {0}")]
    NotFound(String),

    /// Config file is invalid
    #[error("Invalid config: {0}")]
    Invalid(String),

    /// Failed to write config
    #[error("Failed to write config: {0}")]
    WriteError(String),

    /// Missing required configuration value
    #[error("Missing required config: {0}")]
    MissingValue(String),

    /// Invalid configuration value
    #[error("Invalid config value for '{key}': {message}")]
    InvalidValue { key: String, message: String },

    /// Config directory could not be determined
    #[error("Could not determine config directory")]
    NoConfigDir,
}

/// Download-related errors
#[derive(Error, Debug)]
pub enum DownloadError {
    /// Content not found
    #[error("Content not found: {0}")]
    NotFound(String),

    /// Content is region locked
    #[error("Content is not available in your region")]
    RegionLocked,

    /// No streams available
    #[error("No streams available for this content")]
    NoStreams,

    /// Failed to parse manifest
    #[error("Failed to parse manifest: {0}")]
    ManifestError(String),

    /// Segment download failed
    #[error("Segment download failed: {0}")]
    SegmentFailed(String),

    /// All retries exhausted
    #[error("Download failed after {0} retries")]
    RetriesExhausted(u32),

    /// Output file already exists
    #[error("Output file already exists: {0}")]
    FileExists(String),

    /// Output directory does not exist
    #[error("Output directory does not exist: {0}")]
    OutputDirNotFound(String),

    /// Insufficient disk space
    #[error("Insufficient disk space")]
    InsufficientSpace,

    /// Download was cancelled
    #[error("Download cancelled")]
    Cancelled,
}

/// Media processing errors (FFmpeg, subtitles, decryption)
#[derive(Error, Debug)]
pub enum MediaError {
    /// External tool not found
    #[error("Tool not found: {tool}. Install it or set path in config")]
    ToolNotFound { tool: String },

    /// External tool execution failed
    #[error("Tool '{tool}' failed with exit code {code}: {stderr}")]
    ToolFailed {
        tool: String,
        code: i32,
        stderr: String,
    },

    /// Tool execution timed out
    #[error("Tool '{tool}' timed out after {timeout} seconds")]
    ToolTimeout { tool: String, timeout: u64 },

    /// Decryption failed
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// No decryption keys available
    #[error("No decryption keys available")]
    NoKeys,

    /// Muxing failed
    #[error("Muxing failed: {0}")]
    MuxingFailed(String),

    /// Subtitle conversion failed
    #[error("Subtitle conversion failed: {0}")]
    SubtitleError(String),

    /// Invalid input file
    #[error("Invalid input file: {0}")]
    InvalidInput(String),
}

/// Queue management errors
#[derive(Error, Debug)]
pub enum QueueError {
    /// Queue item not found
    #[error("Queue item not found: {0}")]
    ItemNotFound(String),

    /// Queue is empty
    #[error("Queue is empty")]
    Empty,

    /// Queue file corrupted
    #[error("Queue file corrupted: {0}")]
    Corrupted(String),

    /// Queue operation failed
    #[error("Queue operation failed: {0}")]
    OperationFailed(String),

    /// Item already in queue
    #[error("Item already in queue: {0}")]
    AlreadyExists(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::Api(ApiError::Response {
            status: 404,
            message: "Not found".to_string(),
        });
        assert_eq!(err.to_string(), "API error: API error 404: Not found");
    }

    #[test]
    fn test_auth_error() {
        let err = Error::Auth(AuthError::InvalidCredentials);
        assert_eq!(
            err.to_string(),
            "Authentication error: Invalid username or password"
        );
    }

    #[test]
    fn test_config_error() {
        let err = Error::Config(ConfigError::MissingValue("api_key".to_string()));
        assert_eq!(
            err.to_string(),
            "Configuration error: Missing required config: api_key"
        );
    }
}
