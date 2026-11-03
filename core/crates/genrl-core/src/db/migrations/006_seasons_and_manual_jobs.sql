-- Season chains survive the sync, so the Library can group a show's seasons.
ALTER TABLE shows ADD COLUMN franchise_id INTEGER;
CREATE INDEX idx_shows_franchise ON shows(franchise_id);

-- A job the user asked for by hand runs even while automation is paused.
ALTER TABLE jobs ADD COLUMN manual INTEGER NOT NULL DEFAULT 0;

-- MAL relations rarely change: one lookup per show per month, not per sync.
CREATE TABLE mal_relation_cache (
    mal_id     INTEGER PRIMARY KEY,
    relations  TEXT    NOT NULL,   -- JSON: the relation list as MAL returned it
    fetched_at INTEGER NOT NULL
);
