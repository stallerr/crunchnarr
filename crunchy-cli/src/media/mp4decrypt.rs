//! MP4Decrypt command builder.

use super::keycrak::DecryptionKey;
use crate::error::{Error, MediaError, Result};
use crate::utils::{format_elapsed, redact};
use std::time::Instant;
use tracing::{debug, trace};

/// Builds and executes mp4decrypt commands.
pub struct Mp4DecryptBuilder {
    mp4decrypt_path: String,
    keys: Vec<DecryptionKey>,
    input_path: String,
    output_path: String,
}

impl Mp4DecryptBuilder {
    /// Create a new mp4decrypt builder.
    pub fn new(mp4decrypt_path: &str, input_path: &str, output_path: &str) -> Self {
        Self {
            mp4decrypt_path: mp4decrypt_path.to_string(),
            keys: Vec::new(),
            input_path: input_path.to_string(),
            output_path: output_path.to_string(),
        }
    }

    /// Add a decryption key.
    pub fn key(mut self, key: DecryptionKey) -> Self {
        self.keys.push(key);
        self
    }

    /// Add multiple decryption keys.
    pub fn keys(mut self, keys: Vec<DecryptionKey>) -> Self {
        self.keys.extend(keys);
        self
    }

    /// Build the mp4decrypt command arguments.
    pub fn build(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Add keys
        for key in &self.keys {
            args.push("--key".to_string());
            args.push(format!("{}:{}", key.kid, key.key));
        }

        // Input and output
        args.push(self.input_path.clone());
        args.push(self.output_path.clone());

        args
    }

    /// Execute mp4decrypt.
    pub async fn execute(&self) -> Result<()> {
        if self.keys.is_empty() {
            return Err(Error::Media(MediaError::NoKeys));
        }

        let args = self.build();
        debug!(
            "mp4decrypt: {} -> {} ({} keys)",
            self.input_path, self.output_path, self.keys.len()
        );
        for key in &self.keys {
            trace!("Using key: kid={} key={}", redact(&key.kid), redact(&key.key));
        }

        let start = Instant::now();
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = super::tools::execute_command(&self.mp4decrypt_path, &args_ref, 300).await?;
        let elapsed = start.elapsed();

        if !output.success {
            trace!("mp4decrypt stderr: {}", output.stderr);
            return Err(Error::Media(MediaError::DecryptionFailed(format!(
                "mp4decrypt failed with code {}: {}",
                output.code, output.stderr
            ))));
        }

        debug!("Decryption completed in {}", format_elapsed(elapsed));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command() {
        let builder =
            Mp4DecryptBuilder::new("mp4decrypt", "input.mp4", "output.mp4").key(DecryptionKey {
                kid: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                key: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            });

        let args = builder.build();
        assert_eq!(
            args,
            vec![
                "--key",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "input.mp4",
                "output.mp4"
            ]
        );
    }
}
