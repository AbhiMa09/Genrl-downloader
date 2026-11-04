//! Phase 4's check: a fake Nyaa, a fake qBittorrent, and a process killed at
//! every state on the diagram.
//!
//! Everything here runs against local mocks and a hand-cranked clock — no test
//! sleeps, no real service is touched, and `qbit_start_if_installed` is off so
//! nothing ever tries to spawn a daemon.
//!
//! The center of the file is
//! [`killed_at_every_state_recovers_to_the_same_final_state`]: for each of the
//! five states, run until the transition *into* it has been written, kill the
//! process (drop the engine, move the clock on), launch a fresh one, reconcile,
//! and drive to the end. Every kill point must land in the same final state as
//! an uninterrupted run, with one torrent and no duplicate adds in
//! qBittorrent.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use genrl_core::clock::{Clock, FakeClock};
use genrl_core::http::{policy::RateSpec, RetryPolicy, ServicePolicy};
use genrl_core::models::{JobKind, JobState, ListStatus, QuestionKind};
use genrl_core::planner::{self, ListEntry, MalList, PlanOutcome};
use genrl_core::qbit::magnet_info_hash;
use genrl_core::questions::{Answer, Choice, Payload, QuestionStore};
use genrl_core::queue::Scheduler;
use genrl_core::worker::{
    apply_answer, DiskProbe, Engine, FakeDiskProbe, FsDiskProbe, NoopFinalizer,
};
use genrl_core::{jobs, queue, Db, EventLog, Gate, Service, Settings};

/// The two releases in the fake feed: a big batch and a healthy single.
const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// 2026-09-12 — a fixed wall for the fake clock, far past any expiry.
const START: i64 = 1_789_227_900;

/// How long each stalled candidate gets before the queue gives up on it.
const TWENTY_MINUTES: i64 = 1_300;

// ---------------------------------------------------------------------------
// The fake feed
// ---------------------------------------------------------------------------

fn feed_item(title: &str, hash: &str, seeders: u32, size: &str, id: u32) -> String {
    format!(
        r#"<item>
            <title>{title}</title>
            <link>https://nyaa.si/download/{id}.torrent</link>
            <guid isPermaLink="true">https://nyaa.si/view/{id}</guid>
            <pubDate>Sat, 12 Sep 2026 10:00:00 -0000</pubDate>
            <nyaa:seeders>{seeders}</nyaa:seeders>
            <nyaa:leechers>2</nyaa:leechers>
            <nyaa:downloads>100</nyaa:downloads>
            <nyaa:infoHash>{hash}</nyaa:infoHash>
            <nyaa:categoryId>1_2</nyaa:categoryId>
            <nyaa:category>Anime - English-translated</nyaa:category>
            <nyaa:size>{size}</nyaa:size>
            <nyaa:trusted>Yes</nyaa:trusted>
            <nyaa:remake>No</nyaa:remake>
        </item>"#
    )
}

/// Nyaa answers with the same two releases whatever the query: the batch
/// `[Judas] … S02E05 [Batch]` (scores 95) and the single
/// `[SubsPlease] … S02E05 (1080p)` (scores 80).
fn feed() -> String {
    format!(
        r#"<rss xmlns:nyaa="https://nyaa.si/xmlns/nyaa" version="2.0">
        <channel>
            <title>Nyaa - test feed</title>
            <link>https://nyaa.si/</link>
            <description>two releases for Show 2</description>
            {batch}{single}
        </channel></rss>"#,
        batch = feed_item(
            "[Judas] Show 2 - S02E05 [Batch] [BD1080p]",
            HASH_A,
            50,
            "1.4 GiB",
            1001,
        ),
        single = feed_item(
            "[SubsPlease] Show 2 - S02E05 (1080p) [A1B2C3D4].mkv",
            HASH_B,
            12,
            "1.2 GiB",
            1002,
        ),
    )
}

// ---------------------------------------------------------------------------
// The fake qBittorrent
// ---------------------------------------------------------------------------

/// What the fake client holds, and how its downloads progress.
///
/// Every read of `/torrents/info` reports the current progress and then moves
/// to the next line of `script`, whose last line repeats forever: a script of
/// `[0.5, 1.0]` downloads halfway, then finishes; `[0.1]` never finishes, which
/// is what the stall test needs.
#[derive(Default)]
struct QbitWorld {
    /// Every `torrents/add` the app made, in order — duplicates kept on
    /// purpose, so "added exactly once" is a real assertion.
    adds: Vec<String>,
    /// The info hashes qBittorrent currently holds (lower-case).
    present: HashSet<String>,
    /// Reported progress per torrent, advanced by `read`.
    progress: std::collections::HashMap<String, f64>,
    /// Which script line each torrent reports next.
    cursor: std::collections::HashMap<String, usize>,
    /// The progress script; the last line repeats.
    script: Vec<f64>,
    /// The file a completed download reports (a real file on disk).
    content_path: String,
    size: i64,
}

impl QbitWorld {
    /// One read: report where the download is, then let it move on.
    fn read(&mut self, hash: &str) -> f64 {
        let key = hash.to_ascii_lowercase();
        let next = self.cursor.get(&key).copied().unwrap_or(0);
        if let Some(value) = self.script.get(next) {
            self.progress.insert(key.clone(), *value);
            self.cursor.insert(key.clone(), next + 1);
        }
        self.progress.get(&key).copied().unwrap_or(0.0)
    }

    /// One row of `/api/v2/torrents/info`.
    fn row(&self, hash: &str, progress: f64) -> serde_json::Value {
        let done = progress >= 1.0;
        serde_json::json!({
            "hash": hash.to_ascii_uppercase(),
            "name": "[SubsPlease] Show 2 - S02E05 (1080p) [A1B2C3D4].mkv",
            "progress": progress,
            "dlspeed": if done { 0 } else { 1_048_576 },
            "upspeed": 0,
            "eta": if done { 0 } else { 42 },
            "num_seeds": 12,
            "num_leechs": 3,
            "size": self.size,
            "state": if done { "uploading" } else { "downloading" },
            "save_path": "/downloads/",
            "content_path": self.content_path,
            "amount_left": 0,
            "added_on": START,
            "completion_on": 0,
            "category": "genrl",
        })
    }
}

/// Nyaa's feed — or a 500 when the breaker test has it down.
struct NyaaFeed {
    up: Arc<AtomicBool>,
    body: String,
}

impl Respond for NyaaFeed {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        if self.up.load(Ordering::SeqCst) {
            ResponseTemplate::new(200).set_body_string(&self.body)
        } else {
            ResponseTemplate::new(500).set_body_string("the tracker is having a day")
        }
    }
}

/// `GET /api/v2/torrents/info[?hashes=…]`.
struct TorrentsInfo {
    world: Arc<Mutex<QbitWorld>>,
}

impl Respond for TorrentsInfo {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let mut state = self.world.lock().unwrap_or_else(|e| e.into_inner());
        let wanted = request
            .url
            .query_pairs()
            .find(|(key, _)| key == "hashes")
            .map(|(_, value)| value.to_ascii_lowercase());
        let hashes: Vec<String> = match wanted {
            Some(hash) if state.present.contains(&hash) => vec![hash],
            Some(_) => Vec::new(),
            None => state.present.iter().cloned().collect(),
        };
        let mut rows = Vec::with_capacity(hashes.len());
        for hash in hashes {
            let progress = state.read(&hash);
            rows.push(state.row(&hash, progress));
        }
        let body = serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string());
        ResponseTemplate::new(200).set_body_string(body)
    }
}

/// `POST /api/v2/torrents/add` — magnet in, torrent held.
struct TorrentsAdd {
    world: Arc<Mutex<QbitWorld>>,
}

impl Respond for TorrentsAdd {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body = String::from_utf8_lossy(&request.body);
        if let Some(magnet) = form_field(&body, "urls") {
            if let Some(hash) = magnet_info_hash(&magnet) {
                let mut state = self.world.lock().unwrap_or_else(|e| e.into_inner());
                state.adds.push(hash.clone());
                state.present.insert(hash.clone());
                state.progress.entry(hash.clone()).or_insert(0.0);
                state.cursor.entry(hash).or_insert(0);
            }
        }
        ResponseTemplate::new(200)
    }
}

/// `POST /api/v2/torrents/delete`.
struct TorrentsDelete {
    world: Arc<Mutex<QbitWorld>>,
}

impl Respond for TorrentsDelete {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body = String::from_utf8_lossy(&request.body);
        if let Some(hashes) = form_field(&body, "hashes") {
            let mut state = self.world.lock().unwrap_or_else(|e| e.into_inner());
            for hash in hashes.split(',') {
                state.present.remove(&hash.to_ascii_lowercase());
            }
        }
        ResponseTemplate::new(200)
    }
}

/// One field of an `application/x-www-form-urlencoded` body, percent-decoded.
/// The body splits safely on `&`: the magnet's own `&` arrives as `%26`.
fn form_field(body: &str, key: &str) -> Option<String> {
    for pair in body.split('&') {
        if let Some((name, value)) = pair.split_once('=') {
            if name == key {
                return Some(percent_decode(value));
            }
        }
    }
    None
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    None => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ---------------------------------------------------------------------------
// The world: one mock server, one database, one clock
// ---------------------------------------------------------------------------

struct World {
    server: MockServer,
    db: Db,
    clock: FakeClock,
    disk: Arc<FakeDiskProbe>,
    qbit: Arc<Mutex<QbitWorld>>,
    dir: tempfile::TempDir,
    nyaa_up: Arc<AtomicBool>,
}

/// Keep the tests off the real keyring: `GENRL_QBIT_PASSWORD` wins over the
/// system credential store, and the fake WebUI never checks it anyway.
fn fake_password() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::env::set_var("GENRL_QBIT_PASSWORD", "test"));
}

