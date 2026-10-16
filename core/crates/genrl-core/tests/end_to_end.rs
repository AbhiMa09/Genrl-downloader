//! End-to-end flow test: from MAL list planning, Nyaa search, candidate scoring,
//! qBittorrent download, library organization (synopsis, square poster, icons, .mal_id),
//! and library rescan indexing.
//!
//! Exercises Phases 1 through 6 in a single unified pipeline.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use genrl_core::clock::{Clock, FakeClock};
use genrl_core::http::{policy::RateSpec, RetryPolicy, ServicePolicy};
use genrl_core::library::{rescan_library, LibraryFinalizer};
use genrl_core::models::{JobState, ListStatus};
use genrl_core::planner::{self, ListEntry, MalList, PlanOutcome};
use genrl_core::qbit::magnet_info_hash;
use genrl_core::worker::{Engine, FakeDiskProbe};
use genrl_core::{jobs, Db, EventLog, Gate, Service, Settings};

const HASH_FRIEREN: &str = "1111222233334444555566667777888899990000";
const START_TIME: i64 = 1_789_227_900;

#[derive(Default)]
struct MockQbitState {
    adds: Vec<String>,
    present: HashSet<String>,
    progress: HashMap<String, f64>,
    cursor: HashMap<String, usize>,
    script: Vec<f64>,
    content_path: String,
    size: i64,
}

impl MockQbitState {
    fn read(&mut self, hash: &str) -> f64 {
        let key = hash.to_ascii_lowercase();
        let next = self.cursor.get(&key).copied().unwrap_or(0);
        if let Some(val) = self.script.get(next) {
            self.progress.insert(key.clone(), *val);
            self.cursor.insert(key.clone(), next + 1);
        }
        self.progress.get(&key).copied().unwrap_or(0.0)
    }

    fn row(&self, hash: &str, progress: f64) -> serde_json::Value {
        let done = progress >= 1.0;
        serde_json::json!({
            "hash": hash.to_ascii_uppercase(),
            "name": "[SubsPlease] Sousou no Frieren - 02 (1080p) [A1B2C3D4].mkv",
            "progress": progress,
            "dlspeed": if done { 0 } else { 2_000_000 },
            "upspeed": 0,
            "eta": if done { 0 } else { 10 },
            "num_seeds": 45,
            "num_leechs": 5,
            "size": self.size,
            "state": if done { "uploading" } else { "downloading" },
            "save_path": "/downloads/",
            "content_path": self.content_path,
            "amount_left": 0,
            "added_on": START_TIME,
            "completion_on": 0,
            "category": "genrl",
        })
    }
}

struct TorrentsInfo {
    state: Arc<Mutex<MockQbitState>>,
}

impl Respond for TorrentsInfo {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let mut guard = self.state.lock().unwrap();
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
            let p = guard.read(&h);
            rows.push(guard.row(&h, p));
        }
        ResponseTemplate::new(200).set_body_json(rows)
    }
}

struct TorrentsAdd {
    state: Arc<Mutex<MockQbitState>>,
}

impl Respond for TorrentsAdd {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body = String::from_utf8_lossy(&request.body);
        for pair in body.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == "urls" {
                    let decoded = percent_decode(v);
                    if let Some(hash) = magnet_info_hash(&decoded) {
                        let mut guard = self.state.lock().unwrap();
                        guard.adds.push(hash.clone());
                        guard.present.insert(hash.clone());
                        guard.progress.entry(hash.clone()).or_insert(0.0);
                        guard.cursor.entry(hash).or_insert(0);
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
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body = String::from_utf8_lossy(&request.body);
        for pair in body.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                if k == "hashes" {
                    let decoded = percent_decode(v);
                    let mut guard = self.state.lock().unwrap();
                    for h in decoded.split(',') {
                        guard.present.remove(&h.to_ascii_lowercase());
                    }
                }
            }
        }
        ResponseTemplate::new(200)
    }
}

