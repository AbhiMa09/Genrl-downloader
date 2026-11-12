use std::sync::Arc;

use genrl_core::clock::FakeClock;
use genrl_core::db::Db;
use genrl_core::filename::parse_release;
use genrl_core::http::Gate;
use genrl_core::models::{JobState, ShowType};
use genrl_core::planner::{ensure_episode, plan_single};
use genrl_core::rules::Rules;
use genrl_core::score::{evaluate, Candidate, Rejection, Target};
use genrl_core::settings::Settings;
use genrl_core::worker::{Engine, FakeDiskProbe, NoopFinalizer};

const NOW: i64 = 1_789_227_900;

fn mock_candidate(raw: &str, seeders: u32, size_bytes: u64, hash: &str) -> Candidate {
    let release = parse_release(raw);
    Candidate {
        release,
        info_hash: hash.to_string(),
        magnet: format!("magnet:?xt=urn:btih:{hash}"),
        seeders,
        leechers: 5,
        trusted: true,
        remake: false,
        size_bytes,
        published_at: NOW,
        uploader: None,
        show_type: Some(ShowType::Tv),
        category_id: Some("1_2".to_string()),
    }
}

#[test]
fn test_boundary_24_vs_25_missing_episodes() {
    let clock = FakeClock::new(NOW);

    // 1. Airing show with 24 missing episodes -> queues 24 single jobs
    let db24 = Db::open_in_memory().unwrap();
    db24.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, total_episodes, status, metadata_status)
             VALUES (101, 'Show 24', 24, 'airing', 'ready')",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    let queued24 = plan_single(&db24, &clock, 101, None).unwrap();
    assert_eq!(
        queued24, 24,
        "24 missing episodes keeps single-episode behavior (24 jobs)"
    );

    // 2. Airing show with 25 missing episodes -> queues 1 pack-grab search job
    let db25 = Db::open_in_memory().unwrap();
    db25.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, total_episodes, status, metadata_status)
             VALUES (102, 'Show 25', 25, 'airing', 'ready')",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    let queued25 = plan_single(&db25, &clock, 102, None).unwrap();
    assert_eq!(
        queued25, 1,
        "25 missing episodes queues 1 pack-grab search job"
    );
}

#[test]
fn test_waste_rejection_when_owned_exceeds_missing() {
    let names = vec!["one piece".to_string()];
    let others = vec![];
    let settings = Settings::default();
    let rules = Rules::default();

    // 1,000 owned, 6 missing (episodes 1001-1006 missing)
    let owned: Vec<f64> = (1..=1000).map(|i| i as f64).collect();
    let missing: Vec<f64> = (1001..=1006).map(|i| i as f64).collect();

    let target = Target {
        names: &names,
        other_names: &others,
        wanted_episode: Some(1001.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1999),
        fully_aired: false,
        total_episodes: Some(1006),
        season_maps: &[],
        missing_episodes: &missing,
        owned_episodes: &owned,
    };

    // Pack covers 1-1006: 1000 owned vs 6 missing -> wasteful overlap -> rejected
    let cand = mock_candidate(
        "[PsyPlex] One Piece (0001-1006) [1080p] [Batch]",
        50,
        100 * 1024 * 1024 * 1024,
        "1111111111111111111111111111111111111111",
    );

    let res = evaluate(&cand, &target, &rules, &settings, NOW);
    assert_eq!(
        res.unwrap_err(),
        Rejection::WastefulOverlap {
            covered_missing: 6,
            covered_owned: 1000,
        },
        "pack whose owned overlap exceeds missing coverage must be skipped"
    );
}

