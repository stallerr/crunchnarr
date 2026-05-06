-- Tracking-related columns on the downloads table.
-- ALTER TABLE ADD COLUMN is not idempotent in SQLite, so the migration runner
-- swallows "duplicate column" errors (see db/mod.rs).

ALTER TABLE downloads ADD COLUMN audio_tracks TEXT;
ALTER TABLE downloads ADD COLUMN subtitle_tracks TEXT;
ALTER TABLE downloads ADD COLUMN tracked_series_id TEXT REFERENCES tracked_series(id) ON DELETE SET NULL;
ALTER TABLE downloads ADD COLUMN upgrade_checked_at TEXT;
ALTER TABLE downloads ADD COLUMN superseded INTEGER NOT NULL DEFAULT 0;
