-- Per-user series tracking (watchlist) with auto-download.

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

CREATE INDEX IF NOT EXISTS idx_tracked_series_user ON tracked_series(user_id);
CREATE INDEX IF NOT EXISTS idx_tracked_series_enabled ON tracked_series(enabled);
