//! The job queue's state machine.
//!
//! One table of allowed moves and one [`transition`] function enforce the
//! design's diagram — five main steps left to right (search → choose → add →
//! monitor → finalize), three side states (needs_input, blocked, failed), and
//! the two loops back: a stalled torrent returns to `search` for the next
//! candidate, and a blocked job returns to whatever state it was paused in.
//!
//! Two rules shape everything here:
//!
//! 1. **A transition is one transaction, written before the work it enables.**
//!    [`transition`] commits the new state *and* its Activity-log line in the
//!    same statement batch, so a crash can never leave a state change without
//!    its record — and the step that runs afterwards does so on the strength
//!    of state that is already durable.
//! 2. **A job's state means "about to run this step's work".** That is what
//!    makes every step idempotent and the crash-recovery check possible: a
//!    job killed mid-step is simply re-run, because the step's side effects
//!    are keyed (episodes unique on `(mal_id, number)`, torrents unique on
//!    `info_hash`).

use std::time::Duration;

use rusqlite::OptionalExtension;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::events::{self, Level};
use crate::models::{Job, JobKind, JobState};

/// Every legal move.
///
/// The five main steps run left to right. The side states are reachable from
/// the step that discovered the problem and return to it (or to an earlier
/// one) when the problem clears — `blocked` remembers where it came from in
/// `jobs.blocked_from`. `done` is terminal; `failed` is restarted only through
/// a new attempt (`failed → search`), never resurrected in place.
pub const TRANSITIONS: &[(JobState, JobState)] = &[
    // The main line.
    (JobState::Search, JobState::Choose),
    (JobState::Choose, JobState::Add),
    (JobState::Add, JobState::Monitor),
    (JobState::Monitor, JobState::Finalize),
    (JobState::Finalize, JobState::Done),
    // Interruptions the steps discover on their own.
    (JobState::Search, JobState::NeedsInput),
    (JobState::Search, JobState::Blocked),
    (JobState::Search, JobState::Failed),
    (JobState::Choose, JobState::NeedsInput),
    (JobState::Choose, JobState::Blocked),
    (JobState::Choose, JobState::Failed),
    (JobState::Add, JobState::Blocked),
    (JobState::Add, JobState::Failed),
    // A stalled torrent loops back to search for the next candidate.
    (JobState::Monitor, JobState::Search),
    (JobState::Monitor, JobState::NeedsInput),
    (JobState::Monitor, JobState::Blocked),
    (JobState::Monitor, JobState::Failed),
    (JobState::Finalize, JobState::NeedsInput),
    (JobState::Finalize, JobState::Blocked),
    (JobState::Finalize, JobState::Failed),
    // Side states hand back.
    (JobState::NeedsInput, JobState::Search),
    (JobState::NeedsInput, JobState::Choose),
    (JobState::NeedsInput, JobState::Add),
    (JobState::NeedsInput, JobState::Monitor),
    (JobState::NeedsInput, JobState::Finalize),
    (JobState::NeedsInput, JobState::Failed),
    (JobState::NeedsInput, JobState::Done),
    (JobState::Blocked, JobState::Search),
    (JobState::Blocked, JobState::Choose),
    (JobState::Blocked, JobState::Add),
    (JobState::Blocked, JobState::Monitor),
    (JobState::Blocked, JobState::Finalize),
    (JobState::Blocked, JobState::Failed),
    // A retry after failure is a fresh attempt at the first step.
    (JobState::Failed, JobState::Search),
    (JobState::Failed, JobState::Done),
    // Redo the step: recovery found nothing for it to continue from.
    (JobState::Add, JobState::Choose),
    (JobState::Monitor, JobState::Add),
];

/// Is this move on the diagram?
pub fn is_allowed(from: JobState, to: JobState) -> bool {
    if from == to {
        // Staying put is not a transition; rescheduling writes `next_run_at`.
        return false;
    }
    if from.is_terminal() {
        return false;
    }
    TRANSITIONS.contains(&(from, to))
}

