//! Download service - manages background download tasks.

use crate::error::ApiError;
use crate::services::ws::WsBroadcaster;
use chrono::Utc;
use crunchy_cli::api::types::CREpisode;
use crunchy_cli::config::Config;
use crunchy_cli::download::{DownloadManager, DownloadResult, ProgressReporter, StepProgress};
use crunchy_cli::media::{FilenameGenerator, FilenameVars};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Keys that callers may override on a per-request basis via `POST /downloads`'s
/// `options` field. Anything not in this list (`output_dir`, `widevine_*`,
/// `proxy_*`, `storage`, …) is user-scoped and read only from saved settings —
/// allowing arbitrary overrides would be a path-traversal / secret-leak primitive.
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

/// Strip per-request overrides down to the allowlist. Non-object inputs become Null.
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

/// Apply settings-shaped JSON onto a `Config`. Used both for saved user settings
/// and (after filtering) for per-request `options` overrides. Only keys that are
/// present and non-empty take effect — empty-array means "skip", not "clear".
fn apply_overrides_to_config(cfg: &mut crunchy_cli::config::Config, s: &serde_json::Value) {
    // Download preferences
    if let Some(v) = s.get("video_quality").and_then(|v| v.as_str()) {
        cfg.downloads.video_quality = v.to_string();
    }
    if let Some(v) = s.get("simultaneous_downloads").and_then(|v| v.as_u64()) {
        cfg.downloads.simultaneous = v as u8;
    }
    if let Some(v) = s.get("parallel_segments").and_then(|v| v.as_u64()) {
        cfg.downloads.parts = v as u8;
    }
    if let Some(v) = s.get("max_speed_kbps").and_then(|v| v.as_u64()) {
        cfg.downloads.max_speed_kbps = v as u32;
    }
    if let Some(v) = s.get("retry_count").and_then(|v| v.as_u64()) {
        cfg.downloads.retry_count = v as u8;
    }
    if let Some(v) = s.get("output_dir").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            cfg.downloads.output_dir = PathBuf::from(v);
        }
    }
    if let Some(v) = s.get("cache_retention_days").and_then(|v| v.as_u64()) {
        cfg.downloads.cache_retention_hours = (v as u32) * 24;
    }
    if let Some(v) = s.get("concurrent_key_acquisitions").and_then(|v| v.as_u64()) {
        cfg.downloads.max_concurrent_keys = v as u8;
    }

    // Language preferences
    if let Some(v) = s.get("audio_languages").and_then(|v| v.as_array()) {
        let langs: Vec<String> = v.iter().filter_map(|l| l.as_str().map(String::from)).collect();
        if !langs.is_empty() {
            cfg.languages.audio = langs;
        }
    }
    if let Some(v) = s.get("subtitle_languages").and_then(|v| v.as_array()) {
        let langs: Vec<String> = v.iter().filter_map(|l| l.as_str().map(String::from)).collect();
        if !langs.is_empty() {
            cfg.languages.subtitles = langs;
        }
    }
    if let Some(v) = s.get("closed_captions").and_then(|v| v.as_bool()) {
        cfg.languages.include_cc = v;
    }

    // Muxing options
    if let Some(v) = s.get("output_format").and_then(|v| v.as_str()) {
        cfg.muxing.format = v.to_string();
    }
    if let Some(v) = s.get("embed_subtitles").and_then(|v| v.as_bool()) {
        cfg.muxing.embed_subs = v;
    }
    if let Some(v) = s.get("default_audio_track").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            cfg.muxing.default_audio = v.to_string();
        }
    }
    if let Some(v) = s.get("default_subtitle_track").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            cfg.muxing.default_sub = v.to_string();
        }
    }
    if let Some(v) = s.get("prefer_signs_songs").and_then(|v| v.as_bool()) {
        cfg.muxing.prefer_signs_songs = v;
    }
    if let Some(v) = s.get("filename_template").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            cfg.muxing.filename_template = v.to_string();
        }
    }

    // Tools (Widevine) — kept for legacy path-style values; encrypted blobs
    // are materialized to per-request tempfiles separately.
    if let Some(v) = s.get("widevine_client").and_then(|v| v.as_str()) {
        if !v.is_empty() && !crate::services::storage_secrets::is_encrypted_blob(v) {
            cfg.tools.widevine_client = Some(PathBuf::from(v));
        }
    }
    if let Some(v) = s.get("widevine_private_key").and_then(|v| v.as_str()) {
        if !v.is_empty() && !crate::services::storage_secrets::is_encrypted_blob(v) {
            cfg.tools.widevine_private_key = Some(PathBuf::from(v));
        }
    }

    // Proxy
    if let Some(v) = s.get("proxy_enabled").and_then(|v| v.as_bool()) {
        cfg.proxy.enabled = v;
    }
    if let Some(v) = s.get("proxy_url").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            cfg.proxy.url = Some(v.to_string());
        }
    }
}

