use rusqlite::Connection;
use sha2::{Digest, Sha256};

const MIGRATIONS_1_TO_11: &[&str] = &[
    include_str!("../src/db/migrations/001_init.sql"),
    include_str!("../src/db/migrations/002_name_priority.sql"),
    include_str!("../src/db/migrations/003_queue_bookkeeping.sql"),
    include_str!("../src/db/migrations/004_mal_entries_source.sql"),
    include_str!("../src/db/migrations/005_show_metadata.sql"),
    include_str!("../src/db/migrations/006_seasons_and_manual_jobs.sql"),
    include_str!("../src/db/migrations/007_season_map.sql"),
    include_str!("../src/db/migrations/008_reconcile_orphaned_downloading_episodes.sql"),
    include_str!("../src/db/migrations/009_indexers_and_media.sql"),
    include_str!("../src/db/migrations/010_media_metadata.sql"),
    include_str!("../src/db/migrations/011_jobs_nullable_episode.sql"),
];

const MIGRATION_12: &str = include_str!("../src/db/migrations/012_media_episodes.sql");

fn table_checksum(conn: &Connection, table: &str) -> (i64, String) {
    let mut stmt = conn
        .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
        .unwrap();
    let col_count = stmt.column_count();
    let mut hasher = Sha256::new();
    let mut row_count = 0i64;
    let mut rows = stmt.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        row_count += 1;
        for i in 0..col_count {
            let val: rusqlite::types::Value = row.get(i).unwrap();
            hasher.update(format!("{val:?}").as_bytes());
            hasher.update(b"|");
        }
        hasher.update(b"\n");
    }
    let hash: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    (row_count, hash)
}

#[test]
fn test_migration_012_additive_preserves_all_tables_checksums() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

    // 1. Run migrations 1 through 11
    for sql in MIGRATIONS_1_TO_11 {
        let tx = conn.transaction().unwrap();
        tx.execute_batch(sql).unwrap();
        tx.commit().unwrap();
    }

    // 2. Populate existing tables
    conn.execute(
        "INSERT INTO shows (mal_id, title, type, status, total_episodes, start_year, auto, season)
         VALUES (1, 'Frieren', 'tv', 'finished', 28, 2023, 1, 1)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO episodes (id, mal_id, number, state) VALUES (10, 1, 1.0, 'missing')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO torrents (id, info_hash, name, magnet, seeders, leechers, size, found_at)
         VALUES (1, 'abc123hash', 'Frieren - 01', 'magnet:?xt=urn:btih:abc123hash', 10, 2, 500000000, 1000)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO media (id, kind, title, year, tmdb_id, created_at)
         VALUES (1, 'series', 'Severance', 2022, 95396, 1000)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO jobs (id, episode_id, kind, state, attempts, created_at, updated_at, media_kind, media_id)
         VALUES (1, 10, 'search', 'search', 0, 1000, 1000, 'anime', NULL)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO jobs (id, episode_id, kind, state, attempts, created_at, updated_at, media_kind, media_id)
         VALUES (2, NULL, 'search', 'search', 0, 1000, 1000, 'series', 1)",
        [],
    )
    .unwrap();

    let existing_tables = vec![
        "shows",
        "episodes",
        "torrents",
        "jobs",
        "media",
        "settings",
        "events",
        "questions",
    ];

    let pre_checksums: Vec<(String, (i64, String))> = existing_tables
        .iter()
        .map(|&t| (t.to_string(), table_checksum(&conn, t)))
        .collect();

    // 3. Apply migration 12
    {
        let tx = conn.transaction().unwrap();
        tx.execute_batch(MIGRATION_12).unwrap();
        tx.commit().unwrap();
    }

    // 4. Verify integrity and foreign keys
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");

    let mut fk_stmt = conn.prepare("PRAGMA foreign_key_check").unwrap();
    let fk_rows: Vec<String> = fk_stmt
        .query_map([], |r| {
            Ok(format!(
                "table={} rowid={} parent={}",
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?
            ))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(fk_rows.is_empty(), "Foreign key violations: {fk_rows:?}");

    // 5. Verify every existing table has the exact same checksum
    for (t, (pre_count, pre_hash)) in pre_checksums {
        let (post_count, post_hash) = table_checksum(&conn, &t);
        assert_eq!(
            pre_count, post_count,
            "Table {t} row count changed after migration 12"
        );
        assert_eq!(
            pre_hash, post_hash,
            "Table {t} checksum changed after migration 12"
        );
    }

    // 6. Verify media_episodes operates correctly
    conn.execute(
        "INSERT INTO media_episodes (media_id, season, episode, air_date, state)
         VALUES (1, 1, 1, '2022-02-18', 'wanted')",
        [],
    )
    .unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM media_episodes WHERE media_id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
