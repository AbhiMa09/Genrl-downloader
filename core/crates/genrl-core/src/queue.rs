//! The scheduler, and the launch-time reconcile that survives being killed.
//!
//! The scheduler is deliberately thin: it picks due jobs, respects the pause
//! switch and the per-kind concurrency limits, and dispatches them to
//! [`worker::Engine::step`]. Everything interesting — state, retries, blocking —
//! lives in the steps and the transition table, so the scheduler can be
//! replaced or run headless without changing what a job means.
//!
//! [`reconcile`] is the phase's check. On launch it walks every non-terminal
//! job, compares it with qBittorrent by info hash, and resumes it:
//!
//! | what it finds | what it does |
//! |---|---|
//! | torrent present and complete | finalize |
//! | torrent present and downloading | monitor |
//! | absent after the add step | re-add by magnet (idempotent by hash) |
//! | absent before the add step | redo the step |
//!
//! Because a transition is written *before* the work it enables, every
//! possible kill point leaves the job in exactly one of these cases.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::events::EventLog;
use crate::http::{Gate, Service};
use crate::jobs;
use crate::models::{JobKind, JobState};
use crate::qbit;
use crate::questions::QuestionStore;
use crate::settings::Settings;
use crate::worker::Engine;

/// How long the run loop sleeps between ticks.
pub const TICK_SECS: u64 = 1;

/// What one tick did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TickReport {
    /// The global pause switch (settings) stopped the scheduler.
    pub paused: bool,
    /// A resource stopped the scheduler (disk, folder).
    pub resource_reason: Option<String>,
    /// Jobs picked up by this tick.
    pub dispatched: Vec<(JobKind, i64)>,
    /// Blocked jobs whose blocker cleared, and where they went back to.
    pub resumed: Vec<(i64, JobState)>,
}

/// What reconcile did with one job.
#[derive(Debug, Clone, PartialEq)]
pub struct Resume {
    pub job_id: i64,
    pub from: JobState,
    pub to: JobState,
    pub why: String,
}

/// What reconcile found overall.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReconcileReport {
    pub resumed: Vec<Resume>,
    /// Jobs left alone: waiting on a question, failed, blocked, or nothing to
    /// compare against yet.
    pub waiting: Vec<(i64, JobState)>,
}

/// The scheduler: picks due jobs, keeps the limits, spawns the steps.
pub struct Scheduler {
    engine: Arc<Engine>,
    /// Per-kind count of steps currently running.
    in_flight: Arc<Mutex<HashMap<JobKind, usize>>>,
    spawned: Mutex<Vec<JoinHandle<()>>>,
}

impl Scheduler {
    pub fn new(engine: Arc<Engine>) -> Self {
        Scheduler {
            engine,
            in_flight: Arc::new(Mutex::new(HashMap::new())),
            spawned: Mutex::new(Vec::new()),
        }
    }

    pub fn engine(&self) -> &Arc<Engine> {
        &self.engine
    }

    /// Per-kind concurrency: searches share the Nyaa budget, downloads share
    /// bandwidth and disk.
    fn limit_for(kind: JobKind, settings: &Settings) -> usize {
        match kind {
            JobKind::Search | JobKind::Finalize => settings.search_concurrency as usize,
            JobKind::Add | JobKind::Monitor => settings.active_downloads as usize,
        }
        .max(1)
    }

