//! End-to-end integration tests for Movie downloads: from indexer search,
//! candidate scoring (CAM rejection, BluRay selection), acquire (.torrent download and
//! multipart qBit upload), monitor, and library finalization.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha1::{Digest, Sha1};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use genrl_core::clock::{Clock, FakeClock};
use genrl_core::db::Db;
use genrl_core::http::{policy::RateSpec, Gate, RetryPolicy, Service, ServicePolicy};
use genrl_core::indexer::config::IndexerConfig;
use genrl_core::indexer::types::MediaKind;
use genrl_core::library::LibraryFinalizer;
use genrl_core::models::{JobState, ListStatus};
use genrl_core::planner::{self, ListEntry, MalList, PlanOutcome};
use genrl_core::questions::{Answer, Choice, QuestionStore};
use genrl_core::settings::Settings;
use genrl_core::worker::{Engine, FakeDiskProbe};

const START_TIME: i64 = 1_700_000_000;
const PROWLARR_SECRET: &str = "PROWLARR_SECRET_1234567890abcdef";

#[derive(Default)]
struct MockQbitState {
    adds: Vec<String>,
    multipart_received: bool,
    present: HashSet<String>,
    progress: HashMap<String, f64>,
    content_path: String,
    size: i64,
}

struct TorrentsInfo {
    state: Arc<Mutex<MockQbitState>>,
}

impl Respond for TorrentsInfo {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let guard = self.state.lock().unwrap();
        let wanted = request
            .url
            .query_pairs()
            .find(|(k, _)| k == "hashes")
            .map(|(_, v)| v.to_ascii_lowercase());

        let hashes: Vec<String> = match wanted {
            Some(h) if guard.present.contains(&h) => vec![h],
            Some(_) => Vec::new(),
            None => guard.present.iter().cloned().collect(),
        };

        let mut rows = Vec::new();
        for h in hashes {
            let p = guard.progress.get(&h).copied().unwrap_or(1.0);
            rows.push(serde_json::json!({
                "hash": h.to_ascii_uppercase(),
                "name": "Dune.Part.Two.2024.1080p.BluRay.x264-SPARKS",
                "progress": p,
                "dlspeed": if p >= 1.0 { 0 } else { 2_000_000 },
                "upspeed": 0,
                "eta": 0,
                "num_seeds": 80,
                "num_leechs": 10,
                "size": guard.size,
                "state": if p >= 1.0 { "uploading" } else { "downloading" },
                "save_path": "/downloads/",
                "content_path": guard.content_path,
                "amount_left": 0,
                "added_on": START_TIME,
                "completion_on": START_TIME,
                "category": "movies",
                "tags": "genrl,movie",
            }));
        }
        ResponseTemplate::new(200).set_body_json(rows)
    }
}

struct TorrentsAdd {
    state: Arc<Mutex<MockQbitState>>,
    expected_hash: String,
}

impl Respond for TorrentsAdd {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let mut guard = self.state.lock().unwrap();
        let body_str = String::from_utf8_lossy(&request.body);
        if body_str.contains("torrents") || body_str.contains("name=\"torrents\"") {
            guard.multipart_received = true;
            guard.adds.push(self.expected_hash.clone());
            guard.present.insert(self.expected_hash.clone());
            guard.progress.insert(self.expected_hash.clone(), 1.0);
        } else {
            for pair in body_str.split('&') {
                if let Some((k, _)) = pair.split_once('=') {
                    if k == "urls" {
                        guard.adds.push(self.expected_hash.clone());
                        guard.present.insert(self.expected_hash.clone());
                        guard.progress.insert(self.expected_hash.clone(), 1.0);
                    }
                }
            }
        }
        ResponseTemplate::new(200)
    }
}

struct TorrentsDelete {
    state: Arc<Mutex<MockQbitState>>,
}

impl Respond for TorrentsDelete {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let mut guard = self.state.lock().unwrap();
        guard.present.clear();
        ResponseTemplate::new(200)
    }
}

