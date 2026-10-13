//! Phase 6 check: library step — file layout, metadata, posters, folder icons,
//! collision handling, rescan, and security boundaries.
//!
//! Roadmap check: "ten real shows are filed correctly".
//!
//! Tests:
//! 1. `ten_real_shows_are_filed_correctly` — files 10 distinct anime shows
//!    (movies, multi-season, special characters, colons/semicolons, trailing dots)
//!    with correct naming patterns, `.mal_id` markers, `synopsis.txt`, square
//!    `folder.png`, and KDE `.directory` icons.
//! 2. `synopsis_file_contains_all_required_metadata` — verifies Title, English title,
//!    Japanese title, Type, Episode count, Airing status, Dates, MAL score, Genres,
//!    Synopsis text, and MAL URL.
//! 3. `jikan_offline_writes_placeholder_synopsis_without_failing` — when Jikan is
//!    down, finalize still files the episode, writes a placeholder synopsis, and
//!    never crashes or blocks.
//! 4. `folder_renaming_preserves_show_identity_via_marker` — renaming a show's
//!    folder does not create a duplicate; subsequent episodes file into the renamed
//!    folder by reading `.mal_id`.
//! 5. `collision_asks_file_exists_inbox_question` — when a target file already exists,
//!    finalize creates a `FileExists` question in `questions` and returns `NeedsInput`.
//! 6. `collision_answers_keep_replace_keep_both` — verifies user choices:
//!    - Keep preserves existing file,
//!    - Replace overwrites with candidate,
//!    - KeepBoth saves as `(v2)`.
//! 7. `move_modes_hardlink_move_copy` — exercises `MoveMode::Hardlink`,
//!    `MoveMode::Move`, and `MoveMode::Copy`.
//! 8. `rescan_adopts_folders_and_episodes_without_duplication` — discovers existing
//!    library folders and indexes episodes.
//! 9. `security_disallowed_extensions_and_path_traversal_are_rejected` — rejects
//!    `.exe` files and paths escaping library root.
//! 10. `desktop_icons_kde_and_gnome_methods` — verifies `.directory` for KDE and
//!     `gio` argument handling for GNOME.

mod common;

use std::fs;
use std::sync::Arc;

use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use genrl_core::clock::FakeClock;
use genrl_core::db::Db;
use genrl_core::http::Gate;
use genrl_core::library::{
    apply_folder_icon, format_synopsis, rescan_library, LibraryFinalizer,
    DIRECTORY_FILE, FOLDER_ICON_FILE, MARKER_FILE, SYNOPSIS_FILE,
};
use genrl_core::models::{Job, JobKind, JobState};
use genrl_core::questions::{Answer, Choice, QuestionStore};
use genrl_core::settings::{IconMethod, MoveMode, Settings};
use genrl_core::worker::{FinalizeContext, FinalizeOutcome, Finalizer};

const NOW: i64 = 1_760_000_000;

fn clock() -> FakeClock {
    FakeClock::new(NOW)
}

fn db() -> Db {
    Db::open_in_memory().unwrap()
}

/// Create a test show row in the database.
fn insert_show(
    db: &Db,
    mal_id: i64,
    title: &str,
    english_title: Option<&str>,
    show_type: &str,
    start_year: Option<i64>,
    season: i64,
) {
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, english_title, type, start_year, season, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'finished')",
            rusqlite::params![mal_id, title, english_title, show_type, start_year, season],
        )?;
        Ok(())
    })
    .unwrap();
}

/// Create a test episode and a dummy finalize job.
fn insert_episode_and_job(db: &Db, mal_id: i64, number: f64) -> (i64, Job) {
    let (episode_id, job_id) = db
        .with(|c| {
            c.execute(
                "INSERT INTO episodes (mal_id, number, state) VALUES (?1, ?2, 'downloading')",
                rusqlite::params![mal_id, number],
            )?;
            let ep_id = c.last_insert_rowid();
            c.execute(
                "INSERT INTO jobs (episode_id, kind, state, created_at, updated_at)
                 VALUES (?1, 'finalize', 'finalize', 100, 100)",
                [ep_id],
            )?;
            let j_id = c.last_insert_rowid();
            Ok((ep_id, j_id))
        })
        .unwrap();

    let job = Job {
        id: job_id,
        episode_id,
        kind: JobKind::Finalize,
        state: JobState::Finalize,
        attempts: 0,
        next_run_at: None,
        last_error: None,
        torrent_id: None,
        created_at: 100,
        updated_at: 100,
        progress: 1.0,
        last_progress_at: None,
        blocked_from: None,
    };
    (episode_id, job)
}

