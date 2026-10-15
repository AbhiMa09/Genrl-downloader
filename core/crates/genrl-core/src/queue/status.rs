use std::collections::HashMap;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::jobs;
use crate::models::JobState;
use crate::questions::QuestionStore;

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
