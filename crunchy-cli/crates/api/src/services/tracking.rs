//! Tracked series polling + upgrade-on-new-tracks logic.

use crate::db;
use crate::db::tracking::TrackedSeriesRow;
use crate::error::ApiError;
use crate::services::crunchyroll::CrunchyrollService;
use crate::services::download::{DownloadRow, DownloadService};
use chrono::{Duration as ChronoDuration, Utc};
use crunchy_cli::api::CrunchyrollClient;
use crunchy_cli::api::types::CREpisode;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use utoipa::ToSchema;

/// Fallback `Retry-After` when CR returns 429 with no header. The CR client's
/// per-user + global request-rate limiters (see `services/crunchyroll.rs`)
/// prevent most 429s; this is the belt-and-suspenders catch for residuals
/// from Cloudflare WAF burst windows.
const DEFAULT_RATE_LIMIT_BACKOFF: Duration = Duration::from_secs(60);

/// Counts returned by a manual `POST /tracking/:id/check` so the UI can
/// surface "started 3 new downloads, 1 upgrade".
#[derive(Debug, Default, Serialize, ToSchema, Clone)]
pub struct CheckSummary {
    pub new_downloads: u32,
    pub upgrades: u32,
    pub checked_episodes: u32,
}

pub struct TrackingService {
    db: SqlitePool,
    download_service: Arc<DownloadService>,
    crunchyroll_service: CrunchyrollService,
    /// Fallback interval used when no row in `app_settings` overrides it.
    /// Comes from the `TRACKING_INTERVAL_SECS` env var.
    default_interval_secs: u64,
}

impl TrackingService {
    pub fn new(
        db: SqlitePool,
        download_service: Arc<DownloadService>,
        default_interval_secs: u64,
    ) -> Arc<Self> {
        Arc::new(Self {
            crunchyroll_service: CrunchyrollService::new(db.clone()),
            db,
            download_service,
            default_interval_secs,
        })
    }

    /// Read the active polling interval. DB override (set via the web UI)
    /// wins; otherwise fall back to the value passed at construction.
    async fn current_interval(&self) -> Duration {
        let secs = db::app_settings::get_tracking_interval_secs(&self.db)
            .await
            .unwrap_or(self.default_interval_secs);
        Duration::from_secs(secs)
    }