/// What a transition should also say.
#[derive(Debug, Clone, Default)]
pub struct MoveOptions {
    /// Clear the chosen candidate (a stall or a "none of these" answer).
    pub clear_torrent: bool,
    /// The state a blocked job returns to.
    pub blocked_from: Option<JobState>,
    /// Reset the attempt counter (a user-initiated retry starts clean).
    pub reset_attempts: bool,
    /// Bump the attempt counter (a failure).
    pub count_attempt: bool,
}

impl MoveOptions {
    pub fn blocked(from: JobState) -> Self {
        MoveOptions {
            blocked_from: Some(from),
            ..Default::default()
        }
    }
}

/// Move one job to a new state, recording why in the Activity log.
///
/// The state write and the log line share one transaction: **the transition
/// lands before the work it enables**, so a process killed between the two
/// still has consistent state to recover from.
pub fn transition(
    db: &Db,
    clock: &dyn Clock,
    job_id: i64,
    to: JobState,
    reason: &str,
    opts: &MoveOptions,
) -> Result<JobState> {
    let at = clock.now();
    let outcome = db.with(|c| {
        let tx = c.unchecked_transaction()?;
        let current: Option<String> = tx
            .query_row(
                "SELECT state FROM jobs WHERE id = ?1",
                [job_id],
                |r| r.get(0),
            )
            .optional()?;
        let Some(current) = current else {
            return Err(Error::Other(format!("no job with id {job_id}")));
        };
        let from = JobState::parse(&current);
        if from.is_terminal() {
            return Err(Error::Other(format!(
                "job {job_id} is {from:?} and never leaves that state"
            )));
        }
        if from != to && !is_allowed(from, to) {
            return Err(Error::IllegalTransition {
                from,
                to,
                job_id,
            });
        }

        let current_kind_string: String = tx.query_row(
            "SELECT kind FROM jobs WHERE id = ?1",
            [job_id],
            |r| r.get(0),
        )?;
        let current_kind = JobKind::parse(&current_kind_string);

        let attempts = if opts.reset_attempts {
            0
        } else {
            tx.query_row(
                "SELECT attempts FROM jobs WHERE id = ?1",
                [job_id],
                |r| r.get::<_, i64>(0),
            )? + i64::from(opts.count_attempt)
        };

        let mut sql = String::from(
            "UPDATE jobs SET state = ?2, updated_at = ?3, attempts = ?4, kind = ?5",
        );
        if opts.clear_torrent {
            // A new candidate starts its own download: the previous one's
            // progress and stall timestamp must not carry over, or the next
            // candidate would be judged against someone else's twenty minutes.
            sql.push_str(", torrent_id = NULL, progress = 0, last_progress_at = NULL");
        }
        if let Some(from) = opts.blocked_from {
            sql.push_str(&format!(
                ", blocked_from = '{}'",
                sql_text(from.as_str())
            ));
        } else if to != JobState::Blocked {
            sql.push_str(", blocked_from = NULL");
        }
        let kind = kind_for(to, current_kind, opts.blocked_from);
        // The reason travels with the row for the two states whose whole
        // message is "here is why": the resume logic reads `last_error` back
        // to decide whether the blocker cleared, and the Queue page shows it.
        // Leaving those states spends it — the Activity log keeps the history.
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(job_id),
            Box::new(to.as_str()),
            Box::new(at),
            Box::new(attempts),
            Box::new(kind.as_str()),
        ];
        match to {
            JobState::Blocked | JobState::Failed => {
                sql.push_str(", last_error = ?6");
                params.push(Box::new(reason.to_string()));
            }
            _ if from == JobState::Blocked || from == JobState::Failed => {
                sql.push_str(", last_error = NULL");
            }
            _ => {}
        }
        sql.push_str(" WHERE id = ?1");
        tx.execute(&sql, rusqlite::params_from_iter(params.iter()))?;

        let mal_id: Option<i64> = tx
            .query_row(
                "SELECT e.mal_id FROM jobs j JOIN episodes e ON e.id = j.episode_id
                 WHERE j.id = ?1",
                [job_id],
                |r| r.get(0),
            )
            .optional()?
            .flatten();

        // Same transaction: the state change and its log line are one fact.
        let message = if from == to {
            format!("job {job_id}: {reason}")
        } else {
            format!("job {job_id}: {} → {} ({reason})", from.as_str(), to.as_str())
        };
        events::record_in(&tx, Level::Info, Some(job_id), mal_id, &message, at)?;
        tx.commit()?;
        Ok(from)
    })?;
    Ok(outcome)
}