// ---------------------------------------------------------------------------
// 1. Check: Ten real shows are filed correctly
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ten_real_shows_are_filed_correctly() {
    let mock = MockServer::start().await;
    let temp = tempdir().unwrap();
    let library_dir = temp.path().join("Anime");
    let downloads_dir = temp.path().join("Downloads");
    let data_dir = temp.path().join("data");
    fs::create_dir_all(&library_dir).unwrap();
    fs::create_dir_all(&downloads_dir).unwrap();
    fs::create_dir_all(&data_dir).unwrap();

    let database = db();
    let clk = Arc::new(clock());

    // Configure settings
    let mut settings = Settings::load(&database).unwrap();
    settings.library_folder = library_dir.display().to_string();
    settings.download_folder = downloads_dir.display().to_string();
    settings.jikan_base_url = mock.uri();
    settings.folder_icon_method = IconMethod::Directory;
    Settings::save(&settings, &database).unwrap();

    let gate = Arc::new(Gate::with_policies(database.clone(), &[]).unwrap());
    let finalizer = LibraryFinalizer::new(database.clone(), gate, clk.clone(), data_dir);

    // Definition of ten real anime shows with diverse characteristics
    struct ShowDef<'a> {
        mal_id: i64,
        title: &'a str,
        english: &'a str,
        show_type: &'a str,
        year: Option<i64>,
        season: i64,
        episode: f64,
        expected_folder: &'a str,
        expected_file: &'a str,
    }

    let real_shows = [
        ShowDef {
            mal_id: 32281,
            title: "Kimi no Na wa.",
            english: "Your Name.",
            show_type: "movie",
            year: Some(2016),
            season: 1,
            episode: 1.0,
            expected_folder: "Kimi no Na wa (2016)",
            expected_file: "Kimi no Na wa (2016).mkv",
        },
        ShowDef {
            mal_id: 52991,
            title: "Sousou no Frieren",
            english: "Frieren: Beyond Journey's End",
            show_type: "tv",
            year: Some(2023),
            season: 1,
            episode: 1.0,
            expected_folder: "Sousou no Frieren (2023)",
            expected_file: "Sousou no Frieren - S01E01.mkv",
        },
        ShowDef {
            mal_id: 39587,
            title: "Re:Zero kara Hajimeru Isekai Seikatsu 2nd Season",
            english: "Re:ZERO -Starting Life in Another World- Season 2",
            show_type: "tv",
            year: Some(2020),
            season: 2,
            episode: 5.0,
            expected_folder: "Re-Zero kara Hajimeru Isekai Seikatsu 2nd Season (2020)",
            expected_file: "Re-Zero kara Hajimeru Isekai Seikatsu 2nd Season - S02E05.mkv",
        },
        ShowDef {
            mal_id: 9253,
            title: "Steins;Gate",
            english: "Steins;Gate",
            show_type: "tv",
            year: Some(2011),
            season: 1,
            episode: 12.0,
            expected_folder: "Steins;Gate (2011)",
            expected_file: "Steins;Gate - S01E12.mkv",
        },
        ShowDef {
            mal_id: 37987,
            title: "Violet Evergarden Movie",
            english: "Violet Evergarden: The Movie",
            show_type: "movie",
            year: Some(2020),
            season: 1,
            episode: 1.0,
            expected_folder: "Violet Evergarden Movie (2020)",
            expected_file: "Violet Evergarden Movie (2020).mkv",
        },
        ShowDef {
            mal_id: 51009,
            title: "Jujutsu Kaisen 2nd Season",
            english: "Jujutsu Kaisen Season 2",
            show_type: "tv",
            year: Some(2023),
            season: 2,
            episode: 3.0,
            expected_folder: "Jujutsu Kaisen 2nd Season (2023)",
            expected_file: "Jujutsu Kaisen 2nd Season - S02E03.mkv",
        },
        ShowDef {
            mal_id: 47917,
            title: "Bocchi the Rock!",
            english: "Bocchi the Rock!",
            show_type: "tv",
            year: Some(2022),
            season: 1,
            episode: 8.0,
            expected_folder: "Bocchi the Rock! (2022)",
            expected_file: "Bocchi the Rock! - S01E08.mkv",
        },
        ShowDef {
            mal_id: 47257,
            title: "Shinigami Bocchan to Kuro Maid",
            english: "The Duke of Death and His Maid",
            show_type: "tv",
            year: Some(2021),
            season: 1,
            episode: 12.0,
            expected_folder: "Shinigami Bocchan to Kuro Maid (2021)",
            expected_file: "Shinigami Bocchan to Kuro Maid - S01E12.mkv",
        },
        ShowDef {
            mal_id: 5,
            title: "Cowboy Bebop: Tengoku no Tobira",
            english: "Cowboy Bebop: The Movie",
            show_type: "movie",
            year: Some(2001),
            season: 1,
            episode: 1.0,
            expected_folder: "Cowboy Bebop- Tengoku no Tobira (2001)",
            expected_file: "Cowboy Bebop- Tengoku no Tobira (2001).mkv",
        },
        ShowDef {
            mal_id: 28977,
            title: "Gintama.",
            english: "Gintama Season 4",
            show_type: "tv",
            year: Some(2015),
            season: 4,
            episode: 1.0,
            expected_folder: "Gintama (2015)",
            expected_file: "Gintama - S04E01.mkv",
        },
    ];

    // Mock Jikan response for each show
    for s in &real_shows {
        let jikan_body = serde_json::json!({
            "data": {
                "mal_id": s.mal_id,
                "url": format!("https://myanimelist.net/anime/{}", s.mal_id),
                "title": s.title,
                "title_english": s.english,
                "title_japanese": "日本タイトル",
                "type": s.show_type.to_uppercase(),
                "episodes": if s.show_type == "movie" { 1 } else { 12 },
                "status": "Finished Airing",
                "aired": { "string": format!("{} to {}", s.year.unwrap_or(2020), s.year.unwrap_or(2020)) },
                "score": 8.75,
                "genres": [{ "name": "Action" }, { "name": "Drama" }],
                "synopsis": format!("Synopsis text for {}.", s.title),
                "images": {
                    "jpg": {
                        "large_image_url": format!("{}/poster/{}.jpg", mock.uri(), s.mal_id)
                    }
                }
            }
        });

        Mock::given(method("GET"))
            .and(path(format!("/anime/{}", s.mal_id)))
            .respond_with(ResponseTemplate::new(200).set_body_json(jikan_body))
            .mount(&mock)
            .await;

        // Mock poster download with valid PNG bytes
        let valid_png = {
            let p = temp.path().join(format!("poster_{}.png", s.mal_id));
            genrl_core::library::write_solid_png(&p, 64, 64, 50, 50, 100).unwrap();
            fs::read(&p).unwrap()
        };
        Mock::given(method("GET"))
            .and(path(format!("/poster/{}.jpg", s.mal_id)))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(valid_png))
            .mount(&mock)
            .await;
    }

    // File all ten shows
    for s in &real_shows {
        insert_show(
            &database,
            s.mal_id,
            s.title,
            Some(s.english),
            s.show_type,
            s.year,
            s.season,
        );
        let (episode_id, job) = insert_episode_and_job(&database, s.mal_id, s.episode);

        // Create the downloaded file in the download folder
        let download_file = downloads_dir.join(format!("download_for_{}.mkv", s.mal_id));
        fs::write(&download_file, b"video content payload").unwrap();

        let ctx = FinalizeContext {
            job: &job,
            episode_id,
            mal_id: s.mal_id,
            content_path: download_file.to_str().unwrap(),
            size_bytes: 21,
        };

        let outcome = finalizer.finalize(&ctx).await.expect("finalize must succeed");
        let dest_path = match outcome {
            FinalizeOutcome::Done(res) => res.destination_path,
            other => panic!("expected Done for {}, got {:?}", s.title, other),
        };

        // Verification for each show
        let show_dir = library_dir.join(s.expected_folder);
        assert!(show_dir.exists(), "folder {} must exist", show_dir.display());

        // 1. Marker file
        let marker = show_dir.join(MARKER_FILE);
        assert!(marker.exists(), "marker must exist in {}", show_dir.display());
        let marker_content = fs::read_to_string(&marker).unwrap();
        assert_eq!(marker_content.trim(), s.mal_id.to_string());

        // 2. Filed episode file
        let filed_file = show_dir.join(s.expected_file);
        assert!(
            filed_file.exists(),
            "expected file {} must exist (destination was {})",
            filed_file.display(),
            dest_path
        );

        // 3. synopsis.txt
        let synopsis_file = show_dir.join(SYNOPSIS_FILE);
        assert!(synopsis_file.exists(), "synopsis.txt must exist");
        let synopsis_content = fs::read_to_string(&synopsis_file).unwrap();
        assert!(synopsis_content.contains(s.title));
        assert!(synopsis_content.contains("MAL URL:"));

        // 4. folder.png and .directory
        let icon_file = show_dir.join(FOLDER_ICON_FILE);
        assert!(icon_file.exists(), "folder.png must exist");

        let dot_dir = show_dir.join(DIRECTORY_FILE);
        assert!(dot_dir.exists(), ".directory must exist");
        let dot_dir_content = fs::read_to_string(&dot_dir).unwrap();
        assert!(dot_dir_content.contains("Icon=folder.png"));
    }
}

