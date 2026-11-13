use genrl_core::db::Db;
use genrl_core::metadata::EpisodeMetadata;
use genrl_core::planner::series::{plan_season_jobs, sync_tmdb_episodes, PlannedSeriesJob};
use genrl_core::score::Candidate;

fn make_candidate(name: &str, is_batch: bool, range: Option<(f64, f64)>) -> Candidate {
    Candidate {
        release: {
            let mut rel = genrl_core::filename::parse_release(name);
            rel.is_batch = is_batch;
            rel.episode_range = range;
            rel
        },
        info_hash: "1111111111111111111111111111111111111111".to_string(),
        magnet: "magnet:?xt=urn:btih:1111111111111111111111111111111111111111".to_string(),
        seeders: 10,
        leechers: 2,
        trusted: true,
        remake: false,
        size_bytes: 4_000_000_000,
        published_at: 1_700_000_000,
        uploader: None,
        show_type: None,
        category_id: None,
    }
}

#[test]
fn test_series_planner_12_episodes_season_scenarios() {
    let db = Db::open_in_memory().unwrap();
    let today = "2024-03-15";

    // Insert media
    let media_id = db.with(|c| {
        c.execute(
            "INSERT INTO media (kind, title, year, created_at) VALUES ('series', 'Severance', 2022, 1000)",
            [],
        )?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    // 12 episodes, all aired
    let episodes: Vec<EpisodeMetadata> = (1..=12)
        .map(|i| EpisodeMetadata {
            season: 1,
            episode: i,
            title: format!("Episode {i}"),
            air_date: Some("2022-02-18".to_string()),
        })
        .collect();

    sync_tmdb_episodes(&db, media_id, &episodes, today).unwrap();

    // Mark 3 episodes already done (1, 2, 3)
    db.with(|c| {
        c.execute(
            "UPDATE media_episodes SET state = 'done' WHERE media_id = ?1 AND episode IN (1, 2, 3)",
            [media_id],
        )?;
        Ok(())
    })
    .unwrap();

    // Case A: 9 missing, N=4, only singles available -> 9 single jobs
    let single_candidates = vec![
        make_candidate("Severance S01E04 1080p", false, None),
        make_candidate("Severance S01E05 1080p", false, None),
    ];
    let jobs_singles =
        plan_season_jobs(&db, media_id, 1, &single_candidates, 4, today, Some(12)).unwrap();
    assert_eq!(jobs_singles.len(), 9, "Must plan 9 single jobs");
    for (idx, job) in jobs_singles.iter().enumerate() {
        assert_eq!(
            *job,
            PlannedSeriesJob::Single {
                season: 1,
                episode: (idx as u32) + 4
            }
        );
    }

    // Case B: 9 missing, N=4, covering pack available -> 1 pack job
    let pack_candidates = vec![make_candidate(
        "Severance S01 1080p WEB-DL Complete",
        true,
        Some((1.0, 12.0)),
    )];
    let jobs_pack =
        plan_season_jobs(&db, media_id, 1, &pack_candidates, 4, today, Some(12)).unwrap();
    assert_eq!(jobs_pack, vec![PlannedSeriesJob::Pack { season: 1 }]);

    // Case C: with only 3 missing (mark 4..=9 done) and pack available -> 3 single jobs (3 < 4)
    db.with(|c| {
        c.execute(
            "UPDATE media_episodes SET state = 'done' WHERE media_id = ?1 AND episode BETWEEN 4 AND 9",
            [media_id],
        )?;
        Ok(())
    }).unwrap();

    let jobs_few_missing =
        plan_season_jobs(&db, media_id, 1, &pack_candidates, 4, today, Some(12)).unwrap();
    assert_eq!(
        jobs_few_missing,
        vec![
            PlannedSeriesJob::Single {
                season: 1,
                episode: 10
            },
            PlannedSeriesJob::Single {
                season: 1,
                episode: 11
            },
            PlannedSeriesJob::Single {
                season: 1,
                episode: 12
            },
        ],
        "With only 3 missing and N=4, must plan 3 single jobs even with pack available"
    );
}

#[test]
fn test_series_planner_ongoing_season_pack_rules() {
    let db = Db::open_in_memory().unwrap();
    let today = "2024-03-15";

    let media_id = db.with(|c| {
        c.execute(
            "INSERT INTO media (kind, title, year, created_at) VALUES ('series', 'House of the Dragon', 2022, 1000)",
            [],
        )?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    // Total 12 episodes, but only 8 aired by 'today'
    let episodes: Vec<EpisodeMetadata> = (1..=12)
        .map(|i| EpisodeMetadata {
            season: 2,
            episode: i,
            title: format!("Episode {i}"),
            air_date: if i <= 8 {
                Some("2024-03-01".to_string())
            } else {
                Some("2024-04-01".to_string()) // in future
            },
        })
        .collect();

    sync_tmdb_episodes(&db, media_id, &episodes, today).unwrap();

    // 2 done (episodes 1 and 2), 6 missing aired
    db.with(|c| {
        c.execute(
            "UPDATE media_episodes SET state = 'done' WHERE media_id = ?1 AND episode IN (1, 2)",
            [media_id],
        )?;
        Ok(())
    })
    .unwrap();

    // Unaired episodes (9-12) are NOT wanted
    let unaired_count: i64 = db.with(|c| {
        Ok(c.query_row(
            "SELECT COUNT(*) FROM media_episodes WHERE media_id = ?1 AND state = 'wanted' AND air_date > ?2",
            rusqlite::params![media_id, today],
            |r| r.get(0),
        )?)
    }).unwrap();
    assert_eq!(unaired_count, 0, "Unaired episodes must not be wanted");

    // Subcase 1: Pack that covers only 8 (episodes 1..=8) is accepted for ongoing season
    let pack_8 = make_candidate(
        "House of the Dragon S02E01-E08 1080p",
        true,
        Some((1.0, 8.0)),
    );
    let jobs_pack8 = plan_season_jobs(&db, media_id, 2, &[pack_8], 4, today, Some(12)).unwrap();
    assert_eq!(jobs_pack8, vec![PlannedSeriesJob::Pack { season: 2 }]);

    // Subcase 2: Pack labeled complete but with parsed range 1-12 covers all 8 aired -> accepted
    let pack_12 = make_candidate(
        "House of the Dragon S02 1080p Complete",
        true,
        Some((1.0, 12.0)),
    );
    let jobs_pack12 = plan_season_jobs(&db, media_id, 2, &[pack_12], 4, today, Some(12)).unwrap();
    assert_eq!(jobs_pack12, vec![PlannedSeriesJob::Pack { season: 2 }]);

    // Subcase 3: Pack covering only 1-6 does NOT cover all 8 aired -> rejected
    let pack_6 = make_candidate(
        "House of the Dragon S02E01-E06 1080p",
        true,
        Some((1.0, 6.0)),
    );
    let jobs_pack6 = plan_season_jobs(&db, media_id, 2, &[pack_6], 4, today, Some(12)).unwrap();
    assert_eq!(
        jobs_pack6.len(),
        6,
        "Must fall back to 6 single jobs for episodes 3..=8"
    );
    for (idx, job) in jobs_pack6.iter().enumerate() {
        assert_eq!(
            *job,
            PlannedSeriesJob::Single {
                season: 2,
                episode: (idx as u32) + 3
            }
        );
    }
}

#[test]
fn test_series_planner_season_0_ignored() {
    let db = Db::open_in_memory().unwrap();
    let today = "2024-03-15";

    let media_id = db.with(|c| {
        c.execute(
            "INSERT INTO media (kind, title, year, created_at) VALUES ('series', 'Doctor Who', 2005, 1000)",
            [],
        )?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    let episodes = vec![
        EpisodeMetadata {
            season: 0,
            episode: 1,
            title: "Special 1".into(),
            air_date: Some("2005-12-25".into()),
        },
        EpisodeMetadata {
            season: 1,
            episode: 1,
            title: "Rose".into(),
            air_date: Some("2005-03-26".into()),
        },
    ];

    sync_tmdb_episodes(&db, media_id, &episodes, today).unwrap();

    let s0_count: i64 = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM media_episodes WHERE media_id = ?1 AND season = 0",
                [media_id],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(s0_count, 0, "Season 0 must be ignored and not inserted");
}

#[test]
fn test_series_planner_null_air_date_not_planned() {
    let db = Db::open_in_memory().unwrap();
    let today = "2024-03-15";

    let media_id = db.with(|c| {
        c.execute(
            "INSERT INTO media (kind, title, year, created_at) VALUES ('series', 'Severance', 2022, 1000)",
            [],
        )?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    let episodes = vec![
        EpisodeMetadata {
            season: 2,
            episode: 1,
            title: "TBA Ep 1".into(),
            air_date: None, // Null air_date
        },
        EpisodeMetadata {
            season: 2,
            episode: 2,
            title: "TBA Ep 2".into(),
            air_date: Some("".into()), // Empty air_date
        },
    ];

    sync_tmdb_episodes(&db, media_id, &episodes, today).unwrap();

    let wanted_count: i64 = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM media_episodes WHERE media_id = ?1 AND state = 'wanted'",
                [media_id],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(wanted_count, 0, "Null air_date must not be wanted");

    let jobs = plan_season_jobs(&db, media_id, 2, &[], 4, today, Some(10)).unwrap();
    assert!(
        jobs.is_empty(),
        "No jobs must be planned for null air_date episodes"
    );
}
