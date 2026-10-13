//! The five steps: search, choose, add, monitor, finalize.
//!
//! Each step is one async function on [`Engine`] that (a) reads what it needs,
//! (b) does its work through the [`Gate`], and (c) writes the *next* transition
//! before returning. Because a transition lands before the work it enables, a
//! process killed anywhere inside a step leaves the job in the state that step
//! was about to run — so [`crate::queue::reconcile`] can finish the job by
//! re-running it, and every step is idempotent by construction:
//!
//! * `search` writes `torrents` rows keyed by `info_hash` (re-seeing a release
//!   updates it);
//! * `choose` re-derives its verdict from those rows — no network, so a crash
//!   costs nothing;
//! * `add` re-adding the same magnet is a no-op in qBittorrent, keyed by hash;
//! * `monitor` only reads and timestamps;
//! * `finalize` writes the episode row once and calls the library hook, which
//!   the phase-6 implementation makes idempotent (a size-and-existence check
//!   guards the no-op used here).

use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use rusqlite::OptionalExtension;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::http::{Gate, Service};
use crate::jobs::{self, MoveOptions};
use crate::matching::{self, TargetInputs};
use crate::models::{Job, JobState, Question};
use crate::planner;
use crate::qbit;
use crate::questions::{Answer, Choice, ExpectedShow, MatchOption, Payload, QuestionStore};
use crate::rules::Rules;
use crate::score::Candidate;
use crate::settings::Settings;

/// How often the monitor step re-reads qBittorrent.
pub const MONITOR_POLL_SECS: i64 = 10;

/// qBittorrent's category for everything this app adds.
pub const CATEGORY: &str = "genrl";

/// Where disk facts come from.
pub trait DiskProbe: Send + Sync {
    /// Free bytes on the filesystem holding `folder`; `None` when unknown.
    fn free_bytes(&self, folder: &Path) -> Option<u64>;
}

/// The real filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct FsDiskProbe;

impl DiskProbe for FsDiskProbe {
    fn free_bytes(&self, folder: &Path) -> Option<u64> {
        if !folder.exists() {
            return None;
        }
        fs2::available_space(folder).ok()
    }
}

/// A probe tests control.
#[derive(Debug, Default)]
pub struct FakeDiskProbe {
    free: AtomicI64,
}

impl FakeDiskProbe {
    pub fn new(free_bytes: u64) -> Self {
        FakeDiskProbe {
            free: AtomicI64::new(free_bytes.min(i64::MAX as u64) as i64),
        }
    }

    pub fn set(&self, free_bytes: u64) {
        self.free.store(free_bytes.min(i64::MAX as u64) as i64, Ordering::SeqCst);
    }
}

impl DiskProbe for FakeDiskProbe {
    fn free_bytes(&self, _folder: &Path) -> Option<u64> {
        let free = self.free.load(Ordering::SeqCst);
        if free < 0 {
            None
        } else {
            Some(free as u64)
        }
    }
}

use std::future::Future;
use std::pin::Pin;

/// What the library step does with a finished download.
///
/// Phase 6 implements this (rename/move, synopsis, poster, folder icon). Until
/// then [`NoopFinalizer`] leaves the file where qBittorrent saved it, and the
/// queue stops at "completed, file verified".
#[derive(Debug, Clone, PartialEq)]
pub struct FinalizeSuccess {
    pub destination_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FinalizeOutcome {
    Done(FinalizeSuccess),
    NeedsInput { question_id: i64 },
}

pub trait Finalizer: Send + Sync {
    fn finalize<'a>(
        &'a self,
        context: &'a FinalizeContext<'_>,
    ) -> Pin<Box<dyn Future<Output = Result<FinalizeOutcome>> + Send + 'a>>;
}

/// Everything the library step needs, owned by the caller.
pub struct FinalizeContext<'a> {
    pub job: &'a Job,
    pub episode_id: i64,
    pub mal_id: i64,
    /// Where qBittorrent put the file.
    pub content_path: &'a str,
    pub size_bytes: i64,
}

/// The phase-4 finalizer: verify, then stop.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopFinalizer;

impl Finalizer for NoopFinalizer {
    fn finalize<'a>(
        &'a self,
        context: &'a FinalizeContext<'_>,
    ) -> Pin<Box<dyn Future<Output = Result<FinalizeOutcome>> + Send + 'a>> {
        let dest = context.content_path.to_string();
        Box::pin(async move {
            Ok(FinalizeOutcome::Done(FinalizeSuccess {
                destination_path: dest,
            }))
        })
    }
}