    fn in_flight(&self, kind: JobKind) -> usize {
        self.in_flight
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&kind)
            .copied()
            .unwrap_or(0)
    }

    fn in_flight_total(&self) -> usize {
        self.in_flight
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .sum()
    }

    /// One pass: resume what cleared, dispatch what is due.
    pub async fn tick(&self) -> Result<TickReport> {
        // Reap finished tasks before deciding what may run.
        self.reap();

        let settings = Settings::load(self.engine.db())?;
        if !settings.automation_on {
            // The pause switch stops the scheduler, not the downloads that are
            // already running: qBittorrent keeps going either way.
            return Ok(TickReport {
                paused: true,
                ..Default::default()
            });
        }

        let resource_reason = self.engine.blocking_reason(&settings);
        let mut resumed = Vec::new();
        if resource_reason.is_none() {
            resumed = self.resume_blocked(&settings);
        }

        let mut dispatched = Vec::new();
        if resource_reason.is_none() {
            for kind in [JobKind::Search, JobKind::Add, JobKind::Monitor, JobKind::Finalize] {
                let limit = Self::limit_for(kind, &settings);
                let room = limit.saturating_sub(self.in_flight(kind));
                if room == 0 {
                    continue;
                }
                let due = jobs::due_jobs(self.engine.db(), self.engine.clock().now(), &[kind], room)?;
                for job in due {
                    // A side state is never "due work"; only main steps run.
                    if !job.state.is_active() {
                        continue;
                    }
                    self.dispatch(job.id, kind);
                    dispatched.push((kind, job.id));
                }
            }
        }

        Ok(TickReport {
            paused: false,
            resource_reason,
            dispatched,
            resumed,
        })
    }

    /// Run until nothing is dispatched and nothing is in flight.
    ///
    /// The tests drive this instead of sleeping: every step either finishes or
    /// reschedules itself into the future, so the loop terminates on its own.
    pub async fn run_until_idle(&self) -> Result<TickReport> {
        let mut last = TickReport::default();
        for _ in 0..128 {
            last = self.tick().await?;
            self.settle().await;
            if last.dispatched.is_empty() && self.in_flight_total() == 0 {
                break;
            }
        }
        Ok(last)
    }

    /// Wait for the spawned steps to land.
    pub async fn settle(&self) {
        loop {
            let handles = {
                let mut spawned = self.spawned.lock().unwrap_or_else(|e| e.into_inner());
                std::mem::take(&mut *spawned)
            };
            let busy = self.in_flight_total();
            if handles.is_empty() && busy == 0 {
                return;
            }
            for handle in handles {
                let _ = handle.await;
            }
        }
    }

    /// The run loop: sweep, tick, sleep, repeat, until the shutdown signal.
    ///
    /// State changes are logged once, on the edge: a queue parked for hours
    /// must not write the same line into the Activity log every second.
    pub async fn run(&self) -> Result<()> {
        let log = EventLog::new(self.engine.db().clone());
        let mut last_paused: Option<bool> = None;
        let mut last_resource: Option<String> = None;
        loop {
            let _ = sweep(self.engine.db(), self.engine.clock());
            // MAL sync rides the same loop: at most every `mal_sync_interval_min`,
            // only when a refresh token is stored, and a failure never stops the
            // workers. The weekly planner takes over afterwards.
            if let Some(report) = crate::mal::sync_if_due(
                self.engine.db(),
                self.engine.gate(),
                self.engine.clock(),
                &crate::mal::KeyringStore,
            )
            .await
            {
                let log = EventLog::new(self.engine.db().clone());
                let _ = log.info(None, None, report.summary());
                let list = crate::planner::MalList {
                    entries: report.entries,
                };
                if let Ok(crate::planner::PlanOutcome::Applied { queued, .. }) =
                    crate::planner::plan(self.engine.db(), self.engine.clock(), &list, false)
                {
                    if queued > 0 {
                        let _ = log.info(
                            None,
                            None,
                            format!("planned {queued} new episode(s) from the MAL list"),
                        );
                    }
                }
            }
            let report = self.tick().await?;
            self.settle().await;

            if last_paused != Some(report.paused) {
                match (last_paused, report.paused) {
                    (Some(true), false) => {
                        let _ = log.info(None, None, "workers resumed");
                    }
                    (_, true) => {
                        let _ = log.info(
                            None,
                            None,
                            "workers are paused; downloads already in qBittorrent continue",
                        );
                    }
                    _ => {}
                }
                last_paused = Some(report.paused);
            }
            if report.resource_reason != last_resource {
                match &report.resource_reason {
                    Some(reason) => tracing::warn!("queue paused: {reason}"),
                    None if last_resource.is_some() => {
                        tracing::info!("queue resources available again")
                    }
                    None => {}
                }
                last_resource = report.resource_reason;
            }

            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(TICK_SECS)) => {}
                _ = tokio::signal::ctrl_c() => {
                    self.settle().await;
                    let _ = log.info(None, None, "workers stopped (Ctrl-C); downloads continue in qBittorrent");
                    return Ok(());
                }
            }
        }
    }

    fn dispatch(&self, job_id: i64, kind: JobKind) {
        {
            let mut in_flight = self.in_flight.lock().unwrap_or_else(|e| e.into_inner());
            *in_flight.entry(kind).or_insert(0) += 1;
        }
        let engine = Arc::clone(&self.engine);
        let in_flight = Arc::clone(&self.in_flight);
        let handle = tokio::spawn(async move {
            if let Err(err) = engine.step(job_id).await {
                // The step could not even decide: park the job for a retry
                // rather than losing it, and say why in the log.
                let _ = jobs::reschedule(
                    engine.db(),
                    engine.clock(),
                    job_id,
                    engine.clock().now() + jobs::backoff_delay(1).as_secs() as i64,
                    &format!("step errored: {err}"),
                );
            }
            let mut counts = in_flight.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(count) = counts.get_mut(&kind) {
                *count = count.saturating_sub(1);
            }
        });
        self.spawned
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(handle);
    }

    fn reap(&self) {
        let mut spawned = self.spawned.lock().unwrap_or_else(|e| e.into_inner());
        spawned.retain(|handle| !handle.is_finished());
    }

    /// Blocked jobs whose blocker cleared go back to where they came from.
    ///
    /// Disk and folder blocks are checked against the live probe. The Nyaa
    /// breaker is read from the gate. qBittorrent blocks resume *optimistically*:
    /// one failed step costs a health check and re-blocks itself, which is
    /// cheaper than a second health path with its own failure modes.
    fn resume_blocked(&self, settings: &Settings) -> Vec<(i64, JobState)> {
        let mut out = Vec::new();
        let blocked = match jobs::list(self.engine.db(), Some(JobState::Blocked), 512) {
            Ok(blocked) => blocked,
            Err(err) => {
                tracing::warn!("could not list blocked jobs: {err}");
                return out;
            }
        };
        let gate: &Gate = self.engine.gate();
        for job in blocked {
            let Some(from) = job.blocked_from else {
                continue;
            };
            let reason = job.last_error.clone().unwrap_or_default();
            let cleared = if reason.contains("Nyaa") {
                // Not "is it closed" but "may a request go out": the cooldown
                // can elapse while the job waits, and the probe that closes the
                // breaker happens on the step's own request.
                gate.would_allow(Service::Nyaa)
            } else if reason.contains("space") || reason.contains("folder") {
                self.engine.blocking_reason(settings).is_none()
            } else {
                true
            };
            if !cleared {
                continue;
            }
            match jobs::transition(
                self.engine.db(),
                self.engine.clock(),
                job.id,
                from,
                &format!("resumed after: {reason}"),
                &jobs::MoveOptions::default(),
            ) {
                Ok(_) => out.push((job.id, from)),
                Err(err) => tracing::warn!("could not resume job {}: {err}", job.id),
            }
        }
        out
    }
}