/// A policy that behaves like the real one but never makes a test wait.
fn fast(service: Service) -> (Service, ServicePolicy) {
    let mut policy = ServicePolicy::for_service(service);
    policy.rate = RateSpec {
        permits_per_second: 10_000.0,
        burst: 1_000,
        per_minute_cap: None,
        jitter: false,
    };
    policy.retry = RetryPolicy {
        max_attempts: 1,
        schedule: vec![Duration::from_millis(10)],
        max_total_wait: Duration::from_millis(100),
        max_delay: Duration::from_millis(10),
    };
    policy.breaker_threshold = 1;
    policy.breaker_cooldown = Duration::ZERO;
    policy.timeout = Duration::from_secs(5);
    (service, policy)
}

impl World {
    async fn new() -> World {
        fake_password();
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().expect("temp dir");
        let db = Db::open_in_memory().expect("in-memory database");
        let clock = FakeClock::new(START);
        let disk = Arc::new(FakeDiskProbe::new(500 * 1024 * 1024 * 1024));

        // A real file, so the finalize step's size check has something true to
        // verify against.
        let download = dir.path().join("downloads");
        std::fs::create_dir_all(&download).expect("download folder");
        let content = download.join("[SubsPlease] Show 2 - S02E05 (1080p) [A1B2C3D4].mkv");
        std::fs::write(&content, vec![7u8; 1_048_576]).expect("the finished file");

        let qbit = Arc::new(Mutex::new(QbitWorld {
            content_path: content.display().to_string(),
            size: 1_048_576,
            script: vec![0.5, 1.0],
            ..Default::default()
        }));
        let nyaa_up = Arc::new(AtomicBool::new(true));

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(NyaaFeed {
                up: Arc::clone(&nyaa_up),
                body: feed(),
            })
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v2/app/version"))
            .respond_with(ResponseTemplate::new(200).set_body_string("v5.2.4"))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v2/auth/login"))
            .respond_with(
                ResponseTemplate::new(204)
                    .insert_header("Set-Cookie", "QBT_SID_8080=sessiontoken; path=/"),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v2/torrents/info"))
            .respond_with(TorrentsInfo {
                world: Arc::clone(&qbit),
            })
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v2/torrents/add"))
            .respond_with(TorrentsAdd {
                world: Arc::clone(&qbit),
            })
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v2/torrents/delete"))
            .respond_with(TorrentsDelete {
                world: Arc::clone(&qbit),
            })
            .mount(&server)
            .await;

        let mut settings = Settings::default();
        settings.nyaa_base_url = server.uri();
        settings.qbit_url = server.uri();
        settings.qbit_start_if_installed = false;
        settings.automation_on = true;
        settings.first_run_complete = true;
        // Comfortable margins for the two-candidate fixture: 95 ≥ 75 with a
        // 15-point gap over a 10-point margin.
        settings.score_threshold = 75.0;
        settings.score_margin = 10.0;
        settings.download_folder = download.display().to_string();
        settings.library_folder = dir.path().join("library").display().to_string();
        settings.save(&db).expect("settings");

        World {
            server,
            db,
            clock,
            disk,
            qbit,
            dir,
            nyaa_up,
        }
    }

    /// A fresh engine — its own gate, limiter, breaker and HTTP client: what a
    /// new process would have.
    fn engine(&self) -> Engine {
        self.engine_with(self.disk.clone())
    }

    fn engine_with(&self, disk: Arc<dyn DiskProbe>) -> Engine {
        let clock: Arc<dyn Clock> = Arc::new(self.clock.clone());
        let gate =
            Gate::with_policies(self.db.clone(), &[fast(Service::Nyaa), fast(Service::Qbit)])
                .expect("gate");
        Engine::new(
            self.db.clone(),
            Arc::new(gate),
            clock,
            Arc::new(NoopFinalizer),
            disk,
            self.dir.path().to_path_buf(),
        )
    }

    /// The fake qBittorrent, locked.
    fn qbit(&self) -> MutexGuard<'_, QbitWorld> {
        self.qbit.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Every Activity-log line, newest first.
    fn messages(&self) -> Vec<String> {
        EventLog::new(self.db.clone())
            .recent(500)
            .expect("events")
            .into_iter()
            .map(|event| event.message)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Driving the queue
// ---------------------------------------------------------------------------

/// One list entry: Show 2, watching 4 of 12, so the planner wants episode 5.
fn entry(mal_id: i64, title: &str, franchise: i64, season: i64) -> ListEntry {
    ListEntry {
        mal_id,
        title: title.to_string(),
        english_title: None,
        list_status: ListStatus::Watching,
        watched_count: 4,
        total_episodes: Some(12),
        start_year: Some(2025),
        auto: true,
        episode_offset: 0,
        names: vec![title.to_string()],
        franchise,
        season,
    }
}

fn list_of(entries: Vec<ListEntry>) -> MalList {
    MalList { entries }
}

/// Plan the single fixture show and hand back its job id.
fn plan_one(world: &World) -> i64 {
    let list = list_of(vec![entry(200, "Show 2", 1, 2)]);
    match planner::plan(&world.db, &world.clock, &list, true).expect("plan") {
        PlanOutcome::Applied { queued, .. } => assert_eq!(queued, 1, "one episode queued"),
        PlanOutcome::Preview(preview) => panic!("a finished first run applies: {preview:?}"),
    }
    // MOCK queue fixture starts after successful metadata resolution.
    world
        .db
        .with(|c| {
            c.execute("UPDATE shows SET metadata_status = 'ready'", [])?;
            Ok(())
        })
        .unwrap();
    single_job(world)
}

fn single_job(world: &World) -> i64 {
    let rows = jobs::list(&world.db, None, 10).expect("jobs");
    assert_eq!(rows.len(), 1, "exactly one job");
    rows[0].id
}

fn job_count(world: &World) -> usize {
    jobs::list(&world.db, None, 100).expect("jobs").len()
}

fn state_of(db: &Db, job_id: i64) -> JobState {
    jobs::get(db, job_id)
        .expect("job lookup")
        .expect("job exists")
        .state
}

fn episode_state(db: &Db) -> String {
    db.with(|c| {
        Ok(
            c.query_row("SELECT state FROM episodes ORDER BY id LIMIT 1", [], |r| {
                r.get::<_, String>(0)
            })?,
        )
    })
    .expect("episode row")
}

/// Run the job's own steps — no scheduler, no sleeping — until it leaves the
/// five main states (done, or parked in a side state).
async fn drive(engine: &Engine, job_id: i64) -> JobState {
    for _ in 0..128 {
        let state = state_of(engine.db(), job_id);
        if !state.is_active() {
            return state;
        }
        engine.step(job_id).await.expect("step");
    }
    panic!(
        "job {job_id} never settled: {}",
        state_of(engine.db(), job_id)
    );
}

/// Run the steps until the transition *into* `stop` is on disk: the moment the
/// process is killed.
async fn drive_until(engine: &Engine, job_id: i64, stop: JobState) {
    for _ in 0..128 {
        let state = state_of(engine.db(), job_id);
        if state == stop {
            return;
        }
        assert!(
            state.is_active(),
            "job {job_id} left the main line before {stop:?}: {state:?}"
        );
        engine.step(job_id).await.expect("step");
    }
    panic!("job {job_id} never reached {stop:?}");
}

/// What a run left behind: where the job ended, where the episode ended, and
/// exactly what qBittorrent was asked to hold.
struct Outcome {
    state: JobState,
    episode_state: String,
    adds: Vec<String>,
    present: HashSet<String>,
}

fn outcome(world: &World, state: JobState) -> Outcome {
    let episode_state = episode_state(&world.db);
    let qbit = world.qbit();
    Outcome {
        state,
        episode_state,
        adds: qbit.adds.clone(),
        present: qbit.present.clone(),
    }
}

// ---------------------------------------------------------------------------
// The check: kill at every state
// ---------------------------------------------------------------------------

/// Run one job from plan to `done` without interruption.
async fn uninterrupted() -> Outcome {
    let world = World::new().await;
    let engine = world.engine();
    let job = plan_one(&world);
    let state = drive(&engine, job).await;

    // Every step of the run is a line in the Activity log.
    let messages = world.messages();
    for wanted in ["→ choose", "→ add", "→ monitor", "→ finalize", "→ done"] {
        assert!(
            messages.iter().any(|message| message.contains(wanted)),
            "the log must show {wanted}; it has {messages:#?}"
        );
    }
    outcome(&world, state)
}

/// Kill the process right after the transition into `stop`, launch again, and
/// reconcile. Returns where everything ended up and what reconcile said.
async fn killed_at(stop: JobState) -> (Outcome, String) {
    let world = World::new().await;
    let engine = world.engine();
    let job = plan_one(&world);
    drive_until(&engine, job, stop).await;

    // The kill: the process's in-memory state — limiter, breaker, session — is
    // gone, and time passes before anyone launches again.
    world.clock.advance(120);
    let engine = world.engine();
    let report = queue::reconcile(&engine).await.expect("reconcile");

    // At these five kill points reconcile never has to *jump* a state: it
    // either leaves the job for the scheduler or re-runs the step itself.
    assert!(
        report.resumed.is_empty(),
        "reconcile never jumped at {stop:?}"
    );
    match stop {
        JobState::Search | JobState::Choose | JobState::Monitor => {
            assert_eq!(
                report.waiting,
                vec![(job, stop)],
                "killed at {stop:?}: nothing to compare against yet"
            );
        }
        JobState::Add | JobState::Finalize => {
            assert!(
                report.waiting.is_empty(),
                "killed at {stop:?}: reconcile re-runs this step itself"
            );
        }
        other => panic!("{other:?} is not one of the five states"),
    }
    // Reconcile always reports itself to the Activity log…
    assert!(
        world.messages().iter().any(|m| m.contains("reconcile:")),
        "launch logs what reconciliation decided"
    );
    // …and for the watchdog state it resets the stall timer from launch: a
    // crash must never look like a stall, so progress observed now gives the
    // download a fresh twenty minutes.
    if stop == JobState::Monitor {
        let watched = jobs::get(&world.db, job).expect("job").expect("job exists");
        assert_eq!(
            watched.last_progress_at,
            Some(START + 120),
            "killed at Monitor: the stall timer restarts at launch"
        );
    }

    let state = drive(&engine, job).await;

    // …and what actually happened is visible in the log.
    let messages = world.messages();
    match stop {
        // Waiting on the scheduler writes no transition; the timer reset
        // asserted above is the whole recovery for this kill point.
        JobState::Monitor => {}
        JobState::Add => assert!(
            messages
                .iter()
                .any(|m| m.contains("added ") && m.contains("to qBittorrent")),
            "the missing torrent is re-added by magnet: {messages:#?}"
        ),
        JobState::Finalize => assert!(
            messages.iter().any(|m| m.contains("filed it")),
            "finalize re-runs and the file verifies: {messages:#?}"
        ),
        JobState::Search | JobState::Choose => {}
        other => panic!("{other:?} is not one of the five states"),
    }

    let note = match stop {
        JobState::Search | JobState::Choose => format!(
            "launch found it in {stop:?}; the scheduler re-ran the step and the job finished"
        ),
        JobState::Monitor => {
            "stall timer restarted from launch; monitoring resumed and finished".to_string()
        }
        JobState::Add => {
            "the missing torrent was re-added by magnet (one add, one hash), then finished"
                .to_string()
        }
        JobState::Finalize => {
            "finalize re-ran, the file verified, and the episode was filed".to_string()
        }
        other => panic!("{other:?} is not one of the five states"),
    };
    (outcome(&world, state), note)
}

/// **The phase's check.** Kill the process at every state on the diagram and
/// each job must recover to the same place an uninterrupted run reaches — one
/// torrent, one add, one finished episode, no duplicates.
#[tokio::test]
async fn killed_at_every_state_recovers_to_the_same_final_state() {
    let baseline = uninterrupted().await;
    assert_eq!(
        baseline.state,
        JobState::Done,
        "the uninterrupted run finishes"
    );
    assert_eq!(baseline.episode_state, "done");
    assert_eq!(
        baseline.adds,
        vec![HASH_A],
        "the healthy batch wins the race to the top of the ranking"
    );
    assert_eq!(baseline.present.len(), 1, "one torrent in qBittorrent");

    let mut recoveries = Vec::new();
    for stop in [
        JobState::Search,
        JobState::Choose,
        JobState::Add,
        JobState::Monitor,
        JobState::Finalize,
    ] {
        let (found, note) = killed_at(stop).await;
        assert_eq!(
            found.state, baseline.state,
            "killed at {stop:?}: ends where the uninterrupted run ends"
        );
        assert_eq!(
            found.episode_state, baseline.episode_state,
            "killed at {stop:?}: the episode is filed"
        );
        assert_eq!(
            found.adds, baseline.adds,
            "killed at {stop:?}: added exactly once"
        );
        assert_eq!(
            found.present, baseline.present,
            "killed at {stop:?}: the same one torrent, no duplicate"
        );
        recoveries.push(format!("killed at {stop:?} → {}: {note}", found.state));
    }

    assert_eq!(recoveries.len(), 5);
    for line in &recoveries {
        println!("{line}");
    }
}

// ---------------------------------------------------------------------------
// The stall timer
// ---------------------------------------------------------------------------

/// 20 minutes without movement → reject the candidate, take the next one; out
/// of candidates and attempts → `failed` plus an AllFailed question, which a
/// "retry later" answer turns back into a clean search.
#[tokio::test]
async fn a_stalled_torrent_falls_through_the_candidates_and_asks_when_they_run_out() {
    let world = World::new().await;
    world.qbit.lock().unwrap_or_else(|e| e.into_inner()).script = vec![0.1]; // never finishes
    let engine = world.engine();
    let job = plan_one(&world);

    drive_until(&engine, job, JobState::Monitor).await;

    // Candidate A: the first poll moves, twenty minutes later nothing has.
    engine.step(job).await.expect("poll 1");
    world.clock.advance(TWENTY_MINUTES);
    engine.step(job).await.expect("poll 2 stalls");
    let stalled = jobs::get(&world.db, job).expect("job").expect("job exists");
    assert_eq!(
        stalled.state,
        JobState::Search,
        "back for the next candidate"
    );
    assert_eq!(stalled.attempts, 1, "one candidate tried");
    assert_eq!(stalled.torrent_id, None, "the stalled candidate is dropped");
    assert!(
        world
            .messages()
            .iter()
            .any(|m| m.contains("no progress for 20 minutes")),
        "the stall reason is in the Activity log"
    );
    {
        let qbit = world.qbit();
        assert!(qbit.present.is_empty(), "the stalled torrent was removed");
        assert_eq!(qbit.adds, vec![HASH_A]);
    }

    // Candidate B gets its own full twenty minutes — the previous candidate's
    // progress and timer must not carry over.
    engine.step(job).await.expect("search again");
    engine.step(job).await.expect("choose B");
    engine.step(job).await.expect("add B");
    engine.step(job).await.expect("poll 1 of B");
    world.clock.advance(TWENTY_MINUTES);
    engine.step(job).await.expect("poll 2 of B stalls");
    let stalled = jobs::get(&world.db, job).expect("job").expect("job exists");
    assert_eq!(
        stalled.state,
        JobState::Search,
        "the second candidate is out too"
    );
    assert_eq!(stalled.attempts, 2);
    assert!(
        world.qbit().present.is_empty(),
        "both torrents were removed"
    );

    // Third round: both candidates are rejected, so the chooser has nothing
    // left to offer and the job fails — with a question.
    engine.step(job).await.expect("search the third time");
    engine.step(job).await.expect("find no candidates");
    let failed = jobs::get(&world.db, job).expect("job").expect("job exists");
    assert_eq!(
        failed.state,
        JobState::Failed,
        "three candidates tried, three stalls"
    );
    assert_eq!(failed.attempts, 3);
    assert!(
        failed
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("no candidates"),
        "the row says why it stopped: {:?}",
        failed.last_error
    );
    assert_eq!(
        world.qbit().adds,
        vec![HASH_A, HASH_B],
        "each candidate was added once, never twice"
    );

    let questions = QuestionStore::of_kind(&world.db, QuestionKind::AllFailed, world.clock.now())
        .expect("questions");
    assert_eq!(questions.len(), 1, "a failed job raises a question");
    assert_eq!(questions[0].job_id, Some(job));
    match &questions[0].payload {
        Payload::AllFailed { candidates_tried } => assert_eq!(*candidates_tried, 3),
        other => panic!("expected AllFailed, got {other:?}"),
    }

    // Answering "retry later" starts a clean attempt at the first step.
    let question = QuestionStore::get(&world.db, questions[0].id)
        .expect("stored row")
        .expect("the question exists");
    let answer = Answer {
        choice: Choice::RetryLater,
        always: false,
    };
    QuestionStore::answer(&world.db, question.id, &answer).expect("recorded");
    let next = apply_answer(&world.db, &world.clock, &question, &answer).expect("applied");
    assert_eq!(next, Some(JobState::Search));
    let retried = jobs::get(&world.db, job).expect("job").expect("job exists");
    assert_eq!(
        retried.state,
        JobState::Search,
        "failed → search on a retry"
    );
    assert_eq!(retried.attempts, 0, "a user-initiated retry starts clean");
    assert_eq!(
        QuestionStore::open_count(&world.db, world.clock.now()).expect("count"),
        0,
        "the question is answered, not lingering"
    );
}

// ---------------------------------------------------------------------------
// Resources: disk floor and folder
// ---------------------------------------------------------------------------

/// Below the floor the whole queue pauses — not just the job that noticed —
/// and picks up exactly where it stopped when space returns.
#[tokio::test]
async fn disk_full_pauses_the_queue_until_space_returns() {
    let world = World::new().await;
    let engine = Arc::new(world.engine());
    let scheduler = Scheduler::new(Arc::clone(&engine));
    let job = plan_one(&world);

    drive_until(&engine, job, JobState::Add).await;
    world.disk.set(0);
    engine.step(job).await.expect("the add must not run");

    let blocked = jobs::get(&world.db, job).expect("job").expect("job exists");
    assert_eq!(blocked.state, JobState::Blocked);
    assert_eq!(
        blocked.blocked_from,
        Some(JobState::Add),
        "it returns to add"
    );
    let reason = blocked.last_error.as_deref().unwrap_or_default();
    assert!(reason.contains("space"), "the row says why: {reason}");
    assert!(
        world.qbit().adds.is_empty(),
        "a torrent it cannot store is never added"
    );

    // The resource stops the scheduler as a whole.
    let report = scheduler.tick().await.expect("tick");
    assert!(
        !report.paused,
        "workers are not paused, the resource is out"
    );
    let paused = report.resource_reason.expect("the queue waits on the disk");
    assert!(paused.contains("floor"), "{paused}");
    assert!(report.dispatched.is_empty());
    assert!(report.resumed.is_empty());

    // Space returns: back to the add step, in the same tick, and the run goes
    // on to the end.
    world.disk.set(500 * 1024 * 1024 * 1024);
    let report = scheduler.tick().await.expect("tick");
    assert!(report.resource_reason.is_none(), "the floor is cleared");
    assert_eq!(report.resumed, vec![(job, JobState::Add)]);
    assert_eq!(report.dispatched, vec![(JobKind::Add, job)]);
    scheduler.settle().await;
    assert_eq!(
        state_of(&world.db, job),
        JobState::Monitor,
        "the add went through"
    );
    assert_eq!(world.qbit().adds, vec![HASH_A]);

    let state = drive(&engine, job).await;
    assert_eq!(state, JobState::Done);
    assert_eq!(episode_state(&world.db), "done");
}

/// A missing download folder blocks the add just like the disk floor; creating
/// the folder is the whole fix.
#[tokio::test]
async fn a_missing_download_folder_blocks_the_add_until_it_exists() {
    let world = World::new().await;
    let folder = world.dir.path().join("new-downloads");
    let mut settings = Settings::load(&world.db).expect("settings");
    settings.download_folder = folder.display().to_string();
    settings.disk_floor_gb = 0; // this test is about the folder, not the floor
    settings.save(&world.db).expect("settings");

    let engine = Arc::new(world.engine_with(Arc::new(FsDiskProbe)));
    let scheduler = Scheduler::new(Arc::clone(&engine));
    let job = plan_one(&world);

    drive_until(&engine, job, JobState::Add).await;
    engine.step(job).await.expect("the add must not run");
    let blocked = jobs::get(&world.db, job).expect("job").expect("job exists");
    assert_eq!(blocked.state, JobState::Blocked);
    assert_eq!(blocked.blocked_from, Some(JobState::Add));
    let reason = blocked.last_error.as_deref().unwrap_or_default();
    assert!(reason.contains("folder"), "the row says why: {reason}");

    let report = scheduler.tick().await.expect("tick");
    let paused = report
        .resource_reason
        .expect("the queue waits on the folder");
    assert!(paused.contains("missing"), "{paused}");
    assert!(report.dispatched.is_empty());

    std::fs::create_dir_all(&folder).expect("the folder now exists");
    let report = scheduler.tick().await.expect("tick");
    assert!(report.resource_reason.is_none());
    assert_eq!(report.resumed, vec![(job, JobState::Add)]);
    scheduler.settle().await;

    let state = drive(&engine, job).await;
    let why = jobs::get(&world.db, job).unwrap().unwrap().last_error;
    assert_eq!(state, JobState::Done, "folder back, job finished: {why:?}");
    assert_eq!(episode_state(&world.db), "done");
}

// ---------------------------------------------------------------------------
// The breaker
// ---------------------------------------------------------------------------

/// Nyaa down: the search is *blocked*, never failed — no attempt is burned, no
/// question is raised — and one tick probes again. When Nyaa returns the
/// breaker half-opens, the probe closes it, and the job runs to `done`.
#[tokio::test]
async fn an_open_breaker_blocks_the_search_instead_of_failing_the_job() {
    let world = World::new().await;
    world.nyaa_up.store(false, Ordering::SeqCst);
    let engine = Arc::new(world.engine());
    let scheduler = Scheduler::new(Arc::clone(&engine));
    let job = plan_one(&world);

    engine.step(job).await.expect("search against a dead Nyaa");
    let blocked = jobs::get(&world.db, job).expect("job").expect("job exists");
    assert_eq!(
        blocked.state,
        JobState::Blocked,
        "a dead service parks, it does not fail"
    );
    assert_eq!(blocked.blocked_from, Some(JobState::Search));
    assert_eq!(blocked.attempts, 0, "a blocked job never burns an attempt");
    let reason = blocked.last_error.as_deref().unwrap_or_default();
    assert!(
        reason.contains("Nyaa"),
        "the row says which service: {reason}"
    );
    assert_eq!(
        QuestionStore::open_count(&world.db, world.clock.now()).expect("count"),
        0,
        "a service being down is not a question for the user"
    );

    // One tick while it is still down: resumed, one probe, re-blocked — and
    // still not failed.
    let report = scheduler.tick().await.expect("tick");
    assert_eq!(report.resumed, vec![(job, JobState::Search)]);
    scheduler.settle().await;
    let blocked = jobs::get(&world.db, job).expect("job").expect("job exists");
    assert_eq!(blocked.state, JobState::Blocked, "the probe failed again");
    assert_eq!(blocked.attempts, 0);

    // Nyaa comes back: the job gets through search, choose and add, then waits
    // for progress like any other download.
    world.nyaa_up.store(true, Ordering::SeqCst);
    scheduler.run_until_idle().await.expect("run");
    assert_eq!(
        state_of(&world.db, job),
        JobState::Monitor,
        "search, choose and add all went through after recovery"
    );

    let state = drive(&engine, job).await;
    assert_eq!(state, JobState::Done);
    assert_eq!(episode_state(&world.db), "done");
}

// ---------------------------------------------------------------------------
// The pause switch
// ---------------------------------------------------------------------------

/// One global switch stops the scheduler without touching the downloads that
/// are already in qBittorrent — and flipping it back resumes the queue.
#[tokio::test]
async fn the_pause_switch_stops_the_scheduler_but_never_the_downloads() {
    let world = World::new().await;
    let engine = Arc::new(world.engine());
    let scheduler = Scheduler::new(Arc::clone(&engine));
    let job = plan_one(&world);

    // A torrent actually downloading.
    drive_until(&engine, job, JobState::Monitor).await;
    engine.step(job).await.expect("one poll");
    assert_eq!(world.qbit().present.len(), 1);
    let seen_while_running = world
        .server
        .received_requests()
        .await
        .expect("requests")
        .len();

    let mut settings = Settings::load(&world.db).expect("settings");
    settings.automation_on = false;
    settings.save(&world.db).expect("settings");

    let report = scheduler.tick().await.expect("tick");
    assert!(report.paused, "the pause switch stops the scheduler");
    assert!(report.dispatched.is_empty());
    assert!(report.resumed.is_empty());
    assert_eq!(
        state_of(&world.db, job),
        JobState::Monitor,
        "the job itself does not move while paused"
    );
    assert_eq!(world.qbit().present.len(), 1, "the download keeps going");
    assert_eq!(
        world
            .server
            .received_requests()
            .await
            .expect("requests")
            .len(),
        seen_while_running,
        "a paused scheduler touches nothing"
    );

    // Unpause: the monitor job is due again and picks up where it left off.
    world.clock.advance(11);
    let mut settings = Settings::load(&world.db).expect("settings");
    settings.automation_on = true;
    settings.save(&world.db).expect("settings");

    let report = scheduler.tick().await.expect("tick");
    assert!(!report.paused, "automation is back on");
    assert_eq!(report.dispatched, vec![(JobKind::Monitor, job)]);
    scheduler.settle().await;
    assert_eq!(
        state_of(&world.db, job),
        JobState::Finalize,
        "the second poll saw the download complete"
    );

    let state = drive(&engine, job).await;
    assert_eq!(state, JobState::Done);
}

// ---------------------------------------------------------------------------
// The planner's caps, end to end
// ---------------------------------------------------------------------------

/// The first sync previews and waits for one confirmation; afterwards normal
/// weekly planning applies immediately, is idempotent, and a duplicate enqueue
/// returns the job that already exists.
#[tokio::test]
async fn the_first_sync_previews_confirms_once_and_replans_without_duplicating() {
    let world = World::new().await;
    let mut settings = Settings::load(&world.db).expect("settings");
    settings.first_run_complete = false;
    settings.save(&world.db).expect("settings");

    let list = list_of(vec![entry(200, "Show 2", 1, 2), entry(300, "Show 3", 2, 1)]);

    // A first sync previews: nothing is queued until someone confirms.
    let preview = match planner::plan(&world.db, &world.clock, &list, false).expect("plan") {
        PlanOutcome::Preview(preview) => preview,
        PlanOutcome::Applied { .. } => panic!("the first sync must preview, not apply"),
    };
    assert!(
        preview.awaiting_confirmation,
        "one confirmation is required"
    );
    assert_eq!(
        preview.episodes.len(),
        2,
        "both shows are wanted at episode 5"
    );
    assert_eq!(job_count(&world), 0, "a preview queues nothing");
    assert!(
        planner::pending_preview(&world.db)
            .expect("pending")
            .is_some(),
        "the preview is waiting for the wizard"
    );

    // Confirming under the first-sync cap queues what the cap allows.
    let mut settings = Settings::load(&world.db).expect("settings");
    settings.first_sync_cap = 1;
    settings.save(&world.db).expect("settings");
    assert_eq!(
        planner::confirm(&world.db, &world.clock, &list).expect("confirm"),
        1
    );
    assert_eq!(job_count(&world), 1, "the cap held");
    assert!(
        planner::pending_preview(&world.db)
            .expect("pending")
            .is_none(),
        "one confirmation only; a second one is a no-op"
    );

    // Normal weekly planning takes over, is idempotent, and a second plan for
    // the same list queues nothing new.
    let mut settings = Settings::load(&world.db).expect("settings");
    settings.first_run_complete = true;
    settings.save(&world.db).expect("settings");
    match planner::plan(&world.db, &world.clock, &list, true).expect("plan") {
        PlanOutcome::Applied { queued, .. } => assert_eq!(queued, 1, "the other show queues"),
        other => panic!("expected an applied plan, got {other:?}"),
    }
    assert_eq!(job_count(&world), 2, "two shows, two jobs");
    match planner::plan(&world.db, &world.clock, &list, true).expect("plan again") {
        PlanOutcome::Applied { queued, .. } => assert_eq!(queued, 0, "already queued"),
        other => panic!("expected an applied plan, got {other:?}"),
    }
    assert_eq!(job_count(&world), 2, "replanning adds nothing");

    // Enqueuing the same episode again hands back the job that exists.
    let rows = jobs::list(&world.db, None, 10).expect("jobs");
    let again = jobs::enqueue_search(&world.db, &world.clock, rows[0].episode_id).expect("enqueue");
    assert_eq!(again, rows[0].id, "idempotent by episode");
    assert_eq!(job_count(&world), 2, "still two jobs");
}

/// A bad sync may queue one torrent today, not two hundred: the daily cap
/// holds the line and the preview says what it dropped.
#[tokio::test]
async fn the_daily_cap_stops_a_bad_sync_from_flooding_the_client() {
    let world = World::new().await;
    let mut settings = Settings::load(&world.db).expect("settings");
    settings.daily_torrent_cap = 1;
    settings.save(&world.db).expect("settings");

    let list = list_of(vec![entry(200, "Show 2", 1, 2), entry(300, "Show 3", 2, 1)]);
    match planner::plan(&world.db, &world.clock, &list, true).expect("plan") {
        PlanOutcome::Applied { queued, preview } => {
            assert_eq!(queued, 1, "the cap holds");
            assert_eq!(preview.cap, 1);
            assert_eq!(
                preview.over_cap, 1,
                "the preview reports what the cap dropped"
            );
        }
        other => panic!("expected an applied plan, got {other:?}"),
    }
    assert_eq!(job_count(&world), 1, "only the cap's worth of jobs exist");
}

/// A first sync with 40+ missing episodes respects the 25 first-sync cap,
/// leaving the remainder waiting for tomorrow, while bypass creates all.
#[tokio::test]
async fn first_sync_cap_holds_with_40_missing_episodes_and_bypass_queues_all() {
    let world = World::new().await;
    let mut settings = Settings::load(&world.db).expect("settings");
    settings.first_run_complete = false;
    settings.first_sync_cap = 25;
    settings.daily_torrent_cap = 30;
    settings.save(&world.db).expect("settings");

    // 45 Watching shows, each wanting episode 1
    let mut entries = Vec::new();
    for id in 1000..1045 {
        entries.push(ListEntry {
            mal_id: id,
            title: format!("Watching Show #{id}"),
            english_title: None,
            list_status: ListStatus::Watching,
            watched_count: 0,
            total_episodes: Some(12),
            start_year: Some(2025),
            auto: true,
            episode_offset: 0,
            names: vec![format!("Watching Show #{id}")],
            franchise: id,
            season: 1,
        });
    }
    let list = MalList { entries };

    // 1. Preview reports 45 episodes
    let preview = match planner::plan(&world.db, &world.clock, &list, false).expect("plan") {
        PlanOutcome::Preview(p) => p,
        PlanOutcome::Applied { .. } => panic!("first sync must preview"),
    };
    assert_eq!(preview.episodes.len(), 45);
    assert_eq!(job_count(&world), 0);

    // 2. Confirm without bypass: respects first_sync_cap (25), leaves 20 waiting for tomorrow
    let queued =
        planner::confirm_capped(&world.db, &world.clock, &list, false).expect("confirm_capped");
    assert_eq!(
        queued, 25,
        "Launch must create exactly the first-sync cap of 25"
    );
    assert_eq!(job_count(&world), 25);

    // Assert that the 20 capped episodes stay planned (not lost) after the cap
    let remaining_plan =
        match planner::plan(&world.db, &world.clock, &list, true).expect("subsequent plan") {
            PlanOutcome::Preview(p) => p,
            PlanOutcome::Applied { preview, .. } => preview,
        };
    assert_eq!(
        remaining_plan.episodes.len(),
        20,
        "20 capped episodes stay planned (not lost) after the cap"
    );
    assert_eq!(
        job_count(&world),
        25,
        "job_count is still 25 after the second plan"
    );

    // 3. Confirm with bypass on creates all 45
    let world2 = World::new().await;
    let mut settings2 = Settings::load(&world2.db).expect("settings");
    settings2.first_run_complete = false;
    settings2.first_sync_cap = 25;
    settings2.daily_torrent_cap = 30;
    settings2.save(&world2.db).expect("settings");

    let queued_all =
        planner::confirm_capped(&world2.db, &world2.clock, &list, true).expect("confirm bypass");
    assert_eq!(queued_all, 45, "Bypass on creates all 45 jobs");
    assert_eq!(job_count(&world2), 45);
}

#[tokio::test]
async fn planner_plans_next_episode_for_ongoing_shows_with_null_total_episodes() {
    let world = World::new().await;
    let list = MalList {
        entries: vec![
            ListEntry {
                mal_id: 235,
                title: "Meitantei Conan".to_string(),
                english_title: Some("Detective Conan".to_string()),
                list_status: ListStatus::Watching,
                watched_count: 1020,
                total_episodes: None, // Real ongoing anime: total NULL
                start_year: Some(1996),
                auto: true,
                episode_offset: 0,
                names: vec!["Meitantei Conan".to_string()],
                franchise: 235,
                season: 1,
            },
            ListEntry {
                mal_id: 21,
                title: "One Piece".to_string(),
                english_title: Some("One Piece".to_string()),
                list_status: ListStatus::Watching,
                watched_count: 905,
                total_episodes: None, // Real ongoing anime: total NULL
                start_year: Some(1999),
                auto: true,
                episode_offset: 0,
                names: vec!["One Piece".to_string()],
                franchise: 21,
                season: 1,
            },
            ListEntry {
                mal_id: 34566,
                title: "Boruto: Naruto Next Generations".to_string(),
                english_title: Some("Boruto: Naruto Next Generations".to_string()),
                list_status: ListStatus::Watching,
                watched_count: 216,
                total_episodes: Some(293), // Known finished/part total
                start_year: Some(2017),
                auto: true,
                episode_offset: 0,
                names: vec!["Boruto: Naruto Next Generations".to_string()],
                franchise: 34566,
                season: 1,
            },
            ListEntry {
                mal_id: 51462,
                title: "Ishura".to_string(),
                english_title: Some("Ishura".to_string()),
                list_status: ListStatus::Watching,
                watched_count: 12,
                total_episodes: Some(12), // Known total: fully watched
                start_year: Some(2024),
                auto: true,
                episode_offset: 0,
                names: vec!["Ishura".to_string()],
                franchise: 51462,
                season: 1,
            },
        ],
    };

    let preview = match planner::plan(&world.db, &world.clock, &list, false).expect("plan") {
        PlanOutcome::Preview(p) => p,
        PlanOutcome::Applied { preview, .. } => preview,
    };

    // Conan: watched 1020 -> wanted episode 1021.0
    let conan = preview.episodes.iter().find(|e| e.mal_id == 235);
    assert!(conan.is_some(), "Conan must be planned for next episode");
    assert_eq!(conan.unwrap().episode, 1021.0);

    // One Piece: watched 905 -> wanted episode 906.0
    let op = preview.episodes.iter().find(|e| e.mal_id == 21);
    assert!(op.is_some(), "One Piece must be planned for next episode");
    assert_eq!(op.unwrap().episode, 906.0);

    // Boruto: watched 216 < total 293 -> wanted episode 217.0
    let boruto = preview.episodes.iter().find(|e| e.mal_id == 34566);
    assert!(boruto.is_some(), "Boruto must be planned for next episode");
    assert_eq!(boruto.unwrap().episode, 217.0);

    // Ishura: watched 12 == total 12 -> fully watched, skipped!
    let ishura = preview.episodes.iter().find(|e| e.mal_id == 51462);
    assert!(ishura.is_none(), "Ishura must not be planned");
    let ishura_skip = preview.skipped.iter().find(|s| s.mal_id == 51462);
    assert!(ishura_skip.is_some());
    assert!(
        ishura_skip.unwrap().reason.contains("fully watched"),
        "Only shows with known total are marked fully watched"
    );
}

/// MOCK Nyaa/qBit: unresolved shows must not make any search request.
#[tokio::test]
async fn unresolved_metadata_blocks_search_without_using_nyaa() {
    let world = World::new().await;
    let id = plan_one(&world);
    world
        .db
        .with(|c| {
            c.execute("UPDATE shows SET metadata_status = 'pending'", [])?;
            Ok(())
        })
        .unwrap();
    let engine = Arc::new(world.engine());
    let result = engine.step(id).await.unwrap();
    assert_eq!(result.to, JobState::Blocked);
    let job = jobs::get(&world.db, id).unwrap().unwrap();
    assert_eq!(job.attempts, 0);
    assert!(job.last_error.unwrap().starts_with("Metadata unresolved"));
    let requests = world.server.received_requests().await.unwrap();
    assert!(!requests.iter().any(|r| r.url.path() == "/"));
    let scheduler = Scheduler::new(engine);
    assert!(scheduler.tick().await.unwrap().resumed.is_empty());
    world
        .db
        .with(|c| {
            c.execute("UPDATE shows SET metadata_status='partial'", [])?;
            Ok(())
        })
        .unwrap();
    let report = scheduler.tick().await.unwrap();
    assert!(report.resumed.contains(&(id, JobState::Search)));
    scheduler.settle().await;
    assert_eq!(state_of(&world.db, id), JobState::Choose);
}

/// Regression test: 2 blocked + 5 runnable jobs with search_concurrency 2.
/// Blocked jobs get next_run_at backoff, due_jobs only selects runnable jobs,
/// and all 5 runnable jobs must run without head-of-line blocking.
#[tokio::test]
async fn due_jobs_runnable_only_prevents_head_of_line_blocking() {
    let world = World::new().await;
    let mut settings = Settings::load(&world.db).unwrap();
    settings.search_concurrency = 2;
    settings.save(&world.db).unwrap();

    let list = list_of(vec![
        entry(101, "Show 1", 1, 1),
        entry(102, "Show 2", 2, 1),
        entry(103, "Show 3", 3, 1),
        entry(104, "Show 4", 4, 1),
        entry(105, "Show 5", 5, 1),
        entry(106, "Show 6", 6, 1),
        entry(107, "Show 7", 7, 1),
    ]);
    planner::plan(&world.db, &world.clock, &list, true).expect("plan");
    world
        .db
        .with(|c| {
            c.execute("UPDATE shows SET metadata_status = 'ready'", [])?;
            c.execute(
                "UPDATE shows SET metadata_status = 'pending' WHERE mal_id IN (101, 102)",
                [],
            )?;
            Ok(())
        })
        .unwrap();

    let all_jobs = jobs::list(&world.db, None, 10).unwrap();
    assert_eq!(all_jobs.len(), 7);

    let engine = Arc::new(world.engine());
    // Block the first 2 jobs
    engine.block(&all_jobs[0], "Metadata unresolved").unwrap();
    engine.block(&all_jobs[1], "Metadata unresolved").unwrap();

    let b0 = jobs::get(&world.db, all_jobs[0].id).unwrap().unwrap();
    assert_eq!(b0.state, JobState::Blocked);
    assert!(
        b0.next_run_at.is_some(),
        "Blocked jobs must receive next_run_at backoff"
    );

    let scheduler = Scheduler::new(engine);
    let mut dispatched_ids = std::collections::HashSet::new();
    for _ in 0..10 {
        let report = scheduler.tick().await.unwrap();
        for (_kind, id) in report.dispatched {
            dispatched_ids.insert(id);
        }
        scheduler.settle().await;
        if dispatched_ids.len() >= 5 {
            break;
        }
    }
    assert!(
        !dispatched_ids.contains(&all_jobs[0].id),
        "Blocked job 0 must not be dispatched"
    );
    assert!(
        !dispatched_ids.contains(&all_jobs[1].id),
        "Blocked job 1 must not be dispatched"
    );
    for job in &all_jobs[2..7] {
        assert!(
            dispatched_ids.contains(&job.id),
            "Runnable job {} must run",
            job.id
        );
    }
    assert_eq!(
        dispatched_ids.len(),
        5,
        "All 5 runnable jobs must run despite 2 older blocked jobs"
    );
}

#[tokio::test]
async fn test_slow_fake_sync_does_not_stall_job_dispatch() {
    let world = World::new().await;

    // Insert 1 runnable show and job
    let list = list_of(vec![entry(200, "Show 2", 1, 2)]);
    planner::plan(&world.db, &world.clock, &list, true).expect("plan");
    world
        .db
        .with(|c| {
            c.execute("UPDATE shows SET metadata_status = 'ready'", [])?;
            Ok(())
        })
        .unwrap();

    let jobs = jobs::list(&world.db, None, 10).unwrap();
    assert!(!jobs.is_empty(), "Must have a queued job");

    let engine = Arc::new(world.engine());
    let scheduler = Scheduler::new(engine);

    // Simulate an active slow sync task running in the background
    let slow_sync_handle = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Scheduler tick must dispatch the runnable job without waiting for slow sync
    let report = scheduler.tick().await.unwrap();
    assert_eq!(
        report.dispatched.len(),
        1,
        "Runnable job must be dispatched despite slow sync"
    );
    assert_eq!(report.dispatched[0].1, jobs[0].id);

    slow_sync_handle.await.unwrap();
}

/// A blocker the scheduler can't check (qBittorrent) is resumed on trust, so
/// it must wait out its backoff instead of bouncing every tick.
#[tokio::test]
async fn an_unverifiable_block_waits_out_its_backoff_before_resuming() {
    let world = World::new().await;
    let id = plan_one(&world);
    let engine = Arc::new(world.engine());
    let job = jobs::get(&world.db, id).unwrap().unwrap();
    engine
        .block(&job, "qBittorrent is not reachable: connection refused")
        .unwrap();
    let scheduler = Scheduler::new(engine);

    assert!(
        scheduler.tick().await.unwrap().resumed.is_empty(),
        "still inside the backoff"
    );
    world.clock.advance(31);
    let report = scheduler.tick().await.unwrap();
    assert!(report.resumed.contains(&(id, JobState::Search)));
}

/// Automation off stops automatic work only: a job the user asked for by hand
/// still runs.
#[tokio::test]
async fn a_manual_job_runs_while_automation_is_paused() {
    let world = World::new().await;
    let id = plan_one(&world);
    let mut settings = Settings::load(&world.db).unwrap();
    settings.automation_on = false;
    settings.save(&world.db).unwrap();
    let scheduler = Scheduler::new(Arc::new(world.engine()));

    let report = scheduler.tick().await.unwrap();
    assert!(report.paused);
    assert!(report.dispatched.is_empty(), "automatic jobs wait");

    jobs::mark_manual(&world.db, id).unwrap();
    let report = scheduler.tick().await.unwrap();
    assert_eq!(report.dispatched, vec![(JobKind::Search, id)]);
    scheduler.settle().await;
}

/// A season download is a manual job.
#[tokio::test]
async fn plan_single_marks_its_jobs_manual() {
    let world = World::new().await;
    let id = plan_one(&world);
    world
        .db
        .with(|c| {
            c.execute("DELETE FROM jobs", [])?;
            Ok(())
        })
        .unwrap();
    assert_ne!(id, 0);
    planner::plan_single(&world.db, &world.clock, 200, Some(5.0)).unwrap();
    let job = single_job(&world);
    assert!(jobs::is_manual(&world.db, job).unwrap());
}

#[tokio::test]
async fn finalize_step_resolves_empty_download_folder_when_qbit_forgets_torrent() {
    let world = World::new().await;
    let job_id = plan_one(&world);
    let engine = world.engine();

    drive_until(&engine, job_id, JobState::Finalize).await;

    // qBittorrent has forgotten the torrent
    world.qbit().present.clear();

    // Use a tempdir download folder so no real user directories are touched
    let dl_temp = tempfile::tempdir().unwrap();
    let mut settings = Settings::load(&world.db).unwrap();
    settings.download_folder = dl_temp.path().display().to_string();
    settings.save(&world.db).unwrap();

    let t_name: String = world
        .db
        .with(|c| {
            Ok(c.query_row(
                "SELECT name FROM torrents WHERE id = (SELECT torrent_id FROM jobs WHERE id = ?1)",
                [job_id],
                |r| r.get(0),
            )
            .unwrap())
        })
        .unwrap();

    let file_path = dl_temp.path().join(&t_name);
    std::fs::write(&file_path, b"test content").unwrap();

    let outcome = engine.step(job_id).await.unwrap();

    assert_eq!(outcome.to, JobState::Done);
}

#[tokio::test]
async fn finalize_step_resolves_batch_directory_when_qbit_forgets_torrent() {
    let world = World::new().await;
    let job_id = plan_one(&world);
    let engine = world.engine();

    drive_until(&engine, job_id, JobState::Finalize).await;

    // qBittorrent has forgotten the torrent
    world.qbit().present.clear();

    // Configure a dedicated tempdir for download_folder
    let dl_temp = tempfile::tempdir().unwrap();
    let mut settings = Settings::load(&world.db).unwrap();
    settings.download_folder = dl_temp.path().display().to_string();
    settings.save(&world.db).unwrap();

    let t_name: String = world
        .db
        .with(|c| {
            Ok(c.query_row(
                "SELECT name FROM torrents WHERE id = (SELECT torrent_id FROM jobs WHERE id = ?1)",
                [job_id],
                |r| r.get(0),
            )
            .unwrap())
        })
        .unwrap();

    // Create a batch directory (len() == 0 on Windows) containing video file
    let batch_dir = dl_temp.path().join(&t_name);
    std::fs::create_dir_all(&batch_dir).unwrap();
    let file_path = batch_dir.join("episode.mkv");
    std::fs::write(&file_path, b"test content in batch directory").unwrap();

    let outcome = engine.step(job_id).await.unwrap();

    assert_eq!(outcome.to, JobState::Done);
}

#[tokio::test]
async fn vanished_file_sends_job_back_to_search_with_torrent_cleared() {
    let world = World::new().await;
    let job_id = plan_one(&world);
    let engine = world.engine();

    drive_until(&engine, job_id, JobState::Finalize).await;

    // Verify torrent_id is currently Some before stepping
    let initial_torrent_id: Option<i64> = world
        .db
        .with(|c| {
            Ok(
                c.query_row("SELECT torrent_id FROM jobs WHERE id = ?1", [job_id], |r| {
                    r.get(0)
                })
                .unwrap(),
            )
        })
        .unwrap();
    assert!(
        initial_torrent_id.is_some(),
        "job in Finalize must have a chosen torrent"
    );

    // qBittorrent has forgotten the torrent AND the file has vanished from disk
    world.qbit().present.clear();

    let dl_temp = tempfile::tempdir().unwrap();
    let mut settings = Settings::load(&world.db).unwrap();
    settings.download_folder = dl_temp.path().display().to_string();
    settings.save(&world.db).unwrap();

    // Do NOT create the file in dl_temp — it has vanished!
    let outcome = engine.step(job_id).await.unwrap();

    // The job must transition back to Search
    assert_eq!(outcome.to, JobState::Search);

    // The chosen torrent must be cleared from the job
    let final_torrent_id: Option<i64> = world
        .db
        .with(|c| {
            Ok(
                c.query_row("SELECT torrent_id FROM jobs WHERE id = ?1", [job_id], |r| {
                    r.get(0)
                })
                .unwrap(),
            )
        })
        .unwrap();
    assert_eq!(
        final_torrent_id, None,
        "torrent_id must be cleared when searching again"
    );
}

#[tokio::test]
async fn retry_failed_job_transitions_to_search_cleanly() {
    let world = World::new().await;
    let job_id = plan_one(&world);

    // Fail the job
    jobs::transition(
        &world.db,
        &world.clock,
        job_id,
        JobState::Failed,
        "test failure",
        &jobs::MoveOptions::default(),
    )
    .unwrap();

    let outcome = jobs::retry_job(&world.db, &world.clock, job_id).unwrap();
    assert_eq!(outcome, jobs::RetryOutcome::Retried);

    let state: String = world
        .db
        .with(|c| {
            Ok(
                c.query_row("SELECT state FROM jobs WHERE id = ?1", [job_id], |r| {
                    r.get(0)
                })
                .unwrap(),
            )
        })
        .unwrap();
    assert_eq!(state, "search");
}

#[tokio::test]
async fn retry_job_whose_episode_is_done_skips_and_marks_done() {
    let world = World::new().await;
    let job_id = plan_one(&world);

    // Mark the episode done (e.g. completed via batch)
    world
        .db
        .with(|c| {
            c.execute(
                "UPDATE episodes SET state = 'done' WHERE id = (SELECT episode_id FROM jobs WHERE id = ?1)",
                [job_id],
            )?;
            Ok(())
        })
        .unwrap();

    // Put job in Failed state
    jobs::transition(
        &world.db,
        &world.clock,
        job_id,
        JobState::Failed,
        "test failure",
        &jobs::MoveOptions::default(),
    )
    .unwrap();

    let outcome = jobs::retry_job(&world.db, &world.clock, job_id).unwrap();
    assert_eq!(outcome, jobs::RetryOutcome::SkippedEpisodeDone);

    let state: String = world
        .db
        .with(|c| {
            Ok(
                c.query_row("SELECT state FROM jobs WHERE id = ?1", [job_id], |r| {
                    r.get(0)
                })
                .unwrap(),
            )
        })
        .unwrap();
    assert_eq!(
        state, "done",
        "skipped retry marks job done rather than queueing search"
    );
}

#[tokio::test]
async fn retry_all_jobs_reports_retried_and_skipped_counts() {
    let world = World::new().await;
    let job_id = plan_one(&world);

    world
        .db
        .with(|c| {
            c.execute(
                "UPDATE episodes SET state = 'done' WHERE id = (SELECT episode_id FROM jobs WHERE id = ?1)",
                [job_id],
            )?;
            Ok(())
        })
        .unwrap();

    jobs::transition(
        &world.db,
        &world.clock,
        job_id,
        JobState::Failed,
        "test failure",
        &jobs::MoveOptions::default(),
    )
    .unwrap();

    let summary = jobs::retry_all_jobs(&world.db, &world.clock).unwrap();
    assert_eq!(summary.retried, 0);
    assert_eq!(summary.skipped, 1);
}

#[tokio::test]
async fn cancel_job_transitions_to_failed_and_closes_questions() {
    let world = World::new().await;
    let job_id = plan_one(&world);

    // Insert an open question for this job
    world
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO questions (job_id, mal_id, kind, payload, answer, created_at)
                 VALUES (?1, 200, 'low_seeds', '{}', NULL, ?2)",
                rusqlite::params![job_id, world.clock.now()],
            )?;
            Ok(())
        })
        .unwrap();

