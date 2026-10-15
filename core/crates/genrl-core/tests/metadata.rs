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
    clean_plain_text, clean_single_line, fetch_show_metadata, queue_stale_metadata,
    MetadataOutcome,
};
use genrl_core::settings::{self, Settings};

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
    assert_eq!(queued, 2, "Expected 2 shows to be queued for metadata refresh");

    // Verify Show 1 and Show 3 are marked pending with next_run_at = NOW
    let statuses: Vec<(i64, String, Option<i64>)> = db
        .with(|c| {
            let mut stmt = c.prepare("SELECT mal_id, metadata_status, metadata_next_retry_at FROM shows ORDER BY mal_id")?;
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

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, metadata_status) VALUES (100, 'Test Show 100', 'pending')",
            [],
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
        MetadataOutcome::PendingRetry { mal_id, next_run_at, .. } => {
            assert_eq!(mal_id, 100);
            assert!(next_run_at > NOW, "next_run_at should be in the future (backoff)");
        }
        other => panic!("expected PendingRetry, got {other:?}"),
    }

    // Verify placeholder was stored in SQLite
    let (syn, status, retry_cnt): (String, String, i64) = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT synopsis, metadata_status, metadata_retry_count FROM shows WHERE mal_id = 100",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?)
        })
        .unwrap();

    assert!(syn.contains("Synopsis loading"), "synopsis should hold placeholder");
    assert_eq!(status, "pending");
    assert_eq!(retry_cnt, 1);

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
        MetadataOutcome::Updated { mal_id, synopsis, .. } => {
            assert_eq!(mal_id, 100);
            assert_eq!(synopsis, "This is the real recovered synopsis from Jikan.");
        }
        other => panic!("expected Updated after retry, got {other:?}"),
    }

    // Verify database now contains real synopsis and ready status
    let (final_syn, final_status, final_retry): (String, String, i64) = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT synopsis, metadata_status, metadata_retry_count FROM shows WHERE mal_id = 100",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?)
        })
        .unwrap();

    assert_eq!(final_syn, "This is the real recovered synopsis from Jikan.");
    assert_eq!(final_status, "ready");
    assert_eq!(final_retry, 0);
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

    // Show was updated 3 days ago (< 7 days) and has synopsis + poster
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, synopsis, poster_path, metadata_status, metadata_updated_at)
             VALUES (200, 'Cached Show', 'Existing fresh synopsis', '/tmp/posters/200.jpg', 'ready', ?1)",
            [NOW - (3 * 86400)],
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
    let single = clean_single_line("Title with \x1b[34mANSI\x1b[0m\nand \r\n multiple   spaces\t\x00!");
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

    // Store mock MAL OAuth access token
    settings::write_state(&db, "mal_oauth_access_token", "mock_access_token_123").unwrap();

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

    let outcome = fetch_show_metadata(&db, &gate, &settings, &clk, &data_dir, 300, true)
        .await
        .unwrap();

    match outcome {
        MetadataOutcome::Updated { mal_id, synopsis, .. } => {
            assert_eq!(mal_id, 300);
            assert_eq!(synopsis, "Official synopsis fetched directly from MyAnimeList API.");
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

    assert_eq!(syn, "Official synopsis fetched directly from MyAnimeList API.");
    assert_eq!(status, "ready");
    assert_eq!(en_title.as_deref(), Some("Show Three Hundred"));
}
