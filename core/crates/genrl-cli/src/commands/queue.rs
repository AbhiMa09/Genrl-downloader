use anyhow::Result;
use genrl_core::clock::{Clock, SystemClock};
use genrl_core::http::Gate;
use genrl_core::models::JobState;
use genrl_core::questions::QuestionStore;
use genrl_core::worker::{Engine, FsDiskProbe};
use genrl_core::{jobs, qbit, queue, settings, EventLog, Settings};
use std::path::Path;
use std::sync::Arc;

use super::common::{human_bytes, open_db};

pub async fn status_cmd(data_dir: &Path, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db.clone())?;
    let client = qbit::Qbit::from_settings(&settings)?;

    // Probe qBit
    let (qbit_connected, qbit_version, dlspeed, upspeed, active_torrents) =
        match client.health(&gate).await {
            Ok(health) => {
                let speeds =
                    client
                        .speeds(&gate)
                        .await
                        .unwrap_or(genrl_core::qbit::TransferSpeeds {
                            download: 0,
                            upload: 0,
                        });
                let torrents = client.list(&gate, None).await.unwrap_or_default();
                let active = torrents
                    .iter()
                    .filter(|t| t.dlspeed > 0 || t.upspeed > 0)
                    .count();
                (
                    true,
                    health.version,
                    speeds.download.max(0) as u64,
                    speeds.upload.max(0) as u64,
                    active as u32,
                )
            }
            Err(_) => (false, "disconnected".to_string(), 0, 0, 0),
        };

    // Storage info
    let locations = genrl_core::locations::get_locations(&db)?;
    let free_disk_space_gb = locations.library_dir.free_space_gb.unwrap_or(0.0).round() as u64;

    // Counts from database
    let total_shows: i64 = db.with(|c| {
        c.query_row("SELECT COUNT(*) FROM shows", [], |r| r.get(0))
            .or(Ok(0))
    })?;
    let total_episodes: i64 = db.with(|c| {
        c.query_row(
            "SELECT COUNT(*) FROM episodes WHERE state = 'done'",
            [],
            |r| r.get(0),
        )
        .or(Ok(0))
    })?;
    let open_questions = QuestionStore::open_count(&db, SystemClock.now())?;

    // Last sync
    let last_sync_time = settings::read_state_time(&db, "last_mal_sync_at")?;
    let shows_changed_at = settings::read_state_time(&db, "shows_changed_at")?;

    let report = serde_json::json!({
        "daemon_connected": true,
        "daemon_port": "CLI IPC",
        "automation_on": settings.automation_on,
        "qbit_connected": qbit_connected,
        "qbit_version": qbit_version,
        "download_speed_bytes": dlspeed,
        "upload_speed_bytes": upspeed,
        "active_torrents": active_torrents,
        "total_shows": total_shows,
        "total_episodes": total_episodes,
        "inbox_questions": open_questions,
        "free_disk_space_gb": free_disk_space_gb,
        "total_disk_space_gb": free_disk_space_gb + 100, // Approximate available
        "last_mal_sync": last_sync_time,
        "shows_changed_at": shows_changed_at,
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Genrl System Status:");
        println!(
            "  Automation: {}",
            if settings.automation_on {
                "ON"
            } else {
                "PAUSED"
            }
        );
        println!(
            "  qBittorrent: {} ({})",
            if qbit_connected {
                "Connected"
            } else {
                "Offline"
            },
            qbit_version
        );
        println!(
            "  Speeds: {} down, {} up",
            human_bytes(dlspeed),
            human_bytes(upspeed)
        );
        println!(
            "  Shows: {} | Episodes Done: {}",
            total_shows, total_episodes
        );
        println!("  Free Disk Space: {} GiB", free_disk_space_gb);
    }
    Ok(())
}

