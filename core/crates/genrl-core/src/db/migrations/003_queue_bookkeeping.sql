-- Phase 4: the queue's own per-job bookkeeping.
--
-- torrents gains the show it was found for and a rejected flag, so a stalled
-- candidate can be marked "tried, failed" and the search step moves on to the
-- next one (UNIQUE(info_hash) keeps re-seeing the same release a no-op). The
-- trusted flag rides along so ranking scores it as Nyaa reported it.
ALTER TABLE torrents ADD COLUMN mal_id INTEGER REFERENCES shows(mal_id) ON DELETE SET NULL;
ALTER TABLE torrents ADD COLUMN rejected INTEGER NOT NULL DEFAULT 0;
ALTER TABLE torrents ADD COLUMN trusted INTEGER NOT NULL DEFAULT 0;
CREATE INDEX idx_torrents_mal ON torrents(mal_id, rejected, score DESC);

-- Each season is its own MAL entry: shows.season is that entry's position in
-- its franchise chain (1 for a standalone show), so the matcher can reject a
-- release that names a different season.
ALTER TABLE shows ADD COLUMN season INTEGER NOT NULL DEFAULT 1;

-- jobs gains monitor progress (for the 20-minute stall timer) and the state a
-- blocked job returns to, so "resume when it comes back" is data, not memory.
ALTER TABLE jobs ADD COLUMN progress REAL NOT NULL DEFAULT 0;
ALTER TABLE jobs ADD COLUMN last_progress_at INTEGER;
ALTER TABLE jobs ADD COLUMN blocked_from TEXT;

-- The Phase 4 state names. The pre-Phase-4 spellings still parse, but rewriting
-- them here keeps every row readable by one mapping.
UPDATE jobs SET state = 'search'   WHERE state IN ('queued', 'searching');
UPDATE jobs SET state = 'monitor'  WHERE state = 'downloading';
UPDATE jobs SET state = 'finalize' WHERE state = 'finalizing';