/// Launch-time recovery: compare every non-terminal job with qBittorrent and
/// resume it.
pub async fn reconcile(engine: &Engine) -> Result<ReconcileReport> {
    let settings = Settings::load(engine.db())?;
    let client = qbit::Qbit::from_settings(&settings)?;
    let health = client.health(engine.gate()).await?;

    let mut report = ReconcileReport::default();
    for job in jobs::non_terminal_jobs(engine.db())? {
        let why;
        let to;
        match job.state {
            JobState::Search | JobState::Choose => {
                // Nothing on the wire yet: the scheduler redoes the step.
                report.waiting.push((job.id, job.state));
                continue;
            }
            JobState::Add => match &job.torrent_id {
                Some(_) => {
                    let hash = hash_of(engine, &job)?;
                    match client.get_torrent(engine.gate(), &hash).await? {
                        Some(info) if info.is_complete() => {
                            why = "the download finished while the app was down".to_string();
                            to = JobState::Finalize;
                        }
                        Some(_) => {
                            why = "the download is running again".to_string();
                            to = JobState::Monitor;
                        }
                        None => {
                            // Added, then the add never confirmed: add again.
                            // The magnet is keyed by info hash, so this cannot
                            // duplicate a torrent in qBittorrent.
                            engine.step(job.id).await?;
                            continue;
                        }
                    }
                }
                None => {
                    why = "no candidate was chosen before the kill; choosing again".to_string();
                    to = JobState::Choose;
                }
            },
            JobState::Monitor => match &job.torrent_id {
                Some(_) => {
                    let hash = hash_of(engine, &job)?;
                    match client.get_torrent(engine.gate(), &hash).await? {
                        Some(info) if info.is_complete() => {
                            why = "the download finished while the app was down".to_string();
                            to = JobState::Finalize;
                        }
                        Some(_) => {
                            // Restart the stall timer from now: progress was
                            // just observed, so a crash must not look like a
                            // stall.
                            engine.restart_stall_timer(&job);
                            why = "still downloading; monitoring resumed".to_string();
                            to = JobState::Monitor;
                        }
                        None => {
                            why = "the torrent left qBittorrent; searching for the next candidate"
                                .to_string();
                            to = JobState::Search;
                        }
                    }
                }
                None => {
                    why = "no torrent was being watched; choosing again".to_string();
                    to = JobState::Choose;
                }
            },
            JobState::Finalize => {
                // Re-run it: the library step is idempotent and the episode row
                // is written once.
                engine.step(job.id).await?;
                continue;
            }
            other => {
                report.waiting.push((job.id, other));
                continue;
            }
        };

        if job.state != to {
            jobs::transition(
                engine.db(),
                engine.clock(),
                job.id,
                to,
                &why,
                &jobs::MoveOptions::default(),
            )?;
            report.resumed.push(Resume {
                job_id: job.id,
                from: job.state,
                to,
                why,
            });
        } else {
            report.waiting.push((job.id, to));
        }
    }

    let log = EventLog::new(engine.db().clone());
    log.info(
        None,
        None,
        format!(
            "reconcile: {} resumed, {} waiting (qBittorrent {})",
            report.resumed.len(),
            report.waiting.len(),
            health.version
        ),
    )?;
    Ok(report)
}

