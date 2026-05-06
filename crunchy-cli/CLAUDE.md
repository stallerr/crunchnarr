# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`crunchy-cli` is a Rust CLI tool for downloading content from Crunchyroll. It uses an async-first architecture with Tokio and interacts with Crunchyroll's undocumented API.

**External tools required at runtime**: `ffmpeg`, `mp4decrypt`

**Workspace layout**: The root crate is the CLI app. `crates/widevine` and `crates/widevine-proto` are vendored local crates for in-process Widevine CDM (DRM key acquisition). A `Dockerfile` exists for containerized builds.

## Commands

```bash
# Build
cargo build               # debug
cargo build --release     # optimized (LTO, stripped)
cargo check               # type-check without building

# Test
cargo test                            # all tests
cargo test test_name                  # specific test
cargo test module_name::              # module tests
cargo test -- --nocapture             # show stdout

# Lint & Format
cargo fmt                             # format
cargo fmt -- --check                  # check formatting
cargo clippy -- -D warnings           # lint (warnings as errors)

# Run
cargo run -- [ARGS]
RUST_LOG=debug cargo run -- [ARGS]    # with logging
```

## CLI Commands

`login`, `logout`, `whoami`, `search`, `info`, `download`, `queue`, `cache`, `config` — each handled by a `cmd_*` function in `src/main.rs`. Subcommands exist for `queue` (add/list/remove/clear/start/pause), `cache` (list/clean/path), and `config` (show/edit/set/reset/path/init).

## Architecture

### Core Download Pipeline (`src/download/manager.rs`)

The download flow is the most complex part of the codebase. `DownloadManager::download_episode_with_options` orchestrates:

1. **Episode metadata** — Fetches episode info via API
2. **Playback resolution** — Gets stream versions from `cr-play-service`, activating tokens per-version. Each version represents an audio dub (ja-JP, en-US, etc.) with its own stream URLs and DRM keys
3. **Manifest parsing** — Parses DASH/MPD XML (`src/download/manifest.rs`) to extract segment URLs and stream metadata per audio version
4. **Stream selection** — `StreamSelector` picks video/audio representations by quality preference
5. **Segment download** — Parallel downloads via `SegmentDownloader` with semaphore limits, speed throttling, SHA-256 checksum verification, and resumable JSON-persisted caching
6. **DRM decryption** — Acquires Widevine keys via vendored `widevine` crate (in-process CDM, semaphore-limited concurrency), decrypts with `mp4decrypt`
7. **Subtitle processing** — Downloads and converts subtitles (ASS format via `rsubs-lib`)
8. **Muxing** — `FfmpegBuilder` combines video + all audio tracks + subtitles into final MKV/MP4

Multi-audio is the key complexity: the manager iterates a `version_audio_map` built from playback data, downloads each audio track from its own stream version, then muxes all tracks together.

### Module Responsibilities

- **`src/api/`** — HTTP client wrapping Crunchyroll's API. `CrunchyrollClient` handles OAuth token refresh (proactive 60s before expiry + reactive on 401), multiple user agent profiles (Android TV/Mobile/iOS/Firefox with corresponding basic auth tokens in `client.rs`), and Cloudflare detection
- **`src/cli/`** — Clap-derive CLI definitions. `url_parser.rs` extracts content IDs from URLs via regex. `filter.rs` implements the `[S1E4-S3]` episode filter syntax appended to URLs
- **`src/config/`** — TOML config with `set_key` supporting dot-separated paths (e.g., `downloads.output_dir`). Atomic writes via temp file + rename
- **`src/download/`** — Download engine: manifest parsing, segment downloading, stream selection, cache management, speed throttling
- **`src/media/`** — External tool wrappers (`FfmpegBuilder`, `Mp4DecryptBuilder`) using builder pattern, in-process Widevine CDM (`acquire_keys`). `FilenameGenerator` with template variables (`{series}`, `{season:02}`, `{episode:02}`, `{title}`)
- **`src/queue/`** — JSON-persisted download queue for session recovery
- **`src/error.rs`** — Domain-specific error hierarchy (`ApiError`, `AuthError`, `DownloadError`, `MediaError`, etc.)

### Shared State

`Arc<RwLock<Config>>` is passed through the app. The API client caches its own access token in a separate `RwLock<Option<String>>` for fast access without locking the full config.

Command handlers live in `src/main.rs` (not in the library crate), using `anyhow::Result`. Library code uses `crate::error::Result<T>`.

### Download Modes

The `DownloadMode` enum supports: `Full`, `OnlySubs`, `OnlyAudio`, `OnlyVideo` — each skips irrelevant pipeline stages.

## Key Conventions

**Error handling**: `thiserror` for domain-specific error enums in `src/error.rs`. Use `anyhow::Result` in command handlers (`src/main.rs`), `crate::error::Result<T>` in library code.

**Logging**: `tracing` crate. Verbosity controlled by `-v`/`-vv`/`-vvv` flags. Sensitive values (tokens, URLs) must be redacted using `utils::redact()` / `utils::redact_url()` before logging.

**Config location** (via `directories` crate):
- macOS: `~/Library/Application Support/crunchy-cli/config.toml`
- Linux: `~/.config/crunchy-cli/config.toml`

**Import order**: std → external crates (alphabetical) → `crate::` → module-local

**Tests**: In-file with `#[cfg(test)] mod tests { ... }`, named `test_<what>_<condition>_<expected>`.