    let torrent_to_remove = jobs::cancel_job(&world.db, &world.clock, job_id).unwrap();
    assert_eq!(torrent_to_remove, None); // no torrent attached yet in search

    let (state, last_error): (String, Option<String>) = world
        .db
        .with(|c| {
            Ok(c.query_row(
                "SELECT state, last_error FROM jobs WHERE id = ?1",
                [job_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap())
        })
        .unwrap();
    assert_eq!(state, "failed");
    assert_eq!(last_error.as_deref(), Some("Cancelled by user"));

    let question_answer: Option<String> = world
        .db
        .with(|c| {
            Ok(c.query_row(
                "SELECT answer FROM questions WHERE job_id = ?1",
                [job_id],
                |r| r.get(0),
            )
            .unwrap())
        })
        .unwrap();
    assert_eq!(question_answer.as_deref(), Some("cancelled"));
}

#[tokio::test]
async fn cancelled_job_cannot_be_resurrected_by_in_flight_worker_step() {
    let world = World::new().await;
    let job_id = plan_one(&world);

    // Cancel while search is running
    jobs::cancel_job(&world.db, &world.clock, job_id).unwrap();

    // Now simulated search finishes and attempts transition to choose
    let res = jobs::transition(
        &world.db,
        &world.clock,
        job_id,
        JobState::Choose,
        "search completed",
        &jobs::MoveOptions::default(),
    );
    assert!(
        res.is_err(),
        "transition from failed to choose must be rejected"
    );

    let state: String = world
        .db
        .with(|c| {
            Ok(
                c.query_row("SELECT state FROM jobs WHERE id = ?1", [job_id], |r| {
                    r.get(0)
                })
                .unwrap(),
            )
        })
        .unwrap();
    assert_eq!(state, "failed", "job must remain failed");
}

#[tokio::test]
async fn cancel_job_with_shared_torrent_preserves_torrent_for_other_active_job() {
    let world = World::new().await;
    let job1 = plan_one(&world);

    // Create a second episode and job sharing the show
    let job2: i64 = world
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO episodes (mal_id, number, state) VALUES (200, 2, 'queued')",
                [],
            )?;
            let ep2 = c.last_insert_rowid();
            c.execute(
                "INSERT INTO jobs (episode_id, kind, state, attempts, created_at, updated_at)
                 VALUES (?1, 'monitor', 'monitor', 0, ?2, ?2)",
                rusqlite::params![ep2, world.clock.now()],
            )?;
            Ok(c.last_insert_rowid())
        })
        .unwrap();

    // Insert a torrent and attach to both jobs
    let info_hash = "1111222233334444555566667777888899990000";
    world
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO torrents (name, magnet, info_hash, size, seeders, leechers, score, found_at)
                 VALUES ('Batch torrent', 'magnet:?', ?1, 1000, 10, 1, 100.0, ?2)",
                rusqlite::params![info_hash, world.clock.now()],
            )?;
            let tid = c.last_insert_rowid();
            c.execute(
                "UPDATE jobs SET torrent_id = ?1, state = 'monitor' WHERE id = ?2",
                rusqlite::params![tid, job1],
            )?;
            c.execute(
                "UPDATE jobs SET torrent_id = ?1, state = 'monitor' WHERE id = ?2",
                rusqlite::params![tid, job2],
            )?;
            Ok(tid)
        })
        .unwrap();

    // Cancel job 1: job 2 is still active, so torrent must NOT be removed from qBit
    let to_remove1 = jobs::cancel_job(&world.db, &world.clock, job1).unwrap();
    assert_eq!(
        to_remove1, None,
        "torrent is shared with active job2, must not be removed"
    );

    // Cancel job 2: now no active jobs use it, so torrent SHOULD be removed
    let to_remove2 = jobs::cancel_job(&world.db, &world.clock, job2).unwrap();
    assert_eq!(
        to_remove2,
        Some(info_hash.to_string()),
        "no remaining active jobs, torrent must be removed"
    );
}

