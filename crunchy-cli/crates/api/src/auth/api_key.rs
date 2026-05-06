//! API key generation and hashing helpers.

use ring::digest;
use ring::rand::{SecureRandom, SystemRandom};

pub const KEY_PREFIX: &str = "crl_";
pub const PREFIX_DISPLAY_LEN: usize = 8;

/// Generates a new API key: "crl_" + 64 hex chars (32 random bytes).
pub fn generate_api_key() -> String {
    let rng = SystemRandom::new();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes).expect("rng failed");
    format!("{}{}", KEY_PREFIX, hex::encode(bytes))
}

/// SHA-256 hash of the key, hex-encoded. Used for DB storage and lookup.
pub fn hash_api_key(key: &str) -> String {
    let d = digest::digest(&digest::SHA256, key.as_bytes());
    hex::encode(d.as_ref())
}

/// First `PREFIX_DISPLAY_LEN` chars after the `crl_` prefix, for display only.
/// Returns `None` if the key doesn't have the expected shape.
pub fn key_prefix(key: &str) -> Option<&str> {
    let body = key.strip_prefix(KEY_PREFIX)?;
    if body.len() < PREFIX_DISPLAY_LEN {
        return None;
    }
    Some(&body[..PREFIX_DISPLAY_LEN])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_correctly_shaped_key() {
        let k = generate_api_key();
        assert!(k.starts_with("crl_"));
        assert_eq!(k.len(), 4 + 64);
    }

    #[test]
    fn hash_is_deterministic_and_64_hex_chars() {
        let h1 = hash_api_key("crl_abc");
        let h2 = hash_api_key("crl_abc");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn prefix_extraction() {
        assert_eq!(key_prefix("crl_a1b2c3d4ffffffff"), Some("a1b2c3d4"));
        assert_eq!(key_prefix("crl_short"), None);
        assert_eq!(key_prefix("nope_a1b2c3d4ffffffff"), None);
    }
}
