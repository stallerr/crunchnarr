# Plan: Honor `options` in `POST /downloads`

## Problem

In `services/download.rs`, the `start_download` method signature is:

```rust
pub async fn start_download(
    &self,
    user_id: &str,
    url: &str,
    _options_json: serde_json::Value,  // <-- underscore prefix = completely ignored
    db: &SqlitePool,
)
```

The `options` field sent in the request body is never applied. Every download uses only the user's saved settings from DB. There is no way to override settings per-download, and the web UI's per-download track/quality options silently do nothing.

---

## Goal

`options` in `POST /downloads` should act as a per-download override on top of saved user settings:

- **`options` omitted or `null`** → use user's saved settings (existing behavior, keep it)
- **`options` provided** → user settings apply first, then `options` keys override just those fields for this download only

---

## Change 1 — `crunchy-cli/crates/api/src/services/download.rs`

**Step 1:** Extract the existing settings-application block (lines ~213–308, the big `if let Some(s) = settings_value.as_ref()` block) into a standalone helper function. Name it `apply_overrides_to_config` since it'll be called twice with different sources:

```rust
fn apply_overrides_to_config(cfg: &mut crunchy_cli::config::Config, s: &serde_json::Value) {
    // move the existing body of the if-block here, unchanged
}
```

**Step 2:** Rename the parameter from `_options_json` to `options_json` (remove the underscore).

**Step 3:** Add a per-request allowlist. Storage, output paths, Widevine credentials, and proxy config are global to the user and **must not** be overridable per-download (security: arbitrary `widevine_client` is a file-read primitive, arbitrary `output_dir` is a file-write primitive). Filter the per-request payload before applying:

```rust
const PER_REQUEST_OVERRIDE_KEYS: &[&str] = &[
    "video_quality",
    "parallel_segments",
    "max_speed_kbps",
    "retry_count",
    "audio_languages",
    "subtitle_languages",
    "closed_captions",
    "output_format",
    "embed_subtitles",
    "default_audio_track",
    "default_subtitle_track",
    "prefer_signs_songs",
    "filename_template",
];

/// Returns a new JSON object containing only the entries from `options` whose
/// keys are in `PER_REQUEST_OVERRIDE_KEYS`. Non-object inputs return Null.
fn filter_overrides(options: &serde_json::Value) -> serde_json::Value {
    let Some(obj) = options.as_object() else {
        return serde_json::Value::Null;
    };
    let filtered: serde_json::Map<_, _> = obj
        .iter()
        .filter(|(k, _)| PER_REQUEST_OVERRIDE_KEYS.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    serde_json::Value::Object(filtered)
}
```

Explicitly **excluded** from per-request override: `output_dir`, `cache_retention_days`, `concurrent_key_acquisitions`, `widevine_client`, `widevine_private_key`, `proxy_enabled`, `proxy_url`, the entire `storage` block. These remain user-scoped and read from saved settings only.

**Step 4:** Call the helper twice — user settings first (full set), then filtered per-request overrides on top:

```rust
// Apply saved user settings (no allowlist — full set)
if let Some(s) = settings_value.as_ref() {
    apply_overrides_to_config(&mut cfg, s);
}

// Apply per-request overrides on top, filtered to the allowlist
let filtered = filter_overrides(&options_json);
if filtered.is_object() {
    apply_overrides_to_config(&mut cfg, &filtered);
}
```

No new logic in `apply_overrides_to_config` itself — it already only applies keys that are present and non-empty, so absent keys in the filtered overrides simply don't override anything.

**Note on empty-array semantics.** The existing block treats `audio_languages: []` as "skip" (line 246: `if !langs.is_empty()`). That means a request explicitly sending an empty array cannot override the user's saved non-empty list back to "unconstrained." This is by design — keep it. It's also what makes the frontend's `DEFAULT_DOWNLOAD_OPTIONS` (empty arrays) safe to send as a no-op when the user hasn't configured per-download options.

**Note on storage.** `StorageConfig::from_settings(settings_value, ...)` at line 311 reads from saved settings only, not the merged config. Per-request `storage` overrides are intentionally not supported.

---

## Change 2 — `crunchy-cli/crates/api/src/routes/downloads.rs`

The handler already passes `req.options` through. Once the `_options_json` parameter is renamed in the service, the call site compiles as-is. No change required unless the compiler points to something.

Confirm the handler call looks like this (it should already):

```rust
state
    .download_service
    .start_download(&auth.user_id, &req.url, req.options, &state.db)
    .await?;
```

---

## Change 3 — frontend wire format (CRITICAL)

The frontend currently sends camelCase keys but the backend reads snake_case. Without this change, the per-request override path will silently no-op even after Change 1 ships.

### `crunchy-web/types/download-options.ts`

Replace the type and default:

```ts
export interface DownloadOptions {
  video_quality?: 'best' | '1080p' | '720p' | '480p' | '360p';
  audio_languages?: string[];
  subtitle_languages?: string[];
  output_format?: 'mkv' | 'mp4';
}

export const DEFAULT_DOWNLOAD_OPTIONS: DownloadOptions = {};
```

Notes:
- All fields are now **optional** — the frontend's job is to send only the keys the user actually overrode, not a fully-populated object that would clobber saved settings.
- `downloadMode` is removed entirely. There is no backend field for it (no `download_mode` reader in `apply_overrides_to_config`, no field on `Config`). Re-introducing it is a separate feature with its own plan.
- The four kept fields all map to backend keys in the allowlist.
- Defaulting to `{}` means "no per-download overrides" and matches the new "absent keys → use saved settings" contract.

### `crunchy-web/components/downloads/download-button.tsx`

Replace `buildOptions`:

```ts
const buildOptions = (): DownloadOptions => ({});
```

Until the UI grows actual per-download controls, every download just sends an empty options object. The previous code was *building* options from saved config and then sending them as overrides — net effect was identical to "send nothing", but only by luck (camelCase keys were being ignored). Make the intent explicit.

### `crunchy-web/components/downloads/download-series-button.tsx`

Already uses `DEFAULT_DOWNLOAD_OPTIONS`. After the type change, `DEFAULT_DOWNLOAD_OPTIONS` becomes `{}` and this file needs no further edit.

### Future per-download UI

When the UI grows actual per-download controls (track picker, quality dropdown, etc. in the download confirmation dialog), each control writes into a local `DownloadOptions` object using snake_case keys, and that object is what gets sent. The backend will merge it on top of saved settings.

---

## `options` field shape

`options` accepts a subset of the flat JSON keys that `GET /config` returns — see `PER_REQUEST_OVERRIDE_KEYS` in Change 1 for the allowlist. Examples:

```json
{ "audio_languages": ["ja-JP", "en-US"] }
{ "subtitle_languages": ["en-US"], "video_quality": "1080p" }
{ "output_format": "mkv" }
```

Keys outside the allowlist (`output_dir`, `widevine_*`, `proxy_*`, `storage`, etc.) are silently dropped. Unrecognized keys are silently ignored.

---

## Tests

Add a unit test in `crunchy-cli/crates/api/src/services/download.rs` (or a sibling test module) covering the helper + filter:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_applies_allowlisted_keys() {
        let mut cfg = crunchy_cli::config::Config::default();
        let saved = serde_json::json!({
            "video_quality": "best",
            "audio_languages": ["ja-JP", "en-US"],
        });
        let overrides = serde_json::json!({
            "video_quality": "720p",
            "audio_languages": ["ja-JP"],
        });

        apply_overrides_to_config(&mut cfg, &saved);
        let filtered = filter_overrides(&overrides);
        apply_overrides_to_config(&mut cfg, &filtered);

        assert_eq!(cfg.downloads.video_quality, "720p");
        assert_eq!(cfg.languages.audio, vec!["ja-JP".to_string()]);
    }

    #[test]
    fn filter_drops_global_keys() {
        let overrides = serde_json::json!({
            "video_quality": "720p",
            "output_dir": "/etc",
            "widevine_client": "/etc/passwd",
            "storage": { "kind": "s3" },
        });
        let filtered = filter_overrides(&overrides);
        let obj = filtered.as_object().unwrap();
        assert!(obj.contains_key("video_quality"));
        assert!(!obj.contains_key("output_dir"));
        assert!(!obj.contains_key("widevine_client"));
        assert!(!obj.contains_key("storage"));
    }

    #[test]
    fn filter_handles_non_object() {
        assert_eq!(filter_overrides(&serde_json::Value::Null), serde_json::Value::Null);
        assert_eq!(filter_overrides(&serde_json::json!("hello")), serde_json::Value::Null);
    }
}
```

## Validation

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo check -p crunchy-api --manifest-path crunchy-cli/Cargo.toml
cargo test -p crunchy-api --manifest-path crunchy-cli/Cargo.toml
cd crunchy-web && npm run build
```

Manual smoke test:
1. Confirm user has `audio_languages: ["ja-JP", "en-US"]` saved — `GET /config`
2. `POST /downloads { "url": "..." }` (no options) → download uses both audio tracks
3. `POST /downloads { "url": "...", "options": { "audio_languages": ["ja-JP"] } }` → download uses JP only
4. `POST /downloads { "url": "...", "options": { "output_dir": "/etc" } }` → `output_dir` is silently dropped, download lands in the user's saved `output_dir`
5. After Change 3, every download from the web UI sends `options: {}` and behaves identically to today (which is the goal — no behavior regression, but the override path is now wired).

## What this plan does NOT do

- Does not add UI for per-download options (track picker, quality dropdown, etc.). That's a follow-up — the wire is now ready for it.
- Does not re-introduce `downloadMode`. If you want partial-stream downloads (audio-only, etc.), that's a separate feature: needs a new `Config` field, a download-time stream filter, and frontend controls.