    /// Spawn a background loop that calls `run_check` periodically. The
    /// interval is re-read from the DB on every iteration so changes from
    /// `PATCH /app-settings` apply on the next tick without a restart.
    pub fn spawn(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                let interval = self.current_interval().await;
                sleep(interval).await;
                if let Err(e) = self.run_check().await {
                    error!("Tracking poll failed: {:?}", e);
                }
            }
        });
    }

    async fn run_check(&self) -> Result<(), ApiError> {
        let entries = db::tracking::list_all_enabled(&self.db).await?;
        info!("Tracking poll: {} enabled series", entries.len());
        for (i, entry) in entries.iter().enumerate() {
            match self.check_series(entry).await {
                Ok(summary) => {
                    if summary.new_downloads > 0 || summary.upgrades > 0 {
                        info!(
                            "Tracked {}: {} new, {} upgrade",
                            entry.series_title,
                            summary.new_downloads,
                            summary.upgrades
                        );
                    }
                }
                Err(ApiError::RateLimited { retry_after }) => {
                    // CR is throttling us. Hammering through the rest of the
                    // backlog will only deepen the throttle — bail out of the
                    // tick. Sleep at least the server-suggested Retry-After
                    // so the next tick starts on the right side of the window.
                    let wait = retry_after
                        .map(Duration::from_secs)
                        .unwrap_or(DEFAULT_RATE_LIMIT_BACKOFF);
                    warn!(
                        "Crunchyroll rate-limited the tracking poll \
                         (Retry-After: {}s, {} of {} series checked); \
                         aborting tick and backing off",
                        wait.as_secs(),
                        i,
                        entries.len()
                    );
                    sleep(wait).await;
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        "check_series failed for {} (user {}): {:?}",
                        entry.series_id, entry.user_id, e
                    );
                }
            }
            if let Err(e) = db::tracking::touch_last_checked(&self.db, &entry.id).await {
                warn!("touch_last_checked failed for {}: {:?}", entry.id, e);
            }
        }
        Ok(())
    }

    pub async fn check_series(
        &self,
        entry: &TrackedSeriesRow,
    ) -> Result<CheckSummary, ApiError> {
        let mut summary = CheckSummary::default();

        let client = self.crunchyroll_service.get_client(&entry.user_id).await?;
        let all_episodes = fetch_all_episodes(&client, &entry.series_id).await?;
        summary.checked_episodes = all_episodes.len() as u32;

        let baseline: HashSet<String> = serde_json::from_str(&entry.baseline_episode_ids)
            .unwrap_or_default();

        let (wanted_audio, wanted_subs) =
            self.get_user_wanted_languages(&entry.user_id).await?;

        // Cooldown cutoff for failed/cancelled retries — same 24h window as upgrades.
        let retry_cutoff = (Utc::now() - ChronoDuration::hours(24)).to_rfc3339();

        for episode in &all_episodes {
            // new_only mode: baseline episodes don't get downloaded but are still
            // upgrade-eligible (a baseline episode that was downloaded outside
            // this app could still be missing tracks).
            if entry.download_mode == "new_only" && baseline.contains(&episode.id) {
                if self
                    .maybe_upgrade(
                        &entry.user_id,
                        &episode.id,
                        &wanted_audio,
                        &wanted_subs,
                        &entry.id,
                    )
                    .await?
                {
                    summary.upgrades += 1;
                }
                continue;
            }

            let existing = self
                .find_existing_download(&entry.user_id, &episode.id)
                .await?;

            match existing {
                None => {
                    self.download_service
                        .start_tracking_download(
                            &entry.user_id,
                            &episode.id,
                            &entry.id,
                            None,
                            &self.db,
                        )
                        .await?;
                    summary.new_downloads += 1;
                }
                Some(row) if row.status == "completed" => {
                    if self
                        .maybe_upgrade(
                            &entry.user_id,
                            &episode.id,
                            &wanted_audio,
                            &wanted_subs,
                            &entry.id,
                        )
                        .await?
                    {
                        summary.upgrades += 1;
                    }
                }
                Some(row) if matches!(row.status.as_str(), "failed" | "cancelled" | "publish_failed") => {
                    if row.updated_at > retry_cutoff {
                        // Within 24h cooldown — don't hammer CR.
                        continue;
                    }
                    self.download_service
                        .start_tracking_download(
                            &entry.user_id,
                            &episode.id,
                            &entry.id,
                            None,
                            &self.db,
                        )
                        .await?;
                    summary.new_downloads += 1;
                }
                _ => {} // active, pending, paused — leave alone
            }
        }

        Ok(summary)
    }

    async fn find_existing_download(
        &self,
        user_id: &str,
        episode_id: &str,
    ) -> Result<Option<DownloadRow>, ApiError> {
        let row = sqlx::query_as::<_, DownloadRow>(
            "SELECT * FROM downloads \
             WHERE user_id = ? AND episode_id = ? AND superseded = 0 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(episode_id)
        .fetch_optional(&self.db)
        .await?;
        Ok(row)
    }

    /// Returns `Ok(true)` when an upgrade was actually triggered.
    async fn maybe_upgrade(
        &self,
        user_id: &str,
        episode_id: &str,
        wanted_audio: &[String],
        wanted_subs: &[String],
        tracked_series_id: &str,
    ) -> Result<bool, ApiError> {
        // Manual rows have no track info to compare against and represent files
        // outside our control — never trigger upgrades on them.
        let row = sqlx::query_as::<_, DownloadRow>(
            "SELECT * FROM downloads \
             WHERE user_id = ? AND episode_id = ? AND status = 'completed' \
               AND superseded = 0 AND manual = 0 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(episode_id)
        .fetch_optional(&self.db)
        .await?;
        let Some(row) = row else { return Ok(false) };

        // 24h cooldown — skip if already checked recently.
        if let Some(checked_at) = &row.upgrade_checked_at {
            if let Ok(t) = chrono::DateTime::parse_from_rfc3339(checked_at) {
                if Utc::now().signed_duration_since(t).num_hours() < 24 {
                    return Ok(false);
                }
            }
        }

        let downloaded_audio = parse_locales(row.audio_tracks.as_deref());
        let downloaded_subs = parse_locales(row.subtitle_tracks.as_deref());

        let missing_audio: Vec<&String> = wanted_audio
            .iter()
            .filter(|l| !downloaded_audio.contains(l))
            .collect();
        let missing_subs: Vec<&String> = wanted_subs
            .iter()
            .filter(|l| !downloaded_subs.contains(l))
            .collect();

        if missing_audio.is_empty() && missing_subs.is_empty() {
            // Up to date — refresh timestamp so we don't re-check for 24h.
            sqlx::query("UPDATE downloads SET upgrade_checked_at = ? WHERE id = ?")
                .bind(Utc::now().to_rfc3339())
                .bind(&row.id)
                .execute(&self.db)
                .await?;
            return Ok(false);
        }

        // Atomic supersede gated on cooldown — closes the race where two concurrent
        // polls both spawn duplicate upgrade downloads.
        let now = Utc::now().to_rfc3339();
        let cutoff = (Utc::now() - ChronoDuration::hours(24)).to_rfc3339();
        let result = sqlx::query(
            "UPDATE downloads \
             SET superseded = 1, upgrade_checked_at = ? \
             WHERE id = ? \
               AND superseded = 0 \
               AND (upgrade_checked_at IS NULL OR upgrade_checked_at < ?)",
        )
        .bind(&now)
        .bind(&row.id)
        .bind(&cutoff)
        .execute(&self.db)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(false);
        }

        self.download_service
            .start_tracking_download(
                user_id,
                episode_id,
                tracked_series_id,
                Some(row.id),
                &self.db,
            )
            .await?;

        Ok(true)
    }

    /// Reads the user's `audio_languages` / `subtitle_languages` from settings.
    /// Empty lists when unset — `maybe_upgrade` then never finds anything missing.
    async fn get_user_wanted_languages(
        &self,
        user_id: &str,
    ) -> Result<(Vec<String>, Vec<String>), ApiError> {
        let raw: Option<String> = sqlx::query_scalar::<_, Option<String>>(
            "SELECT settings_json FROM user_settings WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .flatten();
        let Some(raw) = raw else { return Ok((Vec::new(), Vec::new())) };
        let settings: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);

        let audio = settings
            .get("audio_languages")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let subs = settings
            .get("subtitle_languages")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok((audio, subs))
    }
}