/// Keep a job in its state and set when it may run again.
///
/// Used for backoff and for "not due yet" — never a state change, so it is not
/// a transition, but it still records the reason in the log.
pub fn reschedule(
    db: &Db,
    clock: &dyn Clock,
    job_id: i64,
    at: i64,
    reason: &str,
) -> Result<()> {
    let now = clock.now();
    db.with(|c| {
        let tx = c.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE jobs SET next_run_at = ?2, last_error = ?3, updated_at = ?4 WHERE id = ?1",
            rusqlite::params![job_id, at, reason, now],
        )?;
        if changed == 0 {
            return Err(Error::Other(format!("no job with id {job_id}")));
        }
        let mal_id: Option<i64> = tx
            .query_row(
                "SELECT e.mal_id FROM jobs j JOIN episodes e ON e.id = j.episode_id
                 WHERE j.id = ?1",
                [job_id],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        events::record_in(
            &tx,
            Level::Info,
            Some(job_id),
            mal_id,
            &format!("job {job_id}: {reason}"),
            now,
        )?;
        tx.commit()?;
        Ok(())
    })
}

/// Record the most recent error without moving the job.
pub fn record_error(db: &Db, job_id: i64, message: &str) -> Result<()> {
    db.with(|c| {
        c.execute(
            "UPDATE jobs SET last_error = ?2 WHERE id = ?1",
            rusqlite::params![job_id, message],
        )?;
        Ok(())
    })
}

// ------------------------------------------------------------------ enqueueing