/// One step's outcome, for the scheduler's report and the tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepOutcome {
    pub job_id: i64,
    pub from: JobState,
    pub to: JobState,
}

/// The context a step works in.
#[derive(Debug, Clone)]
struct JobContext {
    mal_id: i64,
    number: f64,
    title: String,
}

/// The worker engine: one step at a time, no background tasks of its own.
pub struct Engine {
    db: Db,
    gate: Arc<Gate>,
    clock: Arc<dyn Clock>,
    finalizer: Arc<dyn Finalizer>,
    disk: Arc<dyn DiskProbe>,
}

impl Engine {
    pub fn new(
        db: Db,
        gate: Arc<Gate>,
        clock: Arc<dyn Clock>,
        finalizer: Arc<dyn Finalizer>,
        disk: Arc<dyn DiskProbe>,
    ) -> Self {
        Engine {
            db,
            gate,
            clock,
            finalizer,
            disk,
        }
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn gate(&self) -> &Gate {
        &self.gate
    }

    pub fn clock(&self) -> &dyn Clock {
        self.clock.as_ref()
    }

    /// Restart the stall timer from now.
    ///
    /// Crash recovery calls this: progress was just observed, so the twenty
    /// minutes of silence a crash may have caused must not read as a stall.
    pub fn restart_stall_timer(&self, job: &Job) {
        let _ = self.db.with(|c| {
            c.execute(
                "UPDATE jobs SET last_progress_at = ?2 WHERE id = ?1",
                rusqlite::params![job.id, self.clock.now()],
            )?;
            Ok(())
        });
    }

    /// Run one step of one job.
    ///
    /// The job's state names the step; an unknown or side state is an error
    /// rather than a silent no-op, so the scheduler never spins on a job it
    /// cannot advance.
    pub async fn step(&self, job_id: i64) -> Result<StepOutcome> {
        let Some(job) = jobs::get(&self.db, job_id)? else {
            return Err(Error::Other(format!("no job with id {job_id}")));
        };
        let from = job.state;
        let _ = match from {
            JobState::Search => self.search_step(&job).await?,
            JobState::Choose => self.choose_step(&job).await?,
            JobState::Add => self.add_step(&job).await?,
            JobState::Monitor => self.monitor_step(&job).await?,
            JobState::Finalize => self.finalize_step(&job).await?,
            other => {
                return Err(Error::Other(format!(
                    "job {job_id} is {other:?}: only the five main steps run work"
                )))
            }
        };
        let to = jobs::get(&self.db, job_id)?
            .map(|j| j.state)
            .unwrap_or(from);
        Ok(StepOutcome { job_id, from, to })
    }

    /// A global reason no new job may start, or `None` when the queue may run.
    ///
    /// The design pauses the queue below the disk floor or when the download
    /// folder is missing; qBittorrent being unreachable is per-job (blocked),
    /// because a running download is not ours to stop.
    pub fn blocking_reason(&self, settings: &Settings) -> Option<String> {
        let folder = &settings.download_folder;
        if folder.is_empty() {
            return Some("no download folder is configured".to_string());
        }
        match self.disk.free_bytes(Path::new(folder)) {
            None => Some(format!("download folder {folder} is missing")),
            Some(free) => {
                let floor = settings.disk_floor_gb.saturating_mul(1024 * 1024 * 1024);
                if free < floor {
                    Some(format!(
                        "free space {} GiB is below the {} GiB floor",
                        free / (1024 * 1024 * 1024),
                        settings.disk_floor_gb
                    ))
                } else {
                    None
                }
            }
        }
    }

    /// Park a job in `blocked`, remembering where it returns to.
    fn block(&self, job: &Job, reason: &str) -> Result<JobState> {
        jobs::transition(
            &self.db,
            self.clock.as_ref(),
            job.id,
            JobState::Blocked,
            reason,
            &MoveOptions::blocked(job.state),
        )
    }

    /// Set the chosen candidate on a job (before the transition that uses it).
    fn set_chosen(&self, job_id: i64, info_hash: &str) -> Result<()> {
        self.db.with(|c| {
            c.execute(
                "UPDATE jobs SET torrent_id = (SELECT id FROM torrents WHERE info_hash = ?2)
                 WHERE id = ?1",
                rusqlite::params![job_id, info_hash],
            )?;
            Ok(())
        })
    }

    fn context(&self, job: &Job) -> Result<JobContext> {
        let (mal_id, number, title) = self.db.with(|c| {
            Ok(c.query_row(
                "SELECT e.mal_id, e.number, s.title
                 FROM episodes e JOIN shows s ON s.mal_id = e.mal_id
                 WHERE e.id = ?1",
                [job.episode_id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?, r.get::<_, String>(2)?)),
            )?)
        })?;
        Ok(JobContext {
            mal_id,
            number,
            title,
        })
    }