#[test]
fn test_size_budget_known_range_and_unknown_count() {
    let names = vec!["detective conan".to_string()];
    let others = vec![];
    let settings = Settings::default();
    let rules = Rules::default();

    // 1. 300 GB pack 001-0300, show missing 1,000 -> pass
    let missing_1000: Vec<f64> = (1..=1000).map(|i| i as f64).collect();
    let target_1000 = Target {
        names: &names,
        other_names: &others,
        wanted_episode: Some(1.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1996),
        fully_aired: false,
        total_episodes: Some(1000),
        season_maps: &[],
        missing_episodes: &missing_1000,
        owned_episodes: &[],
    };
    let pack_300gb = mock_candidate(
        "[Group] Detective Conan (001-0300) [1080p] [Batch]",
        50,
        300u64 * 1024 * 1024 * 1024,
        "2222222222222222222222222222222222222222",
    );
    assert!(
        evaluate(&pack_300gb, &target_1000, &rules, &settings, NOW).is_ok(),
        "300 GB for 300 episodes (~1 GB/ep) when missing 1000 must pass"
    );

    // 2. Same 300 GB pack 001-0300, show missing 30 -> fail (30 * 2560 MB budget)
    let missing_30: Vec<f64> = (1..=30).map(|i| i as f64).collect();
    let target_30 = Target {
        names: &names,
        other_names: &others,
        wanted_episode: Some(1.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1996),
        fully_aired: false,
        total_episodes: Some(30),
        season_maps: &[],
        missing_episodes: &missing_30,
        owned_episodes: &[],
    };
    assert!(
        matches!(
            evaluate(&pack_300gb, &target_30, &rules, &settings, NOW),
            Err(Rejection::SizeOutOfRange { .. })
        ),
        "300 GB pack must fail when show is only missing 30 episodes"
    );

    // 3. Unknown-count 300 GB pack, missing 1,093 -> pass
    let missing_1093: Vec<f64> = (1..=1093).map(|i| i as f64).collect();
    let target_unknown_1093 = Target {
        names: &names,
        other_names: &others,
        wanted_episode: Some(1.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1996),
        fully_aired: false,
        total_episodes: Some(0), // unknown total
        season_maps: &[],
        missing_episodes: &missing_1093,
        owned_episodes: &[],
    };
    let unk_pack_300gb = mock_candidate(
        "[PsyPlex] Detective Conan Complete [Batch]",
        50,
        300u64 * 1024 * 1024 * 1024,
        "3333333333333333333333333333333333333333",
    );
    assert!(
        evaluate(
            &unk_pack_300gb,
            &target_unknown_1093,
            &rules,
            &settings,
            NOW
        )
        .is_ok(),
        "unknown-count 300 GB pack with 1,093 missing rows must pass"
    );

    // 4. Unknown-count 300 GB pack, missing 100 -> fail (100 * 2560 MB = 250 GB budget)
    let missing_100: Vec<f64> = (1..=100).map(|i| i as f64).collect();
    let target_unknown_100 = Target {
        names: &names,
        other_names: &others,
        wanted_episode: Some(1.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1996),
        fully_aired: false,
        total_episodes: Some(0),
        season_maps: &[],
        missing_episodes: &missing_100,
        owned_episodes: &[],
    };
    assert!(
        matches!(
            evaluate(&unk_pack_300gb, &target_unknown_100, &rules, &settings, NOW),
            Err(Rejection::SizeOutOfRange { .. })
        ),
        "unknown-count 300 GB pack with 100 missing rows must fail"
    );
}

