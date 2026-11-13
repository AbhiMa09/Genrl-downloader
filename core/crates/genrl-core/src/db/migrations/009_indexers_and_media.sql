-- Indexer capabilities cache
CREATE TABLE IF NOT EXISTS indexer_caps (
    indexer_id TEXT PRIMARY KEY,
    caps_xml   TEXT NOT NULL,
    fetched_at INTEGER NOT NULL
);

-- Media table for movies and tv series
CREATE TABLE IF NOT EXISTS media (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    kind       TEXT NOT NULL,
    title      TEXT NOT NULL,
    year       INTEGER,
    tmdb_id    INTEGER,
    imdb_id    TEXT,
    tvdb_id    INTEGER,
    created_at INTEGER NOT NULL,
    UNIQUE(kind, tmdb_id)
);

-- Associate jobs with non-anime media kinds and media items
ALTER TABLE jobs ADD COLUMN media_kind TEXT DEFAULT 'anime';
ALTER TABLE jobs ADD COLUMN media_id INTEGER REFERENCES media(id);