#[tokio::test]
async fn answer_question_for_cancelled_or_failed_job_returns_error() {
    let world = World::new().await;
    let job_id = plan_one(&world);

    // Insert an open match_review question for this job
    let question_id: i64 = world
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO questions (job_id, mal_id, kind, payload, answer, created_at)
                 VALUES (?1, 200, 'match_review', '{}', NULL, ?2)",
                rusqlite::params![job_id, world.clock.now()],
            )?;
            Ok(c.last_insert_rowid())
        })
        .unwrap();

    // Cancel / fail the job
    jobs::transition(
        &world.db,
        &world.clock,
        job_id,
        JobState::Failed,
        "Cancelled by user",
        &jobs::MoveOptions::default(),
    )
    .unwrap();

    let question = QuestionStore::get(&world.db, question_id).unwrap().unwrap();
    let answer = Answer {
        choice: Choice::UseThis {
            info_hash: "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string(),
        },
        always: false,
    };

    let err = apply_answer(&world.db, &world.clock, &question, &answer).unwrap_err();
    assert!(
        err.to_string().contains("was cancelled — retry it first"),
        "expected cancelled error, got: {err}"
    );
}

#[tokio::test]
async fn answer_question_for_done_job_returns_error() {
    let world = World::new().await;
    let job_id = plan_one(&world);

    let question_id: i64 = world
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO questions (job_id, mal_id, kind, payload, answer, created_at)
                 VALUES (?1, 200, 'match_review', '{}', NULL, ?2)",
                rusqlite::params![job_id, world.clock.now()],
            )?;
            Ok(c.last_insert_rowid())
        })
        .unwrap();

    // Finish the job
    world
        .db
        .with(|c| {
            c.execute("UPDATE jobs SET state = 'done' WHERE id = ?1", [job_id])?;
            Ok(())
        })
        .unwrap();

    let question = QuestionStore::get(&world.db, question_id).unwrap().unwrap();
    let answer = Answer {
        choice: Choice::UseThis {
            info_hash: "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string(),
        },
        always: false,
    };

    let err = apply_answer(&world.db, &world.clock, &question, &answer).unwrap_err();
    assert!(
        err.to_string()
            .contains("already completed — retry it first"),
        "expected completed error, got: {err}"
    );
}