#[test]
fn test_greedy_tiling_prefers_largest_coverage() {
    let names = vec!["one piece".to_string()];
    let others = vec![];
    let settings = Settings::default();
    let rules = Rules::default();

    // Missing 1-782, looking for episode 1.0
    let missing_782: Vec<f64> = (1..=782).map(|i| i as f64).collect();
    let target_initial = Target {
        names: &names,
        other_names: &others,
        wanted_episode: Some(1.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1999),
        fully_aired: false,
        total_episodes: Some(1000),
        season_maps: &[],
        missing_episodes: &missing_782,
        owned_episodes: &[],
    };

    // Candidate A: 001-574 (covers 574 missing), 100 seeders
    let cand_a = mock_candidate(
        "[Judas] One Piece (001-574) [1080p] [Batch]",
        100,
        150 * 1024 * 1024 * 1024,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    // Candidate C: 001-1000 (covers 782 missing), 20 seeders
    let cand_c = mock_candidate(
        "[Anime Time] One Piece (001-1000) [1080p] [Batch]",
        20,
        250 * 1024 * 1024 * 1024,
        "cccccccccccccccccccccccccccccccccccccccc",
    );

    let score_a = evaluate(&cand_a, &target_initial, &rules, &settings, NOW).unwrap();
    let score_c = evaluate(&cand_c, &target_initial, &rules, &settings, NOW).unwrap();

    assert!(
        score_c.total() > score_a.total(),
        "Candidate C (covers 782 missing) must outrank Candidate A (covers 574 missing) even with fewer seeders: {:.1} vs {:.1}",
        score_c.total(),
        score_a.total()
    );

    // After A completes: episodes 1-574 are owned, 575-782 remain missing (208 missing, >= 25).
    // The next cycle looks for episode 575.0.
    let owned_574: Vec<f64> = (1..=574).map(|i| i as f64).collect();
    let missing_rem: Vec<f64> = (575..=782).map(|i| i as f64).collect();
    let target_rem = Target {
        names: &names,
        other_names: &others,
        wanted_episode: Some(575.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1999),
        fully_aired: false,
        total_episodes: Some(1000),
        season_maps: &[],
        missing_episodes: &missing_rem,
        owned_episodes: &owned_574,
    };

    // Candidate B: 575-782 (covers remaining 208 missing)
    let cand_b = mock_candidate(
        "[Judas] One Piece (575-782) [1080p] [Batch]",
        100,
        60 * 1024 * 1024 * 1024,
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );

    let score_b = evaluate(&cand_b, &target_rem, &rules, &settings, NOW).unwrap();
    assert!(
        score_b.signal >= settings.score_threshold,
        "Candidate B covers the remaining 575-782 and passes confident threshold"
    );
}

#[tokio::test]
async fn test_free_space_guard_selects_smaller_pack_or_asks() {
    let clock = FakeClock::new(NOW);
    let db = Db::open_in_memory().unwrap();

    // Show with 100 missing episodes
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, total_episodes, status, metadata_status)
             VALUES (21, 'One Piece', 1000, 'airing', 'ready')",
            [],
        )?;
        c.execute(
            "INSERT INTO show_names (mal_id, name, normalized, source, priority)
             VALUES (21, 'One Piece', 'one piece', 'mal', 1)",
            [],
        )?;
        for i in 1..=100 {
            c.execute(
                "INSERT INTO episodes (mal_id, number, state) VALUES (21, ?1, 'missing')",
                [i as f64],
            )?;
        }
        Ok(())
    })
    .unwrap();

    let ep1_id = ensure_episode(&db, &clock, 21, 1.0).unwrap();
    let job_id = genrl_core::jobs::enqueue_search(&db, &clock, ep1_id).unwrap();

    // Store candidate A: 171 GB (larger than 92 GB usable)
    let hash_a = "1111111111111111111111111111111111111111";
    let hash_b = "2222222222222222222222222222222222222222";
    genrl_core::jobs::remember_candidate(
        &db,
        &clock,
        &genrl_core::jobs::RememberedCandidate {
            info_hash: hash_a.to_string(),
            name: "[Judas] One Piece (001-574) [1080p] [Batch]".to_string(),
            magnet: format!("magnet:?xt=urn:btih:{hash_a}"),
            seeders: 50,
            leechers: 2,
            size_bytes: 171i64 * 1024 * 1024 * 1024,
            group: Some("Judas".into()),
            resolution: Some("1080p".into()),
            source_url: None,
            score: 0.0,
            mal_id: Some(21),
        },
    )
    .unwrap();
    // Store candidate B: 61 GB (fits within 92 GB usable)
    genrl_core::jobs::remember_candidate(
        &db,
        &clock,
        &genrl_core::jobs::RememberedCandidate {
            info_hash: hash_b.to_string(),
            name: "[Erai-raws] One Piece (001-200) [1080p] [Batch]".to_string(),
            magnet: format!("magnet:?xt=urn:btih:{hash_b}"),
            seeders: 45,
            leechers: 2,
            size_bytes: 61i64 * 1024 * 1024 * 1024,
            group: Some("Erai-raws".into()),
            resolution: Some("1080p".into()),
            source_url: None,
            score: 0.0,
            mal_id: Some(21),
        },
    )
    .unwrap();

    // Transition to Choose state so step executes choose
    genrl_core::jobs::transition(
        &db,
        &clock,
        job_id,
        JobState::Choose,
        "test candidates ready",
        &genrl_core::jobs::MoveOptions::default(),
    )
    .unwrap();

    // 112 GB free, floor 20 GB -> 92 GB usable
    let temp_data = tempfile::tempdir().unwrap();
    let temp_dl = tempfile::tempdir().unwrap();
    let mut settings = Settings::load(&db).unwrap();
    settings.download_folder = temp_dl.path().display().to_string();
    settings.disk_floor_gb = 20;
    settings.save(&db).unwrap();

    let disk = Arc::new(FakeDiskProbe::new(112 * 1024 * 1024 * 1024));
    let gate = Arc::new(Gate::new(db.clone()).unwrap());
    let engine = Engine::new(
        db.clone(),
        gate.clone(),
        Arc::new(clock.clone()),
        Arc::new(NoopFinalizer),
        disk.clone(),
        temp_data.path().to_path_buf(),
    );

    // Run choose step: 171 GB is skipped because 171 > 92; 61 GB fits and is chosen
    let next_state = engine.step(job_id).await.unwrap();
    assert_eq!(next_state.to, JobState::Add);

    let updated_job = genrl_core::jobs::get(&db, job_id).unwrap().unwrap();
    let chosen_hash: String = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT info_hash FROM torrents WHERE id = ?1",
                [updated_job.torrent_id.unwrap()],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(
        chosen_hash, hash_b,
        "61 GB candidate must be chosen over 171 GB candidate which exceeded 92 GB usable"
    );

    // Reset job back to Choose state to test when NONE fit
    genrl_core::jobs::transition(
        &db,
        &clock,
        job_id,
        JobState::Choose,
        "test none fit",
        &genrl_core::jobs::MoveOptions::default(),
    )
    .unwrap();

    // Now test when NONE fit: usable = 10 GB
    let disk_small = Arc::new(FakeDiskProbe::new(30 * 1024 * 1024 * 1024)); // 30 - 20 = 10 GB usable
    let engine_small = Engine::new(
        db.clone(),
        gate,
        Arc::new(clock.clone()),
        Arc::new(NoopFinalizer),
        disk_small,
        temp_data.path().to_path_buf(),
    );

    let next_state_none = engine_small.step(job_id).await.unwrap();
    assert_eq!(
        next_state_none.to,
        JobState::NeedsInput,
        "when no pack fits, job must enter needs_input"
    );

    let open_q = genrl_core::questions::QuestionStore::open(&db, NOW).unwrap();
    assert!(
        open_q
            .iter()
            .any(|q| matches!(q.payload, genrl_core::questions::Payload::DiskSpace { .. })),
        "inbox question DiskSpace must be opened"
    );
}