/// Decrypt an `enc:v1:` widevine blob field from `settings` and write it to a
/// per-request temp file. Returns `Ok(None)` for missing/empty/legacy-path
/// values; the caller then leaves `cfg.tools.widevine_*` alone (path-style
/// settings are still parsed in the loop above).
fn materialize_widevine_blob(
    settings: Option<&serde_json::Value>,
    field: &str,
) -> Result<Option<NamedTempFile>, ApiError> {
    let Some(raw) = settings
        .and_then(|s| s.get(field))
        .and_then(|v| v.as_str())
    else {
        return Ok(None);
    };
    if raw.is_empty() || !crate::services::storage_secrets::is_encrypted_blob(raw) {
        return Ok(None);
    }
    let bytes = crate::services::storage_secrets::decrypt_blob_bytes(raw)?;
    let mut tmp = NamedTempFile::new()
        .map_err(|e| ApiError::Internal(format!("create widevine tempfile: {e}")))?;
    tmp.write_all(&bytes)
        .map_err(|e| ApiError::Internal(format!("write widevine tempfile: {e}")))?;
    tmp.flush()
        .map_err(|e| ApiError::Internal(format!("flush widevine tempfile: {e}")))?;
    Ok(Some(tmp))
}

/// Per-episode result returned from `start_download_inner`. Either the
/// download was newly kicked off (`status = "pending"`), or it was skipped
/// because a previous completed/in-flight row exists or because the muxed
/// file is already on disk at the templated output path.
#[derive(Debug, Clone)]
pub struct EpisodeOutcome {
    pub episode_id: String,
    pub episode_title: String,
    pub download_id: Option<String>,
    pub status: &'static str,
    pub skip_reason: Option<&'static str>,
    pub existing_download_id: Option<String>,
    pub existing_path: Option<String>,
}

impl EpisodeOutcome {
    fn started(download_id: String, episode_id: String, title: String) -> Self {
        Self {
            episode_id,
            episode_title: title,
            download_id: Some(download_id),
            status: "pending",
            skip_reason: None,
            existing_download_id: None,
            existing_path: None,
        }
    }

    fn skipped(
        episode_id: String,
        title: String,
        reason: &'static str,
        existing_download_id: Option<String>,
        existing_path: Option<String>,
    ) -> Self {
        Self {
            episode_id,
            episode_title: title,
            download_id: None,
            status: "skipped",
            skip_reason: Some(reason),
            existing_download_id,
            existing_path,
        }
    }
}

/// Compute the output path the publish step would write to, *without* doing
/// any CR-side work. Returns `None` when the user's filename template includes
/// runtime-resolved vars (`{quality}`, `{audio}`, `{year}`) that we'd need a
/// streaming-token-priced playback call to know — in that case the pre-check
/// can't run and the in-pipeline `skip_existing` (inside `DownloadManager`)
/// is the only safety net.
fn precomputed_output_path(cfg: &Config, episode: &CREpisode) -> Option<PathBuf> {
    let template = &cfg.muxing.filename_template;
    if template.contains("{quality}")
        || template.contains("{audio}")
        || template.contains("{year}")
    {
        return None;
    }
    let season = if episode.season_sequence_number > 0 {
        episode.season_sequence_number
    } else {
        episode.season_number
    };
    let vars = FilenameVars {
        series: episode.series_title.clone(),
        season,
        season_title: episode.season_title.clone(),
        episode: episode.episode.clone(),
        episode_number: episode.episode_number,
        title: episode.title.clone(),
        quality: String::new(),
        audio: String::new(),
        year: String::new(),
    };
    let gen = FilenameGenerator::new(template, cfg.downloads.output_dir.clone());
    Some(gen.generate(&vars, &cfg.muxing.format))
}

/// Latest non-superseded `downloads` row for this `user_id` × `episode_id`,
/// if any. Used as the DB-side existence check at download-trigger time.
async fn latest_existing_download(
    db: &SqlitePool,
    user_id: &str,
    episode_id: &str,
) -> Result<Option<(String, String)>, ApiError> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT id, status FROM downloads \
         WHERE user_id = ? AND episode_id = ? AND superseded = 0 \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(episode_id)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

/// Handle to an active download task.
struct DownloadHandle {
    user_id: String,
    cancel_token: CancellationToken,
    _task: JoinHandle<()>,
}

/// Per-user concurrency gate. Sized from the user's `simultaneous_downloads`
/// setting. When the setting changes at the start of a new `start_download`
/// call, the entry is replaced — in-flight tasks keep a clone of the old
/// `Arc<Semaphore>` and finish under the previous cap, new tasks acquire
/// against the resized one.
struct SemaphoreEntry {
    sem: Arc<Semaphore>,
    size: u8,
}

/// Manages active downloads across all users.
pub struct DownloadService {
    active_downloads: RwLock<HashMap<String, DownloadHandle>>,
    /// Per-user `Arc<Semaphore>` cap that enforces `simultaneous_downloads`
    /// across all tasks owned by a user. Created lazily.
    download_semaphores: RwLock<HashMap<String, SemaphoreEntry>>,
    ws_broadcaster: Arc<WsBroadcaster>,
}

/// WebSocket-based progress reporter that sends JSON events.
struct WebSocketProgress {
    download_id: String,
    user_id: String,
    broadcaster: Arc<WsBroadcaster>,
}