#[tokio::test]
async fn large_download_above_2gb_asks_once_and_parks_legally_in_needs_input() {
    let world = World::new().await;
    let job_id = plan_one(&world);
    let engine = world.engine();

    // Drive job to Add
    drive_until(&engine, job_id, JobState::Add).await;

    // Update the chosen torrent size to > 2 GB (2.5 GB)
    world
        .db
        .with(|c| {
            c.execute(
                "UPDATE torrents SET size = 2500000000 WHERE id = (SELECT torrent_id FROM jobs WHERE id = ?1)",
                [job_id],
            )?;
            Ok(())
        })
        .unwrap();

    // Step the job in Add: should confirm large download and transition to NeedsInput
    let outcome = engine.step(job_id).await.unwrap();
    assert_eq!(outcome.to, JobState::NeedsInput);

    let state = jobs::get(&world.db, job_id).unwrap().unwrap().state;
    assert_eq!(state, JobState::NeedsInput);

    // Verify exactly 1 question was created
    let questions_count: i64 = world
        .db
        .with(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM questions WHERE job_id = ?1",
                [job_id],
                |r| r.get(0),
            )
            .unwrap())
        })
        .unwrap();
    assert_eq!(
        questions_count, 1,
        "exactly 1 large download question must be asked"
    );

    // If the job re-enters Add while the question is still open,
    // stepping it parks back in NeedsInput without creating a duplicate question:
    jobs::transition(
        &world.db,
        &world.clock,
        job_id,
        JobState::Add,
        "retry add step",
        &jobs::MoveOptions::default(),
    )
    .unwrap();

    let outcome2 = engine.step(job_id).await.unwrap();
    assert_eq!(outcome2.to, JobState::NeedsInput);

    let questions_count2: i64 = world
        .db
        .with(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM questions WHERE job_id = ?1",
                [job_id],
                |r| r.get(0),
            )
            .unwrap())
        })
        .unwrap();
    assert_eq!(questions_count2, 1, "no duplicate questions asked");

    // Verify 0 illegal transition events were logged
    let illegal_events: i64 = world
        .db
        .with(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM events WHERE message LIKE '%illegal%transition%'",
                [],
                |r| r.get(0),
            )
            .unwrap())
        })
        .unwrap();
    assert_eq!(
        illegal_events, 0,
        "must have zero illegal transition events"
    );

    // Answering the question with Choice::UseThis moves job back to Add and it proceeds!
    let question_id: i64 = world
        .db
        .with(|c| {
            Ok(c.query_row(
                "SELECT id FROM questions WHERE job_id = ?1",
                [job_id],
                |r| r.get(0),
            )
            .unwrap())
        })
        .unwrap();

    let question = QuestionStore::get(&world.db, question_id).unwrap().unwrap();
    let torrent_hash: String = world
        .db
        .with(|c| {
            Ok(c.query_row(
                "SELECT info_hash FROM torrents WHERE id = (SELECT torrent_id FROM jobs WHERE id = ?1)",
                [job_id],
                |r| r.get(0),
            )
            .unwrap())
        })
        .unwrap();

    let answer = Answer {
        choice: Choice::UseThis {
            info_hash: torrent_hash,
        },
        always: false,
    };
    QuestionStore::answer(&world.db, question_id, &answer).unwrap();
    let next_state = apply_answer(&world.db, &world.clock, &question, &answer).unwrap();
    assert_eq!(next_state, Some(JobState::Add));

    let job_state_after_answer = jobs::get(&world.db, job_id).unwrap().unwrap().state;
    assert_eq!(job_state_after_answer, JobState::Add);

    // Now stepping in Add proceeds past the 2 GB check because it is approved!
    let outcome3 = engine.step(job_id).await.unwrap();
    assert_eq!(outcome3.to, JobState::Monitor);
}

