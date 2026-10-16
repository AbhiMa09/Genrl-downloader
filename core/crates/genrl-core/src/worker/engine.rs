use std::sync::Arc;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::http::Gate;
use crate::jobs::{self, MoveOptions};
use crate::models::{Job, JobState};
use crate::settings::Settings;

use super::types::{DiskProbe, Finalizer, JobContext, StepOutcome};

/// The worker engine: one step at a time, no background tasks of its own.
pub struct Engine {
    pub(crate) db: Db,
    pub(crate) gate: Arc<Gate>,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) finalizer: Arc<dyn Finalizer>,
    pub(crate) disk: Arc<dyn DiskProbe>,
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
        let folder = if settings.download_folder.trim().is_empty() {
            crate::locations::default_download_dir()
                .display()
                .to_string()
        } else {
            settings.download_folder.clone()
        };
        let resolved = crate::locations::resolve_home_dir(&folder);
        match self.disk.free_bytes(&resolved) {
            None => Some(format!("download folder {} is missing", resolved.display())),
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
    pub(crate) fn block(&self, job: &Job, reason: &str) -> Result<JobState> {
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
    pub(crate) fn set_chosen(&self, job_id: i64, info_hash: &str) -> Result<()> {
        self.db.with(|c| {
            c.execute(
                "UPDATE jobs SET torrent_id = (SELECT id FROM torrents WHERE info_hash = ?2)
                 WHERE id = ?1",
                rusqlite::params![job_id, info_hash],
            )?;
            Ok(())
        })
    }

    pub(crate) fn context(&self, job: &Job) -> Result<JobContext> {
        let (mal_id, number, title) = self.db.with(|c| {
            Ok(c.query_row(
                "SELECT e.mal_id, e.number, s.title
                 FROM episodes e JOIN shows s ON s.mal_id = e.mal_id
                 WHERE e.id = ?1",
                [job.episode_id],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, f64>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )?)
        })?;
        Ok(JobContext {
            mal_id,
            number,
            title,
        })
    }

    pub(crate) fn now(&self) -> i64 {
        self.clock.now()
    }
}
