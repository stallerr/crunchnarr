# Plan: Series Watchlist & Auto-Download

Track shows, poll for new episodes, download them automatically. Supports two modes — `new_only` (only episodes released after you started tracking) and `all` (backfill + ongoing). Detects when a completed episode can be upgraded with newly available dub/sub tracks and re-downloads it.

**Relationship to bookmarks.** This feature is fully independent from `bookmarks`. A series can be bookmarked without being tracked and vice-versa; one does not imply or modify the other. Different intent: bookmarks = "I want to remember this", tracking = "auto-download this".

---

## Design decisions (agreed)

- **All seasons** — tracking a series monitors every season
- **Two modes**: `new_only` and `all`; switchable after the fact
  - `new_only → all`: triggers a backfill on the next poll (no schema change)
  - `all → new_only`: snapshots the **current** episode list as the new `baseline_episode_ids` at switch-time (one CR fetch on the PATCH). Avoids the empty-baseline foot-gun where every episode would otherwise look "new" and re-download.
- **Upgrade detection** — if a completed episode is missing wanted audio/subtitle tracks and they've since become available on CR, re-download automatically
- **Upgrade cooldown** — skip upgrade check for an episode if it was checked within the last 24 hours (prevents hammering CR when dub hasn't arrived yet)
- **On upgrade complete** — compare *deserialized & sorted* track lists; if the new download has more tracks → delete the old superseded record; if the same tracks → delete the new (redundant) record and clear the superseded flag on the old, applying the 24h cooldown
- **On upgrade failure** — reset `superseded = 0` and clear `upgrade_checked_at` on the old record so the next poll retries (without this, a failed upgrade leaves the old row stuck superseded forever)
- **Failed/cancelled retry cooldown** — the polling worker re-attempts `failed` / `cancelled` downloads only if `updated_at` is older than 24h. Same cooldown shape as upgrades, no schema change. Without this, a CR-side issue (geoblock, auth, rate limit) causes the worker to retry every poll and hammer CR.
- **`POST /tracking/:id/check`** — **synchronous**: awaits the full check and returns `200` with `{ new_downloads: u32, upgrades: u32 }` so the UI can show "started 3 new downloads" immediately
- **Superseded download rows** — hidden by default in `GET /downloads`. List handler filters `WHERE superseded = 0` unless an explicit `include_superseded=true` query param is passed
- **Track button** — lives on the series page next to the Bookmark button. Click opens a small mode-picker dialog (new_only / all). Already tracking → same dialog shows current mode + an Untrack button
- **No auto-bookmark on track** — tracking and bookmarking are independent
- **Notifications** — new and upgrade downloads just appear in the downloads list (no special notification for now)
- **Poll interval** — global, configured via env var `TRACKING_INTERVAL_SECS` (default 3600)

---

## Part 1 — crunchy-cli library: add `audio_languages` to `DownloadResult`

**File:** `crunchy-cli/src/download/manager.rs`

`DownloadResult` currently has `audio_language: String` (singular). Upgrade detection needs to know every audio track that was actually muxed into the output file.

Add a new field:

```rust
pub struct DownloadResult {
    pub output_uri: String,
    pub title: String,
    pub quality: String,
    pub audio_language: String,           // keep (primary audio, already used)
    pub audio_languages: Vec<String>,     // NEW — all audio locales actually downloaded
    pub subtitle_languages: Vec<String>,
}
```

**Three** real construction sites — line 1857 in the file is a unit test, not a real site. Patch all three:

- **`manager.rs:382`** — the `skip_existing` short-circuit (file already exists on disk). Easy to miss; without this, an episode that was downloaded outside the API will never have `audio_languages` populated and upgrade detection silently skips it.
- **`manager.rs:1081`** — the main success path.
- **(test at 1857)** — update the constructor literal so the test still compiles.

Populate `audio_languages` from the **actually muxed** locales, not the keys of `audio_tracks_by_locale` (which are every locale CR exposes for the episode, including ones not selected). At line 1081 use the existing `audio_locales_ordered`:

```rust
audio_languages: audio_locales_ordered.clone(),
```

At line 382 (the skip-existing branch), use the same source the existing `audio_language` field uses — the selected audio tracks:

```rust
audio_languages: selection
    .audio
    .iter()
    .filter_map(|a| a.lang.clone())
    .collect(),
```

If you populated `audio_languages` from `audio_tracks_by_locale.keys()`, every completed download would report having every locale and **upgrade detection would never fire**.

No other crunchy-cli changes needed.

---

## Part 2 — DB migrations

### Migration 008 — `008_tracking.sql`

Create `crunchy-cli/crates/api/src/db/migrations/008_tracking.sql`:

```sql
CREATE TABLE IF NOT EXISTS tracked_series (
    id                   TEXT PRIMARY KEY,
    user_id              TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    series_id            TEXT NOT NULL,
    series_title         TEXT NOT NULL,
    series_thumbnail     TEXT,
    download_mode        TEXT NOT NULL DEFAULT 'new_only',
    baseline_episode_ids TEXT NOT NULL DEFAULT '[]',
    enabled              INTEGER NOT NULL DEFAULT 1,
    added_at             TEXT NOT NULL,
    last_checked_at      TEXT,
    UNIQUE(user_id, series_id)
);
```

`download_mode`: `'new_only'` or `'all'`.
`baseline_episode_ids`: JSON array of episode IDs that existed when the series was first added, used to skip them in `new_only` mode.

This is `CREATE TABLE IF NOT EXISTS` — idempotent, no duplicate-column wrapper needed.

### Migration 009 — `009_downloads_tracking_columns.sql`

Create `crunchy-cli/crates/api/src/db/migrations/009_downloads_tracking_columns.sql`:

```sql
ALTER TABLE downloads ADD COLUMN audio_tracks TEXT;
ALTER TABLE downloads ADD COLUMN subtitle_tracks TEXT;
ALTER TABLE downloads ADD COLUMN tracked_series_id TEXT REFERENCES tracked_series(id) ON DELETE SET NULL;
ALTER TABLE downloads ADD COLUMN upgrade_checked_at TEXT;
ALTER TABLE downloads ADD COLUMN superseded INTEGER NOT NULL DEFAULT 0;
```

These are `ALTER TABLE ADD COLUMN` — not idempotent. Wrap each in the duplicate-column check in `run_migrations`, matching the 003/004 pattern:

```rust
let migration_009 = include_str!("migrations/009_downloads_tracking_columns.sql");
if let Err(e) = sqlx::raw_sql(migration_009).execute(pool).await {
    let msg = e.to_string();
    if !msg.contains("duplicate column") {
        return Err(e);
    }
}
```

### Register both in `src/db/mod.rs` → `run_migrations()`

Add after the existing 007 block:

```rust
let migration_008 = include_str!("migrations/008_tracking.sql");
sqlx::raw_sql(migration_008).execute(pool).await?;

let migration_009 = include_str!("migrations/009_downloads_tracking_columns.sql");
if let Err(e) = sqlx::raw_sql(migration_009).execute(pool).await {
    let msg = e.to_string();
    if !msg.contains("duplicate column") {
        return Err(e);
    }
}
```

Also add `pub mod tracking;` to `src/db/mod.rs`.

---

## Part 3 — DB layer: `src/db/tracking.rs`

Create `crunchy-cli/crates/api/src/db/tracking.rs`:

```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TrackedSeriesRow {
    pub id: String,
    pub user_id: String,
    pub series_id: String,
    pub series_title: String,
    pub series_thumbnail: Option<String>,
    pub download_mode: String,           // "new_only" | "all"
    pub baseline_episode_ids: String,    // JSON array
    pub enabled: bool,
    pub added_at: String,
    pub last_checked_at: Option<String>,
}

pub async fn insert_tracked_series(...) -> Result<(), sqlx::Error>
pub async fn list_tracked_series(pool, user_id) -> Result<Vec<TrackedSeriesRow>, sqlx::Error>
pub async fn get_tracked_series(pool, id, user_id) -> Result<Option<TrackedSeriesRow>, sqlx::Error>
pub async fn update_tracked_series(pool, id, user_id, download_mode, enabled) -> Result<bool, sqlx::Error>
pub async fn delete_tracked_series(pool, id, user_id) -> Result<bool, sqlx::Error>
pub async fn list_all_enabled(pool) -> Result<Vec<TrackedSeriesRow>, sqlx::Error>  // used by polling loop, no user_id filter
pub async fn touch_last_checked(pool, id) -> Result<(), sqlx::Error>
```

---

## Part 4 — `DownloadService`: store track lists on completion

**File:** `crunchy-cli/crates/api/src/services/download.rs`

### 4a — Store `audio_tracks` + `subtitle_tracks` on download completion

In the `Ok(dl_result)` branch inside `tokio::spawn` (currently around line 452), change the UPDATE query to also persist track lists:

```rust
Ok(dl_result) => {
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

    // If this was an upgrade download and succeeded, reconcile against the old row.
    if let Some(ref superseded_id) = superseded_download_id {
        handle_upgrade_completion(
            &db_clone,
            &dl_id,
            superseded_id,
            &dl_result.audio_languages,
            &dl_result.subtitle_languages,
        ).await;
    }
}
```

`superseded_download_id: Option<String>` — a new capture variable in the spawned task closure, passed in from the outer scope. When triggering a normal download it's `None`; when triggering an upgrade it's `Some(old_download_id)`.

### 4b — Add `start_download` overload for tracking

The existing `start_download` already does what we need. The `TrackingService` will call it with:
- `url` = `format!("https://www.crunchyroll.com/watch/{}", episode_id)`
- `options_json` = `serde_json::Value::Null` (use user settings)

However, we also need to set `tracked_series_id` on the new `downloads` row and pass `superseded_download_id` for upgrades. Refactor the shared logic into `start_download_inner` keyed by a parameter struct (six positional args is unwieldy):

```rust
#[derive(Default)]
pub struct StartDownloadParams {
    pub options_json: serde_json::Value,           // Null = use saved settings
    pub tracked_series_id: Option<String>,
    pub superseded_download_id: Option<String>,
}

async fn start_download_inner(
    &self,
    user_id: &str,
    url: &str,
    params: StartDownloadParams,
    db: &SqlitePool,
) -> Result<Vec<(String, String, String)>, ApiError> { ... }

// Public API (HTTP route) — no tracking metadata
pub async fn start_download(
    &self,
    user_id: &str,
    url: &str,
    options_json: serde_json::Value,
    db: &SqlitePool,
) -> Result<Vec<(String, String, String)>, ApiError> {
    self.start_download_inner(user_id, url, StartDownloadParams { options_json, ..Default::default() }, db).await
}

// Tracking-service wrapper
pub async fn start_tracking_download(
    &self,
    user_id: &str,
    episode_id: &str,
    tracked_series_id: &str,
    superseded_download_id: Option<String>,
    db: &SqlitePool,
) -> Result<Vec<(String, String, String)>, ApiError> {
    let url = format!("https://www.crunchyroll.com/watch/{}", episode_id);
    self.start_download_inner(user_id, &url, StartDownloadParams {
        tracked_series_id: Some(tracked_series_id.to_string()),
        superseded_download_id,
        ..Default::default()
    }, db).await
}
```

The INSERT SQL gains a `tracked_series_id` bind. The spawned task carries `superseded_download_id` into both completion branches (see §4a above).

### 4c — Failure-path rollback for upgrades

In the `Err(_)` branch of the spawned task, when `superseded_download_id` is `Some`, reset the old row so the next poll can retry:

```rust
Err(e) => {
    let _ = sqlx::query(
        "UPDATE downloads SET status = 'failed', error = ?, updated_at = ? WHERE id = ?"
    )
    .bind(e.to_string())
    .bind(Utc::now().to_rfc3339())
    .bind(&dl_id)
    .execute(&db_clone).await;

    // If this was an upgrade attempt, un-supersede the old row and clear
    // the cooldown so it gets re-checked on the next poll.
    if let Some(ref superseded_id) = superseded_download_id {
        let _ = sqlx::query(
            "UPDATE downloads SET superseded = 0, upgrade_checked_at = NULL WHERE id = ?"
        )
        .bind(superseded_id)
        .execute(&db_clone).await;
    }
}
```

Without this, a failed upgrade leaves the old row stuck `superseded = 1` indefinitely.

---

## Part 5 — `TrackingService`: `src/services/tracking.rs`

Create `crunchy-cli/crates/api/src/services/tracking.rs`.

### Struct

```rust
pub struct TrackingService {
    db: SqlitePool,
    download_service: Arc<DownloadService>,
    crunchyroll_service: CrunchyrollService,
    check_interval_secs: u64,
}
```

### `new` + `spawn`

```rust
impl TrackingService {
    pub fn new(db: SqlitePool, download_service: Arc<DownloadService>, check_interval_secs: u64) -> Arc<Self> {
        Arc::new(Self { db, download_service, crunchyroll_service: CrunchyrollService::new(db.clone()), check_interval_secs })
    }

    pub fn spawn(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(self.check_interval_secs)
            );
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                if let Err(e) = self.run_check().await {
                    tracing::error!("Tracking check failed: {}", e);
                }
            }
        });
    }
}
```

### `run_check` logic

```rust
#[derive(Default)]
pub struct CheckSummary {
    pub new_downloads: u32,
    pub upgrades: u32,
    pub checked_episodes: u32,
}

async fn run_check(&self) -> Result<(), ApiError> {
    let tracked = db::tracking::list_all_enabled(&self.db).await?;

    for entry in tracked {
        if let Err(e) = self.check_series(&entry).await {
            tracing::warn!("Failed to check series {} for user {}: {}", entry.series_id, entry.user_id, e);
        }
        db::tracking::touch_last_checked(&self.db, &entry.id).await?;
    }
    Ok(())
}

pub async fn check_series(&self, entry: &TrackedSeriesRow) -> Result<CheckSummary, ApiError> {
    let mut summary = CheckSummary::default();

    // 1. Get CR client for this user
    let client = Arc::new(self.crunchyroll_service.get_client(&entry.user_id).await?);

    // 2. Fetch all episode IDs across all seasons (shared helper, see §6)
    let all_episodes = fetch_all_episodes(&client, &entry.series_id).await?;
    summary.checked_episodes = all_episodes.len() as u32;

    // 3. Parse baseline
    let baseline: HashSet<String> = serde_json::from_str(&entry.baseline_episode_ids)
        .unwrap_or_default();

    // 4. Get user's wanted languages from settings
    let (wanted_audio, wanted_subs) = self.get_user_wanted_languages(&entry.user_id).await?;

    let now = Utc::now();
    let cooldown_cutoff = (now - chrono::Duration::hours(24)).to_rfc3339();

    for episode in &all_episodes {
        // 5. new_only: baseline episodes are skipped for new-download, but still
        //    eligible for upgrade detection on completed rows.
        if entry.download_mode == "new_only" && baseline.contains(&episode.id) {
            if self.maybe_upgrade(&entry.user_id, &episode.id, &wanted_audio, &wanted_subs, &entry.id).await? {
                summary.upgrades += 1;
            }
            continue;
        }

        // 6. Decide based on the latest non-superseded download for this episode.
        let existing = self.find_existing_download(&entry.user_id, &episode.id).await?;

        match existing {
            None => {
                self.download_service
                    .start_tracking_download(&entry.user_id, &episode.id, &entry.id, None, &self.db)
                    .await?;
                summary.new_downloads += 1;
            }
            Some(row) if row.status == "completed" => {
                if self.maybe_upgrade(&entry.user_id, &episode.id, &wanted_audio, &wanted_subs, &entry.id).await? {
                    summary.upgrades += 1;
                }
            }
            Some(row) if matches!(row.status.as_str(), "failed" | "cancelled") => {
                // 24h cooldown — don't retry rows that updated within the window.
                if row.updated_at > cooldown_cutoff {
                    continue;
                }
                self.download_service
                    .start_tracking_download(&entry.user_id, &episode.id, &entry.id, None, &self.db)
                    .await?;
                summary.new_downloads += 1;
            }
            _ => {} // active/pending — skip
        }
    }

    Ok(summary)
}

/// Latest non-superseded `downloads` row for this user+episode. `None` when
/// the episode has never been downloaded (or its only attempts were superseded
/// — those rows shouldn't exist in steady state, but the filter is defensive).
async fn find_existing_download(
    &self,
    user_id: &str,
    episode_id: &str,
) -> Result<Option<DownloadRow>, ApiError> {
    let row = sqlx::query_as::<_, DownloadRow>(
        "SELECT * FROM downloads
         WHERE user_id = ? AND episode_id = ? AND superseded = 0
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(user_id)
    .bind(episode_id)
    .fetch_optional(&self.db)
    .await?;
    Ok(row)
}
```

### `maybe_upgrade` logic

Returns `Ok(true)` when an upgrade was actually triggered (so `check_series` can count it), `Ok(false)` otherwise.

```rust
async fn maybe_upgrade(
    &self,
    user_id: &str,
    episode_id: &str,
    wanted_audio: &[String],
    wanted_subs: &[String],
    tracked_series_id: &str,
) -> Result<bool, ApiError> {
    let row = sqlx::query_as::<_, DownloadRow>(
        "SELECT * FROM downloads
         WHERE user_id = ? AND episode_id = ? AND status = 'completed' AND superseded = 0
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(user_id)
    .bind(episode_id)
    .fetch_optional(&self.db)
    .await?;
    let Some(row) = row else { return Ok(false); };

    // Cooldown — skip if checked within 24h.
    if let Some(checked_at) = &row.upgrade_checked_at {
        if let Ok(t) = chrono::DateTime::parse_from_rfc3339(checked_at) {
            if Utc::now().signed_duration_since(t).num_hours() < 24 {
                return Ok(false);
            }
        }
    }

    // Compare wanted vs downloaded.
    let downloaded_audio: Vec<String> = serde_json::from_str(row.audio_tracks.as_deref().unwrap_or("[]")).unwrap_or_default();
    let downloaded_subs: Vec<String> = serde_json::from_str(row.subtitle_tracks.as_deref().unwrap_or("[]")).unwrap_or_default();

    let missing_audio: Vec<_> = wanted_audio.iter().filter(|l| !downloaded_audio.contains(l)).collect();
    let missing_subs: Vec<_> = wanted_subs.iter().filter(|l| !downloaded_subs.contains(l)).collect();

    if missing_audio.is_empty() && missing_subs.is_empty() {
        // Up to date — refresh the timestamp so we don't re-check for 24h.
        sqlx::query("UPDATE downloads SET upgrade_checked_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(&row.id)
            .execute(&self.db)
            .await?;
        return Ok(false);
    }

    // Missing tracks — atomically supersede the row, gated on cooldown to avoid
    // a race with a concurrent poll/manual check both spawning duplicate uploads.
    let now = Utc::now().to_rfc3339();
    let cutoff = (Utc::now() - chrono::Duration::hours(24)).to_rfc3339();
    let result = sqlx::query(
        "UPDATE downloads
         SET superseded = 1, upgrade_checked_at = ?
         WHERE id = ?
           AND superseded = 0
           AND (upgrade_checked_at IS NULL OR upgrade_checked_at < ?)",
    )
    .bind(&now)
    .bind(&row.id)
    .bind(&cutoff)
    .execute(&self.db)
    .await?;

    if result.rows_affected() == 0 {
        // Another check won the race; nothing to do.
        return Ok(false);
    }

    self.download_service
        .start_tracking_download(user_id, episode_id, tracked_series_id, Some(row.id), &self.db)
        .await?;

    Ok(true)
}
```

### `handle_upgrade_completion` (called on download completion in `DownloadService`)

**Critical:** compare *deserialized & sorted* track lists, not JSON strings. String equality is fragile (different array orderings or whitespace cause false negatives/positives) and either could silently delete the wrong row.

```rust
fn parse_locales(json: Option<&str>) -> Vec<String> {
    let mut v: Vec<String> = serde_json::from_str(json.unwrap_or("[]")).unwrap_or_default();
    v.sort();
    v.dedup();
    v
}

async fn handle_upgrade_completion(
    db: &SqlitePool,
    new_download_id: &str,
    superseded_id: &str,
    new_audio: &[String],
    new_subs: &[String],
) {
    // Fetch the old row's track lists.
    let old_row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT audio_tracks, subtitle_tracks FROM downloads WHERE id = ?"
    )
    .bind(superseded_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    let (old_audio_json, old_sub_json) = old_row.unwrap_or((None, None));

    let old_audio = parse_locales(old_audio_json.as_deref());
    let old_subs  = parse_locales(old_sub_json.as_deref());
    let mut new_audio = new_audio.to_vec();
    new_audio.sort();
    new_audio.dedup();
    let mut new_subs = new_subs.to_vec();
    new_subs.sort();
    new_subs.dedup();

    if new_audio != old_audio || new_subs != old_subs {
        // Got more tracks — drop the old superseded record.
        let _ = sqlx::query("DELETE FROM downloads WHERE id = ?")
            .bind(superseded_id)
            .execute(db).await;
    } else {
        // Same tracks — the dub/sub still isn't out. Delete the new (redundant)
        // download, un-supersede the old one, and apply the 24h cooldown.
        let _ = sqlx::query("DELETE FROM downloads WHERE id = ?")
            .bind(new_download_id)
            .execute(db).await;
        let _ = sqlx::query(
            "UPDATE downloads SET superseded = 0, upgrade_checked_at = ? WHERE id = ?"
        )
        .bind(Utc::now().to_rfc3339())
        .bind(superseded_id)
        .execute(db).await;
    }
}
```

The caller in §4a passes `&dl_result.audio_languages` and `&dl_result.subtitle_languages` directly — no JSON serialization needed for the comparison itself.

---

## Part 5b — Filter superseded rows from existing download endpoints

Three endpoints currently read from `downloads` and need a `superseded = 0` filter so an in-flight upgrade doesn't double-count or stale-badge:

1. **`GET /downloads`** (`services/download.rs::list_downloads`) — add `AND superseded = 0` to the WHERE clause. Add an opt-in query param `include_superseded=true` that drops the filter for debugging / admin views.
2. **`GET /downloads/counts`** (`download_counts`) — add `AND superseded = 0` so the dashboard's "Total Downloaded" doesn't tick up by 1 during the brief window between supersede and reconciliation.
3. **`GET /downloads/episode-ids`** (`completed_episode_ids`, the badge endpoint shipped earlier) — add `AND superseded = 0`. Otherwise the "Downloaded" badge shows for an episode whose only completed download was just superseded by an upgrade in progress.

Without these, an in-flight upgrade temporarily shows two rows for the same episode in the user's downloads list (old `superseded=1` row + new in-progress row) and inflates counts.

---

## Part 6 — API routes: `src/routes/tracking.rs`

Create `crunchy-cli/crates/api/src/routes/tracking.rs`.

Handler names: `list_tracked` (GET), `add_tracked` (POST), `update_tracked` (PATCH), `delete_tracked` (DELETE), `check_tracked` (POST :id/check). These match §10.

### `GET /tracking` (handler `list_tracked`)
Returns all tracked series for the current user.

```json
[
  {
    "id": "uuid",
    "series_id": "GG5H5XQX4",
    "series_title": "Frieren: Beyond Journey's End",
    "series_thumbnail": "https://...",
    "download_mode": "new_only",
    "enabled": true,
    "added_at": "2026-05-05T...",
    "last_checked_at": "2026-05-05T..."
  }
]
```

### `POST /tracking` (handler `add_tracked`)
Add a series to the watchlist.

Request:
```json
{
  "series_id": "GG5H5XQX4",
  "download_mode": "new_only"
}
```

Logic:
1. Validate `download_mode` is `"new_only"` or `"all"`
2. Fetch `series_title` and `series_thumbnail` from CR API (`GET /series/{series_id}`)
3. If `download_mode == "new_only"`: call `fetch_all_episodes(client, series_id)` → collect IDs → store as `baseline_episode_ids` JSON array
4. If `download_mode == "all"`: `baseline_episode_ids = "[]"`
5. Insert row; on unique constraint violation return 409 Conflict

Returns **201** with the created row.

### Shared helper: `fetch_all_episodes`

`POST /tracking` (when `new_only`), the `all → new_only` PATCH path, and the `check_series` loop all need the same operation: walk every season of a series and collect episode rows. Factor it once:

```rust
pub async fn fetch_all_episodes(
    client: &CrunchyrollClient,
    series_id: &str,
) -> Result<Vec<CrEpisode>, ApiError> {
    let seasons = client.get_seasons(series_id).await?;
    let mut out = Vec::new();
    for season in seasons {
        let eps = client.get_episodes(&season.id).await?;
        out.extend(eps);
    }
    Ok(out)
}
```

Place in `services/tracking.rs` (or a new `services/cr_helpers.rs` if there's appetite for it).

### `PATCH /tracking/:id` (handler `update_tracked`)
Update `download_mode` and/or `enabled`.

Request (both fields optional):
```json
{ "download_mode": "all", "enabled": true }
```

Notes:
- Switching from `new_only` → `all`: the baseline is ignored on the next poll (poll logic skips the baseline check when mode is `all`). No DB write for the baseline; the existing `baseline_episode_ids` is left as-is in case the user switches back.
- Switching from `all` → `new_only`: the handler **calls `fetch_all_episodes`** and writes the resulting episode IDs as the new `baseline_episode_ids`, so the next poll only picks up episodes that release after the switch. This is one extra CR roundtrip on PATCH but avoids the empty-baseline foot-gun.
- `enabled = false` pauses auto-download without removing the tracking row.
- Returns **404** if not found.

### `DELETE /tracking/:id` (handler `delete_tracked`)
Remove from watchlist. Does **not** delete existing downloads (the `tracked_series_id` column is `ON DELETE SET NULL`).

Returns **204**.

### `POST /tracking/:id/check` (handler `check_tracked`)
Manually trigger an immediate check for one series. **Synchronous** — awaits `check_series` and returns **200** with a summary so the UI can show "started 3 new downloads, 1 upgrade" immediately:

```json
{ "new_downloads": 3, "upgrades": 1, "checked_episodes": 24 }
```

Runs even when `enabled = false` (the user explicitly clicked "Check now"). Returns **404** if the tracked series isn't found, **502** if the CR fetch fails. Implementation note: `check_series` already returns `CheckSummary`; just plumb it up.

### Wire into router

`src/routes/mod.rs`:
```rust
pub mod tracking;
```

`build_router`:
```rust
.merge(tracking::router())
```

---

## Part 7 — `AppState` + server startup

### `src/config.rs`

Add:
```rust
pub check_interval_secs: u64,  // from env var TRACKING_INTERVAL_SECS, default 3600
```

Parse in the `ServerConfig::from_env()` constructor:
```rust
check_interval_secs: std::env::var("TRACKING_INTERVAL_SECS")
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(3600),
```

### `src/state.rs`

Add `tracking_service: Arc<TrackingService>` to `AppState`.

### `src/lib.rs` or `src/main.rs` (wherever services are initialized)

```rust
let tracking_service = TrackingService::new(
    db.clone(),
    download_service.clone(),
    config.check_interval_secs,
);
tracking_service.clone().spawn();
```

---

## Part 8 — `src/services/mod.rs`

Add `pub mod tracking;`.

---

## Part 9 — Frontend

### 9a — `<TrackButton>` (series page, next to `<BookmarkButton>`)

Create `crunchy-web/components/tracking/track-button.tsx`. Mirrors the bookmark button architecturally but with a config dialog instead of a one-click toggle.

- **Not tracking** → button label "Track", outline variant. Click opens a `<TrackingDialog>`:
  - Radio: ( ) New episodes only · (•) Download everything (default new_only)
  - Buttons: Cancel · Track
  - On confirm → `POST /tracking { series_id, download_mode }`
- **Already tracking** → button label "Tracked" with filled icon. Click opens the same dialog pre-filled with current mode + a checkbox **"Pause auto-download"** (bound to `!enabled`) + an **Untrack** destructive button. Save → `PATCH /tracking/:id { download_mode, enabled }`. Untrack → small confirmation dialog ("Remove from watchlist? Existing downloads stay.") → `DELETE /tracking/:id`.

The "Pause auto-download" checkbox is the only place a user can toggle `enabled` from the series page — matches the watchlist Edit affordance for parity.

Hook: `useTrackedSeries()` returns `Set<series_id>` + lookup of full row for an id (to get current mode); `useTrackSeries()` covers POST/PATCH/DELETE with toasts.

Wire into `app/(protected)/series/[id]/page.tsx` in the action area next to `<BookmarkButton>`.

### 9b — `/watchlist` page

Create `crunchy-web/app/(protected)/watchlist/page.tsx`.

- Page header: "Watchlist"
- **Add Show** button → opens a search modal (uses existing search API `GET /search?q=...`) filtered to series → user selects → mode dialog (same `<TrackingDialog>` as 9a) → `POST /tracking`
- Table of tracked series:

| Poster | Title | Mode | Status | Last Checked | Actions |
|---|---|---|---|---|---|
| thumbnail | Frieren | New only | ✅ enabled | 5m ago | Edit · Check now · Remove |

- **Edit** → opens `<TrackingDialog>` → `PATCH /tracking/:id`
- **Check now** → `POST /tracking/:id/check` (synchronous; show spinner; on 200 toast "Started 3 new downloads, 1 upgrade")
- **Remove** → confirmation dialog → `DELETE /tracking/:id`

### 9c — Sidebar nav

Add to `mainNavItems` in `crunchy-web/components/layout/sidebar-content.tsx`:

```ts
{ label: 'Watchlist', href: '/watchlist', icon: <ClapperboardIcon /> },
```

Place after `Bookmarks`, before `Downloads`.

---

## Part 10 — utoipa docs

Register all new routes and schemas in `src/docs.rs`:

```rust
paths(
    // ... existing ...
    tracking::list_tracked,
    tracking::add_tracked,
    tracking::update_tracked,
    tracking::delete_tracked,
    tracking::check_tracked,
)
```

Add schemas: `TrackedSeriesItem`, `AddTrackingRequest`, `UpdateTrackingRequest`.

Add tag:
```rust
(name = "Watchlist", description = "Series tracking and auto-download"),
```

Annotate every new handler with both auth schemes — same as the bookmarks/api-keys pattern:

```rust
security(("bearer_auth" = []), ("api_key" = []))
```

---

## Tests

Add unit tests in `services/tracking.rs` (or a sibling `tests` module). The upgrade-completion logic is the highest-risk path — wrong-row deletion is the failure mode.

```rust
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
    fn comparison_handles_unsorted_arrays() {
        // Same set, different order — must still compare equal.
        let mut a = locales(&["en-US", "ja-JP"]);
        let mut b = locales(&["ja-JP", "en-US"]);
        a.sort();
        b.sort();
        assert_eq!(a, b);
    }

    // Plus integration-style tests against an in-memory SQLite DB for
    // handle_upgrade_completion: with-more-tracks deletes old; same-tracks
    // deletes new + un-supersedes old + sets cooldown.
}
```

These tests must guard the *deserialized + sorted* comparison — they will fail if someone "simplifies" back to JSON-string equality.

---

## Things to verify in the existing code (before implementing)

1. **`CrunchyrollService::get_client(user_id)` token refresh.** The polling worker holds a long-lived loop; per-user access tokens expire. Confirm `get_client` either always issues a fresh token or detects expiry and refreshes via the stored refresh token. If neither, the tracking service must handle 401-from-CR by calling a refresh helper before retrying.
2. **`DownloadService` concurrency limits.** Adding a 200-episode series in `all` mode triggers a backfill burst on the next poll. Verify whether `start_download_inner` already serializes via `simultaneous_downloads`, or whether each call spawns a new Tokio task immediately. If the latter, add a per-poll throttle in the tracking worker (e.g., process episodes for one tracked series at a time, sleep between bursts) — otherwise a single user adding a long-running anime can saturate FFmpeg and the network.

---

## Validation checklist

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo check -p crunchy-api --manifest-path crunchy-cli/Cargo.toml
cargo check --manifest-path crunchy-cli/Cargo.toml  # includes crunchy-cli library changes
cargo test --manifest-path crunchy-cli/Cargo.toml
cd crunchy-web && npm run build && npm run lint
```

Manual smoke tests:
1. `POST /tracking { "series_id": "GG5H5XQX4", "download_mode": "new_only" }` → verify `baseline_episode_ids` is populated
2. `POST /tracking/:id/check` → verify no new downloads are triggered (all baseline) and the response shows `{ new_downloads: 0, upgrades: 0, checked_episodes: N }`
3. Change mode to `all` via `PATCH` → `POST /tracking/:id/check` → verify existing episodes start downloading and `tracked_series_id` is set on each new row
4. Complete a download, then manually clear one `audio_tracks` entry from the row to simulate a missing dub, set a past `upgrade_checked_at` in SQLite → run check → verify the row gets `superseded = 1`, a new download row is created, and the old row is dropped on completion
5. Repeat #4 but with the upgrade producing the same track set → verify the new row is deleted, the old row is un-superseded, and `upgrade_checked_at` is updated to ~now
6. Manually mark a download `failed` with `updated_at` set to 1h ago → run check → verify the row is **not** retried (cooldown). Set `updated_at` to 25h ago → verify it **is** retried
7. `DELETE /tracking/:id` → verify existing `downloads` rows survive but their `tracked_series_id` is `NULL`
8. Switch a tracked series from `all` → `new_only` via PATCH → verify the response and the DB show `baseline_episode_ids` repopulated with the current full episode list

---

## Env vars added

| Var | Required | Default | Purpose |
|---|---|---|---|
| `TRACKING_INTERVAL_SECS` | no | `3600` | How often the background poll runs |