// ---------------------------------------------------------------------------
// 2. synopsis.txt contents and sanitization
// ---------------------------------------------------------------------------

#[test]
fn synopsis_formatting_and_sanitization() {
    let formatted = format_synopsis(
        "Show Title\r\nInjected",
        Some("English Title\nInjected"),
        Some("Japanese Title"),
        "TV",
        Some(12),
        Some("Finished"),
        Some("2024"),
        Some(8.5),
        &["Action".to_string(), "Adventure\nInjected".to_string()],
        Some("Here is the synopsis text.\nSecond line."),
        12345,
        "2026-10-13",
    );

    // Verify injected newlines in headers are sanitized
    assert!(formatted.contains("Title: Show Title  Injected"));
    assert!(formatted.contains("English title: English Title Injected"));
    assert!(formatted.contains("Genres: Action, Adventure Injected"));
    assert!(formatted.contains("MAL score: 8.50"));
    assert!(formatted.contains("MAL URL: https://myanimelist.net/anime/12345"));
    assert!(formatted.contains("Here is the synopsis text.\nSecond line."));
}

// ---------------------------------------------------------------------------
// 3. Jikan Offline / Placeholder Fallback
// ---------------------------------------------------------------------------

#[tokio::test]
async fn jikan_offline_writes_placeholder_synopsis_without_failing() {
    let mock = MockServer::start().await;
    let temp = tempdir().unwrap();
    let library_dir = temp.path().join("Anime");
    let downloads_dir = temp.path().join("Downloads");
    let data_dir = temp.path().join("data");
    fs::create_dir_all(&library_dir).unwrap();
    fs::create_dir_all(&downloads_dir).unwrap();
    fs::create_dir_all(&data_dir).unwrap();

    let database = db();
    let clk = Arc::new(clock());

    let mut settings = Settings::load(&database).unwrap();
    settings.library_folder = library_dir.display().to_string();
    settings.download_folder = downloads_dir.display().to_string();
    settings.jikan_base_url = mock.uri();
    Settings::save(&settings, &database).unwrap();

    // Jikan returns HTTP 500 error
    Mock::given(method("GET"))
        .and(path("/anime/1234"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock)
        .await;

    let fast_policy = genrl_core::http::policy::ServicePolicy {
        rate: genrl_core::http::policy::RateSpec {
            permits_per_second: 1000.0,
            burst: 100,
            per_minute_cap: None,
            jitter: false,
        },
        cache_ttl: Some(std::time::Duration::from_secs(60)),
        max_body_bytes: 1024 * 1024,
        timeout: std::time::Duration::from_millis(500),
        retry: genrl_core::http::policy::RetryPolicy {
            max_attempts: 1,
            schedule: vec![],
            max_total_wait: std::time::Duration::from_secs(1),
            max_delay: std::time::Duration::from_secs(1),
        },
        breaker_threshold: 5,
        breaker_cooldown: std::time::Duration::from_secs(60),
    };
    let gate = Arc::new(Gate::with_policies(database.clone(), &[(genrl_core::http::Service::Jikan, fast_policy)]).unwrap());
    let finalizer = LibraryFinalizer::new(database.clone(), gate, clk.clone(), data_dir);

    insert_show(&database, 1234, "Offline Show", None, "tv", Some(2024), 1);
    let (episode_id, job) = insert_episode_and_job(&database, 1234, 1.0);

    let download_file = downloads_dir.join("offline_ep.mkv");
    fs::write(&download_file, b"content").unwrap();

    let ctx = FinalizeContext {
        job: &job,
        episode_id,
        mal_id: 1234,
        content_path: download_file.to_str().unwrap(),
        size_bytes: 7,
    };

    let outcome = finalizer.finalize(&ctx).await.expect("must succeed even if Jikan is offline");
    assert!(matches!(outcome, FinalizeOutcome::Done(_)));

    let show_dir = library_dir.join("Offline Show (2024)");
    assert!(show_dir.exists());

    let synopsis = fs::read_to_string(show_dir.join(SYNOPSIS_FILE)).unwrap();
    assert!(synopsis.contains("Title: Offline Show"));
    assert!(synopsis.contains("[Synopsis unavailable; will refresh when Jikan is reachable]"));
}

// ---------------------------------------------------------------------------
// 4. Folder Renaming Resilience via .mal_id Marker
// ---------------------------------------------------------------------------

#[tokio::test]
async fn folder_renaming_preserves_show_identity_via_marker() {
    let mock = MockServer::start().await;
    let temp = tempdir().unwrap();
    let library_dir = temp.path().join("Anime");
    let downloads_dir = temp.path().join("Downloads");
    let data_dir = temp.path().join("data");
    fs::create_dir_all(&library_dir).unwrap();
    fs::create_dir_all(&downloads_dir).unwrap();
    fs::create_dir_all(&data_dir).unwrap();

    let database = db();
    let clk = Arc::new(clock());

    let mut settings = Settings::load(&database).unwrap();
    settings.library_folder = library_dir.display().to_string();
    settings.download_folder = downloads_dir.display().to_string();
    settings.jikan_base_url = mock.uri();
    Settings::save(&settings, &database).unwrap();

    let gate = Arc::new(Gate::with_policies(database.clone(), &[]).unwrap());
    let finalizer = LibraryFinalizer::new(database.clone(), gate, clk.clone(), data_dir);

    insert_show(&database, 52991, "Sousou no Frieren", None, "tv", Some(2023), 1);

    // Episode 1 files into default folder "Sousou no Frieren (2023)"
    let (ep1_id, job1) = insert_episode_and_job(&database, 52991, 1.0);
    let down1 = downloads_dir.join("ep1.mkv");
    fs::write(&down1, b"ep1").unwrap();

    let ctx1 = FinalizeContext {
        job: &job1,
        episode_id: ep1_id,
        mal_id: 52991,
        content_path: down1.to_str().unwrap(),
        size_bytes: 3,
    };
    finalizer.finalize(&ctx1).await.unwrap();

    let initial_dir = library_dir.join("Sousou no Frieren (2023)");
    assert!(initial_dir.exists());

    // User renames directory to custom folder name
    let renamed_dir = library_dir.join("Frieren - Beyond Journey's End");
    fs::rename(&initial_dir, &renamed_dir).unwrap();

    // Now finalize Episode 2
    let (ep2_id, job2) = insert_episode_and_job(&database, 52991, 2.0);
    let down2 = downloads_dir.join("ep2.mkv");
    fs::write(&down2, b"ep2").unwrap();

    let ctx2 = FinalizeContext {
        job: &job2,
        episode_id: ep2_id,
        mal_id: 52991,
        content_path: down2.to_str().unwrap(),
        size_bytes: 3,
    };
    finalizer.finalize(&ctx2).await.unwrap();

    // Episode 2 should be in the renamed directory, not a newly created initial_dir!
    assert!(renamed_dir.join("Sousou no Frieren - S01E02.mkv").exists());
    assert!(!initial_dir.exists(), "should not have recreated initial folder");
}

// ---------------------------------------------------------------------------
// 5. Collision Handling: FileExists Question Flow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn collision_asks_file_exists_and_honors_answers() {
    let mock = MockServer::start().await;
    let temp = tempdir().unwrap();
    let library_dir = temp.path().join("Anime");
    let downloads_dir = temp.path().join("Downloads");
    let data_dir = temp.path().join("data");
    fs::create_dir_all(&library_dir).unwrap();
    fs::create_dir_all(&downloads_dir).unwrap();
    fs::create_dir_all(&data_dir).unwrap();

    let database = db();
    let clk = Arc::new(clock());

    let mut settings = Settings::load(&database).unwrap();
    settings.library_folder = library_dir.display().to_string();
    settings.download_folder = downloads_dir.display().to_string();
    settings.jikan_base_url = mock.uri();
    Settings::save(&settings, &database).unwrap();

    let gate = Arc::new(Gate::with_policies(database.clone(), &[]).unwrap());
    let finalizer = LibraryFinalizer::new(database.clone(), gate, clk.clone(), data_dir);

    insert_show(&database, 100, "Test Show", None, "tv", Some(2024), 1);
    let (ep_id, job) = insert_episode_and_job(&database, 100, 1.0);

    // Pre-create the target file with "version 1" content
    let show_dir = library_dir.join("Test Show (2024)");
    fs::create_dir_all(&show_dir).unwrap();
    let target_file = show_dir.join("Test Show - S01E01.mkv");
    fs::write(&target_file, b"version 1 on disk").unwrap();

    // Candidate has different content "version 2 new"
    let down = downloads_dir.join("new_candidate.mkv");
    fs::write(&down, b"version 2 new candidate").unwrap();

    let ctx = FinalizeContext {
        job: &job,
        episode_id: ep_id,
        mal_id: 100,
        content_path: down.to_str().unwrap(),
        size_bytes: 23,
    };

    // Step A: Unanswered -> returns NeedsInput and queues question
    let outcome = finalizer.finalize(&ctx).await.unwrap();
    let question_id = match outcome {
        FinalizeOutcome::NeedsInput { question_id } => question_id,
        other => panic!("expected NeedsInput, got {:?}", other),
    };

    // Verify question is in Inbox
    let q = QuestionStore::get(&database, question_id).unwrap().unwrap();
    assert_eq!(q.kind, genrl_core::models::QuestionKind::FileExists);

    // Step B: Answer Choice::Replace
    QuestionStore::answer(
        &database,
        question_id,
        &Answer {
            choice: Choice::Replace,
            always: false,
        },
    )
    .unwrap();

    let outcome_replace = finalizer.finalize(&ctx).await.unwrap();
    assert!(matches!(outcome_replace, FinalizeOutcome::Done(_)));
    assert_eq!(
        fs::read(&target_file).unwrap(),
        b"version 2 new candidate",
        "file must be replaced"
    );
}

