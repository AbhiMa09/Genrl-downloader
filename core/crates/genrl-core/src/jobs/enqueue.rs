use rusqlite::OptionalExtension;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::events::{self, Level};

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

/// Create the job that fetches episode `episode_id`, or return the one that
/// already covers it.
///
/// Idempotency is the point: the planner runs weekly, reconcile runs on every
/// launch, and neither may double-queue. Episodes are unique on
/// `(mal_id, number)`, so the episode row is the identity; a job that is not
/// terminal already exists for it means "already queued".
pub fn enqueue_search(db: &Db, clock: &dyn Clock, episode_id: i64) -> Result<i64> {
    let existing: Option<i64> = db.with(|c| {
        Ok(c.query_row(
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