impl ProgressReporter for WebSocketProgress {
    fn on_segment_complete(&self, stream_id: &str, completed: u64, total: u64) {
        let percent = if total > 0 {
            (completed as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let msg = serde_json::json!({
            "type": "download_progress",
            "data": {
                "download_id": self.download_id,
                "stream_id": stream_id,
                "completed_segments": completed,
                "total_segments": total,
                "percent": percent,
                "phase": stream_id,
            }
        });
        let broadcaster = self.broadcaster.clone();
        let user_id = self.user_id.clone();
        tokio::spawn(async move {
            broadcaster.send_to_user(&user_id, &msg).await;
        });
    }

    fn on_step_progress(&self, progress: &StepProgress) {
        let percent = if progress.total > 0 {
            (progress.completed as f64 / progress.total as f64) * 100.0
        } else {
            0.0
        };
        let msg = serde_json::json!({
            "type": "download_progress",
            "data": {
                "download_id": self.download_id,
                "phase": progress.label,
                "current_step": progress.current_step,
                "total_steps": progress.total_steps,
                "completed_segments": progress.completed,
                "total_segments": progress.total,
                "percent": percent,
                "downloaded_bytes": 0,
                "total_bytes": null,
                "speed_bps": progress.speed_bps,
                "eta_secs": progress.eta_secs,
            }
        });
        let broadcaster = self.broadcaster.clone();
        let user_id = self.user_id.clone();
        tokio::spawn(async move {
            broadcaster.send_to_user(&user_id, &msg).await;
        });
    }

    fn on_phase_change(&self, _phase: &str, detail: &str) {
        let msg = serde_json::json!({
            "type": "download_progress",
            "data": {
                "download_id": self.download_id,
                "phase": detail,
                "percent": 0,
                "downloaded_bytes": 0,
                "total_bytes": null,
                "speed_bps": 0,
                "eta_secs": null,
            }
        });
        let broadcaster = self.broadcaster.clone();
        let user_id = self.user_id.clone();
        tokio::spawn(async move {
            broadcaster.send_to_user(&user_id, &msg).await;
        });
    }

    fn on_complete(&self, result: &DownloadResult) {
        let msg = serde_json::json!({
            "type": "download_complete",
            "data": {
                "download_id": self.download_id,
                "output_uri": result.output_uri,
                "title": result.title,
                "quality": result.quality,
            }
        });
        let broadcaster = self.broadcaster.clone();
        let user_id = self.user_id.clone();
        tokio::spawn(async move {
            broadcaster.send_to_user(&user_id, &msg).await;
        });
    }

    fn on_error(&self, err: &str) {
        let msg = serde_json::json!({
            "type": "download_failed",
            "data": {
                "download_id": self.download_id,
                "error": err,
            }
        });
        let broadcaster = self.broadcaster.clone();
        let user_id = self.user_id.clone();
        tokio::spawn(async move {
            broadcaster.send_to_user(&user_id, &msg).await;
        });
    }
}

/// Optional metadata + per-request overrides threaded into a download.
/// The watchlist code paths set `tracked_series_id` and (on upgrades)
/// `superseded_download_id`; the public `POST /downloads` route leaves
/// both at their defaults.
#[derive(Default, Debug)]
pub struct StartDownloadParams {
    pub options_json: serde_json::Value,
    pub tracked_series_id: Option<String>,
    pub superseded_download_id: Option<String>,
    /// Per-request override for the existence-guard behaviour.
    ///
    /// - `Some(true)`: skip both guards, mark prior row superseded, overwrite on disk.
    /// - `Some(false)`: run both guards (skip on hit), regardless of user setting.
    /// - `None`: fall back to the user's `on_existing_download` setting in
    ///   `user_settings.settings_json` (default `"skip"`).
    ///
    /// Watchlist upgrades (`superseded_download_id = Some(_)`) always set
    /// this to `Some(true)`.
    pub force: Option<bool>,
}

impl DownloadService {
    pub fn new(ws_broadcaster: Arc<WsBroadcaster>) -> Self {
        Self {
            active_downloads: RwLock::new(HashMap::new()),
            download_semaphores: RwLock::new(HashMap::new()),
            ws_broadcaster,
        }
    }

    /// Return an `Arc<Semaphore>` sized to the caller's current
    /// `simultaneous_downloads` setting. If the user already has a semaphore
    /// at the same size, it is reused — so successive tasks queue against the
    /// same gate. If the setting changed, the entry is replaced and any
    /// in-flight tasks finish under their cloned reference to the old gate.
    async fn get_or_create_semaphore(&self, user_id: &str, max: u8) -> Arc<Semaphore> {
        let max = max.max(1);
        {
            let map = self.download_semaphores.read().await;
            if let Some(entry) = map.get(user_id) {
                if entry.size == max {
                    return entry.sem.clone();
                }
            }
        }
        let mut map = self.download_semaphores.write().await;
        match map.get(user_id) {
            Some(entry) if entry.size == max => entry.sem.clone(),
            _ => {
                let sem = Arc::new(Semaphore::new(max as usize));
                map.insert(
                    user_id.to_string(),
                    SemaphoreEntry { sem: sem.clone(), size: max },
                );
                sem
            }
        }
    }

    /// Public API: kicks off a download for the given URL using the user's
    /// saved settings + per-request overrides. No tracking metadata.
    pub async fn start_download(
        &self,
        user_id: &str,
        url: &str,
        options_json: serde_json::Value,
        db: &SqlitePool,
    ) -> Result<Vec<EpisodeOutcome>, ApiError> {
        // None when the caller didn't pass `options.force` — start_download_inner
        // will then fall back to the user's on_existing_download setting.
        let force = options_json.get("force").and_then(|v| v.as_bool());
        self.start_download_inner(
            user_id,
            url,
            StartDownloadParams { options_json, force, ..Default::default() },
            db,
        )
        .await
    }

    /// Used by `TrackingService` — same as `start_download` but with watchlist
    /// metadata wired in. Always uses saved settings (no per-request overrides
    /// exposed to the polling worker). Upgrades (`superseded_download_id` set)
    /// imply `force = true`.
    pub async fn start_tracking_download(
        &self,
        user_id: &str,
        episode_id: &str,
        tracked_series_id: &str,
        superseded_download_id: Option<String>,
        db: &SqlitePool,
    ) -> Result<Vec<EpisodeOutcome>, ApiError> {
        let url = format!("https://www.crunchyroll.com/watch/{}", episode_id);
        // Tracking always knows whether it wants to bypass the guards; never
        // fall through to the user's on_existing_download setting.
        let force = Some(superseded_download_id.is_some());
        self.start_download_inner(
            user_id,
            &url,
            StartDownloadParams {
                tracked_series_id: Some(tracked_series_id.to_string()),
                superseded_download_id,
                force,
                ..Default::default()
            },
            db,
        )
        .await
    }

    /// Shared implementation. Returns one `EpisodeOutcome` per resolved
    /// episode — either started (with a new `download_id`) or skipped.
    async fn start_download_inner(
        &self,
        user_id: &str,
        url: &str,
        params: StartDownloadParams,
        db: &SqlitePool,
    ) -> Result<Vec<EpisodeOutcome>, ApiError> {
        let StartDownloadParams {
            options_json,
            tracked_series_id,
            superseded_download_id,
            force: force_override,
        } = params;
        // Get user's CR client
        let cr_service = crate::services::crunchyroll::CrunchyrollService::new(db.clone());
        let client = cr_service.get_client(user_id).await?;
        let client = Arc::new(client);

        // Resolve URL to episodes
        let parsed_url = crunchy_cli::cli::CrunchyrollUrl::parse(url)
            .ok_or_else(|| ApiError::BadRequest("Invalid Crunchyroll URL".to_string()))?;

        let episodes = crunchy_cli::download::resolve_episodes(&client, &parsed_url)
            .await
            .map_err(crunchy_cli::Error::from)?;

        // Load user settings from DB and apply to config
        let mut cfg = crunchy_cli::config::Config::default();
        let settings_value: Option<serde_json::Value> = sqlx::query_scalar::<_, Option<String>>(
            "SELECT settings_json FROM user_settings WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok());
        // Apply saved user settings (full set) first…
        if let Some(s) = settings_value.as_ref() {
            apply_overrides_to_config(&mut cfg, s);
        }

        // …then per-request overrides (allowlisted keys only) on top.
        let filtered_overrides = filter_overrides(&options_json);
        if filtered_overrides.is_object() {
            apply_overrides_to_config(&mut cfg, &filtered_overrides);
        }

        // Resolve effective `force`. Per-request `options.force` wins when
        // present; otherwise consult the user's `on_existing_download`
        // setting (default `"skip"` = force off).
        let force = force_override.unwrap_or_else(|| {
            settings_value
                .as_ref()
                .and_then(|s| s.get("on_existing_download"))
                .and_then(|v| v.as_str())
                .map(|s| s == "replace")
                .unwrap_or(false)
        });

        let storage_cfg = crate::services::storage_config::StorageConfig::from_settings(
            settings_value.as_ref().unwrap_or(&serde_json::Value::Null),
            cfg.downloads.output_dir.clone(),
        )?;
        let sink = storage_cfg.build_sink().await?;

        // Per-user concurrency gate. Sized from the just-merged user setting.
        // If the user changes `simultaneous_downloads` between calls, the
        // helper recreates the semaphore — old tasks finish on the old one.
        let user_sem = self
            .get_or_create_semaphore(user_id, cfg.downloads.simultaneous)
            .await;

        let config = Arc::new(tokio::sync::RwLock::new(cfg));

        let now = Utc::now().to_rfc3339();
        let mut results = Vec::new();

        for episode in &episodes {
            // Pre-trigger guards. Skip both the DB and FS check when force is
            // set or when the watchlist is doing an upgrade (superseded_id
            // implies the caller already decided to re-download).
            if !force {
                // DB existence: completed → 'already_downloaded'; active/pending
                // → 'in_progress'; failed/cancelled/paused → allow retry.
                if let Some((existing_id, existing_status)) =
                    latest_existing_download(db, user_id, &episode.id).await?
                {
                    let reason = match existing_status.as_str() {
                        "completed" => Some("already_downloaded"),
                        "active" | "pending" => Some("in_progress"),
                        _ => None,
                    };
                    if let Some(reason) = reason {
                        results.push(EpisodeOutcome::skipped(
                            episode.id.clone(),
                            episode.title.clone(),
                            reason,
                            Some(existing_id),
                            None,
                        ));
                        continue;
                    }
                }

                // Filesystem existence: only when the template has no
                // runtime-resolved vars (otherwise we'd need a streaming-token
                // -priced playback call to know the final path; the in-pipeline
                // skip_existing inside DownloadManager handles those).
                let pre_path = {
                    let cfg_read = config.read().await;
                    precomputed_output_path(&cfg_read, episode)
                };
                if let Some(path) = pre_path {
                    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
                        results.push(EpisodeOutcome::skipped(
                            episode.id.clone(),
                            episode.title.clone(),
                            "file_exists",
                            None,
                            Some(path.to_string_lossy().into_owned()),
                        ));
                        continue;
                    }
                }
            } else if superseded_download_id.is_none() {
                // User-initiated force re-download: mark the prior
                // non-superseded row (if any) as superseded so the UI hides
                // it once the new one starts. Watchlist upgrades manage
                // supersession themselves via `superseded_download_id`.
                if let Some((existing_id, _)) =
                    latest_existing_download(db, user_id, &episode.id).await?
                {
                    let _ = sqlx::query(
                        "UPDATE downloads SET superseded = 1, updated_at = ? WHERE id = ?",
                    )
                    .bind(&now)
                    .bind(&existing_id)
                    .execute(db)
                    .await;
                }
            }

            let download_id = Uuid::new_v4().to_string();
            let season_num = if episode.season_sequence_number > 0 {
                episode.season_sequence_number
            } else {
                episode.season_number
            };

            let thumbnail_url: Option<String> = episode
                .images
                .thumbnail
                .first()
                .and_then(|variants| {
                    variants
                        .iter()
                        .min_by_key(|img| (img.width as i32 - 320).unsigned_abs())
                })
                .map(|img| img.source.clone());

            // Store the per-episode URL, not the original request URL. A
            // season-level POST /downloads passes a series URL which fans
            // out across N episodes; persisting that URL would make every
            // row's retry re-resolve the whole series. Use the canonical
            // /watch/{episode_id} shape (same as start_tracking_download).
            let episode_url =
                format!("https://www.crunchyroll.com/watch/{}", episode.id);

            sqlx::query(
                "INSERT INTO downloads (id, user_id, episode_id, source_url, series_title, episode_title, season_number, episode_number, status, thumbnail_url, tracked_series_id, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?)"
            )
            .bind(&download_id)
            .bind(user_id)
            .bind(&episode.id)
            .bind(&episode_url)
            .bind(&episode.series_title)
            .bind(&episode.title)
            .bind(season_num as i64)
            .bind(episode.episode_number.map(|n| n as f64))
            .bind(&thumbnail_url)
            .bind(tracked_series_id.as_deref())
            .bind(&now)
            .bind(&now)
            .execute(db)
            .await?;

            let reporter = Arc::new(WebSocketProgress {
                download_id: download_id.clone(),
                user_id: user_id.to_string(),
                broadcaster: self.ws_broadcaster.clone(),
            });

            let cancel_token = CancellationToken::new();
            let cancel_clone = cancel_token.clone();
            let episode_id = episode.id.clone();
            let client_clone = client.clone();
            let db_clone = db.clone();
            let dl_id = download_id.clone();
            let uid = user_id.to_string();
            let sink_clone = sink.clone();
            let task_superseded_id = superseded_download_id.clone();
            let sem_clone = user_sem.clone();

            let widevine_client_tmp = materialize_widevine_blob(
                settings_value.as_ref(),
                "widevine_client",
            )?;
            let widevine_private_key_tmp = materialize_widevine_blob(
                settings_value.as_ref(),
                "widevine_private_key",
            )?;

            let mut request_cfg = config.read().await.clone();
            if let Some(ref tmp) = widevine_client_tmp {
                request_cfg.tools.widevine_client = Some(tmp.path().to_path_buf());
            }
            if let Some(ref tmp) = widevine_private_key_tmp {
                request_cfg.tools.widevine_private_key = Some(tmp.path().to_path_buf());
            }
            let request_config = Arc::new(tokio::sync::RwLock::new(request_cfg));

            let task = tokio::spawn(async move {
                let _widevine_client_tmp = widevine_client_tmp;
                let _widevine_private_key_tmp = widevine_private_key_tmp;

                // Wait for our turn under the per-user concurrency cap. The
                // permit is held for the lifetime of the task and dropped
                // automatically on completion, failure, or cancel.
                let _permit = match sem_clone.acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => {
                        warn!("Download {} aborted: user semaphore closed", dl_id);
                        let _ = sqlx::query(
                            "UPDATE downloads SET status = 'failed', error = 'concurrency gate closed', updated_at = ? WHERE id = ?"
                        )
                        .bind(Utc::now().to_rfc3339())
                        .bind(&dl_id)
                        .execute(&db_clone)
                        .await;
                        return;
                    }
                };

                // Cancellation can land while we were queued under the
                // semaphore. Honour it before doing any CR-side work.
                // The WHERE clause prevents racing with pause() — if the
                // caller already set 'paused' or another concurrent path
                // set 'cancelled', don't overwrite.
                if cancel_clone.is_cancelled() {
                    let _ = sqlx::query(
                        "UPDATE downloads SET status = 'cancelled', updated_at = ? \
                         WHERE id = ? AND status NOT IN ('paused', 'cancelled')",
                    )
                    .bind(Utc::now().to_rfc3339())
                    .bind(&dl_id)
                    .execute(&db_clone)
                    .await;
                    return;
                }

                // We got the permit — flip pending → active.
                let _ = sqlx::query(
                    "UPDATE downloads SET status = 'active', updated_at = ? WHERE id = ?"
                )
                .bind(Utc::now().to_rfc3339())
                .bind(&dl_id)
                .execute(&db_clone)
                .await;

                let manager = DownloadManager::with_reporter_and_sink(
                    client_clone,
                    request_config,
                    reporter,
                    sink_clone,
                );

                tokio::select! {
                    result = manager.download_episode_with_options(
                        &episode_id,
                        crunchy_cli::download::DownloadOptions {
                            resume_cache: true,
                            ..Default::default()
                        },
                    ) => {
                        match result {
                            Ok(dl_result) => {
                                info!("Download {} completed: {}", dl_id, dl_result.output_uri);
                                let audio_json = serde_json::to_string(&dl_result.audio_languages).unwrap_or_default();
                                let sub_json = serde_json::to_string(&dl_result.subtitle_languages).unwrap_or_default();
                                let _ = sqlx::query(
                                    "UPDATE downloads SET status = 'completed', output_path = ?, audio_tracks = ?, subtitle_tracks = ?, updated_at = ? WHERE id = ?"
                                )
                                .bind(&dl_result.output_uri)
                                .bind(&audio_json)
                                .bind(&sub_json)
                                .bind(Utc::now().to_rfc3339())
                                .bind(&dl_id)
                                .execute(&db_clone)
                                .await;

                                // If this was an upgrade attempt, reconcile against the old row.
                                if let Some(ref superseded_id) = task_superseded_id {
                                    crate::services::tracking::handle_upgrade_completion(
                                        &db_clone,
                                        &dl_id,
                                        superseded_id,
                                        &dl_result.audio_languages,
                                        &dl_result.subtitle_languages,
                                    )
                                    .await;
                                }
                            }
                            Err(e) => {
                                error!("Download {} failed: {}", dl_id, e);
                                let failure_status = if e.to_string().contains("S3 upload failed") {
                                    "publish_failed"
                                } else {
                                    "failed"
                                };
                                let _ = sqlx::query(
                                    "UPDATE downloads SET status = ?, error = ?, updated_at = ? WHERE id = ?"
                                )
                                .bind(failure_status)
                                .bind(e.to_string())
                                .bind(Utc::now().to_rfc3339())
                                .bind(&dl_id)
                                .execute(&db_clone)
                                .await;

                                // Failed upgrade — un-supersede the old row and clear cooldown
                                // so the next poll retries. Without this the old row stays
                                // superseded indefinitely.
                                if let Some(ref superseded_id) = task_superseded_id {
                                    let _ = sqlx::query(
                                        "UPDATE downloads SET superseded = 0, upgrade_checked_at = NULL WHERE id = ?"
                                    )
                                    .bind(superseded_id)
                                    .execute(&db_clone)
                                    .await;
                                }
                            }
                        }
                    }
                    _ = cancel_clone.cancelled() => {
                        info!("Download {} cancelled", dl_id);
                        // Don't overwrite if pause() already flipped to
                        // 'paused' — that one is RAII-cleaned to 'paused'
                        // by an external write that races our cancel-token.
                        let _ = sqlx::query(
                            "UPDATE downloads SET status = 'cancelled', updated_at = ? \
                             WHERE id = ? AND status NOT IN ('paused', 'cancelled')",
                        )
                        .bind(Utc::now().to_rfc3339())
                        .bind(&dl_id)
                        .execute(&db_clone)
                        .await;
                        // Cancelled upgrade — same rollback as failure.
                        if let Some(ref superseded_id) = task_superseded_id {
                            let _ = sqlx::query(
                                "UPDATE downloads SET superseded = 0, upgrade_checked_at = NULL WHERE id = ?"
                            )
                            .bind(superseded_id)
                            .execute(&db_clone)
                            .await;
                        }
                    }
                }
            });

            {
                let mut active = self.active_downloads.write().await;
                active.insert(
                    download_id.clone(),
                    DownloadHandle {
                        user_id: uid,
                        cancel_token,
                        _task: task,
                    },
                );
            }

            results.push(EpisodeOutcome::started(
                download_id,
                episode.id.clone(),
                episode.title.clone(),
            ));
        }

        Ok(results)
    }

    /// List downloads for a user with optional status filter and cursor pagination.
    /// Hides superseded rows by default (they exist transiently while a watchlist
    /// upgrade is in flight). Pass `include_superseded = true` to drop the filter.
    pub async fn list_downloads(
        &self,
        user_id: &str,
        status: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
        include_superseded: bool,
        db: &SqlitePool,
    ) -> Result<Vec<DownloadRow>, ApiError> {
        let mut sql = String::from("SELECT * FROM downloads WHERE user_id = ?");
        if !include_superseded {
            sql.push_str(" AND superseded = 0");
        }
        let mut args: Vec<String> = vec![user_id.to_string()];

        if let Some(s) = status {
            match s {
                "active" => {
                    sql.push_str(" AND status IN ('active', 'pending', 'paused')");
                }
                "completed" | "failed" | "cancelled" => {
                    sql.push_str(" AND status = ?");
                    args.push(s.to_string());
                }
                _ => {}
            }
        }

        if let Some(c) = cursor {
            sql.push_str(" AND created_at < ?");
            args.push(c.to_string());
        }

        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {}", limit + 1));

        let mut query = sqlx::query_as::<_, DownloadRow>(&sql);
        for arg in &args {
            query = query.bind(arg);
        }

        let rows = query.fetch_all(db).await?;
        Ok(rows)
    }

    pub async fn download_counts(
        &self,
        user_id: &str,
        db: &SqlitePool,
    ) -> Result<crate::routes::downloads::DownloadCounts, ApiError> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT status, COUNT(*) as count FROM downloads \
             WHERE user_id = ? AND superseded = 0 \
             GROUP BY status",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;

        let mut all = 0;
        let mut active = 0;
        let mut completed = 0;
        let mut failed = 0;
        let mut cancelled = 0;

        for (status, count) in rows {
            all += count;
            match status.as_str() {
                "active" | "pending" | "paused" => active += count,
                "completed" => completed += count,
                "failed" | "publish_failed" => failed += count,
                "cancelled" => cancelled += count,
                _ => {}
            }
        }

        Ok(crate::routes::downloads::DownloadCounts {
            all,
            active,
            completed,
            failed,
            cancelled,
        })
    }

