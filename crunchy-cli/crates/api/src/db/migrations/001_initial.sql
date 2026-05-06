-- Initial schema for crunchy-api

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS crunchyroll_credentials (
    user_id TEXT PRIMARY KEY REFERENCES users(id),
    access_token TEXT,
    refresh_token TEXT,
    expires_at TEXT,
    account_id TEXT,
    profile_id TEXT,
    device_id TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS downloads (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    episode_id TEXT NOT NULL,
    series_title TEXT,
    episode_title TEXT,
    season_number INTEGER,
    episode_number REAL,
    status TEXT NOT NULL DEFAULT 'pending',
    options_json TEXT,
    progress_json TEXT,
    output_path TEXT,
    error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS download_queue (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    episode_id TEXT NOT NULL,
    series_title TEXT,
    episode_title TEXT,
    season_number INTEGER,
    episode_number REAL,
    status TEXT NOT NULL DEFAULT 'pending',
    options_json TEXT,
    priority INTEGER DEFAULT 0,
    error TEXT,
    retry_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, episode_id)
);

CREATE INDEX IF NOT EXISTS idx_downloads_user ON downloads(user_id);
CREATE INDEX IF NOT EXISTS idx_downloads_status ON downloads(status);
CREATE INDEX IF NOT EXISTS idx_queue_user ON download_queue(user_id);
CREATE INDEX IF NOT EXISTS idx_queue_status ON download_queue(status);
