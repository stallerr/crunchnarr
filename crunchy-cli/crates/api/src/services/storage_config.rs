//! Per-user storage configuration parsing and sink construction.
//!
//! User settings JSON may contain a `storage` object that selects an output
//! backend. When absent, the legacy `output_dir` field is used to construct
//! a local-filesystem sink (preserving pre-storage behavior).
//!
//! ## v1 caveat
//!
//! S3 credentials are stored **in plaintext** in the per-user `user_settings`
//! row. The DB is per-user-scoped on read, but a follow-up should encrypt
//! `secret_access_key` at rest using a key from `STORAGE_SECRET_KEY` (or a
//! KMS for production).

use std::path::PathBuf;
use std::sync::Arc;

use crunchy_cli::storage::{LocalFsSink, OutputSink};
use serde_json::Value;

use crate::error::ApiError;
use crate::services::s3_sink::S3Sink;
use crate::services::storage_secrets::decrypt_storage_secrets;

/// Storage backend selection parsed from per-user settings.
#[derive(Debug, Clone)]
pub enum StorageConfig {
    Local {
        dir: PathBuf,
    },
    S3 {
        bucket: String,
        region: Option<String>,
        endpoint: Option<String>,
        prefix: Option<String>,
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
        force_path_style: bool,
    },
}

impl StorageConfig {
    /// Read the `storage` object from a settings JSON value, falling back to
    /// the legacy `output_dir` field for `Local`.
    pub fn from_settings(settings: &Value, fallback_dir: PathBuf) -> Result<Self, ApiError> {
        let mut settings = settings.clone();
        decrypt_storage_secrets(&mut settings)?;

        if let Some(storage) = settings.get("storage").and_then(|v| v.as_object()) {
            let kind = storage
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("local");
            match kind {
                // For the local sink there's only one "where files land"
                // knob: the top-level `output_dir` field (passed in as
                // `fallback_dir`). Any `storage.output_dir` in a legacy
                // user_settings payload is ignored.
                "local" => Ok(Self::Local { dir: fallback_dir }),
                "s3" => Ok(Self::S3 {
                    bucket: storage
                        .get("bucket")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ApiError::BadRequest("storage.bucket is required for s3".into())
                        })?
                        .to_string(),
                    region: storage
                        .get("region")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(String::from),
                    endpoint: storage
                        .get("endpoint")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(String::from),
                    prefix: storage
                        .get("prefix")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(String::from),
                    access_key_id: storage
                        .get("access_key_id")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(String::from),
                    secret_access_key: storage
                        .get("secret_access_key")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(String::from),
                    force_path_style: storage
                        .get("force_path_style")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                }),
                other => Err(ApiError::BadRequest(format!(
                    "unknown storage.kind: {other}"
                ))),
            }
        } else {
            Ok(Self::Local { dir: fallback_dir })
        }
    }

    /// Build the sink for this configuration.
    pub async fn build_sink(self) -> Result<Arc<dyn OutputSink>, ApiError> {
        match self {
            Self::Local { dir } => Ok(Arc::new(LocalFsSink::new(dir))),
            Self::S3 {
                bucket,
                region,
                endpoint,
                prefix,
                access_key_id,
                secret_access_key,
                force_path_style,
            } => {
                let sink = S3Sink::new(
                    bucket,
                    region,
                    endpoint,
                    prefix,
                    access_key_id,
                    secret_access_key,
                    force_path_style,
                )
                .await?;
                Ok(Arc::new(sink))
            }
        }
    }
}
