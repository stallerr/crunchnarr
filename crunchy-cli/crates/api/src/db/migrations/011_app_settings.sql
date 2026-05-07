-- Server-wide settings (global across users, single tenant). Mirrors the
-- shape of user_settings but with no user_id; one row per app instance.

CREATE TABLE IF NOT EXISTS app_settings (
    id            INTEGER PRIMARY KEY DEFAULT 1,
    settings_json TEXT NOT NULL DEFAULT '{}',
    updated_at    TEXT NOT NULL,
    CHECK (id = 1)
);

INSERT OR IGNORE INTO app_settings (id, settings_json, updated_at)
VALUES (1, '{}', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
