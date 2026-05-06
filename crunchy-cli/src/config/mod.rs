//! Configuration management module.
//!
//! Handles loading, saving, and validation of application configuration.

mod display;

pub use display::display_pretty;
pub use display::print_config_keys;

use crate::error::{ConfigError, Error, Result};
use crate::utils::expand_path;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info, trace};

/// Application name for config directory.
const APP_NAME: &str = "crunchy-cli";
/// Config file name.
const CONFIG_FILE: &str = "config.toml";
/// Queue file name.
const QUEUE_FILE: &str = "queue.json";

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Authentication settings.
    pub auth: AuthConfig,
    /// Download settings.
    pub downloads: DownloadConfig,
    /// Language preferences.
    pub languages: LanguageConfig,
    /// Muxing settings.
    pub muxing: MuxingConfig,
    /// External tool paths.
    pub tools: ToolsConfig,
    /// Proxy settings.
    pub proxy: ProxyConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auth: AuthConfig::default(),
            downloads: DownloadConfig::default(),
            languages: LanguageConfig::default(),
            muxing: MuxingConfig::default(),
            tools: ToolsConfig::default(),
            proxy: ProxyConfig::default(),
        }
    }
}

/// Authentication configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Device UUID for API authentication.
    pub device_id: Option<String>,
    /// Current access token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// Refresh token for obtaining new access tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Token expiration timestamp (Unix epoch).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// Account ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// Profile ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
}

/// Download settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadConfig {
    /// Output directory for downloads.
    pub output_dir: PathBuf,
    /// Temporary directory for partial downloads.
    pub temp_dir: PathBuf,
    /// Cache directory for resumable downloads (None = use temp_dir).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,
    /// Cache retention in hours (0 = keep forever).
    pub cache_retention_hours: u32,
    /// Maximum download speed in KB/s (0 = unlimited).
    pub max_speed_kbps: u32,
    /// Number of simultaneous downloads.
    pub simultaneous: u8,
    /// Number of parallel segment downloads per file.
    pub parts: u8,
    /// Preferred video quality.
    pub video_quality: String,
    /// Preferred audio quality.
    pub audio_quality: String,
    /// Retry count for failed downloads.
    pub retry_count: u8,
    /// Maximum concurrent key acquisitions (0 = unlimited).
    pub max_concurrent_keys: u8,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        let downloads_dir = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("~/Downloads"))
            .join("Crunchyroll");

        Self {
            output_dir: downloads_dir,
            temp_dir: std::env::temp_dir().join("crunchy-cli"),
            cache_dir: None, // Use temp_dir by default
            cache_retention_hours: 24,
            max_speed_kbps: 0,
            simultaneous: 2,
            parts: 10,
            video_quality: "best".to_string(),
            audio_quality: "best".to_string(),
            retry_count: 3,
            max_concurrent_keys: 4,
        }
    }
}

impl DownloadConfig {
    /// Get the effective cache directory.
    ///
    /// Returns `cache_dir` if set, otherwise falls back to `temp_dir`.
    pub fn get_cache_dir(&self) -> PathBuf {
        self.cache_dir
            .clone()
            .unwrap_or_else(|| self.temp_dir.clone())
    }
}

/// Language preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageConfig {
    /// Preferred audio languages (in order of preference).
    pub audio: Vec<String>,
    /// Preferred subtitle languages (in order of preference).
    pub subtitles: Vec<String>,
    /// Whether to include closed caption (CC) subtitles.
    pub include_cc: bool,
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self {
            audio: vec!["ja-JP".to_string()],
            subtitles: vec!["en-US".to_string()],
            include_cc: false,
        }
    }
}

/// Muxing settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MuxingConfig {
    /// Output format (mkv, mp4).
    pub format: String,
    /// Whether to embed subtitles in the output file.
    pub embed_subs: bool,
    /// Default audio track language.
    pub default_audio: String,
    /// Default subtitle track language.
    pub default_sub: String,
    /// Prefer Signs & Songs sub when default audio and sub are the same language.
    pub prefer_signs_songs: bool,
    /// Output filename template.
    pub filename_template: String,
}

impl Default for MuxingConfig {
    fn default() -> Self {
        Self {
            format: "mkv".to_string(),
            embed_subs: true,
            default_audio: "ja-JP".to_string(),
            default_sub: "en-US".to_string(),
            prefer_signs_songs: false,
            filename_template: "{series}/S{season:02}E{episode:02} - {title}".to_string(),
        }
    }
}

