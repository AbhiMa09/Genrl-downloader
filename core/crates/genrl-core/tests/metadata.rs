//! Metadata ingestion and sync tests:
//! 1. `sync_queues_metadata_jobs` — sync marks shows missing synopsis/poster as pending.
//! 2. `metadata_429_placeholder_and_retry_fills_it` — 429 sets placeholder + backoff, subsequent retry fills it.
//! 3. `cache_hit_within_7_days_makes_no_request` — fresh metadata returns cached without network calls.
//! 4. `control_characters_stripped` — cleans escape codes, nulls, and control characters into plain text.
//! 5. `fallback_to_mal_when_jikan_is_empty` — falls back to MAL API when Jikan has no synopsis.

mod common;

use tempfile::tempdir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use genrl_core::clock::{Clock, FakeClock};
use genrl_core::db::Db;
use genrl_core::http::policy::{RateSpec, RetryPolicy, ServicePolicy};
use genrl_core::http::{Gate, Service};
use genrl_core::metadata::{
    clean_plain_text, clean_single_line, fetch_show_metadata, process_metadata_queue,
    queue_stale_metadata, MetadataOutcome,
};
use genrl_core::settings::Settings;

const NOW: i64 = 1_760_000_000;

fn clock() -> FakeClock {
    FakeClock::new(NOW)
}

fn fast_policy() -> ServicePolicy {
    ServicePolicy {
        rate: RateSpec {
            permits_per_second: 100.0,
            burst: 100,
            per_minute_cap: None,
            jitter: false,
        },
        cache_ttl: None,
        max_body_bytes: 5 * 1024 * 1024,
        timeout: std::time::Duration::from_secs(5),
        retry: RetryPolicy {
            max_attempts: 1,
            schedule: vec![],
            max_total_wait: std::time::Duration::from_millis(50),
            max_delay: std::time::Duration::from_millis(50),
        },
        breaker_threshold: 10,
        breaker_cooldown: std::time::Duration::from_secs(60),
    }
}

// ---------------------------------------------------------------------------
// 1. Sync queues metadata jobs
// ---------------------------------------------------------------------------

#[test]
fn sync_queues_metadata_jobs() {
    let db = Db::open_in_memory().unwrap();
    let clk = clock();

    // Insert 3 shows:
    // Show 1: Missing synopsis and poster -> should be queued
    // Show 2: Has synopsis and poster, updated 2 days ago -> fresh, should NOT be queued
    // Show 3: Has synopsis, but updated 10 days ago (> 7 days) -> stale, should be queued
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, metadata_status, metadata_updated_at)
             VALUES (1, 'Show One', 'pending', NULL)",
            [],
        )?;
        c.execute(
            "INSERT INTO shows (mal_id, title, synopsis, poster_path, metadata_status, metadata_updated_at)
             VALUES (2, 'Show Two', 'Fresh synopsis', '/tmp/posters/2.jpg', 'ready', ?1)",
            [NOW - (2 * 86400)],
        )?;
        c.execute(
            "INSERT INTO shows (mal_id, title, synopsis, poster_path, metadata_status, metadata_updated_at)
             VALUES (3, 'Show Three', 'Old synopsis', '/tmp/posters/3.jpg', 'ready', ?1)",
            [NOW - (10 * 86400)],
        )?;
        Ok(())
    })
    .unwrap();

    let queued = queue_stale_metadata(&db, clk.now()).unwrap();
    assert_eq!(
        queued, 2,
        "Expected 2 shows to be queued for metadata refresh"
    );

    // Verify Show 1 and Show 3 are marked pending with next_run_at = NOW
    let statuses: Vec<(i64, String, Option<i64>)> = db
        .with(|c| {
            let mut stmt = c.prepare(
                "SELECT mal_id, metadata_status, metadata_next_retry_at FROM shows ORDER BY mal_id",
            )?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            Ok(rows.filter_map(Result::ok).collect())
        })
        .unwrap();

    assert_eq!(statuses[0], (1, "pending".into(), Some(NOW)));
    assert_eq!(statuses[1], (2, "ready".into(), None));
    assert_eq!(statuses[2], (3, "pending".into(), Some(NOW)));
}

// ---------------------------------------------------------------------------
// 2. 429 -> placeholder -> retry fills it
// ---------------------------------------------------------------------------

