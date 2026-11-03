use anyhow::Result;
use genrl_core::http::Gate;
use genrl_core::{qbit, EventLog, Settings};
use std::path::Path;

use super::common::open_db;

pub async fn job_retry_cmd(data_dir: &Path, job_id: i64) -> Result<()> {
    let db = open_db(data_dir)?;
    let clock = genrl_core::clock::SystemClock;
    let outcome = genrl_core::jobs::retry_job(&db, &clock, job_id)?;
    match outcome {
        genrl_core::jobs::RetryOutcome::Retried => {
            println!("Job {job_id} reset to search");
        }
        genrl_core::jobs::RetryOutcome::SkippedEpisodeDone => {
            println!("Job {job_id} skipped: episode already done");
        }
    }
    Ok(())
}

pub async fn job_cancel_cmd(data_dir: &Path, job_id: i64, delete_files: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let info_hash: Option<String> = db.with(|c| {
        use rusqlite::OptionalExtension;
        Ok(c.query_row(
            "SELECT t.info_hash FROM jobs j JOIN torrents t ON t.id = j.torrent_id WHERE j.id = ?1",
            [job_id],
            |r| r.get(0),
        )
        .optional()?)
    })?;
    if let Some(hash) = info_hash {
        let settings = Settings::load(&db)?;
        let gate = Gate::new(db.clone())?;
        if let Ok(client) = qbit::Qbit::from_settings(&settings) {
            let _ = client.remove(&gate, &hash, delete_files).await;
        }
    }
    db.with(|c| {
        Ok(c.execute(
            "UPDATE jobs SET state = 'failed', last_error = 'Cancelled by user' WHERE id = ?1",
            [job_id],
        )?)
    })?;
    let log = EventLog::new(db);
    log.warn(None, Some(job_id), format!("Cancelled job #{job_id}"))?;
    println!("Job {job_id} cancelled");
    Ok(())
}

pub async fn job_retry_all_cmd(data_dir: &Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let clock = genrl_core::clock::SystemClock;
    let summary = genrl_core::jobs::retry_all_jobs(&db, &clock)?;
    let log = EventLog::new(db);
    log.info(
        None,
        None,
        format!(
            "Retried {} stuck jobs ({} skipped: episode already done) by user request",
            summary.retried, summary.skipped
        ),
    )?;
    println!(
        "Retried {} jobs ({} skipped: episode already done)",
        summary.retried, summary.skipped
    );
    Ok(())
}

pub fn candidates_cmd(data_dir: &Path, mal_id: i64, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let candidates = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT id, name, \"group\", resolution, size, seeders, leechers, score, rejected, magnet, info_hash
             FROM torrents
             WHERE mal_id = ?1
             ORDER BY score DESC, seeders DESC
             LIMIT 20",
        )?;
        let rows = stmt.query_map([mal_id], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "name": r.get::<_, String>(1)?,
                "group": r.get::<_, Option<String>>(2)?,
                "resolution": r.get::<_, Option<String>>(3)?,
                "size_bytes": r.get::<_, i64>(4)?,
                "seeders": r.get::<_, i64>(5)?,
                "leechers": r.get::<_, i64>(6)?,
                "score": r.get::<_, f64>(7)?,
                "rejected": r.get::<_, i64>(8)? != 0,
                "magnet": r.get::<_, String>(9)?,
                "info_hash": r.get::<_, String>(10)?,
            }))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&candidates)?);
    } else {
        println!("Candidates for MAL #{mal_id}:");
        for c in &candidates {
            println!("  [{}] {} (score: {})", c["group"], c["name"], c["score"]);
        }
    }
    Ok(())
}
