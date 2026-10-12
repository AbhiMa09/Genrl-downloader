-- Genrl Downloader schema v1.
-- The MAL id is the identity of a show; names, synonyms and folders hang off it.
-- Two constraints prevent the worst bugs: episodes is unique on (mal_id, number)
-- and torrents is unique on info_hash, so the same episode or torrent can never
-- be queued twice.

-- One row per anime.
CREATE TABLE shows (
    mal_id          INTEGER PRIMARY KEY,
    title           TEXT    NOT NULL,
    english_title   TEXT,
    type            TEXT    NOT NULL DEFAULT 'tv',   -- tv | movie | ova | ona | special
    status          TEXT,                            -- airing | finished | not_yet_aired
    total_episodes  INTEGER,
    start_year      INTEGER,
    synopsis        TEXT,
    poster_path     TEXT,
    folder_path     TEXT,
    auto            INTEGER NOT NULL DEFAULT 0,      -- bool: on the automatic list
    episode_offset  INTEGER NOT NULL DEFAULT 0       -- absolute vs per-season numbering
);

-- All titles and aliases used for matching (romaji, English, Japanese, synonyms,
-- plus any alias the user adds). Matching scores against this set, never a guess.
CREATE TABLE show_names (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    mal_id     INTEGER NOT NULL REFERENCES shows(mal_id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    normalized TEXT    NOT NULL,
    source     TEXT    NOT NULL CHECK (source IN ('mal', 'jikan', 'user')),
    UNIQUE (mal_id, normalized)
);
CREATE INDEX idx_show_names_normalized ON show_names(normalized);

-- Your MAL list as last synced.
CREATE TABLE mal_entries (
    mal_id        INTEGER PRIMARY KEY REFERENCES shows(mal_id) ON DELETE CASCADE,
    list_status   TEXT    NOT NULL,   -- watching | completed | on_hold | dropped | plan_to_watch
    watched_count INTEGER NOT NULL DEFAULT 0,
    score         INTEGER,
    updated_at    INTEGER NOT NULL
);
CREATE INDEX idx_mal_entries_status ON mal_entries(list_status);

-- One row per episode wanted or owned.
CREATE TABLE episodes (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    mal_id       INTEGER NOT NULL REFERENCES shows(mal_id) ON DELETE CASCADE,
    number       REAL    NOT NULL,    -- 6.5 for OVAs, 1 for movies
    state        TEXT    NOT NULL DEFAULT 'missing',  -- missing | queued | downloading | done | failed
    file_path    TEXT,
    size         INTEGER,
    torrent_id   INTEGER REFERENCES torrents(id) ON DELETE SET NULL,
    last_checked INTEGER,
    UNIQUE (mal_id, number)
);
CREATE INDEX idx_episodes_state ON episodes(state);

-- Candidates found and the one chosen.
CREATE TABLE torrents (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    info_hash   TEXT    NOT NULL UNIQUE,
    name        TEXT    NOT NULL,
    magnet      TEXT    NOT NULL,
    seeders     INTEGER NOT NULL DEFAULT 0,
    leechers    INTEGER NOT NULL DEFAULT 0,
    size        INTEGER NOT NULL DEFAULT 0,
    "group"     TEXT,                            -- fansub group, e.g. SubsPlease
    resolution  TEXT,                            -- 720p | 1080p | 2160p
    source_url  TEXT,
    score       REAL    NOT NULL DEFAULT 0,
    uploader    TEXT,
    found_at    INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_torrents_score ON torrents(score DESC);

-- The work queue. Each state change is written here before the work continues.
CREATE TABLE jobs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    episode_id  INTEGER NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
    kind        TEXT    NOT NULL CHECK (kind IN ('search', 'add', 'monitor', 'finalize')),
    state       TEXT    NOT NULL,   -- queued | searching | downloading | finalizing | done
                                  -- side states: needs_input | blocked | failed
    attempts    INTEGER NOT NULL DEFAULT 0,
    next_run_at INTEGER,
    last_error  TEXT,
    torrent_id  INTEGER REFERENCES torrents(id) ON DELETE SET NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
CREATE INDEX idx_jobs_runnable ON jobs(state, next_run_at);
CREATE INDEX idx_jobs_episode ON jobs(episode_id);

-- Inbox items waiting for you. Questions never block other jobs.
CREATE TABLE questions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id      INTEGER REFERENCES jobs(id) ON DELETE CASCADE,
    mal_id      INTEGER REFERENCES shows(mal_id) ON DELETE CASCADE,
    kind        TEXT    NOT NULL,   -- match_review | low_seeds | file_exists | batch_only
                                  -- | disk_space | all_failed | entry_changed
    payload     TEXT    NOT NULL,   -- JSON
    answer      TEXT,               -- NULL while unanswered
    created_at  INTEGER NOT NULL,
    expires_at  INTEGER
);
CREATE INDEX idx_questions_open ON questions(answer, expires_at);

-- Download preferences, global and per show. A NULL mal_id means the global row.
-- 'group' priorities, min resolution, blocked uploaders and seed thresholds live
-- here so the Rules page and per-show overrides share one store.
CREATE TABLE rules (
    id     INTEGER PRIMARY KEY AUTOINCREMENT,
    mal_id INTEGER REFERENCES shows(mal_id) ON DELETE CASCADE,
    key    TEXT    NOT NULL,
    value  TEXT    NOT NULL
);
-- SQLite treats NULLs as distinct in UNIQUE, so partial indexes enforce the rule.
CREATE UNIQUE INDEX idx_rules_global ON rules(key) WHERE mal_id IS NULL;
CREATE UNIQUE INDEX idx_rules_show   ON rules(mal_id, key) WHERE mal_id IS NOT NULL;

-- Cached MAL, Jikan and Nyaa responses. Keeps the app working when they are down.
CREATE TABLE http_cache (
    service    TEXT    NOT NULL,
    key        TEXT    NOT NULL,
    body       BLOB    NOT NULL,
    etag       TEXT,                 -- lets MAL sync send If-None-Match
    fetched_at INTEGER NOT NULL,
    ttl        INTEGER NOT NULL,
    PRIMARY KEY (service, key)
);
CREATE INDEX idx_http_cache_fetched ON http_cache(fetched_at);

-- Activity log shown on the Activity page: every job transition and question,
-- with the show, job and reason, so you can always see why a release was chosen.
CREATE TABLE events (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    at      INTEGER NOT NULL,
    level   TEXT    NOT NULL CHECK (level IN ('info', 'warn', 'error')),
    job_id  INTEGER,
    mal_id  INTEGER,
    message TEXT    NOT NULL
);
CREATE INDEX idx_events_at ON events(at DESC);

-- App configuration. Values are JSON so types round-trip.
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
