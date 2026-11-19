use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use genrl_core::clock::SystemClock;
use genrl_core::db::Db;
use genrl_core::events::EventLog;
use genrl_core::http::Gate;
use genrl_core::library::media_finalizer::transfer_single_file;
use genrl_core::library::LibraryFinalizer;
use genrl_core::models::{Job, JobKind, JobState};
use genrl_core::settings::Settings;
use genrl_core::worker::{FinalizeContext, FinalizeOutcome, Finalizer};

fn mock_movie_job(job_id: i64, media_id: Option<i64>) -> Job {
    Job {
        id: job_id,
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
        media_kind: Some("movie".to_string()),
        media_id,
        season: None,
        episode: None,
    }
}

#[tokio::test]
async fn test_movie_hardlink_success_and_inode_equal() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let gate = Arc::new(Gate::new(db.clone()).unwrap());
    let clock = Arc::new(SystemClock);
    let finalizer = LibraryFinalizer::new(db.clone(), gate, clock, dir.path().join("data"));

    // Insert media
    let media_id = db.with(|c| {
        c.execute(
            "INSERT INTO media (kind, title, year, created_at) VALUES ('movie', 'Dune Part Two', 2024, 1000)",
            [],
        )?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    let download_dir = dir.path().join("downloads");
    let library_dir = dir.path().join("library");
    fs::create_dir_all(&download_dir).unwrap();
    fs::create_dir_all(&library_dir).unwrap();

    let mut settings = Settings::default();
    settings.library_folder = library_dir.display().to_string();
    settings.save(&db).unwrap();

    let movie_file = download_dir.join("Dune.Part.Two.2024.1080p.mkv");
    fs::write(&movie_file, vec![1u8; 1024 * 1024]).unwrap();

    let job = mock_movie_job(1, Some(media_id));
    let ctx = FinalizeContext {
        job: &job,
        episode_id: 0,
        mal_id: 0,
        content_path: movie_file.to_str().unwrap(),
        size_bytes: 1024 * 1024,
    };

    let outcome = finalizer.finalize(&ctx).await.unwrap();
    match outcome {
        FinalizeOutcome::Done(success) => {
            let dest = PathBuf::from(&success.destination_path);
            assert!(dest.exists());
            assert_eq!(
                dest,
                library_dir
                    .join("Movies")
                    .join("Dune Part Two (2024)")
                    .join("Dune Part Two (2024).mkv")
            );

            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let meta_src = fs::metadata(&movie_file).unwrap();
                let meta_dest = fs::metadata(&dest).unwrap();
                assert_eq!(
                    meta_src.ino(),
                    meta_dest.ino(),
                    "hardlink must share same inode"
                );
            }
        }
        _ => panic!("expected FinalizeOutcome::Done"),
    }
}