/// The queue at a glance: what is due, what is waiting, what is stuck.
pub fn queue_cmd(data_dir: &Path, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let status = queue::status(&db, &SystemClock)?;

    if json {
        // Query active/running jobs with show and episode details
        let jobs = db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT j.id, e.mal_id, s.title, s.season, e.number, t.name, t.\"group\",
                        j.progress, t.size, j.state, t.seeders, t.leechers, j.last_error,
                        t.info_hash, t.magnet
                 FROM jobs j
                 JOIN episodes e ON e.id = j.episode_id
                 JOIN shows s ON s.mal_id = e.mal_id
                 LEFT JOIN torrents t ON t.id = j.torrent_id
                 WHERE j.state != 'done'
                 ORDER BY j.created_at",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "mal_id": r.get::<_, i64>(1)?,
                    "show_title": r.get::<_, String>(2)?,
                    "season": r.get::<_, i64>(3)?,
                    "episode": r.get::<_, f64>(4)? as i64,
                    "release_name": r.get::<_, Option<String>>(5)?,
                    "group": r.get::<_, Option<String>>(6)?,
                    "progress": r.get::<_, f64>(7)?,
                    "size_bytes": r.get::<_, Option<i64>>(8)?,
                    "state": r.get::<_, String>(9)?,
                    "seeders": r.get::<_, Option<i64>>(10)?,
                    "leechers": r.get::<_, Option<i64>>(11)?,
                    "last_error": r.get::<_, Option<String>>(12)?,
                    "info_hash": r.get::<_, Option<String>>(13)?,
                    "magnet": r.get::<_, Option<String>>(14)?,
                }))
            })?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            Ok(out)
        })?;
        println!("{}", serde_json::to_string_pretty(&jobs)?);
        return Ok(());
    }

    println!(
        "automation: {}",
        if settings.automation_on {
            "on"
        } else {
            "paused"
        }
    );
    println!("due now  : {}", status.due);
    println!("questions: {}", status.open_questions);

    const STATES: [JobState; 9] = [
        JobState::Search,
        JobState::Choose,
        JobState::Add,
        JobState::Monitor,
        JobState::Finalize,
        JobState::NeedsInput,
        JobState::Blocked,
        JobState::Failed,
        JobState::Done,
    ];
    for state in STATES {
        if let Some(count) = status.jobs_by_state.get(&state) {
            if *count > 0 {
                println!("  {:<12} {count}", state.as_str());
            }
        }
    }

    for question in QuestionStore::open(&db, SystemClock.now())? {
        let on = match question.job_id {
            Some(job_id) => format!("job {job_id}"),
            None => "general".to_string(),
        };
        println!(
            "  question {}  {} ({on})",
            question.id,
            question.kind().as_str()
        );
    }

    let mut printed_stuck = false;
    for state in [JobState::Blocked, JobState::Failed, JobState::NeedsInput] {
        for job in jobs::list(&db, Some(state), 10)? {
            if !printed_stuck {
                println!("stuck:");
                printed_stuck = true;
            }
            let reason = job.last_error.as_deref().unwrap_or("waiting for an answer");
            println!("  job {} ({}) {reason}", job.id, state.as_str());
        }
    }
    Ok(())
}

