use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use tokio::task::JoinHandle;

use crate::error::Result;
use crate::events::EventLog;
use crate::http::{Gate, Service};
use crate::jobs;
use crate::models::{JobKind, JobState};
use crate::settings::Settings;
use crate::worker::Engine;

use super::status::sweep;

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