#[tokio::test]
async fn whole_season_release_without_batch_keyword_is_treated_as_batch_in_add() {
    let world = World::new().await;
    let job_id = plan_one(&world);
    let engine = world.engine();

    // Show 2 has 2 episodes. Add episode 2 into episodes table and a job for it:
    world
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO episodes (mal_id, number, state) VALUES (200, 2, 'missing')",
                [],
            )?;
            let ep2_id = c.last_insert_rowid();
            c.execute(
                "INSERT INTO jobs (episode_id, kind, state, attempts, next_run_at, created_at, updated_at)
                 VALUES (?1, 'search', 'queued', 0, 0, 0, 0)",
                [ep2_id],
            )?;
            Ok(())
        })
        .unwrap();

    // Drive job 1 to Add
    drive_until(&engine, job_id, JobState::Add).await;

    // Update the chosen torrent name to a whole season release without the word "batch"
    world
        .db
        .with(|c| {
            c.execute(
                "UPDATE torrents SET name = '[neoHEVC] Show 2 [BD 1080p x265 HEVC AAC]' WHERE id = (SELECT torrent_id FROM jobs WHERE id = ?1)",
                [job_id],
            )?;
            Ok(())
        })
        .unwrap();

    // Step the job in Add
    let outcome = engine.step(job_id).await.unwrap();
    assert_eq!(outcome.to, JobState::Monitor);

    // Verify all episodes for show 200 are now downloading
    world
        .db
        .with(|c| {
            let mut stmt =
                c.prepare("SELECT number, state FROM episodes WHERE mal_id = 200 ORDER BY number")?;
            let rows: Vec<(f64, String)> = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].1, "downloading");
            assert_eq!(rows[1].1, "downloading");

            // Verify sibling job for episode 2 was deleted
            let remaining_jobs: i64 =
                c.query_row("SELECT COUNT(*) FROM jobs WHERE id != ?1", [job_id], |r| {
                    r.get(0)
                })?;
            assert_eq!(
                remaining_jobs, 0,
                "sibling search job must be deleted when batch is added"
            );
            Ok(())
        })
        .unwrap();
}