/// Walk every season of a series and collect episode rows. Used by the polling
/// worker and by the `POST /tracking` / `PATCH /tracking/:id` baseline-snapshot
/// paths.
pub async fn fetch_all_episodes(
    client: &CrunchyrollClient,
    series_id: &str,
) -> Result<Vec<CREpisode>, ApiError> {
    let seasons = client
        .get_seasons(series_id)
        .await
        .map_err(crunchy_cli::Error::from)?;
    let mut out = Vec::new();
    for season in seasons {
        let eps = client
            .get_episodes(&season.id)
            .await
            .map_err(crunchy_cli::Error::from)?;
        out.extend(eps);
    }
    Ok(out)
}

/// Deserialize a `audio_tracks` / `subtitle_tracks` JSON column, sort & dedup
/// for stable comparison. Returns empty on missing/malformed input.
pub fn parse_locales(json: Option<&str>) -> Vec<String> {
    let mut v: Vec<String> =
        serde_json::from_str(json.unwrap_or("[]")).unwrap_or_default();
    v.sort();
    v.dedup();
    v
}

/// Called from `DownloadService` on completion of an upgrade attempt
/// (when `superseded_download_id` was Some). Reconciles the new + old rows
/// based on whether the new download actually picked up additional tracks.
pub async fn handle_upgrade_completion(
    db: &SqlitePool,
    new_download_id: &str,
    superseded_id: &str,
    new_audio: &[String],
    new_subs: &[String],
) {
    let old_row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT audio_tracks, subtitle_tracks FROM downloads WHERE id = ?",
    )
    .bind(superseded_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    let (old_audio_json, old_sub_json) = old_row.unwrap_or((None, None));

    let old_audio = parse_locales(old_audio_json.as_deref());
    let old_subs = parse_locales(old_sub_json.as_deref());

    let mut new_audio_sorted = new_audio.to_vec();
    new_audio_sorted.sort();
    new_audio_sorted.dedup();
    let mut new_subs_sorted = new_subs.to_vec();
    new_subs_sorted.sort();
    new_subs_sorted.dedup();

    if new_audio_sorted != old_audio || new_subs_sorted != old_subs {
        // New download picked up tracks the old one didn't have — drop the old row.
        let _ = sqlx::query("DELETE FROM downloads WHERE id = ?")
            .bind(superseded_id)
            .execute(db)
            .await;
    } else {
        // Same tracks. Delete the new (redundant) row, un-supersede the old,
        // and apply the 24h cooldown so we don't re-check immediately.
        let _ = sqlx::query("DELETE FROM downloads WHERE id = ?")
            .bind(new_download_id)
            .execute(db)
            .await;
        let _ = sqlx::query(
            "UPDATE downloads SET superseded = 0, upgrade_checked_at = ? WHERE id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(superseded_id)
        .execute(db)
        .await;
    }
}

/// Pick a thumbnail URL from a `CRImages.poster_tall` set — first variant, smallest
/// width. Returns `None` if the series has no poster_tall images.
pub fn pick_series_thumbnail(images: &crunchy_cli::api::types::CRImages) -> Option<String> {
    images
        .poster_tall
        .first()
        .and_then(|variants| variants.iter().min_by_key(|img| img.width))
        .map(|img| img.source.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locales(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_locales_dedup_and_sort() {
        let raw = r#"["en-US","ja-JP","en-US"]"#;
        let out = parse_locales(Some(raw));
        assert_eq!(out, vec!["en-US".to_string(), "ja-JP".to_string()]);
    }

    #[test]
    fn parse_locales_handles_none_and_malformed() {
        assert_eq!(parse_locales(None), Vec::<String>::new());
        assert_eq!(parse_locales(Some("not-json")), Vec::<String>::new());
        assert_eq!(parse_locales(Some("[]")), Vec::<String>::new());
    }

    #[test]
    fn comparison_handles_unsorted_arrays() {
        let mut a = locales(&["en-US", "ja-JP"]);
        let mut b = locales(&["ja-JP", "en-US"]);
        a.sort();
        b.sort();
        assert_eq!(a, b);
    }
}
