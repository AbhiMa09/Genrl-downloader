-- Fix orphaned downloading episodes whose torrent has no active non-terminal jobs
UPDATE episodes
SET state = 'missing', torrent_id = NULL
WHERE state = 'downloading'
  AND (
    torrent_id IS NULL
    OR NOT EXISTS (
        SELECT 1 FROM jobs j
        WHERE j.torrent_id = episodes.torrent_id
          AND j.state NOT IN ('done', 'failed', 'cancelled')
    )
  );
