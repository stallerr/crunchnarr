# AGENTS.md - Development Guide for Agentic Coding Assistants

This file contains essential information for AI coding agents working on the `crunchy-cli` project.

## Project Overview

**Name**: crunchy-cli  
**Language**: Rust (Edition 2021)  
**Type**: CLI tool for downloading content from Crunchyroll  
**Architecture**: Async-first using Tokio runtime

## Build/Test/Lint Commands

### Building
```bash
# Build in debug mode
cargo build

# Build in release mode (optimized with LTO)
cargo build --release

# Check code without building
cargo check

# Check with all features
cargo check --all-features
```

### Testing
```bash
# Run all tests
cargo test

# Run a specific test by name
cargo test test_error_display

# Run tests for a specific module
cargo test error::

# Run tests in a specific file
cargo test --test integration_test

# Run tests with output shown
cargo test -- --nocapture

# Run tests with verbose output
cargo test -- --nocapture --test-threads=1
```

### Linting and Formatting
```bash
# Format code
cargo fmt

# Check formatting without applying
cargo fmt -- --check

# Run clippy lints
cargo clippy

# Run clippy with all warnings as errors
cargo clippy -- -D warnings

# Apply clippy fixes automatically
cargo clippy --fix
```

### Running
```bash
# Run in debug mode
cargo run -- [ARGS]

# Run with logging
RUST_LOG=debug cargo run -- [ARGS]

# Examples
cargo run -- login -u user@example.com
cargo run -- search "anime name"
cargo run -- queue list
```

## Code Style Guidelines

### Module Organization
- Each module has a `mod.rs` or standalone `.rs` file
- Public API exposed through `lib.rs`
- Binary entry point in `main.rs`
- Use module-level doc comments (`//!`) at the top of each file

### Imports
Organize imports in this order (separated by blank lines):
1. Standard library (`std::`)
2. External crates (alphabetically)
3. Internal crates (`crate::`)
4. Module-local imports

```rust
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::Config;
use crate::error::{Error, Result};
```

### Naming Conventions
- **Types/Traits**: `PascalCase` (e.g., `CrunchyrollClient`, `DownloadManager`)
- **Functions/Methods**: `snake_case` (e.g., `get_profile`, `cmd_download`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `API_BASE`, `USER_AGENT`)
- **Modules**: `snake_case` (e.g., `api`, `download`, `queue`)
- **Lifetimes**: Short, descriptive (e.g., `'a`, `'static`)

### Error Handling
- Use `thiserror` for custom error types with `#[derive(Error, Debug)]`
- Define domain-specific error enums (`ApiError`, `AuthError`, `ConfigError`, etc.)
- Use `anyhow::Result` in `main.rs` and command handlers
- Use `crate::error::Result<T>` in library code
- All errors must implement `Display` and be descriptive

```rust
// Good - Specific error with context
#[error("Failed to parse manifest: {0}")]
ManifestError(String),

// Good - Structured error with fields
#[error("Tool '{tool}' failed with exit code {code}: {stderr}")]
ToolFailed {
    tool: String,
    code: i32,
    stderr: String,
},
```

### Type Annotations
- Prefer explicit types for public APIs
- Use type inference for local variables when obvious
- Always annotate function signatures
- Use type aliases for complex types

```rust
// Good
pub async fn get_profile(&self) -> Result<Profile> { ... }

// Good - type inference
let config = Config::load()?;

// Good - type alias
pub type Result<T> = std::result::Result<T, Error>;
```

### Async/Await
- All I/O operations should be async
- Use `#[tokio::main]` for the main function
- Prefer `async fn` over `impl Future`
- Use `Arc<RwLock<T>>` for shared mutable state
- Always `.await` async calls (don't drop futures)

### Documentation
- All public items must have doc comments (`///`)
- Modules should have module-level docs (`//!`)
- Include examples in docs when helpful
- Document panics, errors, and safety requirements

```rust
/// Handle login command.
///
/// Authenticates with Crunchyroll using either username/password
/// or a refresh token, then saves credentials to config.
async fn cmd_login(config: Arc<RwLock<Config>>, args: LoginArgs) -> Result<()> {
    // ...
}
```

### Logging
- Use `tracing` crate for logging
- Log levels: `error!`, `warn!`, `info!`, `debug!`, `trace!`
- Include context in log messages
- Don't log sensitive data (passwords, tokens)

```rust
use tracing::{debug, info, warn, error};

debug!("Using proxy: {}", proxy_url);
info!("Login successful!");
warn!("Token will expire in {} seconds", expires_in);
error!("Failed to download segment: {}", err);
```

### Formatting
- Follow standard `rustfmt` defaults
- Maximum line length: 100 characters (rustfmt default)
- Use trailing commas in multi-line items
- 4 spaces for indentation (no tabs)

## Project Structure

```
src/
├── main.rs           # Binary entry point, CLI command handlers
├── lib.rs            # Library root, module exports
├── error.rs          # Error types (thiserror-based)
├── api/              # Crunchyroll API client
│   ├── mod.rs
│   ├── client.rs     # HTTP client wrapper
│   ├── auth.rs       # Authentication
│   ├── content.rs    # Content metadata
│   ├── playback.rs   # Playback/streaming
│   └── types.rs      # API response types
├── cli/              # CLI argument parsing (clap)
│   ├── mod.rs
│   ├── commands.rs   # Command definitions
│   └── url_parser.rs # URL parsing utilities
├── config/           # Configuration management
│   └── mod.rs
├── download/         # Download orchestration
│   ├── mod.rs
│   ├── manager.rs    # Download manager
│   ├── segment.rs    # Segment downloader
│   └── progress.rs   # Progress tracking
├── media/            # Media processing (FFmpeg, mp4decrypt)
│   ├── mod.rs
│   ├── ffmpeg.rs     # FFmpeg wrapper
│   ├── mp4decrypt.rs # Decryption
│   ├── subtitles.rs  # Subtitle conversion
│   └── ...
├── queue/            # Download queue management
│   ├── mod.rs
│   ├── manager.rs
│   ├── storage.rs
│   └── types.rs
└── utils/            # Shared utilities
    ├── mod.rs
    └── range.rs
```

## Testing

- Write unit tests in the same file using `#[cfg(test)] mod tests { ... }`
- Write integration tests in `tests/` directory
- Test error conditions and edge cases
- Use descriptive test names: `test_<what>_<condition>_<expected>`

```rust
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
}
```

## Common Patterns

### Shared State
```rust
let config = Arc::new(RwLock::new(Config::load()?));
// Read
let cfg = config.read().await;
// Write
let mut cfg = config.write().await;
cfg.save()?;
```

### HTTP Requests
```rust
let response = self.http
    .get(url)
    .header("Authorization", format!("Bearer {}", token))
    .send()
    .await?;
```

### External Tool Execution
```rust
let output = tokio::process::Command::new("ffmpeg")
    .args(&["-i", "input.mp4"])
    .output()
    .await?;
```

## Dependencies

Key dependencies to be aware of:
- **tokio**: Async runtime (full features)
- **reqwest**: HTTP client (json, cookies, rustls-tls)
- **clap**: CLI parsing (derive, env)
- **serde/serde_json**: Serialization
- **thiserror/anyhow**: Error handling
- **tracing**: Structured logging

## Notes for Agents

- This is an active development project with TODOs in main.rs
- Authentication uses Crunchyroll's undocumented API
- Download flow involves: API → Manifest → Segments → Decrypt → Mux
- Config stored in OS-specific config directory (via `directories` crate)
- Queue persisted to JSON file for resumability
- Release builds use aggressive optimization (LTO, strip)