#[tokio::test]
async fn metadata_429_placeholder_and_retry_fills_it() {
    let server = MockServer::start().await;
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    let db = Db::open_in_memory().unwrap();
    let clk = clock();

    let mut settings = Settings::load(&db).unwrap();
    settings.jikan_base_url = server.uri();
    Settings::save(&settings, &db).unwrap();

    let gate = Gate::with_policies(
        db.clone(),
        &[
            (Service::Jikan, fast_policy()),
            (Service::Images, fast_policy()),
        ],
    )
    .unwrap();

    let show_folder = temp.path().join("shows").join("Test Show 100");
    std::fs::create_dir_all(&show_folder).unwrap();

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, metadata_status, folder_path) VALUES (100, 'Test Show 100', 'pending', ?1)",
            [show_folder.to_str().unwrap()],
        )?;
        Ok(())
    })
    .unwrap();

    // 1. Server returns 429 Too Many Requests
    Mock::given(method("GET"))
        .and(path("/v4/anime/100/full"))
        .respond_with(ResponseTemplate::new(429).set_body_string("Too Many Requests"))
        .expect(1)
        .mount(&server)
        .await;

    let outcome = fetch_show_metadata(&db, &gate, &settings, &clk, &data_dir, 100, true)
        .await
        .unwrap();

    match outcome {
        MetadataOutcome::PendingRetry {
            mal_id,
            next_run_at,
            ..
        } => {
            assert_eq!(mal_id, 100);
            assert!(
                next_run_at > NOW,
                "next_run_at should be in the future (backoff)"
            );
        }
        other => panic!("expected PendingRetry, got {other:?}"),
    }

    // Verify NULL synopsis was kept in SQLite, status is pending, and error stored in metadata_error
    let (syn, status, retry_cnt, err): (Option<String>, String, i64, Option<String>) = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT synopsis, metadata_status, metadata_retry_count, metadata_error FROM shows WHERE mal_id = 100",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?)
        })
        .unwrap();

    assert!(syn.is_none(), "shows.synopsis must remain NULL on failure");
    assert_eq!(status, "pending");
    assert_eq!(retry_cnt, 1);
    assert!(err.is_some(), "metadata_error must hold failure details");

    // Verify synopsis.txt gets placeholder line
    let syn_txt = std::fs::read_to_string(show_folder.join("synopsis.txt")).unwrap();
    assert_eq!(syn_txt, "[Synopsis pending metadata sync for show #100]\n");

    // 2. Server recovers and returns 200 OK with valid Jikan JSON
    server.reset().await;
    let valid_jikan = r#"{
        "data": {
            "mal_id": 100,
            "title": "Test Show 100",
            "title_english": "English 100",
            "title_japanese": "テスト100",
            "synopsis": "This is the real recovered synopsis from Jikan.",
            "score": 8.75,
            "type": "TV",
            "episodes": 12,
            "status": "Finished Airing",
            "year": 2024,
            "images": {
                "jpg": {
                    "large_image_url": null,
                    "image_url": null
                }
            }
        }
    }"#;

    Mock::given(method("GET"))
        .and(path("/v4/anime/100/full"))
        .respond_with(ResponseTemplate::new(200).set_body_string(valid_jikan))
        .expect(1)
        .mount(&server)
        .await;

    // Retry fetch with force (simulating retry button)
    let retry_outcome = fetch_show_metadata(&db, &gate, &settings, &clk, &data_dir, 100, true)
        .await
        .unwrap();

    match retry_outcome {
        MetadataOutcome::Updated {
            mal_id, synopsis, ..
        } => {
            assert_eq!(mal_id, 100);
            assert_eq!(synopsis, "This is the real recovered synopsis from Jikan.");
        }
        other => panic!("expected Updated after retry, got {other:?}"),
    }

    // Verify database now contains real synopsis, ready status, and cleared error
    let (final_syn, final_status, final_retry, final_err): (Option<String>, String, i64, Option<String>) = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT synopsis, metadata_status, metadata_retry_count, metadata_error FROM shows WHERE mal_id = 100",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?)
        })
        .unwrap();

    assert_eq!(
        final_syn.as_deref(),
        Some("This is the real recovered synopsis from Jikan.")
    );
    assert_eq!(final_status, "ready");
    assert_eq!(final_retry, 0);
    assert!(final_err.is_none());
    assert!(
        genrl_core::settings::read_state_time(&db, "shows_changed_at")
            .unwrap()
            .is_some(),
        "shows_changed_at must be recorded on Jikan metadata update"
    );

    // Verify synopsis.txt is cleanly replaced with real synopsis
    let syn_txt_after = std::fs::read_to_string(show_folder.join("synopsis.txt")).unwrap();
    assert!(syn_txt_after.contains("This is the real recovered synopsis from Jikan."));
    assert!(!syn_txt_after.starts_with('['));
}

// ---------------------------------------------------------------------------
// 3. Cache hit within 7 days makes no request
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cache_hit_within_7_days_makes_no_request() {
    let server = MockServer::start().await;
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("data");

    let db = Db::open_in_memory().unwrap();
    let clk = clock();

    let mut settings = Settings::load(&db).unwrap();
    settings.jikan_base_url = server.uri();
    Settings::save(&settings, &db).unwrap();

    let gate = Gate::with_policies(db.clone(), &[(Service::Jikan, fast_policy())]).unwrap();

    let poster_file = temp.path().join("posters").join("200.jpg");
    std::fs::create_dir_all(poster_file.parent().unwrap()).unwrap();
    std::fs::write(&poster_file, b"fake-jpg-data").unwrap();

    // Show was updated 3 days ago (< 7 days) and has synopsis + local poster file
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, synopsis, poster_path, metadata_status, metadata_updated_at)
             VALUES (200, 'Cached Show', 'Existing fresh synopsis', ?1, 'ready', ?2)",
            rusqlite::params![poster_file.to_str().unwrap(), NOW - (3 * 86400)],
        )?;
        Ok(())
    })
    .unwrap();

    // No mocks mounted on MockServer: any HTTP request would fail with 404
    let outcome = fetch_show_metadata(&db, &gate, &settings, &clk, &data_dir, 200, false)
        .await
        .unwrap();

    assert_eq!(outcome, MetadataOutcome::Cached { mal_id: 200 });

    // Verify 0 requests made to server
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// 4. Control characters stripped
// ---------------------------------------------------------------------------

#[test]
fn control_characters_stripped() {
    // String with ANSI color escape codes, NULL bytes, BEL, DEL, and zero-width spaces
    let dirty = "\x1b[31;1mError in feed!\x1b[0m\x00\x07\x08A young boy meets a dragon.\x7f\u{200b}\n\nChapter 1:\tThe beginning.\r\n";
    let cleaned = clean_plain_text(dirty);

    assert_eq!(
        cleaned,
        "Error in feed!A young boy meets a dragon.\n\nChapter 1:\tThe beginning."
    );

    // Single line cleaning replaces newlines and tabs with spaces and collapses whitespace
    let single =
        clean_single_line("Title with \x1b[34mANSI\x1b[0m\nand \r\n multiple   spaces\t\x00!");
    assert_eq!(single, "Title with ANSI and multiple spaces !");
}