/// The info hash of a job's chosen torrent.
fn hash_of(engine: &Engine, job: &crate::models::Job) -> Result<String> {
    let Some(torrent_id) = job.torrent_id else {
        return Err(crate::Error::Other(format!(
            "job {} has no chosen torrent",
            job.id
        )));
    };
    engine.db().with(|c| {
        Ok(c.query_row(
            "SELECT info_hash FROM torrents WHERE id = ?1",
            [torrent_id],
            |r| r.get(0),
        )?)
    })
}

/// Expire stale questions and return their episodes to `missing`.
pub fn sweep(db: &Db, clock: &dyn Clock) -> Result<usize> {
    let expired = QuestionStore::expire(db, clock.now())?;
    let mut released = 0;
    for question in &expired {
        let Some(job_id) = question.job_id else {
            continue;
        };
        let Some(job) = jobs::get(db, job_id)? else {
            continue;
        };
        // A waiting job goes to failed (it asked and got nothing); a job that
        // already failed just loses its episode back to the pool.
        if job.state == JobState::NeedsInput {
            jobs::transition(
                db,
                clock,
                job_id,
                JobState::Failed,
                "question expired; the episode is missing again",
                &jobs::MoveOptions {
                    reset_attempts: true,
                    ..Default::default()
                },
            )?;
        } else if job.state != JobState::Failed {
            continue;
        }
        crate::planner::mark_missing(db, job.episode_id)?;
        released += 1;
    }
    Ok(released)
}

/// A status summary for the CLI and the status strip.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QueueStatus {
    pub jobs_by_state: HashMap<JobState, u64>,
    pub open_questions: u64,
    pub due: u64,
}

pub fn status(db: &Db, clock: &dyn Clock) -> Result<QueueStatus> {
    let mut jobs_by_state = HashMap::new();
    db.with_rusqlite(|c| {
        let mut stmt = c.prepare("SELECT state, COUNT(*) FROM jobs GROUP BY state")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                JobState::parse(&r.get::<_, String>(0)?),
                r.get::<_, i64>(1)? as u64,
            ))
        })?;
        for row in rows {
            let (state, count) = row?;
            jobs_by_state.insert(state, count);
        }
        Ok(())
    })?;
    let open_questions = QuestionStore::open_count(db, clock.now())?;
    let due = jobs::due_jobs(db, clock.now(), &[], usize::MAX)?
        .into_iter()
        .filter(|job| job.state.is_active())
        .count() as u64;
    Ok(QueueStatus {
        jobs_by_state,
        open_questions,
        due,
    })
}