#[tokio::test]
async fn test_movie_existing_same_size_target_treated_as_done() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let gate = Arc::new(Gate::new(db.clone()).unwrap());
    let clock = Arc::new(SystemClock);
    let finalizer = LibraryFinalizer::new(db.clone(), gate, clock, dir.path().join("data"));

    let media_id = db.with(|c| {
        c.execute("INSERT INTO media (kind, title, year, created_at) VALUES ('movie', 'Gladiator', 2000, 1000)", [])?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    let download_dir = dir.path().join("downloads");
    let library_dir = dir.path().join("library");
    fs::create_dir_all(&download_dir).unwrap();
    let dest_dir = library_dir.join("Movies").join("Gladiator (2000)");
    fs::create_dir_all(&dest_dir).unwrap();

    let mut settings = Settings::default();
    settings.library_folder = library_dir.display().to_string();
    settings.save(&db).unwrap();

    let movie_file = download_dir.join("Gladiator.2000.1080p.mkv");
    let existing_dest = dest_dir.join("Gladiator (2000).mkv");
    fs::write(&movie_file, vec![42u8; 500_000]).unwrap();
    fs::write(&existing_dest, vec![42u8; 500_000]).unwrap(); // Same size

    let job = mock_movie_job(2, Some(media_id));
    let ctx = FinalizeContext {
        job: &job,
        episode_id: 0,
        mal_id: 0,
        content_path: movie_file.to_str().unwrap(),
        size_bytes: 500_000,
    };

    let outcome = finalizer.finalize(&ctx).await.unwrap();
    match outcome {
        FinalizeOutcome::Done(success) => {
            assert_eq!(
                success.destination_path,
                existing_dest.display().to_string()
            );
        }
        _ => panic!("expected FinalizeOutcome::Done without asking question"),
    }
}

#[tokio::test]
async fn test_movie_existing_different_size_target_raises_inbox_question() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let gate = Arc::new(Gate::new(db.clone()).unwrap());
    let clock = Arc::new(SystemClock);
    let finalizer = LibraryFinalizer::new(db.clone(), gate, clock, dir.path().join("data"));

    let media_id = db.with(|c| {
        c.execute("INSERT INTO media (kind, title, year, created_at) VALUES ('movie', 'Inception', 2010, 1000)", [])?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    let download_dir = dir.path().join("downloads");
    let library_dir = dir.path().join("library");
    fs::create_dir_all(&download_dir).unwrap();
    let dest_dir = library_dir.join("Movies").join("Inception (2010)");
    fs::create_dir_all(&dest_dir).unwrap();

    let mut settings = Settings::default();
    settings.library_folder = library_dir.display().to_string();
    settings.save(&db).unwrap();

    let movie_file = download_dir.join("Inception.2010.1080p.mkv");
    let existing_dest = dest_dir.join("Inception (2010).mkv");
    fs::write(&movie_file, vec![1u8; 800_000]).unwrap();
    fs::write(&existing_dest, vec![2u8; 400_000]).unwrap(); // Different size!

    let job = mock_movie_job(3, Some(media_id));
    db.with(|c| {
        c.execute(
            "INSERT INTO jobs (id, kind, state, created_at, updated_at, media_kind, media_id) VALUES (3, 'finalize', 'finalize', 1000, 1000, 'movie', ?1)",
            [media_id],
        )?;
        Ok(())
    }).unwrap();
    let ctx = FinalizeContext {
        job: &job,
        episode_id: 0,
        mal_id: 0,
        content_path: movie_file.to_str().unwrap(),
        size_bytes: 800_000,
    };

    let outcome = finalizer.finalize(&ctx).await.unwrap();
    match outcome {
        FinalizeOutcome::NeedsInput { question_id } => {
            assert!(question_id > 0);
        }
        _ => panic!("expected FinalizeOutcome::NeedsInput for collision"),
    }
}

#[tokio::test]
async fn test_movie_sidecar_subtitles_linked() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let gate = Arc::new(Gate::new(db.clone()).unwrap());
    let clock = Arc::new(SystemClock);
    let finalizer = LibraryFinalizer::new(db.clone(), gate, clock, dir.path().join("data"));

    let media_id = db.with(|c| {
        c.execute("INSERT INTO media (kind, title, year, created_at) VALUES ('movie', 'Parasite', 2019, 1000)", [])?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    let download_dir = dir
        .path()
        .join("downloads")
        .join("Parasite.2019.1080p-GROUP");
    let library_dir = dir.path().join("library");
    fs::create_dir_all(&download_dir).unwrap();
    fs::create_dir_all(&library_dir).unwrap();

    let mut settings = Settings::default();
    settings.library_folder = library_dir.display().to_string();
    settings.save(&db).unwrap();

    let movie_file = download_dir.join("Parasite.2019.1080p.mkv");
    let sub_file = download_dir.join("Parasite.2019.1080p.en.srt");
    let f = fs::File::create(&movie_file).unwrap();
    f.set_len(60 * 1024 * 1024).unwrap();
    fs::write(&sub_file, b"1\n00:00:01,000 --> 00:00:02,000\nHello").unwrap();

    let job = mock_movie_job(4, Some(media_id));
    let ctx = FinalizeContext {
        job: &job,
        episode_id: 0,
        mal_id: 0,
        content_path: download_dir.to_str().unwrap(),
        size_bytes: 60 * 1024 * 1024,
    };

    let outcome = finalizer.finalize(&ctx).await.unwrap();
    match outcome {
        FinalizeOutcome::Done(success) => {
            let dest_video = PathBuf::from(&success.destination_path);
            let dest_sub = dest_video.parent().unwrap().join("Parasite (2019).en.srt");
            assert!(dest_video.exists());
            assert!(dest_sub.exists());
        }
        _ => panic!("expected FinalizeOutcome::Done with sidecar subtitle linked"),
    }
}

#[tokio::test]
async fn test_copy_fallback_logs_warning_and_preserves_source() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let event_log = EventLog::new(db.clone());

    let src = dir.path().join("source.mkv");
    let dst = dir.path().join("dest.mkv");
    fs::write(&src, b"important movie content").unwrap();

    // Directly call transfer_single_file
    transfer_single_file(&db, &event_log, 10, 0, &src, &dst).unwrap();
    assert!(src.exists(), "source must NEVER be deleted");
    assert!(dst.exists(), "target must exist after transfer");
}