    /// Returns the set of `episode_id`s the user has at least one completed,
    /// non-superseded download for. Used by the UI to badge already-downloaded
    /// episodes. Excludes `superseded = 1` rows so the badge doesn't show during
    /// an in-flight watchlist upgrade.
    pub async fn completed_episode_ids(
        &self,
        user_id: &str,
        db: &SqlitePool,
    ) -> Result<Vec<String>, ApiError> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT episode_id FROM downloads \
             WHERE user_id = ? AND status = 'completed' AND superseded = 0",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Returns `(real_completed, manual)` — episode IDs split by whether the
    /// "completed" row was a real download or a user-marked one.
    pub async fn episode_id_buckets(
        &self,
        user_id: &str,
        db: &SqlitePool,
    ) -> Result<(Vec<String>, Vec<String>), ApiError> {
        let rows = sqlx::query_as::<_, (String, bool)>(
            "SELECT DISTINCT episode_id, manual FROM downloads \
             WHERE user_id = ? AND status = 'completed' AND superseded = 0",
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        let mut real = Vec::new();
        let mut manual = Vec::new();
        for (episode_id, is_manual) in rows {
            if is_manual {
                manual.push(episode_id);
            } else {
                real.push(episode_id);
            }
        }
        Ok((real, manual))
    }

    /// Mark each episode as manually downloaded. No-op for episodes that
    /// already have a real (manual = 0) completed row. Returns `(marked, skipped)`.
    pub async fn mark_manual(
        &self,
        user_id: &str,
        items: &[crate::routes::downloads::MarkManualRequest],
        db: &SqlitePool,
    ) -> Result<(u32, u32), ApiError> {
        let mut marked = 0u32;
        let mut skipped = 0u32;
        let now = Utc::now().to_rfc3339();

        for item in items {
            // Check whether a real (non-manual) completed row already exists.
            let real_exists: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM downloads \
                 WHERE user_id = ? AND episode_id = ? \
                   AND status = 'completed' AND superseded = 0 AND manual = 0 \
                 LIMIT 1",
            )
            .bind(user_id)
            .bind(&item.episode_id)
            .fetch_optional(db)
            .await?;
            if real_exists.is_some() {
                skipped += 1;
                continue;
            }

            // Idempotent w.r.t. already-manual rows: a UNIQUE-aware upsert isn't
            // possible without a unique constraint, so do a select-then-insert.
            let manual_exists: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM downloads \
                 WHERE user_id = ? AND episode_id = ? AND manual = 1 \
                 LIMIT 1",
            )
            .bind(user_id)
            .bind(&item.episode_id)
            .fetch_optional(db)
            .await?;
            if manual_exists.is_some() {
                continue;
            }

            let id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO downloads (id, user_id, episode_id, series_title, episode_title, \
                                        season_number, episode_number, status, thumbnail_url, \
                                        audio_tracks, subtitle_tracks, manual, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, 'completed', ?, '[]', '[]', 1, ?, ?)",
            )
            .bind(&id)
            .bind(user_id)
            .bind(&item.episode_id)
            .bind(&item.series_title)
            .bind(&item.episode_title)
            .bind(item.season_number)
            .bind(item.episode_number)
            .bind(&item.thumbnail_url)
            .bind(&now)
            .bind(&now)
            .execute(db)
            .await?;
            marked += 1;
        }
        Ok((marked, skipped))
    }

    /// Remove a manual mark by episode_id. Returns `false` if no manual row existed.
    pub async fn unmark_manual(
        &self,
        user_id: &str,
        episode_id: &str,
        db: &SqlitePool,
    ) -> Result<bool, ApiError> {
        let result = sqlx::query(
            "DELETE FROM downloads \
             WHERE user_id = ? AND episode_id = ? AND manual = 1",
        )
        .bind(user_id)
        .bind(episode_id)
        .execute(db)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_download(
        &self,
        user_id: &str,
        download_id: &str,
        db: &SqlitePool,
    ) -> Result<DownloadRow, ApiError> {
        sqlx::query_as::<_, DownloadRow>(
            "SELECT * FROM downloads WHERE id = ? AND user_id = ?",
        )
        .bind(download_id)
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Download not found".to_string()))
    }

    pub async fn cancel_download(
        &self,
        user_id: &str,
        download_id: &str,
        db: &SqlitePool,
    ) -> Result<(), ApiError> {
        let mut active = self.active_downloads.write().await;
        if let Some(handle) = active.remove(download_id) {
            if handle.user_id != user_id {
                return Err(ApiError::Forbidden("Not your download".to_string()));
            }
            handle.cancel_token.cancel();
        }

        sqlx::query(
            "UPDATE downloads SET status = 'cancelled', updated_at = ? WHERE id = ? AND user_id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(download_id)
        .bind(user_id)
        .execute(db)
        .await?;

        Ok(())
    }

    /// Cancel every in-flight download (status active/pending/paused) for
    /// `user_id`. Triggers the in-memory cancel-token so the spawned task
    /// exits at its next await point, then bulk-updates the DB rows.
    /// Returns the number of rows that transitioned to `cancelled`.
    pub async fn cancel_active_for_user(
        &self,
        user_id: &str,
        db: &SqlitePool,
    ) -> Result<u64, ApiError> {
        // Fire the cancel-tokens for every in-memory active download owned
        // by this user. Drop the write lock before doing DB work.
        let to_cancel: Vec<String> = {
            let mut active = self.active_downloads.write().await;
            let ids: Vec<String> = active
                .iter()
                .filter(|(_, h)| h.user_id == user_id)
                .map(|(id, _)| id.clone())
                .collect();
            for id in &ids {
                if let Some(handle) = active.remove(id) {
                    handle.cancel_token.cancel();
                }
            }
            ids
        };
        info!(
            "cancel_active_for_user({}): signalled {} in-memory handles",
            user_id,
            to_cancel.len()
        );

        // Bulk-flip the DB. Includes rows the task-loop hasn't observed
        // yet (e.g. pending rows still waiting on the semaphore).
        let updated = sqlx::query(
            "UPDATE downloads \
             SET status = 'cancelled', updated_at = ? \
             WHERE user_id = ? AND status IN ('active', 'pending', 'paused')",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(user_id)
        .execute(db)
        .await?
        .rows_affected();

        Ok(updated)
    }

    pub async fn pause_download(
        &self,
        user_id: &str,
        download_id: &str,
        db: &SqlitePool,
    ) -> Result<(), ApiError> {
        let mut active = self.active_downloads.write().await;
        if let Some(handle) = active.remove(download_id) {
            if handle.user_id != user_id {
                return Err(ApiError::Forbidden("Not your download".to_string()));
            }
            handle.cancel_token.cancel();
        }

        sqlx::query(
            "UPDATE downloads SET status = 'paused', updated_at = ? WHERE id = ? AND user_id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(download_id)
        .bind(user_id)
        .execute(db)
        .await?;

        Ok(())
    }

    pub async fn resume_download(
        &self,
        user_id: &str,
        download_id: &str,
        db: &SqlitePool,
    ) -> Result<(), ApiError> {
        let row = self.get_download(user_id, download_id, db).await?;

        // failed / publish_failed / paused all share the same recovery
        // shape: delete the old row, re-issue start_download with the
        // original URL. The spawned task's resume_cache=true picks up
        // any on-disk segments from /tmp/crunchy-cli/<episode_id>/ so
        // paused downloads continue rather than restart from zero.
        if matches!(row.status.as_str(), "failed" | "publish_failed" | "paused") {
            let source_url = row.source_url.clone().ok_or_else(|| {
                ApiError::BadRequest(
                    "Download cannot be resumed: source URL missing".to_string(),
                )
            })?;

            sqlx::query("DELETE FROM downloads WHERE id = ? AND user_id = ?")
                .bind(download_id)
                .bind(user_id)
                .execute(db)
                .await?;

            self.start_download(user_id, &source_url, serde_json::Value::Null, db)
                .await?;
            return Ok(());
        }

        Err(ApiError::BadRequest(format!(
            "Download cannot be resumed from status '{}'",
            row.status
        )))
    }
}

#[derive(Debug, sqlx::FromRow, Serialize, utoipa::ToSchema)]
pub struct DownloadRow {
    pub id: String,
    pub user_id: String,
    pub episode_id: String,
    pub series_title: Option<String>,
    pub episode_title: Option<String>,
    pub season_number: Option<i64>,
    pub episode_number: Option<f64>,
    pub status: String,
    pub options_json: Option<String>,
    pub progress_json: Option<String>,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub source_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Watchlist columns (migration 009). All-None when the row predates the
    /// migration or wasn't initiated by the watchlist worker.
    pub audio_tracks: Option<String>,
    pub subtitle_tracks: Option<String>,
    pub tracked_series_id: Option<String>,
    pub upgrade_checked_at: Option<String>,
    #[serde(default)]
    pub superseded: bool,
    /// `true` for rows the user manually marked as downloaded. The watchlist
    /// worker treats these as "we have it, don't auto-download" and skips
    /// upgrade detection (no track lists to compare against).
    #[serde(default)]
    pub manual: bool,
}

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
        let obj = filtered.as_object().expect("filtered should be an object");
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

    #[test]
    fn empty_array_does_not_clear_saved_value() {
        // Documented: empty array means "skip", not "reset to default".
        let mut cfg = crunchy_cli::config::Config::default();
        let saved = serde_json::json!({ "audio_languages": ["ja-JP", "en-US"] });
        apply_overrides_to_config(&mut cfg, &saved);

        let overrides = serde_json::json!({ "audio_languages": [] });
        let filtered = filter_overrides(&overrides);
        apply_overrides_to_config(&mut cfg, &filtered);

        assert_eq!(cfg.languages.audio, vec!["ja-JP".to_string(), "en-US".to_string()]);
    }
}