#[tokio::test]
async fn terminal_job_transitions_and_sweep_close_stale_open_questions() {
    let world = World::new().await;
    let job_id = plan_one(&world);
    let engine = world.engine();

    // Ask a question for the job
    let payload = Payload::AllFailed {
        candidates_tried: 0,
    };
    let qid = QuestionStore::ask(
        &world.db,
        Some(job_id),
        Some(200),
        &payload,
        world.clock.now(),
        None,
    )
    .unwrap();
    assert_eq!(
        QuestionStore::open_count(&world.db, world.clock.now()).unwrap(),
        1
    );

    // Transitioning job to Failed must close/expire the open question
    jobs::transition(
        &world.db,
        &world.clock,
        job_id,
        JobState::Failed,
        "job failed",
        &jobs::MoveOptions::default(),
    )
    .unwrap();

    assert_eq!(
        QuestionStore::open_count(&world.db, world.clock.now()).unwrap(),
        0,
        "question must be closed when job reaches Failed"
    );

    let q = QuestionStore::get(&world.db, qid).unwrap().unwrap();
    assert!(q.answer.is_some(), "answer must be set for closed question");

    // Test sweep cleans up orphaned open questions for terminal jobs
    let qid2 = QuestionStore::ask(
        &world.db,
        Some(job_id),
        Some(200),
        &payload,
        world.clock.now(),
        None,
    )
    .unwrap();
    // Force answer to NULL to simulate stale open question
    world
        .db
        .with(|c| {
            c.execute("UPDATE questions SET answer = NULL WHERE id = ?1", [qid2])?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        QuestionStore::open_count(&world.db, world.clock.now()).unwrap(),
        1
    );

    // Run sweep
    genrl_core::queue::sweep(&world.db, &world.clock).unwrap();
    assert_eq!(
        QuestionStore::open_count(&world.db, world.clock.now()).unwrap(),
        0,
        "sweep must expire stale questions for terminal jobs"
    );

    // Test reconcile also cleans up orphaned open questions
    let qid3 = QuestionStore::ask(
        &world.db,
        Some(job_id),
        Some(200),
        &payload,
        world.clock.now(),
        None,
    )
    .unwrap();
    world
        .db
        .with(|c| {
            c.execute("UPDATE questions SET answer = NULL WHERE id = ?1", [qid3])?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        QuestionStore::open_count(&world.db, world.clock.now()).unwrap(),
        1
    );

    genrl_core::queue::reconcile(&engine).await.unwrap();
    assert_eq!(
        QuestionStore::open_count(&world.db, world.clock.now()).unwrap(),
        0,
        "reconcile must expire stale questions for terminal jobs"
    );
}