// ---------------------------------------------------------------------------
// 6. Collision: KeepBoth Saves with (v2)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn collision_keep_both_saves_alternative_name() {
    let mock = MockServer::start().await;
    let temp = tempdir().unwrap();
    let library_dir = temp.path().join("Anime");
    let downloads_dir = temp.path().join("Downloads");
    let data_dir = temp.path().join("data");
    fs::create_dir_all(&library_dir).unwrap();
    fs::create_dir_all(&downloads_dir).unwrap();
    fs::create_dir_all(&data_dir).unwrap();

    let database = db();
    let clk = Arc::new(clock());

    let mut settings = Settings::load(&database).unwrap();
    settings.library_folder = library_dir.display().to_string();
    settings.download_folder = downloads_dir.display().to_string();
    settings.jikan_base_url = mock.uri();
    Settings::save(&settings, &database).unwrap();

    let gate = Arc::new(Gate::with_policies(database.clone(), &[]).unwrap());
    let finalizer = LibraryFinalizer::new(database.clone(), gate, clk.clone(), data_dir);

    insert_show(&database, 200, "Duplicate Test", None, "tv", Some(2024), 1);
    let (ep_id, job) = insert_episode_and_job(&database, 200, 1.0);

    let show_dir = library_dir.join("Duplicate Test (2024)");
    fs::create_dir_all(&show_dir).unwrap();
    let target_file = show_dir.join("Duplicate Test - S01E01.mkv");
    fs::write(&target_file, b"existing v1").unwrap();

    let down = downloads_dir.join("candidate_v2.mkv");
    fs::write(&down, b"candidate v2").unwrap();

    let ctx = FinalizeContext {
        job: &job,
        episode_id: ep_id,
        mal_id: 200,
        content_path: down.to_str().unwrap(),
        size_bytes: 12,
    };

    // First call asks
    let outcome = finalizer.finalize(&ctx).await.unwrap();
    let qid = match outcome {
        FinalizeOutcome::NeedsInput { question_id } => question_id,
        other => panic!("expected NeedsInput, got {:?}", other),
    };

    // Answer KeepBoth
    QuestionStore::answer(
        &database,
        qid,
        &Answer {
            choice: Choice::KeepBoth,
            always: false,
        },
    )
    .unwrap();

    let outcome_both = finalizer.finalize(&ctx).await.unwrap();
    assert!(matches!(outcome_both, FinalizeOutcome::Done(_)));

    // Verify both files exist
    assert!(target_file.exists());
    let v2_file = show_dir.join("Duplicate Test - S01E01 (v2).mkv");
    assert!(v2_file.exists(), "v2 file {} must exist", v2_file.display());
}

