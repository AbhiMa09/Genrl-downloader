-- Migration 005: Show metadata status, timestamps, and retry counters
ALTER TABLE shows ADD COLUMN metadata_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE shows ADD COLUMN metadata_updated_at INTEGER;
ALTER TABLE shows ADD COLUMN metadata_next_retry_at INTEGER;
ALTER TABLE shows ADD COLUMN metadata_retry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE shows ADD COLUMN metadata_error TEXT;
ALTER TABLE shows ADD COLUMN score REAL;

-- Mark shows that already have synopsis and local poster as ready
UPDATE shows SET metadata_status = 'ready', metadata_updated_at = (strftime('%s', 'now'))
WHERE synopsis IS NOT NULL AND trim(synopsis) != ''
  AND poster_path IS NOT NULL AND trim(poster_path) != '' AND poster_path NOT LIKE 'http%';
