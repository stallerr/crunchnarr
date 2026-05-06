-- Queue feature was never finished (start_queue handler was a stub) and has
-- been removed. Drop the table and its indexes.

DROP INDEX IF EXISTS idx_queue_user;
DROP INDEX IF EXISTS idx_queue_status;
DROP TABLE IF EXISTS download_queue;