    fn now(&self) -> i64 {
        self.clock.now()
    }

    // ------------------------------------------------------------------ search

    async fn search_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let settings = Settings::load(&self.db)?;
        let inputs = TargetInputs::load(&self.db, context.mal_id)?;

        // A service that is out is not a failure: wait for the breaker. The
        // pre-check is non-consuming, so when the cooldown elapses the next
        // request claims the half-open probe and closes it on success.
        if !self.gate.would_allow(Service::Nyaa) {
            return self.block(job, "Nyaa's circuit breaker is open");
        }

        let candidates = match matching::search_candidates(
            &self.gate,
            &settings,
            &inputs.names,
            50,
            25,
        )
        .await
        {
            Ok(candidates) => candidates,
            Err(err) if err.is_transient() || err.is_connection_failure() => {
                return self.block(job, &format!("Nyaa unreachable: {err}"));
            }
            Err(err) => return self.fail_or_reschedule(job, &format!("search failed: {err}")),
        };

        if candidates.is_empty() {
            return self.fail_or_reschedule(job, "no candidates on any of the show's names");
        }

        let now = self.now();
        for candidate in &candidates {
            jobs::remember_candidate(
                &self.db,
                self.clock.as_ref(),
                &jobs::RememberedCandidate {
                    info_hash: candidate.info_hash.clone(),
                    name: candidate.release.raw.clone(),
                    magnet: candidate.magnet.clone(),
                    seeders: candidate.seeders as i64,
                    leechers: candidate.leechers as i64,
                    size_bytes: candidate.size_bytes as i64,
                    group: candidate.release.group.clone(),
                    resolution: candidate.release.resolution.clone(),
                    source_url: None,
                    score: 0.0,
                    mal_id: Some(context.mal_id),
                },
            )?;
        }
        crate::events::EventLog::new(self.db.clone()).info(
            Some(job.id),
            Some(context.mal_id),
            format!(
                "found {} candidate{} for episode {} of {}",
                candidates.len(),
                if candidates.len() == 1 { "" } else { "s" },
                context.number,
                context.title
            ),
        )?;
        let _ = now;