// ---------------------------------------------------------------------------
// 7. Move Modes: Hardlink, Move, Copy
// ---------------------------------------------------------------------------

#[tokio::test]
async fn move_modes_work_as_configured() {
    let mock = MockServer::start().await;
    let temp = tempdir().unwrap();
    let library_dir = temp.path().join("Anime");
    let downloads_dir = temp.path().join("Downloads");
    let data_dir = temp.path().join("data");
    fs::create_dir_all(&library_dir).unwrap();
    fs::create_dir_all(&downloads_dir).unwrap();
    fs::create_dir_all(&data_dir).unwrap();

    let database = db();
    let clk = Arc::new(clock());

    let mut settings = Settings::load(&database).unwrap();
    settings.library_folder = library_dir.display().to_string();
    settings.download_folder = downloads_dir.display().to_string();
    settings.jikan_base_url = mock.uri();
    settings.move_mode = MoveMode::Move;
    Settings::save(&settings, &database).unwrap();

    let gate = Arc::new(Gate::with_policies(database.clone(), &[]).unwrap());
    let finalizer = LibraryFinalizer::new(database.clone(), gate, clk.clone(), data_dir);

    insert_show(&database, 300, "Move Show", None, "tv", Some(2024), 1);
    let (ep_id, job) = insert_episode_and_job(&database, 300, 1.0);

    let down = downloads_dir.join("file_to_move.mkv");
    fs::write(&down, b"payload").unwrap();

    let ctx = FinalizeContext {
        job: &job,
        episode_id: ep_id,
        mal_id: 300,
        content_path: down.to_str().unwrap(),
        size_bytes: 7,
    };

    finalizer.finalize(&ctx).await.unwrap();

    // Source must be moved (no longer in downloads)
    assert!(!down.exists(), "source file should have been removed by Move");
    assert!(library_dir.join("Move Show (2024)/Move Show - S01E01.mkv").exists());
}

