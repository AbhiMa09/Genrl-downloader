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
use genrl_core::planner::{self, MalList, ListEntry, PlanOutcome};
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
        let gate = Gate::with_policies(self.db.clone(), &[fast(Service::Nyaa), fast(Service::Qbit)])
            .expect("gate");
        Engine::new(
            self.db.clone(),
            Arc::new(gate),
            clock,
            Arc::new(NoopFinalizer),
            disk,
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
        Ok(c.query_row(
            "SELECT state FROM episodes ORDER BY id LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )?)
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
    panic!("job {job_id} never settled: {}", state_of(engine.db(), job_id));
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
    assert!(report.resumed.is_empty(), "reconcile never jumped at {stop:?}");
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
            messages.iter().any(|m| m.contains("added ") && m.contains("to qBittorrent")),
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
        JobState::Monitor => "stall timer restarted from launch; monitoring resumed and finished"
            .to_string(),
        JobState::Add => "the missing torrent was re-added by magnet (one add, one hash), then finished"
            .to_string(),
        JobState::Finalize => "finalize re-ran, the file verified, and the episode was filed"
            .to_string(),
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
    assert_eq!(baseline.state, JobState::Done, "the uninterrupted run finishes");
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
    assert_eq!(stalled.state, JobState::Search, "back for the next candidate");
    assert_eq!(stalled.attempts, 1, "one candidate tried");
    assert_eq!(stalled.torrent_id, None, "the stalled candidate is dropped");
    assert!(
        world.messages().iter().any(|m| m.contains("no progress for 20 minutes")),
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
    assert_eq!(stalled.state, JobState::Search, "the second candidate is out too");
    assert_eq!(stalled.attempts, 2);
    assert!(world.qbit().present.is_empty(), "both torrents were removed");

    // Third round: both candidates are rejected, so the chooser has nothing
    // left to offer and the job fails — with a question.
    engine.step(job).await.expect("search the third time");
    engine.step(job).await.expect("find no candidates");
    let failed = jobs::get(&world.db, job).expect("job").expect("job exists");
    assert_eq!(failed.state, JobState::Failed, "three candidates tried, three stalls");
    assert_eq!(failed.attempts, 3);
    assert!(
        failed.last_error.as_deref().unwrap_or_default().contains("no candidates"),
        "the row says why it stopped: {:?}",
        failed.last_error
    );
    assert_eq!(
        world.qbit().adds,
        vec![HASH_A, HASH_B],
        "each candidate was added once, never twice"
    );

    let questions =
        QuestionStore::of_kind(&world.db, QuestionKind::AllFailed, world.clock.now())
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
    assert_eq!(retried.state, JobState::Search, "failed → search on a retry");
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
    assert_eq!(blocked.blocked_from, Some(JobState::Add), "it returns to add");
    let reason = blocked.last_error.as_deref().unwrap_or_default();
    assert!(reason.contains("space"), "the row says why: {reason}");
    assert!(
        world.qbit().adds.is_empty(),
        "a torrent it cannot store is never added"
    );

    // The resource stops the scheduler as a whole.
    let report = scheduler.tick().await.expect("tick");
    assert!(!report.paused, "workers are not paused, the resource is out");
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
    assert_eq!(state_of(&world.db, job), JobState::Monitor, "the add went through");
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
    let paused = report.resource_reason.expect("the queue waits on the folder");
    assert!(paused.contains("missing"), "{paused}");
    assert!(report.dispatched.is_empty());

    std::fs::create_dir_all(&folder).expect("the folder now exists");
    let report = scheduler.tick().await.expect("tick");
    assert!(report.resource_reason.is_none());
    assert_eq!(report.resumed, vec![(job, JobState::Add)]);
    scheduler.settle().await;

    let state = drive(&engine, job).await;
    assert_eq!(state, JobState::Done, "folder back, job finished");
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
    assert_eq!(blocked.state, JobState::Blocked, "a dead service parks, it does not fail");
    assert_eq!(blocked.blocked_from, Some(JobState::Search));
    assert_eq!(blocked.attempts, 0, "a blocked job never burns an attempt");
    let reason = blocked.last_error.as_deref().unwrap_or_default();
    assert!(reason.contains("Nyaa"), "the row says which service: {reason}");
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
    let seen_while_running = world.server.received_requests().await.expect("requests").len();

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
        world.server.received_requests().await.expect("requests").len(),
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
    assert!(preview.awaiting_confirmation, "one confirmation is required");
    assert_eq!(preview.episodes.len(), 2, "both shows are wanted at episode 5");
    assert_eq!(job_count(&world), 0, "a preview queues nothing");
    assert!(
        planner::pending_preview(&world.db).expect("pending").is_some(),
        "the preview is waiting for the wizard"
    );

    // Confirming under the first-sync cap queues what the cap allows.
    let mut settings = Settings::load(&world.db).expect("settings");
    settings.first_sync_cap = 1;
    settings.save(&world.db).expect("settings");
    assert_eq!(planner::confirm(&world.db, &world.clock, &list).expect("confirm"), 1);
    assert_eq!(job_count(&world), 1, "the cap held");
    assert!(
        planner::pending_preview(&world.db).expect("pending").is_none(),
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
    let again = jobs::enqueue_search(&world.db, &world.clock, rows[0].episode_id)
        .expect("enqueue");
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
            assert_eq!(preview.over_cap, 1, "the preview reports what the cap dropped");
        }
        other => panic!("expected an applied plan, got {other:?}"),
    }
    assert_eq!(job_count(&world), 1, "only the cap's worth of jobs exist");
}
