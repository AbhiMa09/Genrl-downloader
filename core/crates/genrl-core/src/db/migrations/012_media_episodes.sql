-- Table media_episodes tracks TV series episodes and their download state
CREATE TABLE IF NOT EXISTS media_episodes (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id   INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    season     INTEGER NOT NULL,
    episode    INTEGER NOT NULL,
    air_date   TEXT,
    state      TEXT NOT NULL DEFAULT 'wanted',
    file_path  TEXT,
    UNIQUE(media_id, season, episode)
);

CREATE INDEX IF NOT EXISTS idx_media_episodes_media_state ON media_episodes(media_id, state);