/// External tool paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolsConfig {
    /// Path to mp4decrypt binary.
    pub mp4decrypt: String,
    /// Path to Widevine client ID file.
    pub widevine_client: Option<PathBuf>,
    /// Path to Widevine private key file.
    pub widevine_private_key: Option<PathBuf>,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            mp4decrypt: "mp4decrypt".to_string(),
            widevine_client: None,
            widevine_private_key: None,
        }
    }
}

/// Proxy settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxyConfig {
    /// Whether proxy is enabled.
    pub enabled: bool,
    /// Proxy URL (e.g., "http://127.0.0.1:8080" or "socks5://127.0.0.1:1080").
    pub url: Option<String>,
}

impl Config {
    /// Get the default config directory path.
    pub fn default_config_dir() -> Result<PathBuf> {
        ProjectDirs::from("", "", APP_NAME)
            .map(|p| p.config_dir().to_path_buf())
            .ok_or(Error::Config(ConfigError::NoConfigDir))
    }

    /// Get the default config file path.
    pub fn default_config_path() -> Result<PathBuf> {
        Ok(Self::default_config_dir()?.join(CONFIG_FILE))
    }

    /// Get the queue file path.
    pub fn queue_path() -> Result<PathBuf> {
        Ok(Self::default_config_dir()?.join(QUEUE_FILE))
    }

    /// Load configuration from the default path.
    pub fn load() -> Result<Self> {
        let path = Self::default_config_path()?;
        Self::load_from(&path)
    }

