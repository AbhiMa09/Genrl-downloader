use std::fs;
use tempfile::tempdir;

use genrl_core::db::Db;
use genrl_core::library::media_finalizer::finalize_media;
use genrl_core::models::{Job, JobKind, JobState};
use genrl_core::settings::Settings;
use genrl_core::worker::FinalizeContext;

#[tokio::test]
async fn test_series_pack_finalize_e2e_maps_files_and_updates_states() {
    let temp_lib = tempdir().unwrap();
    let temp_dl = tempdir().unwrap();

    let db = Db::open_in_memory().unwrap();

    // 1. Settings with library_folder
    let mut settings = Settings::load(&db).unwrap();
    settings.library_folder = temp_lib.path().display().to_string();
    settings.save(&db).unwrap();

    // 2. Insert media row (Series: Severance, 2022)
    let media_id = db
        .with(|c| {
            c.execute(
                "INSERT INTO media (kind, title, year, created_at) VALUES ('series', 'Severance', 2022, 1000)",
                [],
            )?;
            Ok(c.last_insert_rowid())
        })
        .unwrap();

    // 3. Insert 4 episodes into media_episodes: all 'wanted'
    db.with(|c| {
        for ep in 1..=4 {
            c.execute(
                "INSERT INTO media_episodes (media_id, season, episode, air_date, state)
                 VALUES (?1, 1, ?2, '2022-02-18', 'wanted')",
                rusqlite::params![media_id, ep],
            )?;
        }
        Ok(())
    })
    .unwrap();

    // 4. Create downloaded torrent folder with S01E01, S01E02, S01E03 and an unmapped file
    let pack_dir = temp_dl.path().join("Severance.S01.1080p.WEB-DL");
    fs::create_dir_all(pack_dir.join("Extras")).unwrap();

    let video_bytes = vec![0u8; 52 * 1024 * 1024]; // 52 MB (> 50 MB MIN_VIDEO_BYTES)
    fs::write(
        pack_dir.join("Severance.S01E01.1080p.WEB-DL.mkv"),
        &video_bytes,
    )
    .unwrap();
    fs::write(
        pack_dir.join("Severance.S01E02.1080p.WEB-DL.mkv"),
        &video_bytes,
    )
    .unwrap();
    fs::write(
        pack_dir.join("Severance.S01E03.1080p.WEB-DL.mkv"),
        &video_bytes,
    )
    .unwrap();
    fs::write(pack_dir.join("Extras/featurette.mkv"), &video_bytes).unwrap();

    // 5. Build FinalizeContext
    let job = Job {
        id: 42,
        episode_id: None,
        kind: JobKind::Finalize,
        state: JobState::Finalize,
        attempts: 0,
        next_run_at: None,
        last_error: None,
        torrent_id: None,
        created_at: 1000,
        updated_at: 1000,
        progress: 1.0,
        last_progress_at: None,
        blocked_from: None,
        media_kind: Some("series".into()),
        media_id: Some(media_id),
    };

    let pack_path_str = pack_dir.display().to_string();
    let ctx = FinalizeContext {
        job: &job,
        episode_id: 0,
        mal_id: 0,
        content_path: &pack_path_str,
        size_bytes: (video_bytes.len() * 4) as i64,
    };

    // 6. Execute finalize_media
    let outcome = finalize_media(&db, &ctx, &settings).await.unwrap();
    match outcome {
        genrl_core::worker::FinalizeOutcome::Done(success) => {
            assert!(success.target_episode_covered);
        }
        genrl_core::worker::FinalizeOutcome::NeedsInput { .. } => {
            panic!("Unexpected NeedsInput outcome");
        }
    }

    // 7. Verify files laid out according to layout::series_path
    let season_dir = temp_lib
        .path()
        .join("Series")
        .join("Severance (2022)")
        .join("Season 01");
    let ep1_target = season_dir.join("Severance - S01E01.mkv");
    let ep2_target = season_dir.join("Severance - S01E02.mkv");
    let ep3_target = season_dir.join("Severance - S01E03.mkv");

    assert!(ep1_target.exists(), "Ep1 must exist at {ep1_target:?}");
    assert!(ep2_target.exists(), "Ep2 must exist at {ep2_target:?}");
    assert!(ep3_target.exists(), "Ep3 must exist at {ep3_target:?}");

    // 8. Verify media_episodes states: 1..=3 are 'done' with file_path, 4 remains 'wanted'
    db.with(|c| {
        for ep in 1..=3 {
            let (state, path): (String, Option<String>) = c.query_row(
                "SELECT state, file_path FROM media_episodes WHERE media_id = ?1 AND season = 1 AND episode = ?2",
                rusqlite::params![media_id, ep],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            assert_eq!(state, "done", "Ep {ep} must be marked done");
            assert!(path.is_some(), "Ep {ep} must have file_path set");
            assert!(path.unwrap().contains(&format!("S01E0{ep}")));
        }

        let (ep4_state, ep4_path): (String, Option<String>) = c.query_row(
            "SELECT state, file_path FROM media_episodes WHERE media_id = ?1 AND season = 1 AND episode = 4",
            [media_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        assert_eq!(ep4_state, "wanted", "Uncovered ep 4 must remain wanted");
        assert!(ep4_path.is_none(), "Uncovered ep 4 must have no file_path");
        Ok(())
    }).unwrap();

    // 9. Verify unmapped file is recorded in event log
    let unmapped_logged: i64 = db.with(|c| {
        Ok(c.query_row(
            "SELECT COUNT(*) FROM events WHERE job_id = ?1 AND message LIKE '%unmapped file in series pack: Extras/featurette.mkv%'",
            [job.id],
            |r| r.get(0),
        )?)
    }).unwrap();
    assert_eq!(
        unmapped_logged, 1,
        "Unmapped file must be logged in event log"
    );

    // 10. Re-running finalize: skip transfer for identical file sizes
    let outcome2 = finalize_media(&db, &ctx, &settings).await.unwrap();
    assert!(matches!(
        outcome2,
        genrl_core::worker::FinalizeOutcome::Done(_)
    ));
}

#[tokio::test]
async fn test_series_multi_season_pack_maps_only_wanted_season_files() {
    let temp_lib = tempdir().unwrap();
    let temp_dl = tempdir().unwrap();

    let db = Db::open_in_memory().unwrap();
    let mut settings = Settings::load(&db).unwrap();
    settings.library_folder = temp_lib.path().display().to_string();
    settings.save(&db).unwrap();

    let media_id = db
        .with(|c| {
            c.execute(
                "INSERT INTO media (kind, title, year, created_at) VALUES ('series', 'The Bear', 2022, 1000)",
                [],
            )?;
            Ok(c.last_insert_rowid())
        })
        .unwrap();

    // Only Season 2 Episode 1 is wanted
    db.with(|c| {
        c.execute(
            "INSERT INTO media_episodes (media_id, season, episode, air_date, state)
             VALUES (?1, 2, 1, '2023-06-22', 'wanted')",
            [media_id],
        )?;
        Ok(())
    })
    .unwrap();

    // Multi-season pack S01-S03 folder with S01E01, S02E01, S03E01
    let pack_dir = temp_dl.path().join("The.Bear.S01-S03.1080p.WEB-DL");
    fs::create_dir_all(&pack_dir).unwrap();

    let video_bytes = vec![0u8; 52 * 1024 * 1024];
    fs::write(pack_dir.join("The.Bear.S01E01.1080p.mkv"), &video_bytes).unwrap();
    fs::write(pack_dir.join("The.Bear.S02E01.1080p.mkv"), &video_bytes).unwrap();
    fs::write(pack_dir.join("The.Bear.S03E01.1080p.mkv"), &video_bytes).unwrap();

    let job = Job {
        id: 77,
        episode_id: None,
        kind: JobKind::Finalize,
        state: JobState::Finalize,
        attempts: 0,
        next_run_at: None,
        last_error: None,
        torrent_id: None,
        created_at: 1000,
        updated_at: 1000,
        progress: 1.0,
        last_progress_at: None,
        blocked_from: None,
        media_kind: Some("series".into()),
        media_id: Some(media_id),
    };

    let pack_path_str = pack_dir.display().to_string();
    let ctx = FinalizeContext {
        job: &job,
        episode_id: 0,
        mal_id: 0,
        content_path: &pack_path_str,
        size_bytes: (video_bytes.len() * 3) as i64,
    };

    let outcome = finalize_media(&db, &ctx, &settings).await.unwrap();
    assert!(matches!(
        outcome,
        genrl_core::worker::FinalizeOutcome::Done(_)
    ));

    // Season 2 Ep 1 must be placed in library
    let s02e01_target = temp_lib
        .path()
        .join("Series")
        .join("The Bear (2022)")
        .join("Season 02")
        .join("The Bear - S02E01.mkv");
    assert!(s02e01_target.exists(), "Season 2 Ep 1 must be in library");

    // Season 1 and Season 3 must NOT be placed in library
    let s01_dir = temp_lib
        .path()
        .join("Series")
        .join("The Bear (2022)")
        .join("Season 01");
    let s03_dir = temp_lib
        .path()
        .join("Series")
        .join("The Bear (2022)")
        .join("Season 03");
    assert!(
        !s01_dir.exists(),
        "Unwanted Season 1 must NOT be placed in library"
    );
    assert!(
        !s03_dir.exists(),
        "Unwanted Season 3 must NOT be placed in library"
    );

    // Check event log for skipping unwanted seasons
    let skipped_logged: i64 = db.with(|c| {
        Ok(c.query_row(
            "SELECT COUNT(*) FROM events WHERE job_id = ?1 AND message LIKE '%skipping file for unwanted season%'",
            [job.id],
            |r| r.get(0),
        )?)
    }).unwrap();
    assert_eq!(
        skipped_logged, 2,
        "Both unwanted season files must be logged as skipped"
    );
}

#[tokio::test]
async fn test_series_pack_with_unparseable_files_rejected() {
    let temp_lib = tempdir().unwrap();
    let temp_dl = tempdir().unwrap();

    let db = Db::open_in_memory().unwrap();
    let mut settings = Settings::load(&db).unwrap();
    settings.library_folder = temp_lib.path().display().to_string();
    settings.save(&db).unwrap();

    let media_id = db
        .with(|c| {
            c.execute(
                "INSERT INTO media (kind, title, year, created_at) VALUES ('series', 'Severance', 2022, 1000)",
                [],
            )?;
            Ok(c.last_insert_rowid())
        })
        .unwrap();

    let pack_dir = temp_dl.path().join("Severance.Unparseable.Files.Pack");
    fs::create_dir_all(&pack_dir).unwrap();

    let video_bytes = vec![0u8; 52 * 1024 * 1024];
    fs::write(pack_dir.join("Bonus_Video_Feature.mkv"), &video_bytes).unwrap();
    fs::write(pack_dir.join("Interview_Clip.mp4"), &video_bytes).unwrap();

    let job = Job {
        id: 88,
        episode_id: None,
        kind: JobKind::Finalize,
        state: JobState::Finalize,
        attempts: 0,
        next_run_at: None,
        last_error: None,
        torrent_id: None,
        created_at: 1000,
        updated_at: 1000,
        progress: 1.0,
        last_progress_at: None,
        blocked_from: None,
        media_kind: Some("series".into()),
        media_id: Some(media_id),
    };

    let pack_path_str = pack_dir.display().to_string();
    let ctx = FinalizeContext {
        job: &job,
        episode_id: 0,
        mal_id: 0,
        content_path: &pack_path_str,
        size_bytes: (video_bytes.len() * 2) as i64,
    };

    let result = finalize_media(&db, &ctx, &settings).await;
    assert!(
        result.is_err(),
        "Pack with unparseable files must be rejected"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("unparseable file names"),
        "Error message must specify unparseable file names, got: {err_msg}"
    );
}

#[tokio::test]
async fn test_series_job_large_download_size_gate_and_ambiguous_match_inbox() {
    use genrl_core::clock::FakeClock;
    use genrl_core::http::{Gate, Service, ServicePolicy};
    use genrl_core::library::LibraryFinalizer;
    use genrl_core::questions::{Answer, Choice, QuestionStore};
    use genrl_core::worker::{Engine, FakeDiskProbe};
    use std::sync::Arc;

    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/api/v2/app/version"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("v5.2.4"))
        .mount(&server)
        .await;

    let temp = tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let mut settings = Settings::default();
    settings.qbit_url = server.uri();
    settings.download_folder = temp.path().join("dl").display().to_string();
    settings.library_folder = temp.path().join("lib").display().to_string();
    settings.save(&db).unwrap();
    fs::create_dir_all(&settings.download_folder).unwrap();
    fs::create_dir_all(&settings.library_folder).unwrap();

    let clock = Arc::new(FakeClock::new(1000));
    let disk = Arc::new(FakeDiskProbe::new(500 * 1024 * 1024 * 1024));
    let gate = Arc::new(
        Gate::with_policies(
            db.clone(),
            &[
                (Service::Nyaa, ServicePolicy::for_service(Service::Nyaa)),
                (Service::Qbit, ServicePolicy::for_service(Service::Qbit)),
            ],
        )
        .unwrap(),
    );
    let finalizer = Arc::new(LibraryFinalizer::new(
        db.clone(),
        Arc::clone(&gate),
        Arc::clone(&clock) as Arc<dyn genrl_core::clock::Clock>,
        temp.path().to_path_buf(),
    ));

    let engine = Engine::new(
        db.clone(),
        gate,
        clock.clone(),
        finalizer,
        disk,
        temp.path().to_path_buf(),
    );

    // 1. Insert series media
    let media_id = db
        .with(|c| {
            c.execute(
                "INSERT INTO media (kind, title, year, created_at) VALUES ('series', 'Succession', 2018, 1000)",
                [],
            )?;
            Ok(c.last_insert_rowid())
        })
        .unwrap();

    // 2. Insert season pack torrent: 5 GB total (> 2 GB gate)
    let info_hash = "cccccccccccccccccccccccccccccccccccccccc";
    let torrent_id = db
        .with(|c| {
            c.execute(
                "INSERT INTO torrents (info_hash, name, magnet, size)
                 VALUES (?1, 'Succession S01 1080p WEB-DL Complete', 'magnet:?xt=urn:btih:cccccccccccccccccccccccccccccccccccccccc', 5000000000)",
                [info_hash],
            )?;
            Ok(c.last_insert_rowid())
        })
        .unwrap();

    // 3. Insert series job in 'add' state referencing the 5 GB torrent
    let job_id = db
        .with(|c| {
            c.execute(
                "INSERT INTO jobs (kind, media_kind, media_id, state, torrent_id, created_at, updated_at)
                 VALUES ('add', 'series', ?1, 'add', ?2, 1000, 1000)",
                rusqlite::params![media_id, torrent_id],
            )?;
            Ok(c.last_insert_rowid())
        })
        .unwrap();

    // 4. Step engine: 5 GB season pack must trigger >2 GB size gate and enter NeedsInput
    let s = engine.step(job_id).await.unwrap();
    assert_eq!(
        s.to,
        JobState::NeedsInput,
        "5 GB season pack must enter NeedsInput due to size confirmation gate"
    );

    // 5. Verify confirmation question created in questions table
    let questions = QuestionStore::open(&db, 1000).unwrap();
    assert_eq!(
        questions.len(),
        1,
        "Must create 1 match_review question for >2 GB series download"
    );
    let q = &questions[0];
    assert_eq!(q.kind(), genrl_core::models::QuestionKind::MatchReview);
    if let genrl_core::questions::Payload::MatchReview {
        reason,
        options,
        expected,
    } = &q.payload
    {
        assert!(
            reason.contains("exceeds 2 GB"),
            "Reason must mention >2 GB: {reason}"
        );
        assert_eq!(expected.title, "Succession");
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].info_hash, info_hash);
        assert_eq!(options[0].size_bytes, 5_000_000_000);
    } else {
        panic!("Question payload must be MatchReview");
    }

    // 6. Answer question and verify size gate approval
    let answer = Answer {
        choice: Choice::UseThis {
            info_hash: info_hash.to_string(),
        },
        always: false,
    };
    QuestionStore::answer(&db, q.id, &answer).unwrap();
    let full_q = QuestionStore::get(&db, q.id).unwrap().unwrap();
    genrl_core::worker::apply_answer(&db, clock.as_ref(), &full_q, &answer).unwrap();

    // Check size gate approval query in DB
    let is_approved = db
        .with(|c| {
            let count: i64 = c.query_row(
                "SELECT COUNT(*) FROM questions WHERE id = ?1 AND answer IS NOT NULL",
                [q.id],
                |r| r.get(0),
            )?;
            Ok(count == 1)
        })
        .unwrap();
    assert!(
        is_approved,
        "Question is answered and approved in questions store"
    );
}