// ---------------------------------------------------------------------------
// 5. Fallback to MAL when Jikan is empty
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fallback_to_mal_when_jikan_is_empty() {
    let server = MockServer::start().await;
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    let db = Db::open_in_memory().unwrap();
    let clk = clock();

    let mut settings = Settings::load(&db).unwrap();
    settings.jikan_base_url = server.uri();
    settings.mal_base_url = format!("{}/v2", server.uri());
    Settings::save(&settings, &db).unwrap();

    // MOCK OAuth store and endpoint; no SQLite token or real credentials.
    settings.mal_client_id = "fixture-client".into();
    let store = genrl_core::mal::MemoryStore::with("fixture-refresh");
    Mock::given(method("POST"))
        .and(path("/v2/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mock_access_token_123", "expires_in": 3600
        })))
        .expect(1)
        .mount(&server)
        .await;

    let gate = Gate::with_policies(
        db.clone(),
        &[
            (Service::Jikan, fast_policy()),
            (Service::Mal, fast_policy()),
            (Service::Images, fast_policy()),
        ],
    )
    .unwrap();

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, metadata_status) VALUES (300, 'Show 300', 'pending')",
            [],
        )?;
        Ok(())
    })
    .unwrap();

    // 1. Jikan returns empty synopsis
    let empty_jikan = r#"{
        "data": {
            "mal_id": 300,
            "title": "Show 300",
            "synopsis": null
        }
    }"#;
    Mock::given(method("GET"))
        .and(path("/v4/anime/300/full"))
        .respond_with(ResponseTemplate::new(200).set_body_string(empty_jikan))
        .mount(&server)
        .await;

    // 2. MAL API returns synopsis
    let mal_resp = r#"{
        "id": 300,
        "title": "Show 300",
        "synopsis": "Official synopsis fetched directly from MyAnimeList API.",
        "mean": 8.12,
        "media_type": "tv",
        "num_episodes": 24,
        "alternative_titles": {
            "en": "Show Three Hundred",
            "ja": "三百"
        }
    }"#;
    Mock::given(method("GET"))
        .and(path("/v2/anime/300"))
        .and(header("Authorization", "Bearer mock_access_token_123"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mal_resp))
        .expect(1)
        .mount(&server)
        .await;

    let client = genrl_core::mal::MalClient::new(&gate, settings.clone(), &store, &clk);
    let outcome = genrl_core::metadata::fetch_show_metadata_with_client(
        &db, &gate, &settings, &clk, &data_dir, 300, true, &client,
    )
    .await
    .unwrap();

    match outcome {
        MetadataOutcome::Updated {
            mal_id, synopsis, ..
        } => {
            assert_eq!(mal_id, 300);
            assert_eq!(
                synopsis,
                "Official synopsis fetched directly from MyAnimeList API."
            );
        }
        other => panic!("expected Updated from MAL API, got {other:?}"),
    }

    // Verify stored in SQLite as ready
    let (syn, status, en_title): (String, String, Option<String>) = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT synopsis, metadata_status, english_title FROM shows WHERE mal_id = 300",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?)
        })
        .unwrap();

    assert_eq!(
        syn,
        "Official synopsis fetched directly from MyAnimeList API."
    );
    assert_eq!(status, "partial");
    assert_eq!(en_title.as_deref(), Some("Show Three Hundred"));
}

#[tokio::test]
async fn download_season_fetches_metadata_via_mal_fallback_before_queueing() {
    let server = MockServer::start().await;
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    let db = Db::open_in_memory().unwrap();
    let clk = clock();

    let mut settings = Settings::load(&db).unwrap();
    settings.jikan_base_url = server.uri();
    settings.mal_base_url = format!("{}/v2", server.uri());
    settings.mal_client_id = "fixture-client".into();
    Settings::save(&settings, &db).unwrap();

    let store = genrl_core::mal::MemoryStore::with("fixture-refresh");
    Mock::given(method("POST"))
        .and(path("/v2/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mock_token_301", "expires_in": 3600
        })))
        .mount(&server)
        .await;

    let gate = Gate::with_policies(
        db.clone(),
        &[
            (Service::Jikan, fast_policy()),
            (Service::Mal, fast_policy()),
            (Service::Images, fast_policy()),
        ],
    )
    .unwrap();

    // 1. Show starts with pending metadata
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, metadata_status) VALUES (301, 'Oni no Hanayome', 'pending')",
            [],
        )?;
        Ok(())
    })
    .unwrap();

    // 2. Planning before metadata resolution refuses with "Metadata unresolved"
    let err = genrl_core::planner::plan_single(&db, &clk, 301, None).unwrap_err();
    assert!(err.to_string().contains("Metadata unresolved"));

    // 3. Jikan 404 / unavailable, MAL API provides show metadata
    Mock::given(method("GET"))
        .and(path("/v4/anime/301/full"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let mal_resp = r#"{
        "id": 301,
        "title": "Oni no Hanayome",
        "synopsis": "Official synopsis from MyAnimeList.",
        "mean": 8.0,
        "media_type": "tv",
        "num_episodes": 12,
        "alternative_titles": {
            "en": "The Ogre's Bride",
            "ja": "鬼の花嫁"
        }
    }"#;
    Mock::given(method("GET"))
        .and(path("/v2/anime/301"))
        .and(header("Authorization", "Bearer mock_token_301"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mal_resp))
        .mount(&server)
        .await;

    // 4. Fallback fetch resolves show metadata
    let client = genrl_core::mal::MalClient::new(&gate, settings.clone(), &store, &clk);
    let outcome = genrl_core::metadata::fetch_show_metadata_with_client(
        &db, &gate, &settings, &clk, &data_dir, 301, true, &client,
    )
    .await
    .unwrap();
    assert!(matches!(outcome, MetadataOutcome::Updated { .. }));

    // Verify metadata status transitioned to ready/partial
    assert!(genrl_core::metadata::is_resolved(&db, 301).unwrap());

    // 5. Download season now succeeds and queues all 12 episodes
    let queued = genrl_core::planner::plan_single(&db, &clk, 301, None).unwrap();
    assert_eq!(queued, 12, "Queues 12 episodes after metadata resolved");

    let count: i64 = db
        .with(|c| {
            Ok(
                c.query_row("SELECT count(*) FROM jobs WHERE kind='search'", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .unwrap();
    assert_eq!(count, 12, "12 search jobs in database");
}

// ---------------------------------------------------------------------------
// 6. 404 permanent failure not retried and marked failed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn permanent_404_not_retried_and_marked_failed() {
    let server = MockServer::start().await;
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let show_folder = temp.path().join("shows").join("Show 404");
    std::fs::create_dir_all(&show_folder).unwrap();

    let db = Db::open_in_memory().unwrap();
    let clk = clock();

    let mut settings = Settings::load(&db).unwrap();
    settings.jikan_base_url = server.uri();
    Settings::save(&settings, &db).unwrap();

    let gate = Gate::with_policies(db.clone(), &[(Service::Jikan, fast_policy())]).unwrap();

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, metadata_status, folder_path) VALUES (404, 'Show 404', 'pending', ?1)",
            [show_folder.to_str().unwrap()],
        )?;
        Ok(())
    })
    .unwrap();

    // Jikan returns 404 Not Found
    Mock::given(method("GET"))
        .and(path("/v4/anime/404/full"))
        .respond_with(ResponseTemplate::new(404).set_body_string(
            r#"{"status":404,"type":"BadResponseException","message":"Resource not found"}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let outcome = fetch_show_metadata(&db, &gate, &settings, &clk, &data_dir, 404, true)
        .await
        .unwrap();

    match outcome {
        MetadataOutcome::Failed { mal_id, error } => {
            assert_eq!(mal_id, 404);
            assert!(error.contains("404"), "error should mention 404: {error}");
        }
        other => panic!("expected Failed, got {other:?}"),
    }

    // Verify DB state: synopsis is NULL, status is 'failed', metadata_next_retry_at is NULL
    let (syn, status, next_retry, err): (Option<String>, String, Option<i64>, Option<String>) = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT synopsis, metadata_status, metadata_next_retry_at, metadata_error FROM shows WHERE mal_id = 404",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?)
        })
        .unwrap();

    assert!(syn.is_none(), "synopsis must be NULL on failure");
    assert_eq!(status, "failed");
    assert!(
        next_retry.is_none(),
        "404 must NOT be retried (next_retry_at is NULL)"
    );
    assert!(err.as_deref().unwrap_or("").contains("404"));

    // Verify synopsis.txt contains 404 placeholder line
    let syn_txt = std::fs::read_to_string(show_folder.join("synopsis.txt")).unwrap();
    assert!(syn_txt.contains("404 Not Found"));

    // Verify queue_stale_metadata does NOT queue 'failed' shows
    let queued = queue_stale_metadata(&db, clk.now()).unwrap();
    assert_eq!(queued, 0, "failed shows should never be re-queued");
}