    /// Load configuration from a specific path.
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            debug!("Config file not found at {:?}, using defaults", path);
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::Config(ConfigError::Invalid(format!(
                "Failed to read config: {}",
                e
            )))
        })?;

        let config: Config = toml::from_str(&content)?;

        debug!("Loaded config from {:?}", path);
        trace!(
            "Config: video_quality={}, audio={:?}, subs={:?}, simultaneous={}",
            config.downloads.video_quality,
            config.languages.audio,
            config.languages.subtitles,
            config.downloads.simultaneous
        );

        Ok(config)
    }

    /// Save configuration to the default path.
    pub fn save(&self) -> Result<()> {
        let path = Self::default_config_path()?;
        self.save_to(&path)
    }

    /// Save configuration to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Config(ConfigError::WriteError(format!(
                    "Failed to create config directory: {}",
                    e
                )))
            })?;
        }

        let content = toml::to_string_pretty(self)?;

        // Write to temp file first, then rename (atomic write)
        let temp_path = path.with_extension("toml.tmp");
        std::fs::write(&temp_path, &content).map_err(|e| {
            Error::Config(ConfigError::WriteError(format!(
                "Failed to write config: {}",
                e
            )))
        })?;

        std::fs::rename(&temp_path, path).map_err(|e| {
            Error::Config(ConfigError::WriteError(format!(
                "Failed to save config: {}",
                e
            )))
        })?;

        info!("Saved config to {:?}", path);

        Ok(())
    }

    /// Initialize a new config file with defaults.
    pub fn init(force: bool) -> Result<PathBuf> {
        let path = Self::default_config_path()?;

        if path.exists() && !force {
            return Err(Error::Config(ConfigError::Invalid(format!(
                "Config file already exists at {:?}. Use --force to overwrite.",
                path
            ))));
        }

        let config = Self::default();
        config.save_to(&path)?;

        info!("Initialized config at {:?}", path);

        Ok(path)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        // Validate video quality
        let valid_qualities = ["best", "1080p", "720p", "480p", "360p", "240p"];
        if !valid_qualities.contains(&self.downloads.video_quality.as_str()) {
            return Err(Error::Config(ConfigError::InvalidValue {
                key: "downloads.video_quality".to_string(),
                message: format!(
                    "Invalid quality '{}', expected one of: {:?}",
                    self.downloads.video_quality, valid_qualities
                ),
            }));
        }

        // Validate muxing format
        let valid_formats = ["mkv", "mp4"];
        if !valid_formats.contains(&self.muxing.format.as_str()) {
            return Err(Error::Config(ConfigError::InvalidValue {
                key: "muxing.format".to_string(),
                message: format!(
                    "Invalid format '{}', expected one of: {:?}",
                    self.muxing.format, valid_formats
                ),
            }));
        }

        // Validate proxy URL if enabled
        if self.proxy.enabled && self.proxy.url.is_none() {
            return Err(Error::Config(ConfigError::MissingValue(
                "proxy.url".to_string(),
            )));
        }

        Ok(())
    }

    /// Check if authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.auth.access_token.is_some() && self.auth.refresh_token.is_some()
    }

    /// Check if the access token is expired or will expire within buffer_seconds.
    ///
    /// Returns `true` if the token should be refreshed (expired, expiring soon, or no expiration info).
    pub fn is_token_expired(&self, buffer_seconds: u64) -> bool {
        match self.auth.expires_at {
            Some(expires_at) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                now + buffer_seconds >= expires_at
            }
            // No expiration time stored means we should try to refresh
            None => self.auth.refresh_token.is_some(),
        }
    }

    /// Update authentication tokens.
    pub fn set_tokens(
        &mut self,
        access_token: String,
        refresh_token: String,
        expires_in: u64,
        account_id: Option<String>,
        profile_id: Option<String>,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.auth.access_token = Some(access_token);
        self.auth.refresh_token = Some(refresh_token);
        self.auth.expires_at = Some(now + expires_in);
        self.auth.account_id = account_id;
        self.auth.profile_id = profile_id;
    }

    /// Clear authentication tokens.
    pub fn clear_tokens(&mut self) {
        self.auth.access_token = None;
        self.auth.refresh_token = None;
        self.auth.expires_at = None;
        self.auth.account_id = None;
        self.auth.profile_id = None;
    }

    /// Set a configuration value by dot-separated key path.
    ///
    /// Accepts string values and coerces them to the appropriate type.
    /// Comma-separated values are used for list fields (e.g. `"ja-JP,en-US"`).
    pub fn set_key(&mut self, key: &str, value: &str) -> Result<()> {
        let parse_err = |e: &dyn std::fmt::Display| {
            Error::Config(ConfigError::InvalidValue {
                key: key.to_string(),
                message: e.to_string(),
            })
        };

        match key {
            // tools
            "tools.mp4decrypt" => self.tools.mp4decrypt = value.to_string(),
            "tools.widevine_client" => self.tools.widevine_client = Some(PathBuf::from(value)),
            "tools.widevine_private_key" => {
                self.tools.widevine_private_key = Some(PathBuf::from(value))
            }

            // downloads
            "downloads.output_dir" => self.downloads.output_dir = PathBuf::from(expand_path(value)),
            "downloads.temp_dir" => self.downloads.temp_dir = PathBuf::from(expand_path(value)),
            "downloads.cache_dir" => {
                self.downloads.cache_dir = Some(PathBuf::from(expand_path(value)))
            }
            "downloads.video_quality" => self.downloads.video_quality = value.to_string(),
            "downloads.audio_quality" => self.downloads.audio_quality = value.to_string(),
            "downloads.max_speed_kbps" => {
                self.downloads.max_speed_kbps = value.parse().map_err(|e| parse_err(&e))?
            }
            "downloads.simultaneous" => {
                self.downloads.simultaneous = value.parse().map_err(|e| parse_err(&e))?
            }
            "downloads.parts" => {
                self.downloads.parts = value.parse().map_err(|e| parse_err(&e))?
            }
            "downloads.retry_count" => {
                self.downloads.retry_count = value.parse().map_err(|e| parse_err(&e))?
            }
            "downloads.cache_retention_hours" => {
                self.downloads.cache_retention_hours = value.parse().map_err(|e| parse_err(&e))?
            }
            "downloads.max_concurrent_keys" => {
                self.downloads.max_concurrent_keys = value.parse().map_err(|e| parse_err(&e))?
            }

            // muxing
            "muxing.format" => self.muxing.format = value.to_string(),
            "muxing.embed_subs" => {
                self.muxing.embed_subs = value.parse().map_err(|e| parse_err(&e))?
            }
            "muxing.default_audio" => self.muxing.default_audio = value.to_string(),
            "muxing.default_sub" => self.muxing.default_sub = value.to_string(),
            "muxing.prefer_signs_songs" => {
                self.muxing.prefer_signs_songs = value.parse().map_err(|e| parse_err(&e))?
            }
            "muxing.filename_template" => self.muxing.filename_template = value.to_string(),

            // languages (comma-separated: "ja-JP,en-US")
            "languages.audio" => {
                self.languages.audio =
                    value.split(',').map(str::trim).map(String::from).collect()
            }
            "languages.subtitles" => {
                self.languages.subtitles =
                    value.split(',').map(str::trim).map(String::from).collect()
            }
            "languages.include_cc" => {
                self.languages.include_cc = value.parse().map_err(|e| parse_err(&e))?
            }

            // proxy
            "proxy.enabled" => {
                self.proxy.enabled = value.parse().map_err(|e| parse_err(&e))?
            }
            "proxy.url" => self.proxy.url = Some(value.to_string()),

            _ => {
                return Err(Error::Config(ConfigError::InvalidValue {
                    key: key.to_string(),
                    message: format!("unknown config key '{}'", key),
                }))
            }
        }

        self.validate()?;
        Ok(())
    }
    /// Get a configuration value by dot-separated key path.
    ///
    /// Returns the value as a string representation.
    pub fn get_key(&self, key: &str) -> Result<String> {
        let value = match key {
            // tools
            "tools.mp4decrypt" => self.tools.mp4decrypt.clone(),
            "tools.widevine_client" => match &self.tools.widevine_client {
                Some(p) => p.display().to_string(),
                None => String::new(),
            },
            "tools.widevine_private_key" => match &self.tools.widevine_private_key {
                Some(p) => p.display().to_string(),
                None => String::new(),
            },

            // downloads
            "downloads.output_dir" => self.downloads.output_dir.display().to_string(),
            "downloads.temp_dir" => self.downloads.temp_dir.display().to_string(),
            "downloads.cache_dir" => match &self.downloads.cache_dir {
                Some(p) => p.display().to_string(),
                None => String::new(),
            },
            "downloads.cache_retention_hours" => self.downloads.cache_retention_hours.to_string(),
            "downloads.video_quality" => self.downloads.video_quality.clone(),
            "downloads.audio_quality" => self.downloads.audio_quality.clone(),
            "downloads.max_speed_kbps" => self.downloads.max_speed_kbps.to_string(),
            "downloads.simultaneous" => self.downloads.simultaneous.to_string(),
            "downloads.parts" => self.downloads.parts.to_string(),
            "downloads.retry_count" => self.downloads.retry_count.to_string(),
            "downloads.max_concurrent_keys" => self.downloads.max_concurrent_keys.to_string(),

            // muxing
            "muxing.format" => self.muxing.format.clone(),
            "muxing.embed_subs" => self.muxing.embed_subs.to_string(),
            "muxing.default_audio" => self.muxing.default_audio.clone(),
            "muxing.default_sub" => self.muxing.default_sub.clone(),
            "muxing.prefer_signs_songs" => self.muxing.prefer_signs_songs.to_string(),
            "muxing.filename_template" => self.muxing.filename_template.clone(),

            // languages
            "languages.audio" => self.languages.audio.join(","),
            "languages.subtitles" => self.languages.subtitles.join(","),
            "languages.include_cc" => self.languages.include_cc.to_string(),

            // proxy
            "proxy.enabled" => self.proxy.enabled.to_string(),
            "proxy.url" => self.proxy.url.clone().unwrap_or_default(),

            _ => {
                return Err(Error::Config(ConfigError::InvalidValue {
                    key: key.to_string(),
                    message: format!("unknown config key '{}'", key),
                }))
            }
        };

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.downloads.simultaneous, 2);
        assert_eq!(config.downloads.video_quality, "best");
        assert_eq!(config.muxing.format, "mkv");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml = toml::to_string_pretty(&config).unwrap();
        assert!(toml.contains("[auth]"));
        assert!(toml.contains("[downloads]"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml = r#"
            [auth]
            device_id = "test-device"

            [downloads]
            simultaneous = 4
            video_quality = "1080p"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.auth.device_id, Some("test-device".to_string()));
        assert_eq!(config.downloads.simultaneous, 4);
        assert_eq!(config.downloads.video_quality, "1080p");
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.downloads.video_quality = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_is_authenticated() {
        let mut config = Config::default();
        assert!(!config.is_authenticated());

        config.auth.access_token = Some("token".to_string());
        config.auth.refresh_token = Some("refresh".to_string());
        assert!(config.is_authenticated());
    }
}