fn fast(service: Service) -> (Service, ServicePolicy) {
    let mut p = ServicePolicy::for_service(service);
    p.rate = RateSpec {
        permits_per_second: 10_000.0,
        burst: 1_000,
        per_minute_cap: None,
        jitter: false,
    };
    p.retry = RetryPolicy {
        max_attempts: 1,
        schedule: vec![Duration::from_millis(5)],
        max_total_wait: Duration::from_millis(50),
        max_delay: Duration::from_millis(5),
    };
    p.breaker_threshold = 3;
    p.breaker_cooldown = Duration::ZERO;
    p.timeout = Duration::from_secs(5);
    (service, p)
}

fn create_torrent_bytes(name: &str) -> (Vec<u8>, String) {
    let info = format!(
        "d4:name{}:{name}6:pieces20:12345678901234567890e",
        name.len()
    );
    let mut torrent = b"d8:announce17:http://tracker.io4:info".to_vec();
    torrent.extend_from_slice(info.as_bytes());
    torrent.push(b'e');

    let mut hasher = Sha1::new();
    hasher.update(info.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    (torrent, hash)
}

fn movie_feed_xml(server_uri: &str, bluray_hash: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <title>Torznab Feed</title>
    <!-- Item 1: CAM (must be rejected) -->
    <item>
      <title>Dune.Part.Two.2024.CAM.x264</title>
      <guid>{server_uri}/1</guid>
      <link>{server_uri}/download/cam.torrent</link>
      <pubDate>Mon, 01 Jan 2024 12:00:00 +0000</pubDate>
      <size>900000000</size>
      <enclosure url="{server_uri}/download/cam.torrent" length="900000000" type="application/x-bittorrent" />
      <torznab:attr name="category" value="2000" />
      <torznab:attr name="size" value="900000000" />
      <torznab:attr name="seeders" value="100" />
      <torznab:attr name="infohash" value="1111111111111111111111111111111111111111" />
    </item>
    <!-- Item 2: WEB-DL (lower score) -->
    <item>
      <title>Dune.Part.Two.2024.1080p.WEB-DL.DDP5.1.Atmos.H.264-FLUX</title>
      <guid>{server_uri}/2</guid>
      <link>{server_uri}/download/webdl.torrent</link>
      <pubDate>Mon, 01 Jan 2024 13:00:00 +0000</pubDate>
      <size>1500000000</size>
      <enclosure url="{server_uri}/download/webdl.torrent" length="1500000000" type="application/x-bittorrent" />
      <torznab:attr name="category" value="2000" />
      <torznab:attr name="size" value="1500000000" />
      <torznab:attr name="seeders" value="50" />
      <torznab:attr name="infohash" value="2222222222222222222222222222222222222222" />
    </item>
    <!-- Item 3: BluRay (winner) -->
    <item>
      <title>Dune.Part.Two.2024.1080p.BluRay.x264-SPARKS</title>
      <guid>{server_uri}/3</guid>
      <link>{server_uri}/download/bluray.torrent</link>
      <pubDate>Mon, 01 Jan 2024 14:00:00 +0000</pubDate>
      <size>1800000000</size>
      <enclosure url="{server_uri}/download/bluray.torrent" length="1800000000" type="application/x-bittorrent" />
      <torznab:attr name="category" value="2000" />
      <torznab:attr name="size" value="1800000000" />
      <torznab:attr name="seeders" value="80" />
      <torznab:attr name="infohash" value="{bluray_hash}" />
    </item>
  </channel>
</rss>"#
    )
}