// ---------------------------------------------------------------------------
// 7. Remote poster URL is re-downloaded to local file
// ---------------------------------------------------------------------------

#[tokio::test]
async fn remote_poster_url_is_redownloaded() {
    let server = MockServer::start().await;
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    let db = Db::open_in_memory().unwrap();
    let clk = clock();

    let mut settings = Settings::load(&db).unwrap();
    settings.jikan_base_url = server.uri();
    Settings::save(&settings, &db).unwrap();

    let gate = Gate::with_policies(
        db.clone(),
        &[
            (Service::Jikan, fast_policy()),
            (Service::Images, fast_policy()),
        ],
    )
    .unwrap();

    let remote_poster_url = format!("{}/images/500.jpg", server.uri());

    // Show currently has a remote poster URL instead of a local file path
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, synopsis, poster_path, metadata_status, metadata_updated_at)
             VALUES (500, 'Show 500', 'Existing synopsis', ?1, 'ready', ?2)",
            rusqlite::params![&remote_poster_url, NOW - 3600],
        )?;
        Ok(())
    })
    .unwrap();

    // queue_stale_metadata must detect remote poster_path as needing refresh
    let queued = queue_stale_metadata(&db, clk.now()).unwrap();
    assert_eq!(queued, 1, "remote poster URLs must be queued for download");

    let jikan_json = format!(
        r#"{{
        "data": {{
            "mal_id": 500,
            "title": "Show 500",
            "synopsis": "Existing synopsis",
            "images": {{
                "jpg": {{
                    "large_image_url": "{}",
                    "image_url": "{}"
                }}
            }}
        }}
    }}"#,
        remote_poster_url, remote_poster_url
    );

    Mock::given(method("GET"))
        .and(path("/v4/anime/500/full"))
        .respond_with(ResponseTemplate::new(200).set_body_string(jikan_json))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/images/500.jpg"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"downloaded-poster-bytes"))
        .expect(1)
        .mount(&server)
        .await;

    let outcome = fetch_show_metadata(&db, &gate, &settings, &clk, &data_dir, 500, true)
        .await
        .unwrap();

    match outcome {
        MetadataOutcome::Updated {
            mal_id,
            poster_path,
            ..
        } => {
            assert_eq!(mal_id, 500);
            assert!(poster_path.is_some());
            let local_path = poster_path.unwrap();
            assert!(
                !local_path.starts_with("http"),
                "poster_path must be a local file path"
            );
            assert!(
                std::path::Path::new(&local_path).is_file(),
                "poster file must exist on disk"
            );
        }
        other => panic!("expected Updated, got {other:?}"),
    }

    let db_poster: String = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT poster_path FROM shows WHERE mal_id = 500",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();

    assert!(!db_poster.starts_with("http"));
    assert!(std::path::Path::new(&db_poster).is_file());
}

// ---------------------------------------------------------------------------
// 8. Sync queues Watching shows first, then the rest at low priority
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sync_queues_watching_shows_with_priority() {
    let server = MockServer::start().await;
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    let db = Db::open_in_memory().unwrap();
    let clk = clock();

    let mut settings = Settings::load(&db).unwrap();
    settings.jikan_base_url = server.uri();
    Settings::save(&settings, &db).unwrap();

    let gate = Gate::with_policies(db.clone(), &[(Service::Jikan, fast_policy())]).unwrap();

    // Populate shows and mal_entries:
    // Show 10: plan_to_watch
    // Show 20: watching
    // Show 30: completed
    // Show 40: watching
    db.with(|c| {
        c.execute("INSERT INTO shows (mal_id, title, metadata_status) VALUES (10, 'Show 10', 'pending')", [])?;
        c.execute("INSERT INTO mal_entries (mal_id, list_status, updated_at) VALUES (10, 'plan_to_watch', ?1)", [NOW])?;

        c.execute("INSERT INTO shows (mal_id, title, metadata_status) VALUES (20, 'Show 20', 'pending')", [])?;
        c.execute("INSERT INTO mal_entries (mal_id, list_status, updated_at) VALUES (20, 'watching', ?1)", [NOW])?;

        c.execute("INSERT INTO shows (mal_id, title, metadata_status) VALUES (30, 'Show 30', 'pending')", [])?;
        c.execute("INSERT INTO mal_entries (mal_id, list_status, updated_at) VALUES (30, 'completed', ?1)", [NOW])?;

        c.execute("INSERT INTO shows (mal_id, title, metadata_status) VALUES (40, 'Show 40', 'pending')", [])?;
        c.execute("INSERT INTO mal_entries (mal_id, list_status, updated_at) VALUES (40, 'watching', ?1)", [NOW])?;
        Ok(())
    })
    .unwrap();

    for id in [20, 40] {
        let resp = format!(
            r#"{{"data":{{"mal_id":{id},"title":"Show {id}","synopsis":"Synopsis for {id}"}}}}"#
        );
        Mock::given(method("GET"))
            .and(path(format!("/v4/anime/{id}/full")))
            .respond_with(ResponseTemplate::new(200).set_body_string(resp))
            .expect(1)
            .mount(&server)
            .await;
    }

    // Process a batch of limit 2: only the 2 Watching shows (20 and 40) must be processed first!
    let report = process_metadata_queue(&db, &gate, &settings, &clk, &data_dir, Some(2))
        .await
        .unwrap();

    assert_eq!(report.processed, 2);
    assert_eq!(report.updated, 2);

    // Verify Shows 20 and 40 are now ready, while 10 and 30 remain pending
    let statuses: Vec<(i64, String)> = db
        .with(|c| {
            let mut stmt =
                c.prepare("SELECT mal_id, metadata_status FROM shows ORDER BY mal_id")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            Ok(rows.filter_map(Result::ok).collect())
        })
        .unwrap();

    assert_eq!(
        statuses,
        vec![
            (10, "pending".into()),
            (20, "ready".into()),
            (30, "pending".into()),
            (40, "ready".into()),
        ]
    );
}

