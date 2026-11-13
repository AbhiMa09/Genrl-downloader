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