#[tokio::test]
async fn test_movie_e2e_full_flow() {
    std::env::set_var("GENRL_QBIT_PASSWORD", "test");

    let server = MockServer::start().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let download_dir = temp.path().join("downloads");
    let library_dir = temp.path().join("library");
    std::fs::create_dir_all(&download_dir).expect("create downloads");
    std::fs::create_dir_all(&library_dir).expect("create library");

    let release_name = "Dune.Part.Two.2024.1080p.BluRay.x264-SPARKS";
    let (torrent_bytes, expected_hash) = create_torrent_bytes(release_name);

    let torrent_folder = download_dir.join(release_name);
    std::fs::create_dir_all(&torrent_folder).expect("create torrent folder");

    // Main video (100 MB sparse file for fast test, > 50 MB required by movie selector)
    let main_video = torrent_folder.join(format!("{release_name}.mkv"));
    let f = std::fs::File::create(&main_video).expect("create main video");
    f.set_len(100 * 1024 * 1024).expect("set len");

    // Sample video (should be ignored)
    let sample_video = torrent_folder.join("sample.mkv");
    std::fs::write(&sample_video, vec![0x33u8; 100_000]).expect("write sample");

    // NFO file (should be ignored)
    let nfo_file = torrent_folder.join("movie.nfo");
    std::fs::write(&nfo_file, b"release notes").expect("write nfo");

    // Subtitle file (should be linked)
    let sub_file = torrent_folder.join(format!("{release_name}.en.srt"));
    std::fs::write(&sub_file, b"1\n00:00:01,000 --> 00:00:04,000\nHello\n").expect("write sub");

    let qbit_state = Arc::new(Mutex::new(MockQbitState {
        content_path: torrent_folder.display().to_string(),
        size: 1_800_000_000,
        ..Default::default()
    }));

    // 1. Mount qBittorrent endpoints
    Mock::given(method("GET"))
        .and(path("/api/v2/app/version"))
        .respond_with(ResponseTemplate::new(200).set_body_string("v5.2.4"))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(
            ResponseTemplate::new(204).insert_header("Set-Cookie", "QBT_SID=mock_sid; path=/"),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(TorrentsInfo {
            state: Arc::clone(&qbit_state),
        })
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .respond_with(TorrentsAdd {
            state: Arc::clone(&qbit_state),
            expected_hash: expected_hash.clone(),
        })
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/delete"))
        .respond_with(TorrentsDelete {
            state: Arc::clone(&qbit_state),
        })
        .mount(&server)
        .await;

    // 2. Mount Torznab endpoints
    let caps_xml = include_str!("fixtures/torznab/caps_full.xml");
    Mock::given(method("GET"))
        .and(query_param("t", "caps"))
        .respond_with(ResponseTemplate::new(200).set_body_string(caps_xml))
        .mount(&server)
        .await;

    let feed_xml = movie_feed_xml(&server.uri(), &expected_hash);
    Mock::given(method("GET"))
        .and(query_param("t", "movie"))
        .respond_with(ResponseTemplate::new(200).set_body_string(feed_xml))
        .mount(&server)
        .await;

    // 3. Mount .torrent download endpoint
    Mock::given(method("GET"))
        .and(path("/download/bluray.torrent"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(torrent_bytes))
        .mount(&server)
        .await;

    // 4. DB & Settings setup
    let db = Db::open_in_memory().expect("open db");
    let mut settings = Settings::default();
    settings.qbit_url = server.uri();
    settings.download_folder = download_dir.display().to_string();
    settings.library_folder = library_dir.display().to_string();
    settings.remove_when_finished = true;
    settings.save(&db).expect("save settings");

    // Register Torznab indexer
    let idx_config = IndexerConfig::new(
        "prowlarr_mock",
        "Mock Indexer",
        server.uri(),
        Some(PROWLARR_SECRET.to_string()),
        vec![MediaKind::Movie],
    );
    IndexerConfig::save(&db, &idx_config).expect("save indexer config");

    let clock = Arc::new(FakeClock::new(START_TIME));
    let disk = Arc::new(FakeDiskProbe::new(500 * 1024 * 1024 * 1024));

    let gate = Arc::new(
        Gate::with_policies(
            db.clone(),
            &[
                fast(Service::Nyaa),
                fast(Service::Qbit),
                fast(Service::Images),
            ],
        )
        .expect("gate"),
    );

    let finalizer = Arc::new(LibraryFinalizer::new(
        db.clone(),
        Arc::clone(&gate),
        Arc::clone(&clock) as Arc<dyn Clock>,
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

    // 5. Add movie to library
    let media_id = db.with(|c| {
        c.execute(
            "INSERT INTO media (kind, title, year, tmdb_id, status, created_at) VALUES ('movie', 'Dune: Part Two', 2024, 693134, 'wanted', ?1)",
            [START_TIME],
        )?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    let job_id = db.with(|c| {
        c.execute(
            "INSERT INTO jobs (kind, media_kind, media_id, state, created_at, updated_at) VALUES ('search', 'movie', ?1, 'search', ?2, ?2)",
            rusqlite::params![media_id, START_TIME],
        )?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    // 6. Step through Engine
    // Step 1: Search -> Choose
    let s1 = engine.step(job_id).await.expect("search step");
    assert_eq!(s1.to, JobState::Choose);

    // Step 2: Choose -> Add (BluRay chosen, CAM rejected)
    let s2 = engine.step(job_id).await.expect("choose step");
    assert_eq!(s2.to, JobState::Add);

    // Step 3: Add -> Monitor (.torrent downloaded, bencode checked, multipart uploaded to qBit)
    let s3 = engine.step(job_id).await.expect("add step");
    assert_eq!(s3.to, JobState::Monitor);

    {
        let guard = qbit_state.lock().unwrap();
        assert!(
            guard.multipart_received,
            "multipart torrent upload was sent to qBit"
        );
        assert_eq!(guard.adds.len(), 1);
        assert_eq!(guard.adds[0], expected_hash);
    }

    // Step 4: Monitor at 100% -> Finalize
    let s4 = engine.step(job_id).await.expect("monitor step");
    assert_eq!(s4.to, JobState::Finalize);

    // Step 5: Finalize -> Done
    let s5 = engine.step(job_id).await.expect("finalize step");
    assert_eq!(s5.to, JobState::Done);

    // 7. Verify Library Filesystem
    let movie_dir = library_dir.join("Movies").join("Dune Part Two (2024)");
    assert!(
        movie_dir.exists(),
        "movie directory exists at {:?}",
        movie_dir
    );

    let finalized_video = movie_dir.join("Dune Part Two (2024).mkv");
    assert!(finalized_video.exists(), "finalized movie video exists");
    assert_eq!(
        std::fs::metadata(&finalized_video).unwrap().len(),
        100 * 1024 * 1024
    );

    let finalized_sub = movie_dir.join("Dune Part Two (2024).en.srt");
    assert!(finalized_sub.exists(), "sidecar subtitle exists");

    // Sample video must be ignored
    let sample_in_lib = movie_dir.join("sample.mkv");
    assert!(!sample_in_lib.exists(), "sample video must be ignored");

    // 8. Verify DB State
    let (media_status, media_kind): (String, String) = db.with(|c| {
        Ok(c.query_row(
            "SELECT m.status, j.media_kind FROM media m JOIN jobs j ON j.media_id = m.id WHERE m.id = ?1",
            [media_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }).unwrap();
    assert_eq!(media_status, "downloaded");
    assert_eq!(media_kind, "movie");

    // 9. Verify Secret Redaction: PROWLARR_SECRET must never appear anywhere in SQLite
    let db_dump = db
        .with(|c| {
            let mut dump_str = String::new();
            let tables: Vec<String> = c
                .prepare("SELECT name FROM sqlite_master WHERE type='table'")
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap();
            for t in tables {
                let mut stmt = c.prepare(&format!("SELECT * FROM {t}")).unwrap();
                let count = stmt.column_count();
                let rows = stmt
                    .query_map([], |r| {
                        let mut row_str = String::new();
                        for i in 0..count {
                            if let Ok(s) = r.get::<_, String>(i) {
                                row_str.push_str(&s);
                                row_str.push(' ');
                            }
                        }
                        Ok(row_str)
                    })
                    .unwrap();
                for r in rows {
                    dump_str.push_str(&r.unwrap());
                    dump_str.push('\n');
                }
            }
            Ok(dump_str)
        })
        .unwrap();

    assert!(
        !db_dump.contains(PROWLARR_SECRET),
        "Prowlarr secret must never appear in SQLite database dump"
    );

    // 10. Verify torrent cleaned up from client
    {
        let guard = qbit_state.lock().unwrap();
        assert!(guard.present.is_empty(), "torrent removed after completion");
    }
}

#[tokio::test]
async fn test_movie_e2e_large_download_confirmation_gate() {
    std::env::set_var("GENRL_QBIT_PASSWORD", "test");

    let server = MockServer::start().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let download_dir = temp.path().join("downloads");
    let library_dir = temp.path().join("library");
    std::fs::create_dir_all(&download_dir).expect("create downloads");
    std::fs::create_dir_all(&library_dir).expect("create library");

    let release_name = "Dune.Part.Two.2024.1080p.BluRay.x264-SPARKS";
    let (torrent_bytes, expected_hash) = create_torrent_bytes(release_name);

    let torrent_folder = download_dir.join(release_name);
    std::fs::create_dir_all(&torrent_folder).expect("create torrent folder");
    let main_video = torrent_folder.join(format!("{release_name}.mkv"));
    std::fs::write(&main_video, vec![0x42u8; 1_048_576]).expect("write main video");

    // Size is 2.5 GB -> above 2.0 GB confirmation threshold
    let qbit_state = Arc::new(Mutex::new(MockQbitState {
        content_path: torrent_folder.display().to_string(),
        size: 2_500_000_000,
        ..Default::default()
    }));

    Mock::given(method("GET"))
        .and(path("/api/v2/app/version"))
        .respond_with(ResponseTemplate::new(200).set_body_string("v5.2.4"))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(
            ResponseTemplate::new(204).insert_header("Set-Cookie", "QBT_SID=mock_sid; path=/"),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(TorrentsInfo {
            state: Arc::clone(&qbit_state),
        })
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .respond_with(TorrentsAdd {
            state: Arc::clone(&qbit_state),
            expected_hash: expected_hash.clone(),
        })
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/delete"))
        .respond_with(TorrentsDelete {
            state: Arc::clone(&qbit_state),
        })
        .mount(&server)
        .await;

    // Torznab with 2.5 GB BluRay
    let caps_xml = include_str!("fixtures/torznab/caps_full.xml");
    Mock::given(method("GET"))
        .and(query_param("t", "caps"))
        .respond_with(ResponseTemplate::new(200).set_body_string(caps_xml))
        .mount(&server)
        .await;

    let feed_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <title>Torznab Feed</title>
    <item>
      <title>{release_name}</title>
      <guid>{}/3</guid>
      <link>{}/download/bluray.torrent</link>
      <pubDate>Mon, 01 Jan 2024 14:00:00 +0000</pubDate>
      <size>2500000000</size>
      <enclosure url="{}/download/bluray.torrent" length="2500000000" type="application/x-bittorrent" />
      <torznab:attr name="category" value="2000" />
      <torznab:attr name="size" value="2500000000" />
      <torznab:attr name="seeders" value="80" />
      <torznab:attr name="infohash" value="{expected_hash}" />
    </item>
  </channel>
</rss>"#,
        server.uri(),
        server.uri(),
        server.uri()
    );

    Mock::given(method("GET"))
        .and(query_param("t", "movie"))
        .respond_with(ResponseTemplate::new(200).set_body_string(feed_xml))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/download/bluray.torrent"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(torrent_bytes))
        .mount(&server)
        .await;

    let db = Db::open_in_memory().expect("open db");
    let mut settings = Settings::default();
    settings.qbit_url = server.uri();
    settings.download_folder = download_dir.display().to_string();
    settings.library_folder = library_dir.display().to_string();
    settings.remove_when_finished = true;
    settings.save(&db).expect("save settings");

    let idx_config = IndexerConfig::new(
        "prowlarr_mock",
        "Mock Indexer",
        server.uri(),
        Some(PROWLARR_SECRET.to_string()),
        vec![MediaKind::Movie],
    );
    IndexerConfig::save(&db, &idx_config).expect("save indexer config");

    let clock = Arc::new(FakeClock::new(START_TIME));
    let disk = Arc::new(FakeDiskProbe::new(500 * 1024 * 1024 * 1024));

    let gate = Arc::new(
        Gate::with_policies(
            db.clone(),
            &[
                fast(Service::Nyaa),
                fast(Service::Qbit),
                fast(Service::Images),
            ],
        )
        .expect("gate"),
    );

    let finalizer = Arc::new(LibraryFinalizer::new(
        db.clone(),
        Arc::clone(&gate),
        Arc::clone(&clock) as Arc<dyn Clock>,
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

    let media_id = db.with(|c| {
        c.execute(
            "INSERT INTO media (kind, title, year, tmdb_id, status, created_at) VALUES ('movie', 'Dune: Part Two', 2024, 693134, 'wanted', ?1)",
            [START_TIME],
        )?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    let job_id = db.with(|c| {
        c.execute(
            "INSERT INTO jobs (kind, media_kind, media_id, state, created_at, updated_at) VALUES ('search', 'movie', ?1, 'search', ?2, ?2)",
            rusqlite::params![media_id, START_TIME],
        )?;
        Ok(c.last_insert_rowid())
    }).unwrap();

    // Step 1: Search -> Choose
    let s1 = engine.step(job_id).await.expect("search step");
    assert_eq!(s1.to, JobState::Choose);

    // Step 2: Choose -> Add
    let s2 = engine.step(job_id).await.expect("choose step");
    assert_eq!(s2.to, JobState::Add);

    // Step 3: Add -> 2.5 GB confirmation gate stops at NeedsInput
    let s3 = engine.step(job_id).await.expect("add step");
    assert_eq!(
        s3.to,
        JobState::NeedsInput,
        "2.5 GB download must trigger confirmation gate in NeedsInput"
    );

    // Verify confirmation question was asked
    let questions = QuestionStore::open(&db, START_TIME).expect("open questions");
    assert_eq!(questions.len(), 1, "exactly 1 confirmation question");
    let q = &questions[0];

    // Answer the confirmation question with Choice::UseThis
    let answer = Answer {
        choice: Choice::UseThis {
            info_hash: expected_hash.clone(),
        },
        always: false,
    };
    QuestionStore::answer(&db, q.id, &answer).expect("answer question");
    let full_q = QuestionStore::get(&db, q.id).unwrap().unwrap();
    genrl_core::worker::apply_answer(&db, clock.as_ref(), &full_q, &answer).expect("apply answer");

    // Next step resumes add -> Monitor
    let s4 = engine.step(job_id).await.expect("resume add step");
    assert_eq!(
        s4.to,
        JobState::Monitor,
        "after approval, job enters Monitor"
    );
}

#[tokio::test]
async fn test_anime_e2e_flow_still_passes_unchanged() {
    // Proves that Nyaa-only anime planning, search, and job execution is unchanged.
    let db = Db::open_in_memory().expect("open db");
    let mut settings = Settings::default();
    settings.first_run_complete = true;
    settings.save(&db).expect("save settings");

    let clock = FakeClock::new(START_TIME);
    let list_entry = ListEntry {
        mal_id: 52991,
        title: "Sousou no Frieren".to_string(),
        english_title: Some("Frieren: Beyond Journey's End".to_string()),
        list_status: ListStatus::Watching,
        watched_count: 1,
        total_episodes: Some(28),
        start_year: Some(2023),
        auto: true,
        episode_offset: 0,
        names: vec![
            "Sousou no Frieren".to_string(),
            "Frieren: Beyond Journey's End".to_string(),
        ],
        franchise: 52991,
        season: 1,
    };
    let mal_list = MalList {
        entries: vec![list_entry],
    };

    let outcome = planner::plan(&db, &clock, &mal_list, true).expect("plan anime");
    let queued = match outcome {
        PlanOutcome::Applied { queued, .. } => queued,
        _ => panic!("expected applied plan"),
    };
    assert_eq!(queued, 1, "anime episode queued");

    let jobs = genrl_core::jobs::list(&db, None, 10).expect("list jobs");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].media_kind.as_deref(), Some("anime"));
    assert!(jobs[0].episode_id.is_some());
}