// ---------------------------------------------------------------------------
// 9. Per-minute cap respected with a fake clock
// ---------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn jikan_per_minute_cap_respected_with_fake_clock() {
    let jikan_policy = ServicePolicy::for_service(Service::Jikan);
    assert_eq!(jikan_policy.rate.per_minute_cap, Some(50));
    assert_eq!(jikan_policy.rate.permits_per_second, 2.0);

    // Rate spec with 50/minute cap
    let spec = RateSpec {
        permits_per_second: 1000.0,
        burst: 1000,
        per_minute_cap: Some(50),
        jitter: false,
    };
    let limiter = genrl_core::http::limiter::Limiter::new(&spec);

    let started = tokio::time::Instant::now();
    for _ in 0..50 {
        limiter.acquire().await;
    }
    // All 50 initial tokens were consumed instantly
    assert_eq!(started.elapsed(), std::time::Duration::ZERO);

    // The 51st request exceeds the minute cap: must wait for the minute bucket to refill (50/60 tokens/sec = 1.2s per token)
    limiter.acquire().await;
    let elapsed = started.elapsed();
    assert!(
        elapsed >= std::time::Duration::from_millis(1150),
        "51st permit must wait ~1.2s for minute bucket to refill, waited {elapsed:?}"
    );
}

// ---------------------------------------------------------------------------
// 10. Placeholder synopsis & NFO replaced for all existing folders when metadata arrives
// ---------------------------------------------------------------------------

