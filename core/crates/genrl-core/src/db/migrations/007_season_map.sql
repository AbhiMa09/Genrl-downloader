-- Maps community release season numbers to entry-local episode ranges for a show.
CREATE TABLE IF NOT EXISTS season_map (
    mal_id      INTEGER NOT NULL,
    season_no   INTEGER NOT NULL,
    first_ep    INTEGER NOT NULL,
    last_ep     INTEGER NOT NULL,
    title       TEXT,
    year        INTEGER,
    source      TEXT    NOT NULL CHECK(source IN ('bundled', 'user')),
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (mal_id, season_no)
);

CREATE INDEX IF NOT EXISTS idx_season_map_lookup ON season_map(mal_id, season_no);
CREATE INDEX IF NOT EXISTS idx_season_map_range ON season_map(mal_id, first_ep, last_ep);
