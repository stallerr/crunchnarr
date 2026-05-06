# CLAUDE.md

Guidance for Claude Code (claude.ai/code) working in this repository.

## What this is

Crunchnarr — a self-hosted Crunchyroll watchlist + downloader server. See [README.md](README.md) for the user-facing pitch.

## Repository layout

- **`crunchy-cli/`** — Rust workspace.
  - Root crate `crunchy-cli` is the download library + CLI binary.
  - `crates/api` is the API server (`crunchy-api`).
  - `crates/widevine` + `crates/widevine-proto` are the vendored Widevine CDM bindings.
- **`crunchy-web/`** — Next.js 16 / React 19 web UI.
- **`PLAN_*.md`** — design notes for the bigger features (api keys, bookmarks, watchlist, download options).

Internal crate names are still `crunchy-*`. They're not user-visible (binary name, log lines); leaving them avoids a touchy rename.

## Commands

```bash
# API
cd crunchy-cli
cargo check -p crunchy-api
cargo test -p crunchy-api
cargo fmt
cargo clippy -p crunchy-api -- -D warnings

# Web
cd crunchy-web
npm install
npm run build
```

## Runtime dependencies

`ffmpeg` (auto-downloaded via `ffmpeg-sidecar` if missing) and `mp4decrypt` (built from Bento4 in the Docker image, or install manually for native dev).