#[tokio::test]
async fn placeholder_synopsis_and_nfo_replaced_for_all_existing_folders_when_metadata_arrives() {
    let server = MockServer::start().await;
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    let db = Db::open_in_memory().unwrap();
    let clk = clock();

    let library_dir = temp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let mut settings = Settings::load(&db).unwrap();
    settings.jikan_base_url = server.uri();
    settings.library_folder = library_dir.to_str().unwrap().to_string();
    Settings::save(&settings, &db).unwrap();

    let gate = Gate::with_policies(
        db.clone(),
        &[
            (Service::Jikan, fast_policy()),
            (Service::Images, fast_policy()),
        ],
    )
    .unwrap();

    let mal_id = 500;
    // Create folder 1: show's folder_path
    let show_folder1 = library_dir.join("Test Movie (2024)");
    std::fs::create_dir_all(&show_folder1).unwrap();
    std::fs::write(show_folder1.join(".mal_id"), format!("{mal_id}\n")).unwrap();
    std::fs::write(
        show_folder1.join("synopsis.txt"),
        format!("[Synopsis pending metadata sync for show #{mal_id}]\n"),
    )
    .unwrap();
    std::fs::write(
        show_folder1.join("movie.nfo"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<movie>\n  <title>Test Movie</title>\n  <originaltitle>Test Movie</originaltitle>\n  <plot></plot>\n  <uniqueid type=\"mal\" default=\"true\">{mal_id}</uniqueid>\n</movie>\n"
        ),
    )
    .unwrap();

    // Create folder 2: an archive folder referenced by an episode file
    let show_folder2 = temp.path().join("archive").join("Test Movie Extra");
    std::fs::create_dir_all(&show_folder2).unwrap();
    std::fs::write(show_folder2.join(".mal_id"), format!("{mal_id}\n")).unwrap();
    std::fs::write(
        show_folder2.join("synopsis.txt"),
        format!("[Synopsis pending metadata sync for show #{mal_id}]\n"),
    )
    .unwrap();
    std::fs::write(
        show_folder2.join("movie.nfo"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<movie>\n  <title>Test Movie</title>\n  <originaltitle>Test Movie</originaltitle>\n  <plot></plot>\n  <uniqueid type=\"mal\" default=\"true\">{mal_id}</uniqueid>\n</movie>\n"
        ),
    )
    .unwrap();
    let ep2_file = show_folder2.join("Test Movie Extra.mkv");
    std::fs::write(&ep2_file, b"dummy").unwrap();

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, type, start_year, metadata_status, folder_path)
             VALUES (?1, 'Test Movie', 'movie', 2024, 'pending', ?2)",
            rusqlite::params![mal_id, show_folder1.to_str().unwrap()],
        )?;
        c.execute(
            "INSERT INTO episodes (mal_id, number, state, file_path)
             VALUES (?1, 1, 'done', ?2)",
            rusqlite::params![mal_id, ep2_file.to_str().unwrap()],
        )?;
        Ok(())
    })
    .unwrap();

    // Verify initial state: both folders have the 49-byte placeholder and empty plot
    assert_eq!(
        std::fs::read_to_string(show_folder1.join("synopsis.txt")).unwrap(),
        format!("[Synopsis pending metadata sync for show #{mal_id}]\n")
    );
    assert_eq!(
        std::fs::read_to_string(show_folder2.join("synopsis.txt")).unwrap(),
        format!("[Synopsis pending metadata sync for show #{mal_id}]\n")
    );

    // Mock Jikan response with full metadata
    let jikan_body = format!(
        r#"{{
            "data": {{
                "mal_id": {mal_id},
                "title": "Test Movie",
                "title_english": "Test Movie English",
                "title_japanese": "テストムービー",
                "synopsis": "This is the real recovered movie synopsis from Jikan with rich detail.",
                "score": 8.95,
                "type": "Movie",
                "episodes": 1,
                "status": "Finished Airing",
                "year": 2024
            }}
        }}"#
    );

    Mock::given(method("GET"))
        .and(path(format!("/v4/anime/{mal_id}/full")))
        .respond_with(ResponseTemplate::new(200).set_body_string(jikan_body))
        .expect(1)
        .mount(&server)
        .await;

    // Fetch metadata
    let outcome = fetch_show_metadata(&db, &gate, &settings, &clk, &data_dir, mal_id, true)
        .await
        .unwrap();

    match outcome {
        MetadataOutcome::Updated {
            mal_id: id,
            synopsis,
            ..
        } => {
            assert_eq!(id, mal_id);
            assert!(synopsis.contains("This is the real recovered movie synopsis"));
        }
        other => panic!("expected Updated outcome, got {other:?}"),
    }

    // Verify folder 1: synopsis.txt AND movie.nfo replaced with real values
    let syn1 = std::fs::read_to_string(show_folder1.join("synopsis.txt")).unwrap();
    assert!(syn1.contains("This is the real recovered movie synopsis"));
    assert!(!syn1.starts_with('['));

    let nfo1 = std::fs::read_to_string(show_folder1.join("movie.nfo")).unwrap();
    assert!(nfo1.contains(
        "<plot>This is the real recovered movie synopsis from Jikan with rich detail.</plot>"
    ));
    assert!(nfo1.contains("<rating>8.95</rating>"));
    assert!(nfo1.contains("<year>2024</year>"));

    // Verify folder 2: also replaced with real values!
    let syn2 = std::fs::read_to_string(show_folder2.join("synopsis.txt")).unwrap();
    assert!(syn2.contains("This is the real recovered movie synopsis"));
    assert!(!syn2.starts_with('['));

    let nfo2 = std::fs::read_to_string(show_folder2.join("movie.nfo")).unwrap();
    assert!(nfo2.contains(
        "<plot>This is the real recovered movie synopsis from Jikan with rich detail.</plot>"
    ));
    assert!(nfo2.contains("<rating>8.95</rating>"));
    assert!(nfo2.contains("<year>2024</year>"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_workers_claim_disjoint_metadata_jobs() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("genrl.db");

    // Open connection 1, run migrations and insert test shows
    let db1 = Db::open(&db_path).unwrap();
    let clk = clock();
    let now = clk.now();

    // Insert 20 pending shows, 1 stale fetching show (>5 min ago), and 1 active fetching show
    db1.with(|c| {
        for id in 1..=20 {
            c.execute(
                "INSERT INTO shows (mal_id, title, metadata_status) VALUES (?1, ?2, 'pending')",
                rusqlite::params![id, format!("Show {id}")],
            )?;
        }
        c.execute(
            "INSERT INTO shows (mal_id, title, metadata_status, metadata_updated_at) VALUES (21, 'Stale Show', 'fetching', ?1)",
            rusqlite::params![now - 400],
        )?;
        c.execute(
            "INSERT INTO shows (mal_id, title, metadata_status, metadata_updated_at) VALUES (22, 'Active Show', 'fetching', ?1)",
            rusqlite::params![now - 50],
        )?;
        Ok(())
    })
    .unwrap();

    // Open connection 2 to the exact same on-disk database
    let db2 = Db::open(&db_path).unwrap();

    // Worker 1 claims jobs using connection 1
    let handle1 = tokio::spawn(async move {
        let mut claimed = Vec::new();
        while let Ok(Some(id)) = genrl_core::metadata::claim_next_metadata_job(&db1, now) {
            claimed.push(id);
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        claimed
    });

    // Worker 2 claims jobs using separate connection 2
    let handle2 = tokio::spawn(async move {
        let mut claimed = Vec::new();
        while let Ok(Some(id)) = genrl_core::metadata::claim_next_metadata_job(&db2, now) {
            claimed.push(id);
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        claimed
    });

    let (claimed1, claimed2) = tokio::join!(handle1, handle2);
    let list1 = claimed1.unwrap();
    let list2 = claimed2.unwrap();

    assert!(!list1.is_empty(), "Worker 1 should have claimed jobs");
    assert!(!list2.is_empty(), "Worker 2 should have claimed jobs");
    assert_eq!(
        list1.len() + list2.len(),
        21,
        "20 pending shows + 1 stale lease show claimed"
    );

    let set1: std::collections::HashSet<i64> = list1.into_iter().collect();
    let set2: std::collections::HashSet<i64> = list2.into_iter().collect();
    let intersection: Vec<_> = set1.intersection(&set2).collect();
    assert!(
        intersection.is_empty(),
        "Workers claimed the same show(s): {intersection:?}"
    );

    // Stale row #21 was reclaimed past its lease
    assert!(
        set1.contains(&21) || set2.contains(&21),
        "Stale fetching show 21 should be reclaimed"
    );
    // Active row #22 was NOT claimed
    assert!(
        !set1.contains(&22) && !set2.contains(&22),
        "Active fetching show 22 should NOT be reclaimed"
    );
}

// MOCK-only regressions for outage handling; no real endpoints or credentials.
#[tokio::test]
async fn two_jikan_timeouts_do_not_open_the_default_breaker() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(150)))
        .expect(2)
        .mount(&server)
        .await;
    let mut policy = ServicePolicy::for_service(Service::Jikan);
    policy.timeout = std::time::Duration::from_millis(20);
    policy.rate = fast_policy().rate;
    let gate =
        Gate::with_policies(Db::open_in_memory().unwrap(), &[(Service::Jikan, policy)]).unwrap();
    for _ in 0..2 {
        assert!(gate
            .get(Service::Jikan, &server.uri())
            .await
            .unwrap_err()
            .is_timeout());
    }
    assert_eq!(
        gate.breaker_state(Service::Jikan),
        genrl_core::http::breaker::State::Closed
    );
}

#[tokio::test]
async fn open_jikan_breaker_leaves_remaining_metadata_unclaimed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    let db = Db::open_in_memory().unwrap();
    db.with(|c| {
        for id in 1..=8 {
            c.execute(
                "INSERT INTO shows (mal_id,title,metadata_status) VALUES (?1,'Fixture','pending')",
                [id],
            )?;
        }
        Ok(())
    })
    .unwrap();
    let settings = Settings {
        jikan_base_url: server.uri(),
        ..Default::default()
    };
    let mut policy = fast_policy();
    policy.breaker_threshold = 1;
    let gate = Gate::with_policies(db.clone(), &[(Service::Jikan, policy)]).unwrap();
    let dir = tempdir().unwrap();
    let report = process_metadata_queue(&db, &gate, &settings, &clock(), dir.path(), Some(8))
        .await
        .unwrap();
    assert_eq!((report.processed, report.failed), (1, 1));
    let untouched: i64 = db
        .with(|c| {
            Ok(c.query_row(
        "SELECT count(*) FROM shows WHERE metadata_status='pending' AND metadata_retry_count=0",
        [], |r| r.get(0))?)
        })
        .unwrap();
    assert_eq!(untouched, 7);
}