        jobs::transition(
            &self.db,
            self.clock.as_ref(),
            job.id,
            JobState::Choose,
            "candidates remembered",
            &MoveOptions::default(),
        )
    }

    // ------------------------------------------------------------------ choose

    async fn choose_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let settings = Settings::load(&self.db)?;
        let rules = Rules::for_show(&self.db, context.mal_id)?;
        let inputs = TargetInputs::load(&self.db, context.mal_id)?;

        let rows = self.candidate_rows(context.mal_id)?;
        if rows.is_empty() {
            return self.fail_or_reschedule(job, "no candidates left to choose from");
        }

        let (ranked, rejected) = matching::rank(
            rows,
            &inputs,
            Some(context.number),
            &rules,
            &settings,
            self.now(),
        );

        // Scores back into the table, so the Queue page can show why.
        for entry in &ranked {
            self.db.with(|c| {
                c.execute(
                    "UPDATE torrents SET score = ?1 WHERE info_hash = ?2",
                    rusqlite::params![entry.scored.signal, entry.candidate.info_hash],
                )?;
                Ok(())
            })?;
        }

        // Only batches available while the show is still airing: ask.
        if !inputs.fully_aired && !ranked.is_empty() && ranked.iter().all(|r| r.candidate.release.is_batch)
        {
            let best = &ranked[0];
            let payload = Payload::BatchOnly {
                release: best.candidate.release.raw.clone(),
                seeders: best.candidate.seeders,
            };
            self.set_chosen(job.id, &best.candidate.info_hash)?;
            self.ask(job, context.mal_id, payload, "only a batch is available")?;
            return Ok(JobState::NeedsInput);
        }

        match matching::decide_verdict(ranked, rejected, &inputs, &rules, &settings) {
            matching::Verdict::Auto { chosen, thin } => {
                self.set_chosen(job.id, &chosen.candidate.info_hash)?;
                let note = format!(
                    "chose {} (score {:.0}){}",
                    chosen.candidate.release.raw,
                    chosen.scored.signal,
                    if thin { " with thin seeds, flagged" } else { "" }
                );
                jobs::transition(
                    &self.db,
                    self.clock.as_ref(),
                    job.id,
                    JobState::Add,
                    &note,
                    &MoveOptions::default(),
                )
            }
            matching::Verdict::Ask {
                reason: matching::AskReason::ThinSeeds { seeders },
                ranked,
            } => {
                let best = &ranked[0];
                self.set_chosen(job.id, &best.candidate.info_hash)?;
                let payload = Payload::LowSeeds {
                    seeders,
                    release: best.candidate.release.raw.clone(),
                    magnet: best.candidate.magnet.clone(),
                    auto_list: inputs.auto,
                };
                self.ask(job, context.mal_id, payload, &format!("only {seeders} seeders"))?;
                Ok(JobState::NeedsInput)
            }
            matching::Verdict::Ask { reason, ranked } => {
                let options: Vec<MatchOption> =
                    ranked.iter().take(4).map(|r| r.to_option()).collect();
                let payload = Payload::MatchReview {
                    expected: inputs.expected_show(),
                    options,
                    reason: reason.to_string(),
                };
                self.ask(job, context.mal_id, payload, &format!("{reason}"))?;
                Ok(JobState::NeedsInput)
            }
            matching::Verdict::NotFound { rejections } => {
                let reasons = rejections
                    .iter()
                    .map(|(_, rejection)| rejection.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                self.fail_or_reschedule(job, &format!("nothing usable: {reasons}"))
            }
        }
    }

    // --------------------------------------------------------------------- add

    async fn add_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let settings = Settings::load(&self.db)?;

        // Resources first: the design adds no torrent it cannot store.
        if let Some(reason) = self.blocking_reason(&settings) {
            return self.block(job, &reason);
        }

        let health = match qbit::ensure_running(&self.gate, &settings).await {
            Ok(health) => health,
            Err(err) if err.is_connection_failure() => {
                return self.block(job, &format!("qBittorrent is not reachable: {err}"));
            }
            Err(err) => {
                // One Inbox notice for an auth problem, not one per job.
                if matches!(err, Error::Auth { .. }) {
                    self.ensure_service_notice(context.mal_id, &err.to_string())?;
                }
                return self.block(job, &format!("qBittorrent is not usable: {err}"));
            }
        };

        let Some(torrent) = self.chosen_torrent(job)? else {
            return jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Choose,
                "no chosen candidate; choosing again",
                &MoveOptions::default(),
            );
        };

        let client = qbit::Qbit::from_settings(&settings)?;
        match client
            .add(
                &self.gate,
                &[torrent.magnet.clone()],
                Some(&settings.download_folder),
                Some(CATEGORY),
            )
            .await
        {
            Ok(()) => {}
            Err(err) if err.is_transient() || err.is_connection_failure() => {
                return self.block(job, &format!("qBittorrent refused the add: {err}"));
            }
            Err(err) => return self.fail_or_reschedule(job, &format!("add failed: {err}")),
        }

        crate::events::EventLog::new(self.db.clone()).info(
            Some(job.id),
            Some(context.mal_id),
            format!(
                "added {} to qBittorrent {}",
                torrent.name, health.version
            ),
        )?;

        jobs::transition(
            &self.db,
            self.clock.as_ref(),
            job.id,
            JobState::Monitor,
            &format!("added {}", torrent.name),
            &MoveOptions::default(),
        )
    }

    // ----------------------------------------------------------------- monitor

    async fn monitor_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let settings = Settings::load(&self.db)?;
        let Some(torrent) = self.chosen_torrent(job)? else {
            return jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Choose,
                "no torrent to monitor; choosing again",
                &MoveOptions::default(),
            );
        };

        let client = qbit::Qbit::from_settings(&settings)?;
        let info = match client.get_torrent(&self.gate, &torrent.info_hash).await {
            Ok(info) => info,
            Err(err) if err.is_transient() || err.is_connection_failure() => {
                return self.block(job, &format!("qBittorrent unreachable while monitoring: {err}"));
            }
            Err(err) => return self.fail_or_reschedule(job, &format!("monitor failed: {err}")),
        };

        let Some(info) = info else {
            // Gone from qBittorrent (removed by hand, say): try the next one.
            return self.next_candidate(job, context.mal_id, "the torrent left qBittorrent");
        };

        if info.is_complete() {
            return jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Finalize,
                &format!("{} finished", info.name),
                &MoveOptions::default(),
            );
        }

        let moved = info.progress > job.progress + f64::EPSILON;
        jobs::note_progress(&self.db, self.clock.as_ref(), job.id, info.progress, moved)?;
        if !moved {
            let since = job.last_progress_at.unwrap_or(job.updated_at);
            let stalled_for = self.now() - since;
            if stalled_for >= i64::from(settings.stall_timeout_min) * 60 {
                // Remove it, mark the candidate tried, and look for the next.
                let _ = client.remove(&self.gate, &torrent.info_hash, false).await;
                jobs::reject_candidate(&self.db, &torrent.info_hash)?;
                return self.next_candidate(
                    job,
                    context.mal_id,
                    &format!(
                        "no progress for {} minutes; removed {} and trying the next candidate",
                        settings.stall_timeout_min, info.name
                    ),
                );
            }
        }

        jobs::reschedule(
            &self.db,
            self.clock.as_ref(),
            job.id,
            self.now() + MONITOR_POLL_SECS,
            &format!(
                "{} at {:.0}%",
                info.state,
                info.progress * 100.0
            ),
        )?;
        Ok(JobState::Monitor)
    }

    // ---------------------------------------------------------------- finalize

    async fn finalize_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let settings = Settings::load(&self.db)?;
        let Some(torrent) = self.chosen_torrent(job)? else {
            return jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Search,
                "nothing to finalize; searching again",
                &MoveOptions::default(),
            );
        };

        let client = qbit::Qbit::from_settings(&settings)?;
        let info = match client.get_torrent(&self.gate, &torrent.info_hash).await {
            Ok(info) => info,
            Err(err) if err.is_transient() || err.is_connection_failure() => {
                return self.block(job, &format!("qBittorrent unreachable while verifying: {err}"));
            }
            Err(err) => return self.fail_or_reschedule(job, &format!("verify failed: {err}")),
        };

        // The file on disk is the fact that matters; qBittorrent forgetting the
        // torrent must not lose a finished download.
        let (content_path, size_bytes) = match &info {
            Some(info) if info.is_complete() => {
                (info.content_path.clone(), info.size.max(0))
            }
            Some(_) => {
                return jobs::reschedule(
                    &self.db,
                    self.clock.as_ref(),
                    job.id,
                    self.now() + MONITOR_POLL_SECS,
                    "still downloading; waiting",
                )
                .map(|()| JobState::Finalize);
            }
            None => {
                let path = Path::new(&settings.download_folder).join(&torrent.name);
                match std::fs::metadata(&path) {
                    Ok(meta) if meta.len() > 0 => {
                        (path.display().to_string(), meta.len() as i64)
                    }
                    _ => {
                        return jobs::transition(
                            &self.db,
                            self.clock.as_ref(),
                            job.id,
                            JobState::Search,
                            "the finished file is gone; searching again",
                            &MoveOptions::default(),
                        )
                    }
                }
            }
        };

        // Verify: size matches, and the extension is a video type.
        let verified = std::fs::metadata(&content_path)
            .map(|meta| meta.len() as i64 == size_bytes.max(0) || size_bytes == 0)
            .unwrap_or(false);
        if !verified {
            return self.fail_or_reschedule(
                job,
                &format!("{content_path} does not verify against the torrent"),
            );
        }

        let outcome = self
            .finalizer
            .finalize(&FinalizeContext {
                job,
                episode_id: job.episode_id,
                mal_id: context.mal_id,
                content_path: &content_path,
                size_bytes,
            })
            .await;
        let dest_path = match outcome {
            Ok(FinalizeOutcome::Done(success)) => success.destination_path,
            Ok(FinalizeOutcome::NeedsInput { .. }) => {
                return jobs::transition(
                    &self.db,
                    self.clock.as_ref(),
                    job.id,
                    JobState::NeedsInput,
                    "destination file exists; waiting for user choice",
                    &MoveOptions::default(),
                );
            }
            Err(err) if err.is_transient() || matches!(err, Error::Io(_)) => {
                return self.block(job, &format!("library step could not file it: {err}"));
            }
            Err(err) => return self.fail_or_reschedule(job, &format!("finalize failed: {err}")),
        };

        planner::mark_done(&self.db, job.episode_id, &dest_path, size_bytes)?;

        if settings.remove_when_finished {
            let client = qbit::Qbit::from_settings(&settings)?;
            let _ = client.remove(&self.gate, &torrent.info_hash, false).await;
        }

        crate::events::EventLog::new(self.db.clone()).info(
            Some(job.id),
            Some(context.mal_id),
            format!("verified {} and filed it to {}", content_path, dest_path),
        )?;

        jobs::transition(
            &self.db,
            self.clock.as_ref(),
            job.id,
            JobState::Done,
            "episode owned",
            &MoveOptions::default(),
        )
    }

    // ------------------------------------------------------------------ helpers

    /// Back off, or give up and ask: whichever comes first.
    fn fail_or_reschedule(&self, job: &Job, reason: &str) -> Result<JobState> {
        let settings = Settings::load(&self.db)?;
        if job.attempts + 1 >= i64::from(settings.max_failed_candidates) {
            let payload = Payload::AllFailed {
                candidates_tried: tried(job.attempts + 1),
            };
            self.ask(job, self.episode_mal_id(job)?, payload, reason)?;
            jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Failed,
                reason,
                &MoveOptions {
                    count_attempt: true,
                    ..Default::default()
                },
            )
        } else {
            jobs::reschedule(
                &self.db,
                self.clock.as_ref(),
                job.id,
                self.now() + jobs::backoff_delay(job.attempts + 1).as_secs() as i64,
                reason,
            )?;
            Ok(job.state)
        }
    }

    /// Stall or vanished torrent: move to the next candidate, or ask.
    fn next_candidate(&self, job: &Job, mal_id: i64, reason: &str) -> Result<JobState> {
        let settings = Settings::load(&self.db)?;
        if job.attempts + 1 >= i64::from(settings.max_failed_candidates) {
            let payload = Payload::AllFailed {
                candidates_tried: tried(job.attempts + 1),
            };
            // Out of candidates and out of attempts: a failed job, and — per
            // the design — a failed job raises a question too.
            self.ask(job, mal_id, payload, reason)?;
            jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Failed,
                reason,
                &MoveOptions {
                    count_attempt: true,
                    clear_torrent: true,
                    ..Default::default()
                },
            )
        } else {
            jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Search,
                reason,
                &MoveOptions {
                    count_attempt: true,
                    clear_torrent: true,
                    ..Default::default()
                },
            )
        }
    }

    /// Queue a question and park the job in `needs_input`.
    fn ask(&self, job: &Job, mal_id: i64, payload: Payload, reason: &str) -> Result<i64> {
        let settings = Settings::load(&self.db)?;
        let now = self.now();
        let id = QuestionStore::ask(
            &self.db,
            Some(job.id),
            Some(mal_id),
            &payload,
            now,
            Some(i64::from(settings.question_expiry_hours) * 3600),
        )?;
        // The reason is the Activity line: every ask is visible.
        crate::events::EventLog::new(self.db.clone()).warn(
            Some(job.id),
            Some(mal_id),
            format!("asked a question (#{id}): {reason}"),
        )?;
        Ok(id)
    }

    /// One Inbox notice per service, never one per job.
    fn ensure_service_notice(&self, mal_id: i64, detail: &str) -> Result<()> {
        let existing = QuestionStore::of_kind(
            &self.db,
            crate::models::QuestionKind::ServiceNotice,
            self.now(),
        )?;
        if existing
            .iter()
            .any(|question| matches!(&question.payload, Payload::ServiceNotice { service, .. } if service == "qBittorrent"))
        {
            return Ok(());
        }
        QuestionStore::ask(
            &self.db,
            None,
            Some(mal_id),
            &Payload::ServiceNotice {
                service: "qBittorrent".to_string(),
                detail: detail.to_string(),
            },
            self.now(),
            None,
        )?;
        Ok(())
    }

    fn episode_mal_id(&self, job: &Job) -> Result<i64> {
        self.db.with(|c| {
            Ok(c.query_row(
                "SELECT mal_id FROM episodes WHERE id = ?1",
                [job.episode_id],
                |r| r.get(0),
            )?)
        })
    }

    fn chosen_torrent(&self, job: &Job) -> Result<Option<ChosenTorrent>> {
        let Some(torrent_id) = job.torrent_id else {
            return Ok(None);
        };
        self.db.with(|c| {
            Ok(c
                .query_row(
                    "SELECT info_hash, name, magnet FROM torrents WHERE id = ?1",
                    [torrent_id],
                    |r| {
                        Ok(ChosenTorrent {
                            info_hash: r.get(0)?,
                            name: r.get(1)?,
                            magnet: r.get(2)?,
                        })
                    },
                )
                .optional()?)
        })
    }

    /// The show's non-rejected candidates, newest first, as scoring inputs.
    fn candidate_rows(&self, mal_id: i64) -> Result<Vec<Candidate>> {
        self.db.with_rusqlite(|c| {
            let mut stmt = c.prepare(
                "SELECT name, info_hash, magnet, seeders, leechers, size, \"group\",
                        resolution, trusted, found_at
                 FROM torrents WHERE mal_id = ?1 AND rejected = 0
                 ORDER BY found_at DESC, seeders DESC LIMIT 50",
            )?;
            let rows = stmt.query_map([mal_id], |r| {
                let name: String = r.get(0)?;
                let release = crate::filename::parse_release(&name);
                Ok(Candidate {
                    release,
                    info_hash: r.get(1)?,
                    magnet: r.get(2)?,
                    seeders: r.get::<_, i64>(3)? as u32,
                    leechers: r.get::<_, i64>(4)? as u32,
                    trusted: r.get::<_, i64>(8)? != 0,
                    remake: false,
                    size_bytes: r.get::<_, i64>(5)? as u64,
                    published_at: r.get(9)?,
                    uploader: None,
                    show_type: None,
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }
}

/// The torrent a job chose.
#[derive(Debug, Clone)]
struct ChosenTorrent {
    info_hash: String,
    name: String,
    magnet: String,
}

/// Apply an Inbox answer to the job that asked.
///
/// The design's answers are verbs on the queue: "use this one" resumes at add,
/// "none of these" parks the episode as missing, "wait 24 h" defers, and the
/// "always do this" half of the answer is what the caller stores (an alias or
/// a rule) — see [`crate::names::NameSet`].
pub fn apply_answer(
    db: &Db,
    clock: &dyn Clock,
    question: &Question,
    answer: &Answer,
) -> Result<Option<JobState>> {
    let Some(job_id) = question.job_id else {
        return Ok(None);
    };
    let Some(job) = jobs::get(db, job_id)? else {
        return Ok(None);
    };
    // Only a job actually waiting on the Inbox may be answered; anything else
    // was answered, expired or moved on, and applying twice must be safe.
    if !matches!(job.state, JobState::NeedsInput | JobState::Failed) {
        return Ok(Some(job.state));
    }
    let mal_id = question.mal_id;
    let now = clock.now();

    // A saved alias ("always do this") lands before the state moves, so the
    // next search starts from it.
    if answer.always {
        if let Choice::Alias { name } = &answer.choice {
            if let Some(mal_id) = mal_id {
                names_alias(db, mal_id, name)?;
            }
        }
    }

    let next = match &answer.choice {
        Choice::UseThis { info_hash } => {
            db.with(|c| {
                c.execute(
                    "UPDATE jobs SET torrent_id = (SELECT id FROM torrents WHERE info_hash = ?2)
                     WHERE id = ?1",
                    rusqlite::params![job_id, info_hash],
                )?;
                Ok(())
            })?;
            Some(JobState::Add)
        }
        Choice::NoneOfThese => {
            // Parked: the episode returns to missing and the weekly plan can
            // offer it again (the user can edit aliases meanwhile).
            Some(JobState::Failed)
        }
        Choice::SearchAgain => Some(JobState::Search),
        Choice::Alias { name } => {
            if let Some(mal_id) = mal_id {
                names_alias(db, mal_id, name)?;
            }
            Some(JobState::Search)
        }
        Choice::Download => Some(JobState::Add),
        Choice::Wait => {
            jobs::reschedule(db, clock, job_id, now + 24 * 3600, "waiting 24 h for seeds")?;
            Some(JobState::Search)
        }
        Choice::PickAnother => Some(JobState::Search),
        Choice::Keep => {
            // Already on disk: the episode is owned, the job is done.
            let (episode_id, path): (i64, Option<String>) = db.with(|c| {
                Ok(c.query_row(
                    "SELECT j.episode_id, e.file_path FROM jobs j JOIN episodes e ON e.id = j.episode_id
                     WHERE j.id = ?1",
                    [job_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?)
            })?;
            let size = path
                .as_deref()
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            planner::mark_done(db, episode_id, path.as_deref().unwrap_or(""), size)?;
            Some(JobState::Done)
        }
        Choice::Replace | Choice::KeepBoth => {
            if jobs::is_allowed(job.state, JobState::Finalize) {
                Some(JobState::Finalize)
            } else {
                Some(JobState::Add)
            }
        }
        Choice::Batch => Some(JobState::Add),
        Choice::WaitForSingles => Some(JobState::Search),
        Choice::Pause => {
            // The global pause switch: the scheduler stops, downloads continue.
            let mut settings = Settings::load(db)?;
            settings.automation_on = false;
            Settings::save(&settings, db)?;
            Some(JobState::NeedsInput)
        }
        // Changing the folder is the settings page's job; the job keeps waiting
        // so the human can answer again once a folder that works exists.
        Choice::ChangeFolder => None,
        Choice::RetryLater => {
            // Answered on a failed job: a fresh attempt at the first step.
            jobs::transition(
                db,
                clock,
                job_id,
                JobState::Search,
                "retry later",
                &MoveOptions {
                    reset_attempts: true,
                    ..Default::default()
                },
            )?;
            return Ok(Some(JobState::Search));
        }
        Choice::ManualSearch => None,
        Choice::SkipEpisode => Some(JobState::Failed),
        Choice::Expired => None,
    };

    let Some(next) = next else {
        return Ok(Some(job.state));
    };

    // An answer that would move the job somewhere the diagram does not allow
    // is ignored rather than errored: a `failed` job's AllFailed question only
    // offers retry-later / manual-search / skip, and the rest would need a
    // transition that does not exist (failed → add, say).
    if next != job.state && !jobs::is_allowed(job.state, next) {
        return Ok(Some(job.state));
    }

    let opts = MoveOptions {
        reset_attempts: matches!(next, JobState::Search),
        clear_torrent: matches!(
            answer.choice,
            Choice::PickAnother | Choice::NoneOfThese
        ),
        blocked_from: None,
        count_attempt: false,
    };
    jobs::transition(
        db,
        clock,
        job_id,
        next,
        &format!("answered: {}", answer_label(&answer.choice)),
        &opts,
    )?;
    Ok(Some(next))
}

fn answer_label(choice: &Choice) -> String {
    match choice {
        Choice::UseThis { .. } => "use this one".to_string(),
        Choice::NoneOfThese => "none of these".to_string(),
        Choice::SearchAgain => "search again".to_string(),
        Choice::Alias { name } => format!("alias {name}"),
        Choice::Download => "download anyway".to_string(),
        Choice::Wait => "wait 24 h".to_string(),
        Choice::PickAnother => "pick another".to_string(),
        Choice::Keep => "keep what is on disk".to_string(),
        Choice::Replace => "replace".to_string(),
        Choice::KeepBoth => "keep both".to_string(),
        Choice::Batch => "take the batch".to_string(),
        Choice::WaitForSingles => "wait for singles".to_string(),
        Choice::Pause => "pause the queue".to_string(),
        Choice::ChangeFolder => "choose another folder".to_string(),
        Choice::RetryLater => "retry later".to_string(),
        Choice::ManualSearch => "manual search".to_string(),
        Choice::SkipEpisode => "skip this episode".to_string(),
        Choice::Expired => "expired".to_string(),
    }
}

/// Attempts are a counter in the database; a question payload counts them as
/// a `u32`, and a nonsensical value must not wrap.
fn tried(attempts: i64) -> u32 {
    attempts.clamp(0, u32::MAX as i64) as u32
}

fn names_alias(db: &Db, mal_id: i64, name: &str) -> Result<()> {
    crate::names::NameSet::add(
        db,
        mal_id,
        name,
        crate::models::NameSource::User,
        crate::names::PRIORITY_USER,
    )
    .map(|_| ())
}

/// The expected-show card for a question (used by the UI and tests).
pub fn expected_show(mal_id: i64, title: &str, english_title: Option<String>) -> ExpectedShow {
    ExpectedShow {
        mal_id,
        title: title.to_string(),
        english_title,
        year: None,
    }
}
