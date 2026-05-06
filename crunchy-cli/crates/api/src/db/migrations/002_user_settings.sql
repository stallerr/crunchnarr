-- Per-user settings (JSON blob)

CREATE TABLE IF NOT EXISTS user_settings (
    user_id TEXT PRIMARY KEY REFERENCES users(id),
    settings_json TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL
);