fn percent_decode(val: &str) -> String {
    let bytes = val.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                if let Some(byte) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
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

fn nyaa_rss_frieren() -> String {
    format!(
        r#"<rss xmlns:nyaa="https://nyaa.si/xmlns/nyaa" version="2.0">
        <channel>
            <title>Nyaa - Frieren feed</title>
            <item>
                <title>[SubsPlease] Sousou no Frieren - 02 (1080p) [A1B2C3D4].mkv</title>
                <link>https://nyaa.si/download/2000.torrent</link>
                <guid isPermaLink="true">https://nyaa.si/view/2000</guid>
                <pubDate>Fri, 29 Sep 2023 15:00:00 -0000</pubDate>
                <nyaa:seeders>85</nyaa:seeders>
                <nyaa:leechers>4</nyaa:leechers>
                <nyaa:downloads>5000</nyaa:downloads>
                <nyaa:infoHash>{HASH_FRIEREN}</nyaa:infoHash>
                <nyaa:categoryId>1_2</nyaa:categoryId>
                <nyaa:category>Anime - English-translated</nyaa:category>
                <nyaa:size>1.3 GiB</nyaa:size>
                <nyaa:trusted>Yes</nyaa:trusted>
                <nyaa:remake>No</nyaa:remake>
            </item>
        </channel></rss>"#
    )
}

#[tokio::test]
async fn full_end_to_end_flow_from_planning_to_library_and_rescan() {
    std::env::set_var("GENRL_QBIT_PASSWORD", "test");

    let server = MockServer::start().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let download_dir = temp.path().join("downloads");
    let library_dir = temp.path().join("library");
    std::fs::create_dir_all(&download_dir).expect("create downloads");
    std::fs::create_dir_all(&library_dir).expect("create library");

    let finished_file =
        download_dir.join("[SubsPlease] Sousou no Frieren - 02 (1080p) [A1B2C3D4].mkv");
    // Write 1 MB dummy video file
    std::fs::write(&finished_file, vec![0x42u8; 1_048_576]).expect("write dummy video");

    let qbit_state = Arc::new(Mutex::new(MockQbitState {
        content_path: finished_file.display().to_string(),
        size: 1_048_576,
        script: vec![0.5, 1.0], // first read 50%, second read 100% complete
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

    // 2. Mount Nyaa search feed
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(nyaa_rss_frieren()))
        .mount(&server)
        .await;

    // 3. Mount Jikan metadata endpoint
    let jikan_response = serde_json::json!({
        "data": {
            "mal_id": 52991,
            "title": "Sousou no Frieren",
            "title_english": "Frieren: Beyond Journey's End",
            "synopsis": "During their decade-long quest to defeat the Demon King, the party of the hero Himmel brought peace to the realm.",
            "year": 2023,
            "type": "TV",
            "episodes": 28,
            "score": 9.38,
            "studios": [{"name": "Madhouse"}],
            "genres": [{"name": "Adventure"}, {"name": "Fantasy"}],
            "images": {
                "jpg": {
                    "large_image_url": format!("{}/poster.jpg", server.uri()),
                    "image_url": format!("{}/poster.jpg", server.uri())
                }
            }
        }
    });

    Mock::given(method("GET"))
        .and(path("/anime/52991"))
        .respond_with(ResponseTemplate::new(200).set_body_json(jikan_response))
        .mount(&server)
        .await;

    // 4. Mount image poster endpoint with valid 1x1 PNG
    let valid_png: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    Mock::given(method("GET"))
        .and(path("/poster.jpg"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(valid_png))
        .mount(&server)
        .await;

    // 5. Database and Settings initialization
    let db = Db::open_in_memory().expect("open db");
    let mut settings = Settings::default();
    settings.nyaa_base_url = server.uri();
    settings.qbit_url = server.uri();
    settings.jikan_base_url = server.uri();
    settings.mal_base_url = server.uri();
    settings.qbit_start_if_installed = false;
    settings.automation_on = true;
    settings.first_run_complete = true;
    settings.score_threshold = 70.0;
    settings.score_margin = 10.0;
    settings.download_folder = download_dir.display().to_string();
    settings.library_folder = library_dir.display().to_string();
    settings.remove_when_finished = true;
    settings.save(&db).expect("save settings");

    let clock = Arc::new(FakeClock::new(START_TIME));
    let disk = Arc::new(FakeDiskProbe::new(500 * 1024 * 1024 * 1024));

    // 6. Plan from MAL list entry: Frieren, 28 eps, watched 1 -> wants episode 2
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

    let outcome = planner::plan(&db, clock.as_ref(), &mal_list, true).expect("plan");
    let queued = match outcome {
        PlanOutcome::Applied { queued, .. } => queued,
        PlanOutcome::Preview(_) => panic!("expected applied plan"),
    };
    assert_eq!(queued, 1, "exactly 1 episode queued");

    let job_rows = jobs::list(&db, None, 10).expect("list jobs");
    assert_eq!(job_rows.len(), 1);
    let job_id = job_rows[0].id;
    assert_eq!(job_rows[0].state, JobState::Search);

    // 7. Initialize Engine with real LibraryFinalizer
    let gate = Arc::new(
        Gate::with_policies(
            db.clone(),
            &[
                fast(Service::Nyaa),
                fast(Service::Qbit),
                fast(Service::Jikan),
                fast(Service::Images),
                fast(Service::Mal),
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

    let engine = Engine::new(db.clone(), gate, clock.clone(), finalizer, disk);

    // 8. Run Engine steps
    // Step 1: Search -> Choose
    let s1 = engine.step(job_id).await.expect("step 1");
    assert_eq!(s1.to, JobState::Choose);

    // Step 2: Choose -> Add
    let s2 = engine.step(job_id).await.expect("step 2");
    assert_eq!(s2.to, JobState::Add);

    // Step 3: Add -> Monitor (adds torrent to qbit)
    let s3 = engine.step(job_id).await.expect("step 3");
    assert_eq!(s3.to, JobState::Monitor);

    // Verify torrent was added to mock qBittorrent
    {
        let guard = qbit_state.lock().unwrap();
        assert_eq!(guard.adds.len(), 1, "torrent added to qBittorrent");
        assert_eq!(guard.adds[0], HASH_FRIEREN);
    }

    // Step 4: Monitor at 50% progress -> remains Monitor
    let s4 = engine.step(job_id).await.expect("step 4");
    assert_eq!(s4.to, JobState::Monitor, "still downloading");

    // Step 5: Monitor at 100% progress -> transitions to Finalize
    let s5 = engine.step(job_id).await.expect("step 5");
    assert_eq!(s5.to, JobState::Finalize, "download completed");

    // Step 6: Finalize -> Done! LibraryFinalizer executes
    let s6 = engine.step(job_id).await.expect("step 6");
    assert_eq!(s6.to, JobState::Done, "job transitioned to Done");

    let final_job = jobs::get(&db, job_id).expect("get").expect("job");
    assert_eq!(final_job.state, JobState::Done);

    // 9. Verify Library Output on Filesystem
    let show_dir = library_dir.join("Sousou no Frieren (2023)");
    assert!(
        show_dir.exists(),
        "show directory was created: {:?}",
        show_dir
    );

    // Video file moved/organized
    let expected_video = show_dir.join("Sousou no Frieren - S01E02.mkv");
    assert!(
        expected_video.exists(),
        "episode video exists at {:?}",
        expected_video
    );
    assert_eq!(
        std::fs::metadata(&expected_video).unwrap().len(),
        1_048_576,
        "video file size matches original"
    );

    // Metadata file generated
    let synopsis_file = show_dir.join("synopsis.txt");
    assert!(synopsis_file.exists(), "synopsis.txt was created");
    let synopsis_text = std::fs::read_to_string(&synopsis_file).unwrap();
    assert!(synopsis_text.contains("Sousou no Frieren"));
    assert!(synopsis_text.contains("Demon King"));
    assert!(synopsis_text.contains("Adventure, Fantasy"));

    // Square poster generated
    let folder_png = show_dir.join("folder.png");
    assert!(folder_png.exists(), "folder.png was created");

    // Hidden .mal_id marker generated
    let marker_file = show_dir.join(".mal_id");
    assert!(marker_file.exists(), ".mal_id marker was created");
    let marker_id = std::fs::read_to_string(&marker_file).unwrap();
    assert_eq!(marker_id.trim(), "52991");

    // Torrent removed from client as remove_when_finished was true
    {
        let guard = qbit_state.lock().unwrap();
        assert!(
            !guard.present.contains(HASH_FRIEREN),
            "torrent was cleaned from qBittorrent"
        );
    }

    // 10. Run Library Rescan over library folder
    let report = rescan_library(&db, &library_dir).expect("rescan library");
    assert_eq!(report.shows_adopted, 1, "adopted show folder");
    assert_eq!(report.episodes_found, 1, "indexed episode 2");

    // 11. Verify Activity Log
    let events = EventLog::new(db.clone()).recent(50).expect("events");
    assert!(!events.is_empty(), "event log recorded actions");
    let messages: Vec<String> = events.into_iter().map(|e| e.message).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("Sousou no Frieren") || m.contains("library")),
        "activity log contains show entries: {:?}",
        messages
    );
}
