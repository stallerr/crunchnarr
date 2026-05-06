//! In-process Widevine CDM key acquisition.
//!
//! Replaces the external keycrak binary with direct use of the vendored widevine crate.

use crate::error::{Error, MediaError, Result};
use crate::utils::{format_elapsed, redact};
use base64::Engine;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, trace};
use widevine::device::{DeviceType, SecurityLevel};
use widevine::{Cdm, CdmLicenseRequest, Device, KeyType, LicenseType, Pssh};

/// Extracted decryption key.
#[derive(Debug, Clone)]
pub struct DecryptionKey {
    pub kid: String,
    pub key: String,
}

/// License response from the server (JSON format).
#[derive(Debug, serde::Deserialize)]
struct LicenseResponse {
    license: String,
}

/// Parse an RSA private key from either PEM or DER format (PKCS#1 or PKCS#8).
fn parse_private_key(bytes: &[u8]) -> Result<RsaPrivateKey> {
    let pem_str = std::str::from_utf8(bytes).unwrap_or("");

    if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(pem_str) {
        trace!("Parsed private key (PKCS#8 PEM)");
        return Ok(key);
    }
    if let Ok(key) = RsaPrivateKey::from_pkcs1_pem(pem_str) {
        trace!("Parsed private key (PKCS#1 PEM)");
        return Ok(key);
    }
    if let Ok(key) = RsaPrivateKey::from_pkcs8_der(bytes) {
        trace!("Parsed private key (PKCS#8 DER)");
        return Ok(key);
    }
    if let Ok(key) = RsaPrivateKey::from_pkcs1_der(bytes) {
        trace!("Parsed private key (PKCS#1 DER)");
        return Ok(key);
    }

    Err(Error::Media(MediaError::DecryptionFailed(
        "Failed to parse private key (tried PEM and DER, PKCS#1 and PKCS#8)".to_string(),
    )))
}

