use rusqlite::OptionalExtension;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::events::{self, Level};
use crate::models::{JobKind, JobState};

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
            .query_row("SELECT state FROM jobs WHERE id = ?1", [job_id], |r| {
                r.get(0)
            })
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
            return Err(Error::IllegalTransition { from, to, job_id });
        }

        let current_kind_string: String =
            tx.query_row("SELECT kind FROM jobs WHERE id = ?1", [job_id], |r| {
                r.get(0)
            })?;
        let current_kind = JobKind::parse(&current_kind_string);

        let attempts = if opts.reset_attempts {
            0
        } else {
            tx.query_row("SELECT attempts FROM jobs WHERE id = ?1", [job_id], |r| {
                r.get::<_, i64>(0)
            })? + i64::from(opts.count_attempt)
        };

        let mut sql =
            String::from("UPDATE jobs SET state = ?2, updated_at = ?3, attempts = ?4, kind = ?5");
        if opts.clear_torrent {
            // A new candidate starts its own download: the previous one's
            // progress and stall timestamp must not carry over, or the next
            // candidate would be judged against someone else's twenty minutes.
            sql.push_str(", torrent_id = NULL, progress = 0, last_progress_at = NULL");
        }
        if let Some(from) = opts.blocked_from {
            sql.push_str(&format!(", blocked_from = '{}'", sql_text(from.as_str())));
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
            format!(
                "job {job_id}: {} → {} ({reason})",
                from.as_str(),
                to.as_str()
            )
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
pub fn reschedule(db: &Db, clock: &dyn Clock, job_id: i64, at: i64, reason: &str) -> Result<()> {
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

pub(crate) fn sql_text(value: &str) -> String {
    // Table, column and state names come from our own enums, never input; this
    // keeps the interpolated SQL free of user data.
    value.replace('\'', "''")
}

/// Which pipeline limiter a state belongs to.
///
/// The design's four `kind` values map onto the five steps — "choose" rides
/// along with search — so the column tells the scheduler which budget a job is
/// spending while the state says exactly where it is.
pub(crate) fn kind_for(to: JobState, current: JobKind, blocked_from: Option<JobState>) -> JobKind {
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