/// Create the job that fetches episode `episode_id`, or return the one that
/// already covers it.
///
/// Idempotency is the point: the planner runs weekly, reconcile runs on every
/// launch, and neither may double-queue. Episodes are unique on
/// `(mal_id, number)`, so the episode row is the identity; a job that is not
/// terminal already exists for it means "already queued".
pub fn enqueue_search(db: &Db, clock: &dyn Clock, episode_id: i64) -> Result<i64> {
    let existing: Option<i64> = db.with(|c| {
        Ok(c
            .query_row(
                "SELECT id FROM jobs WHERE episode_id = ?1 AND kind = 'search'
                   AND state != 'done' ORDER BY id LIMIT 1",
                [episode_id],
                |r| r.get(0),
            )
            .optional()?)
    })?;
    if let Some(id) = existing {
        return Ok(id);
    }

    let at = clock.now();
    db.with(|c| {
        let tx = c.unchecked_transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO jobs (episode_id, kind, state, created_at, updated_at)
             VALUES (?1, 'search', 'search', ?2, ?2)",
            rusqlite::params![episode_id, at],
        )?;
        let id = tx.last_insert_rowid();
        // The row id is not the episode number; the Activity log reads as
        // "episode 5", the number a human recognises.
        let (mal_id, number): (Option<i64>, f64) = tx.query_row(
            "SELECT mal_id, number FROM episodes WHERE id = ?1",
            [episode_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        events::record_in(
            &tx,
            Level::Info,
            Some(id),
            mal_id,
            &format!("queued search job {id} for episode {number}"),
            at,
        )?;
        tx.commit()?;
        Ok(id)
    })
}

/// Remember a candidate torrent, deduplicated by info hash.
///
/// Re-seeing the same release under another name (or on the weekly re-check)
/// updates its score rather than erroring or duplicating: `torrents` is unique
/// on `info_hash`, so the same torrent can never be queued twice.
pub fn remember_candidate(
    db: &Db,
    clock: &dyn Clock,
    candidate: &RememberedCandidate,
) -> Result<i64> {
    let now = clock.now();
    db.with(|c| {
        let tx = c.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO torrents (info_hash, name, magnet, seeders, leechers, size, \"group\",
                                   resolution, source_url, score, mal_id, found_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT (info_hash) DO UPDATE SET
                seeders = excluded.seeders,
                leechers = excluded.leechers,
                size = excluded.size,
                score = excluded.score,
                mal_id = excluded.mal_id",
            rusqlite::params![
                candidate.info_hash,
                candidate.name,
                candidate.magnet,
                candidate.seeders,
                candidate.leechers,
                candidate.size_bytes,
                candidate.group,
                candidate.resolution,
                candidate.source_url,
                candidate.score,
                candidate.mal_id,
                now,
            ],
        )?;
        // Unique on info_hash, so exactly one row answers either branch.
        let id: i64 = tx.query_row(
            "SELECT id FROM torrents WHERE info_hash = ?1",
            [&candidate.info_hash],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(id)
    })
}

/// What the search step remembers about a candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct RememberedCandidate {
    pub info_hash: String,
    pub name: String,
    pub magnet: String,
    pub seeders: i64,
    pub leechers: i64,
    pub size_bytes: i64,
    pub group: Option<String>,
    pub resolution: Option<String>,
    pub source_url: Option<String>,
    pub score: f64,
    pub mal_id: Option<i64>,
}

/// Mark a candidate as tried and failed, so the search step moves on.
pub fn reject_candidate(db: &Db, info_hash: &str) -> Result<bool> {
    db.with(|c| {
        let changed = c.execute(
            "UPDATE torrents SET rejected = 1 WHERE info_hash = ?1",
            [info_hash],
        )?;
        Ok(changed > 0)
    })
}

// ----------------------------------------------------------------------- queries

/// Jobs whose time has come, oldest first.
pub fn due_jobs(db: &Db, now: i64, kinds: &[JobKind], limit: usize) -> Result<Vec<Job>> {
    // The kind placeholders must be numbered `?3…`: a bare `?` here would be
    // assigned index 2 by SQLite — the same slot `LIMIT ?2` already owns — and
    // the statement would silently accept one parameter fewer than it needs.
    let placeholders: Vec<String> = (0..kinds.len())
        .map(|i| format!("?{}", i + 3))
        .collect();
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
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |r| {
            Ok(job_from_row(r)?)
        })?;
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
        let rows = stmt.query_map([], |r| Ok(job_from_row(r)?))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    })
}

pub fn get(db: &Db, job_id: i64) -> Result<Option<Job>> {
    db.with_rusqlite(|c| {
        Ok(c
            .query_row(
                "SELECT id, episode_id, kind, state, attempts, next_run_at, last_error, torrent_id,
                        created_at, updated_at, progress, last_progress_at, blocked_from
                 FROM jobs WHERE id = ?1",
                [job_id],
                |r| Ok(job_from_row(r)?),
            )
            .optional()?)
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
                sql_text(&state.as_str().to_string())
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
        let rows = stmt.query_map(rusqlite::params_from_iter(filter.iter()), |r| {
            Ok(job_from_row(r)?)
        })?;
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
        blocked_from: r.get::<_, Option<String>>(12)?.as_deref().map(JobState::parse),
    })
}

// ----------------------------------------------------------------------- backoff
/// The design's backoff rule: 5 s, 30 s, 2 min, 10 min, 1 hour, then hold.
pub fn backoff_delay(attempts: i64) -> Duration {
    const LADDER: &[u64] = &[5, 30, 120, 600, 3600];
    let index = (attempts.max(1) - 1) as usize;
    let secs = LADDER.get(index).copied().unwrap_or(3600);
    Duration::from_secs(secs)
}

/// Spread a delay out by up to a quarter, so a whole queue waking at once
/// does not hammer a service that just came back.
///
/// `sample` is a random draw in `0.0 .. 1.0` — the caller supplies the
/// randomness; tests pass `0.0` for an exact ladder.
pub fn with_jitter(delay: Duration, sample: f64) -> Duration {
    let sample = sample.clamp(0.0, 1.0);
    delay + delay.mul_f64(0.25 * sample)
}

