use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use serde_json::Value;

use crate::error::ApiError;

const SECRET_PLACEHOLDER: &str = "********";
const ENCRYPTED_PREFIX: &str = "enc:v1:";
const STORAGE_SECRET_KEY_ENV: &str = "STORAGE_SECRET_KEY";
const STORAGE_SECRET_FIELD: &str = "secret_access_key";

pub fn storage_secret_placeholder() -> &'static str {
    SECRET_PLACEHOLDER
}

pub fn mask_storage_secrets(mut config: Value) -> Value {
    if let Some(storage) = config.get_mut("storage").and_then(|s| s.as_object_mut()) {
        if let Some(secret) = storage.get(STORAGE_SECRET_FIELD).and_then(|v| v.as_str()) {
            if !secret.is_empty() {
                storage.insert(
                    STORAGE_SECRET_FIELD.to_string(),
                    Value::String(SECRET_PLACEHOLDER.to_string()),
                );
            }
        }
    }
    config
}

const WIDEVINE_FIELDS: &[&str] = &["widevine_client", "widevine_private_key"];

/// Replace any *encrypted* widevine field with the placeholder so the secret
/// never leaves the server. Path-style legacy values pass through so users
/// upgrading from the path-based config can still see and edit them.
pub fn mask_widevine_blobs(mut config: Value) -> Value {
    let Some(obj) = config.as_object_mut() else {
        return config;
    };
    for field in WIDEVINE_FIELDS {
        if let Some(value) = obj.get(*field).and_then(|v| v.as_str()) {
            if !value.is_empty() && is_encrypted_blob(value) {
                obj.insert(
                    field.to_string(),
                    Value::String(SECRET_PLACEHOLDER.to_string()),
                );
            }
        }
    }
    config
}

/// If the patch carries `********` for a widevine field, swap in the
/// previously-stored value so the merge keeps the existing blob.
pub fn restore_placeholder_widevine_blobs(updates: &mut Value, saved: &Value) {
    let Some(updates_obj) = updates.as_object_mut() else {
        return;
    };
    for field in WIDEVINE_FIELDS {
        let Some(incoming) = updates_obj.get(*field).and_then(|v| v.as_str()) else {
            continue;
        };
        if incoming != SECRET_PLACEHOLDER {
            continue;
        }
        let stored = saved
            .get(*field)
            .cloned()
            .unwrap_or(Value::String(String::new()));
        updates_obj.insert(field.to_string(), stored);
    }
}

/// Encrypt freshly-supplied widevine fields. The wire format is base64 of the
/// raw file bytes; we decode, encrypt, and store as `enc:v1:<base64>`. Empty
/// values, already-encrypted values, and legacy filesystem paths
/// (heuristically: starts with `/` or `~/`) are left untouched.
pub fn maybe_encrypt_widevine_blobs(settings: &mut Value) -> Result<(), ApiError> {
    let Some(obj) = settings.as_object_mut() else {
        return Ok(());
    };
    for field in WIDEVINE_FIELDS {
        let Some(incoming) = obj.get(*field).and_then(|v| v.as_str()).map(str::to_string) else {
            continue;
        };
        if incoming.is_empty() || is_encrypted_blob(&incoming) || looks_like_path(&incoming) {
            continue;
        }
        let raw = BASE64.decode(&incoming).map_err(|e| {
            ApiError::BadRequest(format!(
                "{field} must be base64-encoded file contents: {e}"
            ))
        })?;
        let ciphertext = encrypt_blob_bytes(&raw)?;
        obj.insert(field.to_string(), Value::String(ciphertext));
    }
    Ok(())
}

/// Heuristic: strings that look like absolute or `~/`-rooted filesystem paths
/// are treated as legacy values rather than base64 blobs. Tracks the path
/// shapes the old config UI accepted.
fn looks_like_path(s: &str) -> bool {
    s.starts_with('/') || s.starts_with("~/") || s.starts_with("./")
}

pub fn restore_placeholder_secret(updates: &mut Value, saved: &Value) {
    let Some(storage) = updates.get_mut("storage").and_then(|s| s.as_object_mut()) else {
        return;
    };
    let Some(incoming) = storage.get(STORAGE_SECRET_FIELD).and_then(|v| v.as_str()) else {
        return;
    };
    if incoming != SECRET_PLACEHOLDER {
        return;
    }
    let stored = saved
        .get("storage")
        .and_then(|s| s.get(STORAGE_SECRET_FIELD))
        .cloned()
        .unwrap_or(Value::String(String::new()));
    storage.insert(STORAGE_SECRET_FIELD.to_string(), stored);
}

pub fn maybe_encrypt_storage_secrets(settings: &mut Value) -> Result<(), ApiError> {
    let Some(storage) = settings.get_mut("storage").and_then(|s| s.as_object_mut()) else {
        return Ok(());
    };
    let Some(secret) = storage
        .get(STORAGE_SECRET_FIELD)
        .and_then(|v| v.as_str())
        .map(str::to_string)
    else {
        return Ok(());
    };

    if secret.is_empty() || secret.starts_with(ENCRYPTED_PREFIX) {
        return Ok(());
    }

    let ciphertext = encrypt_secret(&secret)?;
    storage.insert(
        STORAGE_SECRET_FIELD.to_string(),
        Value::String(ciphertext),
    );
    Ok(())
}