pub fn inbox_cmd(data_dir: &Path, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let questions = QuestionStore::open(&db, SystemClock.now())?;
    if json {
        let items: Vec<serde_json::Value> = questions
            .into_iter()
            .map(|q| {
                let show_title: Option<String> = q.mal_id.and_then(|id| {
                    db.with(|c| {
                        Ok(
                            c.query_row("SELECT title FROM shows WHERE mal_id = ?1", [id], |r| {
                                r.get(0)
                            })
                            .ok(),
                        )
                    })
                    .ok()
                    .flatten()
                });
                serde_json::json!({
                    "id": q.id,
                    "job_id": q.job_id,
                    "mal_id": q.mal_id,
                    "show_title": show_title.unwrap_or_else(|| "System Notification".to_string()),
                    "kind": q.kind().as_str(),
                    "payload": q.payload,
                    "created_at": q.created_at,
                    "expires_at": q.expires_at,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        println!("Action Inbox ({} open questions):", questions.len());
        for q in questions {
            println!("  #{} [{}] (job {:?})", q.id, q.kind().as_str(), q.job_id);
        }
    }
    Ok(())
}

pub fn answer_cmd(data_dir: &Path, id: i64, choice_str: &str, always: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let choice = match choice_str.to_lowercase().as_str() {
        "keep" => genrl_core::questions::Choice::Keep,
        "replace" => genrl_core::questions::Choice::Replace,
        "keep_both" => genrl_core::questions::Choice::KeepBoth,
        "download" => genrl_core::questions::Choice::Download,
        "wait" => genrl_core::questions::Choice::Wait,
        "pick_another" => genrl_core::questions::Choice::PickAnother,
        "batch" => genrl_core::questions::Choice::Batch,
        "wait_for_singles" => genrl_core::questions::Choice::WaitForSingles,
        "pause" => genrl_core::questions::Choice::Pause,
        "change_folder" => genrl_core::questions::Choice::ChangeFolder,
        "retry_later" => genrl_core::questions::Choice::RetryLater,
        "manual_search" => genrl_core::questions::Choice::ManualSearch,
        "skip_episode" => genrl_core::questions::Choice::SkipEpisode,
        "search_again" => genrl_core::questions::Choice::SearchAgain,
        "none_of_these" => genrl_core::questions::Choice::NoneOfThese,
        "wrong_release" | "wrong" => genrl_core::questions::Choice::WrongRelease,
        other => {
            if other.len() == 40 && other.chars().all(|c| c.is_ascii_hexdigit()) {
                genrl_core::questions::Choice::UseThis {
                    info_hash: other.to_string(),
                }
            } else {
                genrl_core::questions::Choice::Alias {
                    name: choice_str.to_string(),
                }
            }
        }
    };
    let ans = genrl_core::questions::Answer { choice, always };
    QuestionStore::answer(&db, id, &ans)?;
    if let Ok(Some(q)) = QuestionStore::get(&db, id) {
        let clock = genrl_core::clock::SystemClock;
        let _ = genrl_core::worker::apply_answer(&db, &clock, &q, &ans);
    }
    let log = EventLog::new(db);
    log.info(
        None,
        None,
        format!("answered question #{id} with choice '{choice_str}' (always: {always})"),
    )?;
    println!("Answered question #{id}");
    Ok(())
}

/// Reconcile after a crash, then run the workers until Ctrl-C.
pub async fn run_cmd(data_dir: &Path, exit_with_parent: bool) -> Result<()> {
    // One worker per data folder: two would race on every job. The OS drops
    // the lock when this process ends, however it ends.
    std::fs::create_dir_all(data_dir)?;
    let lock = std::fs::File::create(data_dir.join("worker.lock"))?;
    if fs2::FileExt::try_lock_exclusive(&lock).is_err() {
        anyhow::bail!("another genrl worker is already running for this data folder");
    }

    // The app holds our stdin open; when it goes away, however it goes, the
    // read ends and the worker follows. Job state is written before the work
    // it enables, so stopping mid-step is safe to recover from.
    if exit_with_parent {
        std::thread::spawn(|| {
            let _ = std::io::copy(&mut std::io::stdin(), &mut std::io::sink());
            std::process::exit(0);
        });
    }

    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Arc::new(Gate::new(db.clone())?);
    let finalizer = Arc::new(genrl_core::library::LibraryFinalizer::new(
        db.clone(),
        gate.clone(),
        Arc::new(SystemClock),
        data_dir.to_path_buf(),
    ));
    let engine = Arc::new(Engine::new(
        db.clone(),
        gate,
        Arc::new(SystemClock),
        finalizer,
        Arc::new(FsDiskProbe),
        data_dir.to_path_buf(),
    ));
    // qBittorrent may not be up yet (just after boot, or across a VM
    // boundary). Each step already blocks and retries on an unreachable
    // client, so a failed recovery pass is reported, never fatal.
    match queue::reconcile(&engine).await {
        Ok(report) => println!(
            "reconcile: {} resumed, {} waiting",
            report.resumed.len(),
            report.waiting.len()
        ),
        Err(err) => {
            eprintln!("reconcile skipped: {err}");
            let _ = EventLog::new(db.clone()).warn(
                None,
                None,
                format!("startup recovery skipped, jobs will retry on their own: {err}"),
            );
        }
    }
    if !settings.automation_on {
        println!("automation is paused; `genrl resume` starts dispatching");
    }
    println!("workers running; Ctrl-C stops them — downloads keep going");
    queue::Scheduler::new(engine).run().await?;
    Ok(())
}

/// The global pause switch: stop dispatching, keep downloading.
pub fn automation_cmd(data_dir: &Path, on: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let mut settings = Settings::load(&db)?;
    settings.automation_on = on;
    settings.save(&db)?;
    let log = EventLog::new(db);
    log.info(
        None,
        None,
        if on {
            "automation enabled: worker dispatches new jobs"
        } else {
            "automation paused: in-flight downloads continue, no new jobs start"
        },
    )?;
    println!("automation {}", if on { "resumed" } else { "paused" });
    Ok(())
}

pub async fn step_cmd(data_dir: &Path, job_id: i64) -> Result<()> {
    let db = open_db(data_dir)?;
    let gate = Arc::new(Gate::new(db.clone())?);
    let finalizer = Arc::new(genrl_core::library::LibraryFinalizer::new(
        db.clone(),
        gate.clone(),
        Arc::new(SystemClock),
        data_dir.to_path_buf(),
    ));
    let engine = Arc::new(Engine::new(
        db.clone(),
        gate,
        Arc::new(SystemClock),
        finalizer,
        Arc::new(FsDiskProbe),
        data_dir.to_path_buf(),
    ));
    println!("Stepping job {job_id}...");
    match engine.step(job_id).await {
        Ok(outcome) => {
            println!(
                "Job {job_id} stepped successfully: {:?} -> {:?}",
                outcome.from, outcome.to
            );
        }
        Err(err) => {
            println!("Step failed with error: {err:?}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reconcile_error_does_not_end_run_cmd() {
        let temp = tempfile::tempdir().unwrap();
        let db = open_db(temp.path()).unwrap();
        let mut settings = Settings::load(&db).unwrap();
        // Point qBittorrent to a closed local port so reconcile fails.
        settings.qbit_url = "http://127.0.0.1:1".to_string();
        settings.save(&db).unwrap();

        // Run with exit_with_parent = false so it doesn't read stdin.
        let path = temp.path().to_path_buf();
        let handle = tokio::spawn(async move { run_cmd(&path, false).await });

        // Poll EventLog every 100 ms up to 30 s for "startup recovery skipped".
        let mut event_found = false;
        let event_log = EventLog::new(db);
        for _ in 0..300 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if let Ok(events) = event_log.recent(10) {
                if events
                    .iter()
                    .any(|e| e.message.contains("startup recovery skipped"))
                {
                    event_found = true;
                    break;
                }
            }
            if handle.is_finished() {
                break;
            }
        }

        assert!(
            event_found,
            "startup recovery skipped event should appear in event log"
        );
        assert!(
            !handle.is_finished(),
            "run_cmd must keep running despite reconcile failure"
        );

        handle.abort();
        let _ = handle.await;
        drop(temp);
    }
}