/// Start of the UTC day containing `at`.
pub fn start_of_day(at: i64) -> i64 {
    at - at.rem_euclid(86_400)
}

fn sql_text(value: &str) -> String {
    // Table, column and state names come from our own enums, never input; this
    // keeps the interpolated SQL free of user data.
    value.replace('\'', "''")
}

/// Which pipeline limiter a state belongs to.
///
/// The design's four `kind` values map onto the five steps — "choose" rides
/// along with search — so the column tells the scheduler which budget a job is
/// spending while the state says exactly where it is.
fn kind_for(to: JobState, current: JobKind, blocked_from: Option<JobState>) -> JobKind {
    match to {
        JobState::Search | JobState::Choose => JobKind::Search,
        JobState::Add => JobKind::Add,
        JobState::Monitor => JobKind::Monitor,
        JobState::Finalize => JobKind::Finalize,
        // Side and terminal states keep the limiter of the step they will
        // return to (or the one they were already spending).
        _ => match blocked_from {
            Some(from) => kind_of_state(from),
            None => current,
        },
    }
}

fn kind_of_state(state: JobState) -> JobKind {
    match state {
        JobState::Add => JobKind::Add,
        JobState::Monitor => JobKind::Monitor,
        JobState::Finalize => JobKind::Finalize,
        _ => JobKind::Search,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;
    use crate::events::EventLog;
    use crate::models::{JobKind, JobState};

    fn db() -> Db {
        Db::open_in_memory().unwrap()
    }

    fn clock() -> FakeClock {
        FakeClock::new(1_789_227_900)
    }

    /// A show and an episode to hang a job off.
    fn episode(db: &Db, clock: &dyn Clock) -> i64 {
        crate::planner::upsert_show(
            db,
            &crate::planner::ListEntry {
                mal_id: 1,
                title: "Show".into(),
                english_title: None,
                list_status: crate::models::ListStatus::Watching,
                watched_count: 0,
                total_episodes: Some(12),
                start_year: Some(2025),
                auto: false,
                episode_offset: 0,
                names: vec!["Show".into()],
                franchise: 1,
                season: 1,
            },
        )
        .unwrap();
        crate::planner::ensure_episode(db, clock, 1, 5.0).unwrap()
    }

    fn state_of(db: &Db, job_id: i64) -> JobState {
        get(db, job_id).unwrap().unwrap().state
    }

    #[test]
    fn every_documented_move_is_allowed() {
        for (from, to) in TRANSITIONS {
            assert!(
                is_allowed(*from, *to),
                "the table lists {from} → {to}, so it must be allowed"
            );
        }
    }

    #[test]
    fn moves_off_the_diagram_are_refused() {
        // The forbidden directions that matter: backwards along the main line,
        // skipping steps, and leaving a terminal state.
        let forbidden = [
            (JobState::Search, JobState::Add),
            (JobState::Search, JobState::Monitor),
            (JobState::Search, JobState::Finalize),
            (JobState::Search, JobState::Done),
            (JobState::Choose, JobState::Monitor),
            (JobState::Add, JobState::Finalize),
            (JobState::Add, JobState::Search),
            (JobState::Monitor, JobState::Choose),
            (JobState::Finalize, JobState::Add),
            (JobState::NeedsInput, JobState::Blocked),
            (JobState::Blocked, JobState::NeedsInput),
            (JobState::Failed, JobState::Add),
            (JobState::Done, JobState::Search),
            (JobState::Done, JobState::Failed),
        ];
        for (from, to) in forbidden {
            assert!(!is_allowed(from, to), "{from} → {to} must not be allowed");
        }
    }

    #[test]
    fn staying_put_is_not_a_transition() {
        for state in [JobState::Search, JobState::Add, JobState::Blocked] {
            assert!(!is_allowed(state, state), "{state} → {state} is not a move");
        }
    }

    #[test]
    fn a_legal_move_lands_and_logs_in_one_transaction() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();
        assert_eq!(state_of(&db, job), JobState::Search);

        let from = transition(
            &db,
            &clock,
            job,
            JobState::Choose,
            "candidates remembered",
            &MoveOptions::default(),
        )
        .unwrap();
        assert_eq!(from, JobState::Search);
        assert_eq!(state_of(&db, job), JobState::Choose);

        // The transition and its Activity line are one fact.
        let events = EventLog::new(db.clone()).recent(5).unwrap();
        assert_eq!(events.len(), 2, "queued, then the move");
        assert!(events[0].message.contains("search → choose"), "{:?}", events[0]);
        assert!(events[1].message.contains("queued"), "{:?}", events[1]);
    }

    #[test]
    fn an_illegal_move_is_an_error_and_changes_nothing() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();

        let err = transition(
            &db,
            &clock,
            job,
            JobState::Monitor,
            "skip a step",
            &MoveOptions::default(),
        )
        .unwrap_err();
        assert!(
            matches!(err, Error::IllegalTransition { from, to, .. } if from == JobState::Search && to == JobState::Monitor),
            "{err:?}"
        );
        assert_eq!(state_of(&db, job), JobState::Search, "nothing moved");
    }

    #[test]
    fn a_done_job_never_moves_again() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();
        for to in [
            JobState::Choose,
            JobState::Add,
            JobState::Monitor,
            JobState::Finalize,
            JobState::Done,
        ] {
            transition(&db, &clock, job, to, "walk it home", &MoveOptions::default()).unwrap();
        }
        assert_eq!(state_of(&db, job), JobState::Done);
        let err = transition(
            &db,
            &clock,
            job,
            JobState::Search,
            "back from the dead",
            &MoveOptions::default(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("never leaves"), "{err}");
    }

    #[test]
    fn blocked_remembers_where_it_returns_to() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();

        transition(&db, &clock, job, JobState::Choose, "candidates", &MoveOptions::default()).unwrap();
        transition(&db, &clock, job, JobState::Add, "added", &MoveOptions::default()).unwrap();
        transition(
            &db,
            &clock,
            job,
            JobState::Blocked,
            "qBittorrent is down",
            &MoveOptions::blocked(JobState::Add),
        )
        .unwrap();

        let stored = get(&db, job).unwrap().unwrap();
        assert_eq!(stored.state, JobState::Blocked);
        assert_eq!(stored.blocked_from, Some(JobState::Add));
        // The kind follows the step, so the download limiter still applies.
        assert_eq!(stored.kind, JobKind::Add);

        transition(&db, &clock, job, JobState::Add, "back again", &MoveOptions::default()).unwrap();
        assert_eq!(get(&db, job).unwrap().unwrap().blocked_from, None);
    }

    #[test]
    fn kind_follows_the_step() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();

        let expect = [
            (JobState::Choose, JobKind::Search),
            (JobState::Add, JobKind::Add),
            (JobState::Monitor, JobKind::Monitor),
            (JobState::Finalize, JobKind::Finalize),
            (JobState::Blocked, JobKind::Finalize),
            (JobState::Search, JobKind::Search),
        ];
        for (to, kind) in expect {
            let opts = if to == JobState::Blocked {
                MoveOptions::blocked(JobState::Finalize)
            } else {
                MoveOptions::default()
            };
            transition(&db, &clock, job, to, "walk", &opts).unwrap();
            assert_eq!(get(&db, job).unwrap().unwrap().kind, kind);
        }
    }

    #[test]
    fn attempts_count_failures_and_reset_on_retry() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();

        transition(&db, &clock, job, JobState::Failed, "no candidates", &MoveOptions { count_attempt: true, ..Default::default() }).unwrap();
        assert_eq!(get(&db, job).unwrap().unwrap().attempts, 1);
        transition(&db, &clock, job, JobState::Search, "retry", &MoveOptions::default()).unwrap();
        transition(&db, &clock, job, JobState::Failed, "again", &MoveOptions { count_attempt: true, ..Default::default() }).unwrap();
        assert_eq!(get(&db, job).unwrap().unwrap().attempts, 2);
        transition(&db, &clock, job, JobState::Search, "fresh attempt", &MoveOptions { reset_attempts: true, ..Default::default() }).unwrap();
        assert_eq!(get(&db, job).unwrap().unwrap().attempts, 0);
    }

    #[test]
    fn enqueueing_the_same_episode_twice_is_a_no_op() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let first = enqueue_search(&db, &clock, episode).unwrap();
        let second = enqueue_search(&db, &clock, episode).unwrap();
        assert_eq!(first, second);

        let count: i64 = db
            .with(|c| Ok(c.query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn a_finished_episode_can_be_replanned_without_a_second_job() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();
        for to in [JobState::Choose, JobState::Add, JobState::Monitor, JobState::Finalize, JobState::Done] {
            transition(&db, &clock, job, to, "run", &MoveOptions::default()).unwrap();
        }
        let replanned = enqueue_search(&db, &clock, episode).unwrap();
        assert_ne!(replanned, job, "a done job is never reopened");
        assert_eq!(state_of(&db, replanned), JobState::Search);
    }

    #[test]
    fn candidates_deduplicate_by_info_hash() {
        let db = db();
        let clock = clock();
        episode(&db, &clock);
        let candidate = || RememberedCandidate {
            info_hash: "abc".repeat(13) + "abcd",
            name: "[SubsPlease] Show - 05 (1080p).mkv".to_string(),
            magnet: "magnet:?xt=urn:btih:abc".to_string(),
            seeders: 20,
            leechers: 1,
            size_bytes: 1_500_000_000,
            group: Some("SubsPlease".to_string()),
            resolution: Some("1080p".to_string()),
            source_url: None,
            score: 90.0,
            mal_id: Some(1),
        };
        let first = remember_candidate(&db, &clock, &candidate()).unwrap();
        let mut changed = candidate();
        changed.seeders = 99;
        let second = remember_candidate(&db, &clock, &changed).unwrap();
        assert_eq!(first, second);

        let (rows, seeders): (i64, i64) = db
            .with(|c| {
                Ok(c.query_row(
                    "SELECT COUNT(*), COALESCE(MAX(seeders), 0) FROM torrents",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?)
            })
            .unwrap();
        assert_eq!((rows, seeders), (1, 99));
    }

    #[test]
    fn due_jobs_respect_their_time() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();

        assert_eq!(due_jobs(&db, clock.now(), &[], 10).unwrap().len(), 1);
        reschedule(&db, &clock, job, clock.now() + 600, "not yet").unwrap();
        assert!(due_jobs(&db, clock.now(), &[], 10).unwrap().is_empty());
        clock.advance(601);
        assert_eq!(due_jobs(&db, clock.now(), &[], 10).unwrap().len(), 1);
        let error = get(&db, job).unwrap().unwrap().last_error.unwrap();
        assert_eq!(error, "not yet");
    }

    #[test]
    fn the_backoff_ladder_matches_the_design() {
        assert_eq!(backoff_delay(1), Duration::from_secs(5));
        assert_eq!(backoff_delay(2), Duration::from_secs(30));
        assert_eq!(backoff_delay(3), Duration::from_secs(120));
        assert_eq!(backoff_delay(4), Duration::from_secs(600));
        assert_eq!(backoff_delay(5), Duration::from_secs(3600));
        assert_eq!(backoff_delay(50), Duration::from_secs(3600), "then hold");
    }

    #[test]
    fn jitter_spreads_a_delay_by_up_to_a_quarter() {
        let base = Duration::from_secs(100);
        assert_eq!(with_jitter(base, 0.0), base);
        assert_eq!(with_jitter(base, 1.0), Duration::from_secs(125));
        assert!(with_jitter(base, 0.5) > base && with_jitter(base, 0.5) < Duration::from_secs(125));
    }

    #[test]
    fn the_daily_cap_counts_from_the_start_of_the_day() {
        assert_eq!(start_of_day(0), 0);
        assert_eq!(start_of_day(86_399), 0);
        assert_eq!(start_of_day(86_400), 86_400);
        let midday = 86_400 + 3_600;
        assert_eq!(start_of_day(midday), 86_400);
    }
}
