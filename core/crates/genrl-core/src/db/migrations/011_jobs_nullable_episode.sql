-- Allow nullable episode_id on jobs for non-anime media jobs (movies, tv series)
CREATE TEMPORARY TABLE IF NOT EXISTS _jobs_seq_backup AS SELECT seq FROM sqlite_sequence WHERE name = 'jobs';

CREATE TABLE jobs_new (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    episode_id       INTEGER REFERENCES episodes(id) ON DELETE CASCADE,
    kind             TEXT    NOT NULL CHECK (kind IN ('search', 'add', 'monitor', 'finalize')),
    state            TEXT    NOT NULL,
    attempts         INTEGER NOT NULL DEFAULT 0,
    next_run_at      INTEGER,
    last_error       TEXT,
    torrent_id       INTEGER REFERENCES torrents(id) ON DELETE SET NULL,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL,
    progress         REAL NOT NULL DEFAULT 0,
    last_progress_at INTEGER,
    blocked_from     TEXT,
    manual           INTEGER NOT NULL DEFAULT 0,
    media_kind       TEXT DEFAULT 'anime',
    media_id         INTEGER REFERENCES media(id)
);

INSERT INTO jobs_new (
    id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id,
    created_at, updated_at, progress, last_progress_at, blocked_from, manual, media_kind, media_id
)
SELECT 
    id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id,
    created_at, updated_at, progress, last_progress_at, blocked_from, manual, media_kind, media_id
FROM jobs;

DROP TABLE jobs;
ALTER TABLE jobs_new RENAME TO jobs;

CREATE INDEX IF NOT EXISTS idx_jobs_runnable ON jobs(state, next_run_at);
CREATE INDEX IF NOT EXISTS idx_jobs_episode ON jobs(episode_id);

DELETE FROM sqlite_sequence WHERE name = 'jobs';
INSERT INTO sqlite_sequence (name, seq) SELECT 'jobs', seq FROM _jobs_seq_backup;
DROP TABLE _jobs_seq_backup;