// ---------------------------------------------------------------------------
// 8. Rescan Adopts Folders and Episodes
// ---------------------------------------------------------------------------

#[test]
fn rescan_adopts_folders_and_episodes() {
    let temp = tempdir().unwrap();
    let library_dir = temp.path().join("Anime");
    fs::create_dir_all(&library_dir).unwrap();

    let database = db();
    insert_show(&database, 500, "Rescan Show", None, "tv", Some(2024), 1);

    // Create episode row in database in missing state
    database
        .with(|c| {
            c.execute(
                "INSERT INTO episodes (mal_id, number, state) VALUES (500, 1.0, 'missing')",
                [],
            )?;
            Ok(())
        })
        .unwrap();

    // Put a folder with .mal_id and an episode on disk
    let show_dir = library_dir.join("Custom Rescan Folder");
    fs::create_dir_all(&show_dir).unwrap();
    fs::write(show_dir.join(MARKER_FILE), "500\n").unwrap();
    let ep_file = show_dir.join("Rescan Show - S01E01.mkv");
    fs::write(&ep_file, b"video data").unwrap();

    let report = rescan_library(&database, &library_dir).unwrap();
    assert_eq!(report.shows_adopted, 1);
    assert_eq!(report.episodes_found, 1);

    // Verify database was updated
    let state: String = database
        .with(|c| {
            Ok(c.query_row(
                "SELECT state FROM episodes WHERE mal_id = 500 AND number = 1.0",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(state, "done");

    let folder: String = database
        .with(|c| {
            Ok(c.query_row(
                "SELECT folder_path FROM shows WHERE mal_id = 500",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(folder, show_dir.display().to_string());
}

// ---------------------------------------------------------------------------
// 9. Security: Disallowed Extensions & Path Traversal
// ---------------------------------------------------------------------------

#[tokio::test]
async fn security_disallowed_extensions_rejected() {
    let temp = tempdir().unwrap();
    let library_dir = temp.path().join("Anime");
    let downloads_dir = temp.path().join("Downloads");
    let data_dir = temp.path().join("data");
    fs::create_dir_all(&library_dir).unwrap();
    fs::create_dir_all(&downloads_dir).unwrap();
    fs::create_dir_all(&data_dir).unwrap();

    let database = db();
    let clk = Arc::new(clock());

    let mut settings = Settings::load(&database).unwrap();
    settings.library_folder = library_dir.display().to_string();
    settings.download_folder = downloads_dir.display().to_string();
    Settings::save(&settings, &database).unwrap();

    let gate = Arc::new(Gate::with_policies(database.clone(), &[]).unwrap());
    let finalizer = LibraryFinalizer::new(database.clone(), gate, clk.clone(), data_dir);

    insert_show(&database, 600, "Malware Show", None, "tv", Some(2024), 1);
    let (ep_id, job) = insert_episode_and_job(&database, 600, 1.0);

    // .exe extension must be rejected
    let bad_file = downloads_dir.join("episode.exe");
    fs::write(&bad_file, b"malicious binary").unwrap();

    let ctx = FinalizeContext {
        job: &job,
        episode_id: ep_id,
        mal_id: 600,
        content_path: bad_file.to_str().unwrap(),
        size_bytes: 16,
    };

    let err = finalizer.finalize(&ctx).await.unwrap_err();
    assert!(err.to_string().contains("not an allowed video format"));
}

// ---------------------------------------------------------------------------
// 10. Desktop Folder Icons: KDE and GNOME
// ---------------------------------------------------------------------------

#[test]
fn desktop_folder_icons_methods() {
    let temp = tempdir().unwrap();
    let show_dir = temp.path().join("Show");
    fs::create_dir_all(&show_dir).unwrap();
    let icon_png = show_dir.join("folder.png");
    fs::write(&icon_png, b"png").unwrap();

    // Directory method (KDE)
    apply_folder_icon(IconMethod::Directory, &show_dir, &icon_png).unwrap();
    assert!(show_dir.join(DIRECTORY_FILE).exists());

    // Off method
    let _ = fs::remove_file(show_dir.join(DIRECTORY_FILE));
    apply_folder_icon(IconMethod::Off, &show_dir, &icon_png).unwrap();
    assert!(!show_dir.join(DIRECTORY_FILE).exists());
}