#[tokio::test]
async fn metadata_retry_honors_server_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "120"))
        .expect(1)
        .mount(&server)
        .await;
    let db = Db::open_in_memory().unwrap();
    db.with(|c| {
        c.execute("INSERT INTO shows (mal_id,title) VALUES (1,'Fixture')", [])?;
        Ok(())
    })
    .unwrap();
    let settings = Settings {
        jikan_base_url: server.uri(),
        ..Default::default()
    };
    let mut policy = fast_policy();
    policy.breaker_threshold = 1;
    let gate = Gate::with_policies(db.clone(), &[(Service::Jikan, policy)]).unwrap();
    let dir = tempdir().unwrap();
    match fetch_show_metadata(&db, &gate, &settings, &clock(), dir.path(), 1, true)
        .await
        .unwrap()
    {
        MetadataOutcome::PendingRetry { next_run_at, .. } => assert_eq!(next_run_at, NOW + 120),
        other => panic!("expected deferred retry, got {other:?}"),
    }
    let fresh_gate = Gate::with_policies(db.clone(), &[(Service::Jikan, fast_policy())]).unwrap();
    let retry = fetch_show_metadata(&db, &fresh_gate, &settings, &clock(), dir.path(), 1, true)
        .await
        .unwrap();
    assert!(
        matches!(retry, MetadataOutcome::PendingRetry{next_run_at,..} if next_run_at == NOW+120)
    );
}

#[tokio::test]
async fn jikan_timeout_uses_authenticated_mal_partial_metadata_without_sqlite_token() {
    let server = MockServer::start().await;
    let db = Db::open_in_memory().unwrap();
    db.with(|c| {
        c.execute("INSERT INTO shows (mal_id,title) VALUES (1,'Fixture')", [])?;
        Ok(())
    })
    .unwrap();
    let settings = Settings {
        jikan_base_url: server.uri(),
        ..Default::default()
    };
    settings.mal_base_url = server.uri();
    settings.mal_client_id = "fixture-client".into();
    Mock::given(method("GET"))
        .and(path("/v4/anime/1/full"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(150)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token":"fixture-access","expires_in":3600
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/anime/1"))
        .and(header("Authorization", "Bearer fixture-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id":1,"title":"Fixture","num_episodes":12,"status":"currently_airing",
            "start_date":"2024-01-01"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let mut policy = fast_policy();
    policy.timeout = std::time::Duration::from_millis(20);
    let gate = Gate::with_policies(
        db.clone(),
        &[(Service::Jikan, policy), (Service::Mal, fast_policy())],
    )
    .unwrap();
    let store = genrl_core::mal::MemoryStore::with("fixture-refresh");
    let clk = clock();
    let client = genrl_core::mal::MalClient::new(&gate, settings.clone(), &store, &clk);
    let dir = tempdir().unwrap();
    let result = genrl_core::metadata::fetch_show_metadata_with_client(
        &db,
        &gate,
        &settings,
        &clk,
        dir.path(),
        1,
        true,
        &client,
    )
    .await
    .unwrap();
    assert!(matches!(result, MetadataOutcome::Updated { .. }));
    let row: (String, i64, i64) = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT metadata_status,total_episodes,start_year FROM shows WHERE mal_id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?)
        })
        .unwrap();
    assert_eq!(row, ("partial".into(), 12, 2024));
    assert!(
        genrl_core::settings::read_state_time(&db, "shows_changed_at")
            .unwrap()
            .is_some(),
        "shows_changed_at must be recorded on MAL partial metadata update"
    );
    let tokens: i64 = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM settings WHERE key LIKE '%access_token%'",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(tokens, 0);
}

#[test]
fn targeted_download_refuses_unresolved_metadata_and_accepts_partial_metadata() {
    let db = Db::open_in_memory().unwrap();
    db.with(|c| { c.execute(
        "INSERT INTO shows (mal_id,title,total_episodes,metadata_status) VALUES (1,'Fixture',3,'pending')",[])?;
        Ok(()) }).unwrap();
    let error = genrl_core::planner::plan_single(&db, &clock(), 1, None).unwrap_err();
    assert!(error.to_string().contains("Metadata unresolved"));
    let count: i64 = db
        .with(|c| Ok(c.query_row("SELECT count(*) FROM jobs", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(count, 0);
    db.with(|c| {
        c.execute("UPDATE shows SET metadata_status='partial'", [])?;
        Ok(())
    })
    .unwrap();
    assert!(genrl_core::planner::plan_single(&db, &clock(), 1, None).unwrap() > 0);
}

#[test]
fn jikan_cooldown_is_configurable() {
    let db = Db::open_in_memory().unwrap();
    let mut settings = Settings::default();
    settings.jikan_breaker_cooldown_secs = 17;
    settings.save(&db).unwrap();
    let gate = Gate::new(db).unwrap();
    assert_eq!(
        gate.policy(Service::Jikan).breaker_cooldown,
        std::time::Duration::from_secs(17)
    );
    assert_eq!(gate.policy(Service::Jikan).retry.max_attempts, 1);
}

#[tokio::test]
async fn test_scratch_isolation_acquire_never_touches_default_data_dir() {
    use std::sync::Arc;
    let empty_home = tempfile::tempdir().unwrap();
    let scratch_dir = tempfile::tempdir().unwrap();

    // 1. Set environment to empty_home with GENRL_REQUIRE_SCRATCH=1
    std::env::set_var("GENRL_REQUIRE_SCRATCH", "1");
    std::env::set_var("XDG_DATA_HOME", empty_home.path());
    std::env::set_var("HOME", empty_home.path());

    // Verify default_data_dir() panics when GENRL_REQUIRE_SCRATCH=1
    let panic_res = std::panic::catch_unwind(|| {
        genrl_core::default_data_dir();
    });
    assert!(
        panic_res.is_err(),
        "default_data_dir() must panic when GENRL_REQUIRE_SCRATCH=1"
    );

    // 2. Setup scratch database
    let db_path = scratch_dir.path().join("genrl.db");
    let db = Db::open(&db_path).unwrap();
    let settings = Settings::default();
    settings.save(&db).unwrap();

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, total_episodes, metadata_status) VALUES (999, 'Test Scratch Show', 12, 'pending')",
            [],
        )?;
        c.execute(
            "INSERT INTO episodes (mal_id, number, state) VALUES (999, 1.0, 'missing')",
            [],
        )?;
        c.execute(
            "INSERT INTO jobs (episode_id, kind, state, attempts, created_at, updated_at) VALUES (1, 'search', 'search', 0, 1000, 1000)",
            [],
        )?;
        Ok(())
    })
    .unwrap();

    let clock = Arc::new(FakeClock::new(1000));
    let gate = Arc::new(Gate::new(db.clone()).unwrap());

    let engine = genrl_core::worker::Engine::new(
        db.clone(),
        gate,
        clock,
        Arc::new(genrl_core::worker::NoopFinalizer),
        Arc::new(genrl_core::worker::FakeDiskProbe::new(
            100 * 1024 * 1024 * 1024,
        )),
        scratch_dir.path().to_path_buf(),
    );

    // Run search step on unresolved metadata show
    let outcome = engine.step(1).await;
    assert!(outcome.is_ok(), "step execution should complete");

    // 3. Verify job was blocked due to unresolved metadata
    let state_str: String = db
        .with(|c| Ok(c.query_row("SELECT state FROM jobs WHERE id = 1", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(state_str, "blocked");

    // 4. Assert that empty_home was NEVER touched and no file/folder was created inside it!
    let entries: Vec<_> = std::fs::read_dir(empty_home.path()).unwrap().collect();
    assert!(
        entries.is_empty(),
        "empty_home was touched! Created entries: {:?}",
        entries
    );
}

#[tokio::test]
async fn test_sync_does_not_reset_ready_or_partial_metadata_to_pending() {
    let db = Db::open_in_memory().unwrap();
    let clk = genrl_core::clock::FakeClock::new(100_000);

    // Show 1 is 'ready' with local poster
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, synopsis, poster_path, metadata_status, metadata_updated_at)
             VALUES (101, 'Ready Show', 'Valid synopsis', '/local/path/101.jpg', 'ready', 99_000)",
            [],
        )?;
        // Show 2 is 'partial' with remote poster (from MAL fallback)
        c.execute(
            "INSERT INTO shows (mal_id, title, synopsis, poster_path, metadata_status, metadata_updated_at)
             VALUES (102, 'Partial Show', 'Fallback synopsis', 'https://cdn.myanimelist.net/102.jpg', 'partial', 99_000)",
            [],
        )?;
        Ok(())
    }).unwrap();

    let queued = queue_stale_metadata(&db, clk.now()).unwrap();
    assert_eq!(
        queued, 0,
        "Already-resolved shows within 7 days must NOT be reset to pending"
    );

    let s101: String = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT metadata_status FROM shows WHERE mal_id = 101",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    let s102: String = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT metadata_status FROM shows WHERE mal_id = 102",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(s101, "ready");
    assert_eq!(s102, "partial");
}