/// Fetch a license from the Crunchyroll license server.
async fn fetch_license(
    http_client: &reqwest::Client,
    url: &str,
    challenge: &[u8],
    bearer_token: &str,
    video_token: &str,
    content_id: &str,
) -> Result<Vec<u8>> {
    debug!("Sending license request to {}", url);
    trace!("Challenge size: {} bytes", challenge.len());

    let response = http_client
        .post(url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/octet-stream")
        .header("x-cr-content-id", content_id)
        .header("x-cr-video-token", video_token)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36")
        .header("Accept", "text/html, application/xhtml+xml, application/xml; q=0.9, image/webp, */*; q=0.8")
        .header("Accept-Encoding", "gzip, deflate, br")
        .header("Accept-Language", "en-US,en;q=0.5")
        .body(challenge.to_vec())
        .send()
        .await
        .map_err(|e| {
            Error::Media(MediaError::DecryptionFailed(format!(
                "License request failed: {}",
                e
            )))
        })?;

    let status = response.status();
    trace!("License response status: {}", status);

    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        debug!("License response body: {}", body);
        return Err(Error::Media(MediaError::DecryptionFailed(format!(
            "License server returned HTTP {}: {}",
            status, body
        ))));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    trace!("License response content-type: {}", content_type);

    let body_bytes = response.bytes().await.map_err(|e| {
        Error::Media(MediaError::DecryptionFailed(format!(
            "Failed to read license response: {}",
            e
        )))
    })?;
    trace!("License response size: {} bytes", body_bytes.len());

    // Determine format and extract license bytes
    let license_bytes = if content_type.contains("application/json") {
        let lr: LicenseResponse = serde_json::from_slice(&body_bytes).map_err(|e| {
            Error::Media(MediaError::DecryptionFailed(format!(
                "Failed to parse license JSON: {}",
                e
            )))
        })?;
        base64::engine::general_purpose::STANDARD
            .decode(&lr.license)
            .map_err(|e| {
                Error::Media(MediaError::DecryptionFailed(format!(
                    "Failed to decode license base64: {}",
                    e
                )))
            })?
    } else if content_type.contains("application/octet-stream")
        || content_type.contains("application/x-protobuf")
        || body_bytes.first() == Some(&0x08)
    {
        body_bytes.to_vec()
    } else {
        // Unknown format - try JSON first, fall back to raw bytes
        match serde_json::from_slice::<LicenseResponse>(&body_bytes) {
            Ok(lr) => base64::engine::general_purpose::STANDARD
                .decode(&lr.license)
                .unwrap_or_else(|_| body_bytes.to_vec()),
            Err(_) => body_bytes.to_vec(),
        }
    };

    trace!("Extracted license: {} bytes", license_bytes.len());
    Ok(license_bytes)
}

/// Acquire Widevine decryption keys in-process.
///
/// This replaces the external keycrak binary with direct CDM logic.
#[allow(clippy::too_many_arguments)]
pub async fn acquire_keys(
    http_client: &reqwest::Client,
    client_id_path: &Path,
    private_key_path: &Path,
    license_url: &str,
    pssh_b64: &str,
    bearer_token: &str,
    video_token: &str,
    content_id: &str,
) -> Result<Vec<DecryptionKey>> {
    debug!("Requesting Widevine license keys (in-process)");
    trace!("License URL: {}", license_url);
    trace!("Content ID: {}", content_id);
    trace!("PSSH: {}...", &pssh_b64[..pssh_b64.len().min(32)]);
    trace!("Bearer token: {}", redact(bearer_token));
    trace!("Video token: {}", redact(video_token));

    let start = Instant::now();

    // 1. Read credential files
    let client_id_bytes = tokio::fs::read(client_id_path).await.map_err(|e| {
        Error::Media(MediaError::DecryptionFailed(format!(
            "Failed to read client_id file {:?}: {}",
            client_id_path, e
        )))
    })?;

    let private_key_bytes = tokio::fs::read(private_key_path).await.map_err(|e| {
        Error::Media(MediaError::DecryptionFailed(format!(
            "Failed to read private_key file {:?}: {}",
            private_key_path, e
        )))
    })?;

    // 2. Parse key, create device, generate challenge (CPU-bound crypto)
    let pssh_b64_owned = pssh_b64.to_string();
    let (challenge_bytes, license_request) =
        tokio::task::spawn_blocking(move || -> Result<(Vec<u8>, CdmLicenseRequest)> {
            let private_key = parse_private_key(&private_key_bytes)?;

            let device = Device::new(
                DeviceType::ANDROID,
                SecurityLevel::L3,
                private_key,
                &client_id_bytes,
            )
            .map_err(|e| {
                Error::Media(MediaError::DecryptionFailed(format!(
                    "Failed to create Widevine device: {}",
                    e
                )))
            })?;

            let pssh = Pssh::from_b64(&pssh_b64_owned).map_err(|e| {
                Error::Media(MediaError::DecryptionFailed(format!(
                    "Failed to parse PSSH: {}",
                    e
                )))
            })?;

            let cdm = Cdm::new(device);
            let session = cdm.open();
            let license_request = session
                .get_license_request(pssh, LicenseType::STREAMING)
                .map_err(|e| {
                    Error::Media(MediaError::DecryptionFailed(format!(
                        "Failed to create license request: {}",
                        e
                    )))
                })?;

            let challenge = license_request.challenge().map_err(|e| {
                Error::Media(MediaError::DecryptionFailed(format!(
                    "Failed to generate challenge: {}",
                    e
                )))
            })?;

            Ok((challenge, license_request))
        })
        .await
        .map_err(|e| {
            Error::Media(MediaError::DecryptionFailed(format!(
                "Task join error: {}",
                e
            )))
        })??;

    trace!("Generated challenge: {} bytes", challenge_bytes.len());

    // 3. POST challenge to license server (async HTTP)
    let license_bytes = fetch_license(
        http_client,
        license_url,
        &challenge_bytes,
        bearer_token,
        video_token,
        content_id,
    )
    .await?;

    // 4. Extract keys from license response (CPU-bound crypto)
    let keys = tokio::task::spawn_blocking(move || -> Result<Vec<DecryptionKey>> {
        let key_set = license_request.get_keys(&license_bytes).map_err(|e| {
            Error::Media(MediaError::DecryptionFailed(format!(
                "Failed to extract keys from license: {}",
                e
            )))
        })?;

        let mut keys = Vec::new();
        for key in key_set.of_type(KeyType::CONTENT) {
            keys.push(DecryptionKey {
                kid: hex::encode(key.kid),
                key: hex::encode(&key.key),
            });
        }

        Ok(keys)
    })
    .await
    .map_err(|e| {
        Error::Media(MediaError::DecryptionFailed(format!(
            "Task join error: {}",
            e
        )))
    })??;

    let elapsed = start.elapsed();

    if keys.is_empty() {
        debug!("No content keys found in license response");
        return Err(Error::Media(MediaError::NoKeys));
    }

    debug!(
        "Acquired {} decryption key(s) in {}",
        keys.len(),
        format_elapsed(elapsed)
    );
    for key in &keys {
        trace!("Key: kid={} key={}", redact(&key.kid), redact(&key.key));
    }

    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_private_key_invalid_input() {
        let invalid_bytes = b"not a valid key";
        assert!(parse_private_key(invalid_bytes).is_err());
    }

    #[test]
    fn test_decryption_key_debug() {
        let key = DecryptionKey {
            kid: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            key: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        };
        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    }
}
