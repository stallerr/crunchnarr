-- Add thumbnail_url to downloads for episode thumbnails

ALTER TABLE downloads ADD COLUMN thumbnail_url TEXT;
