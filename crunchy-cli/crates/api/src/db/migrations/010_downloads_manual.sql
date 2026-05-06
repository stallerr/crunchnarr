-- Mark-as-downloaded support: a row with manual = 1 represents an episode
-- the user has on disk independently of this app. The watchlist worker
-- treats these as "we already have it, don't auto-download," and skips
-- them in upgrade detection.

ALTER TABLE downloads ADD COLUMN manual INTEGER NOT NULL DEFAULT 0;
