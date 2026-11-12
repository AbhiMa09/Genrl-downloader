//! Question objects: the Inbox.
//!
//! "Never guess silently": when the title match or the torrent choice is
//! uncertain, the app queues a question *instead of* downloading something
//! wrong. Questions never block other jobs — each one sits in the Inbox until
//! answered, and unanswered ones expire back to `missing` so a show cannot
//! silently hang forever.
//!
//! Each kind carries a typed payload (what the card shows) and an answer that
//! names a choice plus the "always do this" flag from the checkbox, which the
//! caller turns into a saved alias or rule — the mitigation for a wrong answer
//! repeating forever.

pub mod choice;
pub mod payload;

pub use choice::*;
pub use payload::*;

use crate::db::Db;
use crate::error::Result;
use crate::models::{Question, QuestionKind};

/// Create, answer, expire and list the Inbox.
pub struct QuestionStore;

/// The stored answer of a question nobody answered in time.
fn expired_answer() -> Result<String> {
    Ok(serde_json::to_string(&Answer {
        choice: Choice::Expired,
        always: false,
    })?)
}

impl QuestionStore {
    /// Queue a question. The caller supplies `now` and the expiry, so the
    /// function stays pure time-wise and the tests never wait.
    pub fn ask(
        db: &Db,
        job_id: Option<i64>,
        mal_id: Option<i64>,
        payload: &Payload,
        now: i64,
        expires_in: Option<i64>,
    ) -> Result<i64> {
        let kind = payload.kind();
        let body = serde_json::to_string(payload)?;
        let id = db.with(|c| {
            c.execute(
                "INSERT INTO questions (job_id, mal_id, kind, payload, created_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    job_id,
                    mal_id,
                    kind.as_str(),
                    body,
                    now,
                    expires_in.map(|secs| now + secs),
                ],
            )?;
            Ok(c.last_insert_rowid())
        })?;
        Ok(id)
    }

    /// Every still-open question, oldest first (the Inbox reads in order).
    pub fn open(db: &Db, now: i64) -> Result<Vec<OpenQuestion>> {
        Self::query(
            db,
            "answer IS NULL AND (expires_at IS NULL OR expires_at > ?1)",
            [now],
        )
    }

    /// Open questions for one show.
    pub fn for_show(db: &Db, mal_id: i64, now: i64) -> Result<Vec<OpenQuestion>> {
        Self::query(
            db,
            "answer IS NULL AND (expires_at IS NULL OR expires_at > ?1) AND mal_id = ?2",
            rusqlite::params![now, mal_id],
        )
    }

    /// Open questions of one kind.
    pub fn of_kind(db: &Db, kind: QuestionKind, now: i64) -> Result<Vec<OpenQuestion>> {
        Self::query(
            db,
            "answer IS NULL AND (expires_at IS NULL OR expires_at > ?1) AND kind = ?2",
            rusqlite::params![now, kind.as_str()],
        )
    }

    /// Record an answer. Answers overwrite, so re-answering is harmless.
    pub fn answer(db: &Db, id: i64, answer: &Answer) -> Result<()> {
        let answer = serde_json::to_string(answer)?;
        db.with(|c| {
            c.execute(
                "UPDATE questions SET answer = ?1 WHERE id = ?2",
                rusqlite::params![answer, id],
            )?;
            Ok(())
        })
    }

    /// Close a stopped job's open questions with the same `expired` answer a
    /// timeout writes, so every reader that parses answers sees a choice.
    /// A failed job keeps its own `all_failed` notice unless `include_notice`.
    pub fn close_for_job(
        c: &rusqlite::Connection,
        job_id: i64,
        at: i64,
        include_notice: bool,
    ) -> Result<usize> {
        let sql = if include_notice {
            "UPDATE questions SET answer = ?1, expires_at = ?3
             WHERE job_id = ?2 AND answer IS NULL"
        } else {
            "UPDATE questions SET answer = ?1, expires_at = ?3
             WHERE job_id = ?2 AND answer IS NULL AND kind != 'all_failed'"
        };
        Ok(c.execute(sql, rusqlite::params![expired_answer()?, job_id, at])?)
    }

    /// Sweep form of [`Self::close_for_job`]: every done or failed job at once.
    pub fn close_for_stopped_jobs(db: &Db, now: i64) -> Result<usize> {
        let answer = expired_answer()?;
        db.with(|c| {
            Ok(c.execute(
                "UPDATE questions SET answer = ?1, expires_at = ?2
                 WHERE answer IS NULL AND (
                     job_id IN (SELECT id FROM jobs WHERE state = 'done')
                     OR (job_id IN (SELECT id FROM jobs WHERE state = 'failed')
                         AND kind != 'all_failed')
                 )",
                rusqlite::params![answer, now],
            )?)
        })
    }

    /// One question by id, with its payload parsed.
    pub fn get(db: &Db, id: i64) -> Result<Option<Question>> {
        db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, job_id, mal_id, kind, payload, answer, created_at, expires_at
                 FROM questions WHERE id = ?1",
            )?;
            let mut rows = stmt.query_map([id], |r| {
                Ok(Question {
                    id: r.get(0)?,
                    job_id: r.get(1)?,
                    mal_id: r.get(2)?,
                    kind: QuestionKind::parse(r.get::<_, String>(3)?.as_str()),
                    payload: serde_json::from_str(r.get::<_, String>(4)?.as_str())
                        .unwrap_or(serde_json::Value::Null),
                    answer: r.get(5)?,
                    created_at: r.get(6)?,
                    expires_at: r.get(7)?,
                })
            })?;
            match rows.next().transpose()? {
                Some(question) => Ok(Some(question)),
                None => Ok(None),
            }
        })
    }

    /// Expire unanswered questions past their deadline and return them, so
    /// the caller can return the episodes to `missing`.
    pub fn expire(db: &Db, now: i64) -> Result<Vec<Question>> {
        let expired = db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, job_id, mal_id, kind, payload, answer, created_at, expires_at
                 FROM questions
                 WHERE answer IS NULL AND expires_at IS NOT NULL AND expires_at <= ?1",
            )?;
            let rows = stmt.query_map([now], |r| {
                Ok(Question {
                    id: r.get(0)?,
                    job_id: r.get(1)?,
                    mal_id: r.get(2)?,
                    kind: QuestionKind::parse(r.get::<_, String>(3)?.as_str()),
                    payload: serde_json::from_str(r.get::<_, String>(4)?.as_str())
                        .unwrap_or(serde_json::Value::Null),
                    answer: None,
                    created_at: r.get(6)?,
                    expires_at: r.get(7)?,
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })?;

        if !expired.is_empty() {
            let expired_answer = expired_answer()?;
            db.with(|c| {
                let tx = c.unchecked_transaction()?;
                for question in &expired {
                    tx.execute(
                        "UPDATE questions SET answer = ?1 WHERE id = ?2",
                        rusqlite::params![expired_answer, question.id],
                    )?;
                }
                tx.commit()?;
                Ok(())
            })?;
        }
        Ok(expired)
    }

    /// Remove a question outright (undo of a "never ask this again" answer).
    pub fn delete(db: &Db, id: i64) -> Result<bool> {
        db.with(|c| {
            let changed = c.execute("DELETE FROM questions WHERE id = ?1", [id])?;
            Ok(changed > 0)
        })
    }

    /// How many are waiting, for the Inbox badge.
    pub fn open_count(db: &Db, now: i64) -> Result<u64> {
        db.with(|c| {
            let count: i64 = c.query_row(
                "SELECT COUNT(*) FROM questions
                 WHERE answer IS NULL AND (expires_at IS NULL OR expires_at > ?1)",
                [now],
                |r| r.get(0),
            )?;
            Ok(count as u64)
        })
    }

    fn query(
        db: &Db,
        where_clause: &str,
        args: impl rusqlite::Params,
    ) -> Result<Vec<OpenQuestion>> {
        let sql = format!(
            "SELECT id, job_id, mal_id, payload, created_at, expires_at
             FROM questions WHERE {where_clause} ORDER BY id"
        );
        db.with(|c| {
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map(args, |r| {
                let payload: String = r.get(3)?;
                Ok(OpenQuestion {
                    id: r.get(0)?,
                    job_id: r.get(1)?,
                    mal_id: r.get(2)?,
                    payload: serde_json::from_str(&payload).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                    created_at: r.get(4)?,
                    expires_at: r.get(5)?,
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

#[cfg(test)]
mod tests;
