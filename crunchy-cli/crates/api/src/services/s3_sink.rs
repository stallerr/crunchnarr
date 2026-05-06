//! S3-compatible [`OutputSink`] — uploads finalized downloads to a bucket
//! and returns an `s3://bucket/key` URI.

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use crunchy_cli::error::{Error, Result};
use crunchy_cli::storage::{OutputSink, OutputTarget};
use tokio::fs;
use tracing::{debug, warn};

use crate::error::ApiError;

const PRESIGNED_URL_EXPIRY: Duration = Duration::from_secs(15 * 60);

/// Uploads to an S3-compatible bucket. The bucket itself must already exist.
pub struct S3Sink {
    client: Client,
    bucket: String,
    prefix: Option<String>,
}

impl S3Sink {
    /// Build an `S3Sink`. `endpoint` lets you target MinIO / R2 / B2 etc.;
    /// when `None`, AWS defaults apply.
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        bucket: String,
        region: Option<String>,
        endpoint: Option<String>,
        prefix: Option<String>,
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
        force_path_style: bool,
    ) -> std::result::Result<Self, ApiError> {
        let region_provider = region
            .clone()
            .map(Region::new)
            .unwrap_or_else(|| Region::new("us-east-1"));

        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider);

        if let Some(ep) = endpoint.as_ref() {
            loader = loader.endpoint_url(ep.clone());
        }

        if let (Some(ak), Some(sk)) = (access_key_id, secret_access_key) {
            loader = loader.credentials_provider(Credentials::new(
                ak,
                sk,
                None,
                None,
                "crunchy-api-user-settings",
            ));
        }

        let shared = loader.load().await;
        let mut s3_builder = aws_sdk_s3::config::Builder::from(&shared);
        if force_path_style {
            s3_builder = s3_builder.force_path_style(true);
        }
        let client = Client::from_conf(s3_builder.build());

        Ok(Self {
            client,
            bucket,
            prefix,
        })
    }

    /// Compose the final object key for a target, prefixed if configured.
    pub fn object_key(&self, target: &OutputTarget) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(prefix) = self.prefix.as_ref() {
            parts.push(prefix.trim_matches('/').to_string());
        }
        parts.extend(target.dirs.iter().cloned());
        parts.push(format!("{}.{}", target.stem, target.ext));
        parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join("/")
    }

    /// Generate a short-lived presigned GET URL for a stored object. Used by
    /// `serve_file` to redirect clients directly to the bucket.
    pub async fn presigned_get(
        &self,
        bucket: &str,
        key: &str,
    ) -> std::result::Result<String, ApiError> {
        let presigned = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .presigned(
                PresigningConfig::expires_in(PRESIGNED_URL_EXPIRY)
                    .map_err(|e| ApiError::Internal(format!("presign config: {e}")))?,
            )
            .await
            .map_err(|e| ApiError::Internal(format!("presign get_object: {e}")))?;
        Ok(presigned.uri().to_string())
    }
}

#[async_trait]
impl OutputSink for S3Sink {
    async fn publish(&self, source: &Path, target: &OutputTarget) -> Result<String> {
        let key = self.object_key(target);
        debug!(
            "Uploading {:?} to s3://{}/{}",
            source, self.bucket, key
        );

        let body = ByteStream::from_path(source)
            .await
            .map_err(|e| Error::other(format!("read source for upload: {e}")))?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(body)
            .send()
            .await
            .map_err(|e| Error::other(format!("S3 upload failed: {e}")))?;

        // Best-effort cleanup of the local file once the upload is durable.
        if let Err(e) = fs::remove_file(source).await {
            warn!("Failed to remove local file {:?} after upload: {}", source, e);
        }

        Ok(format!("s3://{}/{}", self.bucket, key))
    }
}
