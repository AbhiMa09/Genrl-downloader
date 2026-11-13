use rusqlite::Connection;

const MIGRATIONS_1_TO_10: &[&str] = &[
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
];

const MIGRATION_11: &str = include_str!("../src/db/migrations/011_jobs_nullable_episode.sql");

#[test]
fn test_migration_011_preserves_anime_jobs_and_passes_integrity_checks() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

    // 1. Run migrations 1 through 10
    for sql in MIGRATIONS_1_TO_10 {
        let tx = conn.transaction().unwrap();
        tx.execute_batch(sql).unwrap();
        tx.commit().unwrap();
    }

    // 2. Set up anime show, episodes, and torrents
    conn.execute(
        "INSERT INTO shows (mal_id, title) VALUES (1, 'Frieren')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO episodes (id, mal_id, number, state) VALUES (101, 1, 1.0, 'missing')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO episodes (id, mal_id, number, state) VALUES (102, 1, 2.0, 'downloading')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO episodes (id, mal_id, number, state) VALUES (103, 1, 3.0, 'failed')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO torrents (id, info_hash, name, magnet) VALUES (50, 'hash50', 'Frieren 02', 'magnet:?xt=urn:btih:hash50')",
        [],
    ).unwrap();

    // 3. Insert several anime jobs with varying states, attempts, errors, etc.
    conn.execute(
        "INSERT INTO jobs (id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id, created_at, updated_at, progress, last_progress_at, blocked_from, manual, media_kind, media_id)
         VALUES (1, 101, 'search', 'search', 0, 1000, NULL, NULL, 100, 100, 0.0, NULL, NULL, 0, 'anime', NULL)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO jobs (id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id, created_at, updated_at, progress, last_progress_at, blocked_from, manual, media_kind, media_id)
         VALUES (2, 102, 'monitor', 'monitor', 1, 2000, 'tracker timeout', 50, 200, 250, 0.45, 240, 'downloading', 1, 'anime', NULL)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO jobs (id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id, created_at, updated_at, progress, last_progress_at, blocked_from, manual, media_kind, media_id)
         VALUES (3, 103, 'finalize', 'failed', 3, NULL, 'disk full', 50, 300, 350, 1.0, 340, NULL, 0, 'anime', NULL)",
        [],
    ).unwrap();

    // Insert a child question referencing job 2
    conn.execute(
        "INSERT INTO questions (id, job_id, mal_id, kind, payload, created_at)
         VALUES (1, 2, 1, 'low_seeds', '{}', 200)",
        [],
    )
    .unwrap();

    // 4. Run migration 11
    {
        let tx = conn.transaction().unwrap();
        tx.execute_batch(MIGRATION_11).unwrap();
        tx.commit().unwrap();
    }

    // 5. Verify all rows and column values in jobs survived unchanged
    conn.query_row(
        "SELECT id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id, created_at, updated_at, progress, last_progress_at, blocked_from, manual, media_kind, media_id FROM jobs WHERE id = 1",
        [],
        |r| {
            assert_eq!(r.get::<_, i64>(0)?, 1);
            assert_eq!(r.get::<_, Option<i64>>(1)?, Some(101));
            assert_eq!(r.get::<_, String>(2)?, "search");
            assert_eq!(r.get::<_, String>(3)?, "search");
            assert_eq!(r.get::<_, i64>(4)?, 0);
            assert_eq!(r.get::<_, Option<i64>>(5)?, Some(1000));
            assert_eq!(r.get::<_, Option<String>>(6)?, None);
            assert_eq!(r.get::<_, Option<i64>>(7)?, None);
            assert_eq!(r.get::<_, i64>(8)?, 100);
            assert_eq!(r.get::<_, i64>(9)?, 100);
            assert_eq!(r.get::<_, f64>(10)?, 0.0);
            assert_eq!(r.get::<_, Option<i64>>(11)?, None);
            assert_eq!(r.get::<_, Option<String>>(12)?, None);
            assert_eq!(r.get::<_, i64>(13)?, 0);
            assert_eq!(r.get::<_, Option<String>>(14)?, Some("anime".into()));
            assert_eq!(r.get::<_, Option<i64>>(15)?, None);
            Ok(())
        },
    ).unwrap();

    conn.query_row(
        "SELECT id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id, created_at, updated_at, progress, last_progress_at, blocked_from, manual, media_kind, media_id FROM jobs WHERE id = 2",
        [],
        |r| {
            assert_eq!(r.get::<_, i64>(0)?, 2);
            assert_eq!(r.get::<_, Option<i64>>(1)?, Some(102));
            assert_eq!(r.get::<_, String>(2)?, "monitor");
            assert_eq!(r.get::<_, String>(3)?, "monitor");
            assert_eq!(r.get::<_, i64>(4)?, 1);
            assert_eq!(r.get::<_, Option<i64>>(5)?, Some(2000));
            assert_eq!(r.get::<_, Option<String>>(6)?, Some("tracker timeout".into()));
            assert_eq!(r.get::<_, Option<i64>>(7)?, Some(50));
            assert_eq!(r.get::<_, i64>(8)?, 200);
            assert_eq!(r.get::<_, i64>(9)?, 250);
            assert_eq!(r.get::<_, f64>(10)?, 0.45);
            assert_eq!(r.get::<_, Option<i64>>(11)?, Some(240));
            assert_eq!(r.get::<_, Option<String>>(12)?, Some("downloading".into()));
            assert_eq!(r.get::<_, i64>(13)?, 1);
            assert_eq!(r.get::<_, Option<String>>(14)?, Some("anime".into()));
            assert_eq!(r.get::<_, Option<i64>>(15)?, None);
            Ok(())
        },
    ).unwrap();

    conn.query_row(
        "SELECT id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id, created_at, updated_at, progress, last_progress_at, blocked_from, manual, media_kind, media_id FROM jobs WHERE id = 3",
        [],
        |r| {
            assert_eq!(r.get::<_, i64>(0)?, 3);
            assert_eq!(r.get::<_, Option<i64>>(1)?, Some(103));
            assert_eq!(r.get::<_, String>(2)?, "finalize");
            assert_eq!(r.get::<_, String>(3)?, "failed");
            assert_eq!(r.get::<_, i64>(4)?, 3);
            assert_eq!(r.get::<_, Option<i64>>(5)?, None);
            assert_eq!(r.get::<_, Option<String>>(6)?, Some("disk full".into()));
            assert_eq!(r.get::<_, Option<i64>>(7)?, Some(50));
            assert_eq!(r.get::<_, i64>(8)?, 300);
            assert_eq!(r.get::<_, i64>(9)?, 350);
            assert_eq!(r.get::<_, f64>(10)?, 1.0);
            assert_eq!(r.get::<_, Option<i64>>(11)?, Some(340));
            assert_eq!(r.get::<_, Option<String>>(12)?, None);
            assert_eq!(r.get::<_, i64>(13)?, 0);
            assert_eq!(r.get::<_, Option<String>>(14)?, Some("anime".into()));
            assert_eq!(r.get::<_, Option<i64>>(15)?, None);
            Ok(())
        },
    ).unwrap();

    // Verify child question still exists and references job 2
    let q_job_id: i64 = conn
        .query_row("SELECT job_id FROM questions WHERE id = 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(q_job_id, 2);

    // 6. PRAGMA foreign_key_check: must return 0 rows (clean)
    let fk_violations: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA foreign_key_check;").unwrap();
        let rows = stmt
            .query_map([], |r| {
                let table: String = r.get(0)?;
                let rowid: i64 = r.get(1)?;
                let parent: String = r.get(2)?;
                let fkid: i64 = r.get(3)?;
                Ok(format!(
                    "{table} row {rowid} references {parent} (fk {fkid})"
                ))
            })
            .unwrap();
        rows.map(|r| r.unwrap()).collect()
    };
    assert!(
        fk_violations.is_empty(),
        "foreign_key_check reported violations: {:?}",
        fk_violations
    );

    // 7. PRAGMA integrity_check: must return "ok"
    let integrity: String = conn
        .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok", "integrity_check failed: {integrity}");

    // 8. Verify indexes on jobs exist
    let mut indexes = Vec::new();
    let mut stmt = conn.prepare("PRAGMA index_list('jobs');").unwrap();
    let rows = stmt.query_map([], |r| r.get::<_, String>(1)).unwrap();
    for r in rows {
        indexes.push(r.unwrap());
    }
    assert!(
        indexes.contains(&"idx_jobs_runnable".to_string()),
        "missing idx_jobs_runnable: {:?}",
        indexes
    );
    assert!(
        indexes.contains(&"idx_jobs_episode".to_string()),
        "missing idx_jobs_episode: {:?}",
        indexes
    );

    // 9. Verify movie job with episode_id = NULL and media_id = Some(...) can be inserted
    conn.execute(
        "INSERT INTO media (id, kind, title, year, created_at) VALUES (10, 'movie', 'Dune', 2021, 500)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO jobs (episode_id, kind, state, created_at, updated_at, media_kind, media_id)
         VALUES (NULL, 'search', 'search', 500, 500, 'movie', 10)",
        [],
    )
    .unwrap();
    let inserted_media_id: Option<i64> = conn
        .query_row(
            "SELECT media_id FROM jobs WHERE episode_id IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(inserted_media_id, Some(10));
}

#[test]
fn test_migration_failure_leaves_db_untouched() {
    let mut conn = Connection::open_in_memory().unwrap();
    // Run migration 1
    let tx = conn.transaction().unwrap();
    tx.execute_batch(MIGRATIONS_1_TO_10[0]).unwrap();
    tx.commit().unwrap();

    let initial_count: i64 = conn
        .query_row("SELECT count(*) FROM shows", [], |r| r.get(0))
        .unwrap();

    // A failing migration inside a transaction
    let result = (|| -> rusqlite::Result<()> {
        let tx = conn.transaction()?;
        tx.execute("INSERT INTO shows (mal_id, title) VALUES (999, 'Test')", [])?;
        tx.execute_batch("INVALID SQL THAT FAILS;")?;
        tx.commit()?;
        Ok(())
    })();
    assert!(result.is_err());

    // Verify rollback left database untouched
    let after_count: i64 = conn
        .query_row("SELECT count(*) FROM shows", [], |r| r.get(0))
        .unwrap();
    assert_eq!(initial_count, after_count);
}