#[tokio::test]
async fn test_download_season_for_finished_show_prefers_exact_season_batch() {
    let db = Db::open_in_memory().unwrap();
    let clk = genrl_core::clock::FakeClock::new(100_000);

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, status, total_episodes, metadata_status)
             VALUES (61814, 'Oni no Hanayome', 'finished', 12, 'ready')",
            [],
        )?;
        Ok(())
    })
    .unwrap();

    let queued = genrl_core::planner::plan_single(&db, &clk, 61814, None).unwrap();
    assert_eq!(queued, 1, "Download season for a finished show must queue exactly 1 season batch job, not 12 single jobs");

    let jobs: Vec<(i64, f64)> = db.with(|c| {
        let mut stmt = c.prepare("SELECT j.id, e.number FROM jobs j JOIN episodes e ON e.id = j.episode_id WHERE e.mal_id = 61814")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.map(|r| r.unwrap()).collect())
    }).unwrap();
    assert_eq!(
        jobs.len(),
        1,
        "Exactly 1 search job must be queued for season batch"
    );
}

#[tokio::test]
async fn test_open_jikan_breaker_pauses_batch_without_retrying_or_failing_remaining_shows() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    let db = Db::open_in_memory().unwrap();
    db.with(|c| {
        for id in 1..=5 {
            c.execute(
                "INSERT INTO shows (mal_id, title, metadata_status, metadata_retry_count) VALUES (?1, 'Show', 'pending', 0)",
                [id],
            )?;
        }
        Ok(())
    })
    .unwrap();

    let settings = Settings {
        jikan_base_url: server.uri(),
        ..Default::default()
    };
    let mut policy = fast_policy();
    policy.breaker_threshold = 1;
    let gate = Gate::with_policies(db.clone(), &[(Service::Jikan, policy)]).unwrap();
    let dir = tempdir().unwrap();

    // First batch: show 1 gets 500, trips breaker, remaining shows 2..5 are NOT processed
    let report1 = process_metadata_queue(&db, &gate, &settings, &clock(), dir.path(), Some(5))
        .await
        .unwrap();
    assert_eq!(report1.processed, 1);
    assert_eq!(report1.failed, 1);

    // Second batch: breaker is already open. It must stop immediately without claiming or failing any show
    let report2 = process_metadata_queue(&db, &gate, &settings, &clock(), dir.path(), Some(5))
        .await
        .unwrap();
    assert_eq!(report2.processed, 0);
    assert_eq!(report2.failed, 0);

    // Shows 2..5 must still be pending with retry count 0
    let pending_cnt: i64 = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM shows WHERE mal_id IN (2, 3, 4, 5) AND metadata_status = 'pending' AND metadata_retry_count = 0",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(
        pending_cnt, 4,
        "Remaining shows must be untouched with retry count 0"
    );

    // Edge-only logging: exactly 1 warning about breaker being open was written to EventLog
    let log_count: i64 = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM events WHERE message LIKE '%Jikan circuit breaker open%'",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(
        log_count, 1,
        "Must log at most one warning for breaker open, not one per show or per tick"
    );
}
