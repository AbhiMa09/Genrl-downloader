use std::path::Path;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::http::Gate;
use crate::settings::Settings;

use super::apply::MetadataOutcome;
use super::fetch::fetch_show_metadata;

/// Queue all shows missing synopsis/poster or with stale metadata (> 7 days).
pub fn queue_stale_metadata(db: &Db, now: i64) -> Result<usize> {
    let stale_threshold = now - (7 * 86400);
    let count: usize = db.with(|c| {
        let changed = c.execute(
            "UPDATE shows SET
                metadata_status = 'pending',
                metadata_next_retry_at = ?1
             WHERE (metadata_status != 'ready'
                    OR metadata_updated_at IS NULL
                    OR metadata_updated_at < ?2
                    OR synopsis IS NULL
                    OR trim(synopsis) = ''
                    OR poster_path IS NULL
                    OR trim(poster_path) = ''
                    OR poster_path LIKE 'http%')
               AND metadata_status != 'failed'
               AND (metadata_next_retry_at IS NULL OR metadata_next_retry_at <= ?1)",
            rusqlite::params![now, stale_threshold],
        )?;
        Ok(changed)
    })?;
    Ok(count)
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
    let now = clock.now();
    let max_items = limit.unwrap_or(20);

    let pending_ids: Vec<i64> = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT s.mal_id FROM shows s
             LEFT JOIN mal_entries m ON m.mal_id = s.mal_id
             WHERE s.metadata_status = 'pending'
               AND (s.metadata_next_retry_at IS NULL OR s.metadata_next_retry_at <= ?1)
             ORDER BY
               CASE WHEN lower(coalesce(m.list_status, '')) = 'watching' THEN 0 ELSE 1 END ASC,
               s.metadata_retry_count ASC,
               s.mal_id ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![now, max_items as i64], |r| r.get(0))?;
        Ok(rows.filter_map(std::result::Result::ok).collect())
    })?;

    let mut report = MetadataBatchReport {
        processed: pending_ids.len(),
        ..Default::default()
    };

    for id in pending_ids {
        match fetch_show_metadata(db, gate, settings, clock, data_dir, id, false).await {
            Ok(MetadataOutcome::Updated { .. }) => report.updated += 1,
            Ok(MetadataOutcome::Cached { .. }) => report.cached += 1,
            Ok(MetadataOutcome::PendingRetry { .. }) | Ok(MetadataOutcome::Failed { .. }) => {
                report.failed += 1
            }
            Err(_) => report.failed += 1,
        }
    }

    Ok(report)
}
