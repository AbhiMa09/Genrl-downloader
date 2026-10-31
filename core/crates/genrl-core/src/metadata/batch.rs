use std::path::Path;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::http::Gate;
use crate::settings::Settings;

use rusqlite::OptionalExtension;

use super::apply::MetadataOutcome;
use super::fetch::fetch_show_metadata_with_client;
use crate::mal::{KeyringStore, MalClient};

/// Queue all shows missing synopsis/poster or with stale metadata (> 7 days).
pub fn queue_stale_metadata(db: &Db, now: i64) -> Result<usize> {
    let stale_threshold = now - (7 * 86400);
    let count: usize = db.with(|c| {
        let changed = c.execute(
            "UPDATE shows SET
                metadata_status = 'pending',
                metadata_next_retry_at = ?1
             WHERE (
                 (metadata_status NOT IN ('ready', 'partial'))
                 OR (metadata_updated_at IS NULL OR metadata_updated_at < ?2)
                 OR (synopsis IS NULL OR trim(synopsis) = '')
                 OR (metadata_status = 'ready' AND poster_path LIKE 'http%')
             )
             AND metadata_status != 'failed'
             AND (metadata_next_retry_at IS NULL OR metadata_next_retry_at <= ?1)",
            rusqlite::params![now, stale_threshold],
        )?;
        Ok(changed)
    })?;
    Ok(count)
}

pub const METADATA_LEASE_SECONDS: i64 = 300; // 5 minutes

/// Atomically claim the next pending metadata job.
/// Uses BEGIN IMMEDIATE to lock out concurrent workers, returning the mal_id
/// and setting metadata_status = 'fetching' so no other worker can claim it.
/// Stale 'fetching' jobs older than METADATA_LEASE_SECONDS are re-claimed.
pub fn claim_next_metadata_job(db: &Db, now: i64) -> Result<Option<i64>> {
    let stale_cutoff = now - METADATA_LEASE_SECONDS;
    db.with(|c| {
        c.execute("BEGIN IMMEDIATE", [])?;
        let res = (|| -> Result<Option<i64>> {
            let id: Option<i64> = c
                .query_row(
                    "SELECT s.mal_id FROM shows s
                     LEFT JOIN mal_entries m ON m.mal_id = s.mal_id
                     WHERE (s.metadata_status = 'pending'
                            OR (s.metadata_status = 'fetching' AND (s.metadata_updated_at IS NULL OR s.metadata_updated_at < ?2)))
                       AND (s.metadata_next_retry_at IS NULL OR s.metadata_next_retry_at <= ?1)
                     ORDER BY
                       CASE WHEN lower(coalesce(m.list_status, '')) = 'watching' THEN 0 ELSE 1 END ASC,
                       s.metadata_retry_count ASC,
                       s.mal_id ASC
                     LIMIT 1",
                    rusqlite::params![now, stale_cutoff],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(mal_id) = id {
                c.execute(
                    "UPDATE shows SET metadata_status = 'fetching', metadata_updated_at = ?2 WHERE mal_id = ?1",
                    rusqlite::params![mal_id, now],
                )?;
            }
            Ok(id)
        })();
        match res {
            Ok(id) => {
                c.execute("COMMIT", [])?;
                Ok(id)
            }
            Err(e) => {
                let _ = c.execute("ROLLBACK", []);
                Err(e)
            }
        }
    })
}

#[derive(Debug, Clone, Default)]
pub struct MetadataBatchReport {
    pub processed: usize,
    pub updated: usize,
    pub failed: usize,
    pub cached: usize,
}

/// Process up to `limit` pending metadata jobs in the queue through Gate.
/// Watching shows are queued first, then the rest at low priority.
pub async fn process_metadata_queue(
    db: &Db,
    gate: &Gate,
    settings: &Settings,
    clock: &dyn Clock,
    data_dir: &Path,
    limit: Option<usize>,
) -> Result<MetadataBatchReport> {
    let max_items = limit.unwrap_or(20);
    let mut report = MetadataBatchReport::default();
    let store = KeyringStore;
    let mal = MalClient::new(gate, settings.clone(), &store, clock);

    for _ in 0..max_items {
        let now = clock.now();
        let mal_id = match claim_next_metadata_job(db, now)? {
            Some(id) => id,
            None => break,
        };
        report.processed += 1;

        match fetch_show_metadata_with_client(
            db, gate, settings, clock, data_dir, mal_id, false, &mal,
        )
        .await
        {
            Ok(MetadataOutcome::Updated { .. }) => report.updated += 1,
            Ok(MetadataOutcome::Cached { .. }) => report.cached += 1,
            Ok(MetadataOutcome::PendingRetry { .. }) | Ok(MetadataOutcome::Failed { .. }) => {
                report.failed += 1;
                // Do not claim and instantly fail every remaining show.
                let service_retry_at =
                    crate::settings::read_state_time(db, "jikan_retry_after_at")?.unwrap_or(0);
                if !gate.would_allow(crate::http::Service::Jikan) || service_retry_at > clock.now()
                {
                    break;
                }
            }
            Err(_) => report.failed += 1,
        }

        if limit.is_some() {
            let delay_ms = (1000.0 / settings.jikan_per_second.max(0.1)) as u64;
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
    }

    Ok(report)
}
