//! External tool validation and execution.

use crate::config::ToolsConfig;
use crate::error::{Error, MediaError, Result};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, warn};

/// Ensures ffmpeg is available, downloading it if necessary.
/// Returns the resolved path to the ffmpeg binary.
pub async fn ensure_ffmpeg() -> Result<String> {
    tokio::task::spawn_blocking(|| {
        if let Err(e) = ffmpeg_sidecar::download::auto_download() {
            warn!("FFmpeg auto-download failed: {}", e);
        }
        Ok(ffmpeg_sidecar::paths::ffmpeg_path()
            .to_string_lossy()
            .to_string())
    })
    .await
    .map_err(|e| {
        Error::Media(MediaError::ToolNotFound {
            tool: format!("ffmpeg (join error: {})", e),
        })
    })?
}

/// Validates and checks external tool availability.
pub struct ToolValidator;

impl ToolValidator {
    /// Check if a tool is available and executable.
    pub async fn check_tool(name: &str, path: &str) -> Result<bool> {
        let tool_path = if path.starts_with('~') {
            shellexpand::tilde(path).to_string()
        } else {
            path.to_string()
        };

        // Try to execute with --version or -version
        let result = Command::new(&tool_path)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match result {
            Ok(status) => {
                if status.success() {
                    debug!("{} found at {}", name, tool_path);
                    Ok(true)
                } else {
                    // Try without arguments (some tools don't support --version)
                    let result2 = Command::new(&tool_path)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .await;

                    match result2 {
                        Ok(_) => {
                            debug!("{} found at {} (no --version support)", name, tool_path);
                            Ok(true)
                        }
                        Err(_) => {
                            warn!("{} not working at {}", name, tool_path);
                            Ok(false)
                        }
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    debug!("{} not found at {}", name, tool_path);
                    Ok(false)
                } else {
                    warn!("{} check failed: {}", name, e);
                    Ok(false)
                }
            }
        }
    }

    /// Validate all required tools.
    pub async fn validate_all(config: &ToolsConfig) -> Result<ValidationResult> {
        let mp4decrypt = Self::check_tool("mp4decrypt", &config.mp4decrypt).await?;

        let widevine_client = config
            .widevine_client
            .as_ref()
            .map(|p| p.exists())
            .unwrap_or(false);

        let widevine_key = config
            .widevine_private_key
            .as_ref()
            .map(|p| p.exists())
            .unwrap_or(false);

        Ok(ValidationResult {
            mp4decrypt,
            widevine_client,
            widevine_key,
        })
    }

    /// Ensure a tool is available, returning an error if not.
    pub async fn require_tool(name: &str, path: &str) -> Result<()> {
        if !Self::check_tool(name, path).await? {
            return Err(Error::Media(MediaError::ToolNotFound {
                tool: name.to_string(),
            }));
        }
        Ok(())
    }
}

/// Result of tool validation.
#[derive(Debug)]
pub struct ValidationResult {
    pub mp4decrypt: bool,
    pub widevine_client: bool,
    pub widevine_key: bool,
}

impl ValidationResult {
    /// Check if all tools for downloading are available.
    pub fn can_download(&self) -> bool {
        self.mp4decrypt && self.widevine_client && self.widevine_key
    }

    /// Get a list of missing tools.
    pub fn missing_tools(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.mp4decrypt {
            missing.push("mp4decrypt");
        }
        if !self.widevine_client {
            missing.push("widevine_client (client_id.bin)");
        }
        if !self.widevine_key {
            missing.push("widevine_private_key (private_key.pem)");
        }
        missing
    }
}

/// Execute an external command and capture output.
pub async fn execute_command(
    program: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<CommandOutput> {
    debug!("Executing: {} {:?}", program, args);
    debug!("Command: {}", format!(
        "{} {}",
        program,
        args.iter()
            .map(|arg| {
                if arg.starts_with('-') {
                    arg.to_string()
                } else {
                    format!("'{}'", arg)
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    ));

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await
    .map_err(|_| {
        Error::Media(MediaError::ToolTimeout {
            tool: program.to_string(),
            timeout: timeout_secs,
        })
    })?
    .map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::Media(MediaError::ToolNotFound {
                tool: program.to_string(),
            })
        } else {
            Error::Io(e)
        }
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(CommandOutput {
        success: output.status.success(),
        code: output.status.code().unwrap_or(-1),
        stdout,
        stderr,
    })
}

/// Output from an external command.
#[derive(Debug)]
pub struct CommandOutput {
    pub success: bool,
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}
