use crate::error::Result;
use crate::events::EventLog;
use crate::jobs;
use crate::models::JobState;
use crate::qbit;
use crate::settings::Settings;
use crate::worker::Engine;

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