pub fn decrypt_storage_secrets(settings: &mut Value) -> Result<(), ApiError> {
    let Some(storage) = settings.get_mut("storage").and_then(|s| s.as_object_mut()) else {
        return Ok(());
    };
    let Some(secret) = storage
        .get(STORAGE_SECRET_FIELD)
        .and_then(|v| v.as_str())
        .map(str::to_string)
    else {
        return Ok(());
    };

    if !secret.starts_with(ENCRYPTED_PREFIX) {
        return Ok(());
    }

    let plaintext = decrypt_secret(&secret)?;
    storage.insert(
        STORAGE_SECRET_FIELD.to_string(),
        Value::String(plaintext),
    );
    Ok(())
}

pub fn normalize_storage_uris(settings: &Value) -> Option<Value> {
    let output_dir = settings.get("output_dir")?.as_str()?.trim_end_matches('/');
    let downloads = settings.get("downloads")?.as_array()?;
    let mut changed = false;
    let normalized: Vec<Value> = downloads
        .iter()
        .map(|item| {
            let mut item = item.clone();
            if let Some(obj) = item.as_object_mut() {
                if let Some(path) = obj.get("output_path").and_then(|v| v.as_str()) {
                    if !path.is_empty() && !path.contains("://") {
                        let absolute = if path.starts_with('/') {
                            path.to_string()
                        } else {
                            format!("{}/{}", output_dir, path)
                        };
                        obj.insert(
                            "output_path".to_string(),
                            Value::String(format!("file://{}", absolute)),
                        );
                        changed = true;
                    }
                }
            }
            item
        })
        .collect();

    if !changed {
        return None;
    }

    let mut root = settings.clone();
    if let Some(obj) = root.as_object_mut() {
        obj.insert("downloads".to_string(), Value::Array(normalized));
    }
    Some(root)
}

fn encrypt_secret(plaintext: &str) -> Result<String, ApiError> {
    encrypt_blob_bytes(plaintext.as_bytes())
}

fn decrypt_secret(ciphertext: &str) -> Result<String, ApiError> {
    let bytes = decrypt_blob_bytes(ciphertext)?;
    String::from_utf8(bytes)
        .map_err(|_| ApiError::Internal("decrypted storage secret is not utf-8".into()))
}

/// Encrypt arbitrary bytes (e.g. a Widevine client_id.bin or private key file)
/// under [`STORAGE_SECRET_KEY`] and return the `enc:v1:<base64>` envelope.
pub fn encrypt_blob_bytes(plaintext: &[u8]) -> Result<String, ApiError> {
    let key = load_secret_key()?;
    let unbound = UnboundKey::new(&AES_256_GCM, &key)
        .map_err(|_| ApiError::Internal("invalid STORAGE_SECRET_KEY".into()))?;
    let key = LessSafeKey::new(unbound);

    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| ApiError::Internal("failed to generate nonce".into()))?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let mut in_out = plaintext.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| ApiError::Internal("failed to encrypt blob".into()))?;

    let mut payload = nonce_bytes.to_vec();
    payload.extend_from_slice(&in_out);
    Ok(format!("{}{}", ENCRYPTED_PREFIX, BASE64.encode(payload)))
}

/// Reverse of [`encrypt_blob_bytes`]. Returns the raw plaintext bytes.
pub fn decrypt_blob_bytes(ciphertext: &str) -> Result<Vec<u8>, ApiError> {
    let encoded = ciphertext
        .strip_prefix(ENCRYPTED_PREFIX)
        .ok_or_else(|| ApiError::Internal("not an encrypted blob".into()))?;
    let payload = BASE64
        .decode(encoded)
        .map_err(|_| ApiError::Internal("invalid encrypted blob encoding".into()))?;
    if payload.len() <= NONCE_LEN {
        return Err(ApiError::Internal("encrypted blob payload too short".into()));
    }

    let key = load_secret_key()?;
    let unbound = UnboundKey::new(&AES_256_GCM, &key)
        .map_err(|_| ApiError::Internal("invalid STORAGE_SECRET_KEY".into()))?;
    let key = LessSafeKey::new(unbound);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    nonce_bytes.copy_from_slice(&payload[..NONCE_LEN]);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = payload[NONCE_LEN..].to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| ApiError::Internal("failed to decrypt blob".into()))?;
    Ok(plaintext.to_vec())
}

/// Returns true if the value is the `enc:v1:` envelope.
pub fn is_encrypted_blob(value: &str) -> bool {
    value.starts_with(ENCRYPTED_PREFIX)
}

fn load_secret_key() -> Result<[u8; 32], ApiError> {
    let raw = std::env::var(STORAGE_SECRET_KEY_ENV).map_err(|_| {
        ApiError::Internal(
            "STORAGE_SECRET_KEY is required to store or read encrypted S3 secrets".into(),
        )
    })?;

    let bytes = if raw.len() == 64 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(raw).map_err(|_| ApiError::Internal("invalid hex STORAGE_SECRET_KEY".into()))?
    } else {
        BASE64
            .decode(raw)
            .map_err(|_| ApiError::Internal("STORAGE_SECRET_KEY must be base64 or 64-char hex".into()))?
    };

    if bytes.len() != 32 {
        return Err(ApiError::Internal(
            "STORAGE_SECRET_KEY must decode to exactly 32 bytes".into(),
        ));
    }

    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_round_trip() {
        let mut saved = serde_json::json!({
            "storage": { "secret_access_key": "enc:v1:abc" }
        });
        let mut updates = serde_json::json!({
            "storage": { "secret_access_key": "********" }
        });
        restore_placeholder_secret(&mut updates, &saved);
        assert_eq!(updates["storage"]["secret_access_key"], saved["storage"]["secret_access_key"]);
        decrypt_storage_secrets(&mut saved).ok();
    }
}
