use rusqlite::OptionalExtension;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::models::{Job, JobKind, JobState};

use super::transitions::sql_text;

/// Jobs whose time has come, oldest first.
pub fn due_jobs(db: &Db, now: i64, kinds: &[JobKind], limit: usize) -> Result<Vec<Job>> {
    // The kind placeholders must be numbered `?3…`: a bare `?` here would be
    // assigned index 2 by SQLite — the same slot `LIMIT ?2` already owns — and
    // the statement would silently accept one parameter fewer than it needs.
    let placeholders: Vec<String> = (0..kinds.len()).map(|i| format!("?{}", i + 3)).collect();
    let kind_filter = if kinds.is_empty() {
        String::new()
    } else {
        format!(" AND kind IN ({})", placeholders.join(","))
    };
    let sql = format!(
        "SELECT id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id,
                created_at, updated_at, progress, last_progress_at, blocked_from
         FROM jobs
         WHERE (next_run_at IS NULL OR next_run_at <= ?1){kind_filter}
         ORDER BY created_at LIMIT ?2"
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now), Box::new(limit as i64)];
    for kind in kinds {
        params.push(Box::new(kind.as_str()));
    }
    db.with_rusqlite(|c| {
        let mut stmt = c.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), job_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    })
}

/// Every job that is not `done`: the set crash recovery walks.
pub fn non_terminal_jobs(db: &Db) -> Result<Vec<Job>> {
    db.with_rusqlite(|c| {
        let mut stmt = c.prepare(
            "SELECT id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id,
                    created_at, updated_at, progress, last_progress_at, blocked_from
             FROM jobs WHERE state != 'done' ORDER BY id",
        )?;
        let rows = stmt.query_map([], job_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    })
}

pub fn get(db: &Db, job_id: i64) -> Result<Option<Job>> {
    db.with_rusqlite(|c| {
        c.query_row(
            "SELECT id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id,
                    created_at, updated_at, progress, last_progress_at, blocked_from
             FROM jobs WHERE id = ?1",
            [job_id],
            job_from_row,
        )
        .optional()
    })
}

/// Jobs, optionally filtered by state, for the Queue page.
pub fn list(db: &Db, state: Option<JobState>, limit: usize) -> Result<Vec<Job>> {
    let (sql, filter): (String, Vec<String>) = match state {
        Some(state) => (
            format!(
                "SELECT id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id,
                        created_at, updated_at, progress, last_progress_at, blocked_from
                 FROM jobs WHERE state = '{}' ORDER BY created_at LIMIT ?1",
                sql_text(state.as_str())
            ),
            vec![limit.to_string()],
        ),
        None => (
            "SELECT id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id,
                    created_at, updated_at, progress, last_progress_at, blocked_from
             FROM jobs ORDER BY created_at LIMIT ?1"
                .to_string(),
            vec![limit.to_string()],
        ),
    };
    db.with_rusqlite(|c| {
        let mut stmt = c.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(filter.iter()), job_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    })
}

/// How many torrents have been added since the start of `now`'s day: the
/// daily cap counts real additions, not planned episodes.
pub fn added_since(db: &Db, since: i64) -> Result<u64> {
    db.with_rusqlite(|c| {
        Ok(c.query_row(
            "SELECT COUNT(*) FROM torrents WHERE mal_id IS NOT NULL AND rejected = 0
               AND found_at >= ?1",
            [since],
            |r| r.get::<_, i64>(0),
        )? as u64)
    })
}

/// Record monitor progress, so the stall timer has something to compare.
pub fn note_progress(
    db: &Db,
    clock: &dyn Clock,
    job_id: i64,
    progress: f64,
    moved: bool,
) -> Result<()> {
    let now = clock.now();
    db.with(|c| {
        c.execute(
            "UPDATE jobs SET progress = ?2, last_progress_at = CASE WHEN ?3 = 1 THEN ?4 ELSE last_progress_at END
             WHERE id = ?1",
            rusqlite::params![job_id, progress, moved as i64, now],
        )?;
        Ok(())
    })
}

pub fn job_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    Ok(Job {
        id: r.get(0)?,
        episode_id: r.get(1)?,
        kind: JobKind::parse(&r.get::<_, String>(2)?),
        state: JobState::parse(&r.get::<_, String>(3)?),
        attempts: r.get(4)?,
        next_run_at: r.get(5)?,
        last_error: r.get(6)?,
        torrent_id: r.get(7)?,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
        progress: r.get(10)?,
        last_progress_at: r.get(11)?,
        blocked_from: r
            .get::<_, Option<String>>(12)?
            .as_deref()
            .map(JobState::parse),
    })
}
