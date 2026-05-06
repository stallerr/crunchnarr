-- Per-user bookmarked Crunchyroll series.

CREATE TABLE IF NOT EXISTS bookmarks (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    series_id  TEXT NOT NULL,
    note       TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (user_id, series_id)
);

CREATE INDEX IF NOT EXISTS idx_bookmarks_user_created
    ON bookmarks(user_id, created_at DESC);
