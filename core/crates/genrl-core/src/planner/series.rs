//! Series planner: turning TMDB episodes and indexer candidates into series jobs.

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::metadata::EpisodeMetadata;
use crate::score::Candidate;

pub const DEFAULT_MIN_MISSING_FOR_PACK: usize = 4;

/// A planned unit of work for a series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannedSeriesJob {
    Pack { season: u32 },
    Single { season: u32, episode: u32 },
}

/// Convert unix seconds to YYYY-MM-DD date string without external dependencies.
pub fn unix_secs_to_date_string(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1020 + doe / 1460 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Fill `media_episodes` from TMDB season data.
/// - Skips season 0 (specials).
/// - Only episodes with air_date <= today are "wanted".
/// - Null or future air_date counts as unaired.
/// - Existing episodes with state='done' are preserved.
pub fn sync_tmdb_episodes(
    db: &Db,
    media_id: i64,
    episodes: &[EpisodeMetadata],
    today: &str,
) -> Result<()> {
    db.with(|c| {
        let tx = c.unchecked_transaction()?;
        for ep in episodes {
            if ep.season == 0 {
                continue;
            }
            let is_aired = ep
                .air_date
                .as_deref()
                .map(|d| !d.is_empty() && d <= today)
                .unwrap_or(false);

            let initial_state = if is_aired { "wanted" } else { "unaired" };

            tx.execute(
                "INSERT INTO media_episodes (media_id, season, episode, air_date, state)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(media_id, season, episode) DO UPDATE SET
                    air_date = excluded.air_date,
                    state = CASE
                        WHEN media_episodes.state = 'done' THEN 'done'
                        WHEN excluded.state = 'wanted' AND media_episodes.state != 'done' THEN 'wanted'
                        ELSE media_episodes.state
                    END",
                rusqlite::params![media_id, ep.season, ep.episode, ep.air_date, initial_state],
            )?;
        }
        tx.commit()?;
        Ok(())
    })
}

/// Check if a candidate release qualifies as a covering season pack for `season`.
pub fn is_covering_pack(
    candidate: &Candidate,
    season: u32,
    all_aired: &[u32],
    is_ongoing_season: bool,
) -> bool {
    if all_aired.is_empty() {
        return false;
    }

    let cand_season = candidate.release.season.unwrap_or_else(|| {
        let lower = candidate.release.raw.to_ascii_lowercase();
        if lower.contains(&format!("s{:02}", season)) || lower.contains(&format!("season {season}"))
        {
            season
        } else {
            0
        }
    });

    if cand_season != season {
        return false;
    }

    let is_pack = candidate.release.is_batch
        || candidate.release.episode_range.is_some()
        || candidate
            .release
            .raw
            .to_ascii_lowercase()
            .contains("complete")
        || candidate
            .release
            .raw
            .to_ascii_lowercase()
            .contains(&format!("season {season}"));

    if !is_pack {
        return false;
    }

    let min_aired = *all_aired.iter().min().unwrap();
    let max_aired = *all_aired.iter().max().unwrap();

    if let Some((start, end)) = candidate.release.episode_range {
        start as u32 <= min_aired && end as u32 >= max_aired
    } else if is_ongoing_season {
        false
    } else {
        candidate.release.is_batch
            || candidate
                .release
                .raw
                .to_ascii_lowercase()
                .contains("complete")
    }
}

/// Decide jobs for a season:
/// - If >= N missing aired episodes AND covering pack candidate exists -> 1 pack job.
/// - Otherwise -> 1 job per missing aired episode.
pub fn plan_season_jobs(
    db: &Db,
    media_id: i64,
    season: u32,
    candidates: &[Candidate],
    min_missing_for_pack: usize,
    today: &str,
    total_season_episodes: Option<u32>,
) -> Result<Vec<PlannedSeriesJob>> {
    let rows: Vec<(u32, String)> = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT episode, state FROM media_episodes
             WHERE media_id = ?1 AND season = ?2 AND air_date IS NOT NULL AND length(trim(air_date)) > 0 AND air_date <= ?3
             ORDER BY episode",
        )?;
        let mapped = stmt.query_map(rusqlite::params![media_id, season, today], |r| {
            Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut list = Vec::new();
        for item in mapped {
            list.push(item?);
        }
        Ok(list)
    })?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let all_aired: Vec<u32> = rows.iter().map(|(ep, _)| *ep).collect();
    let missing_aired: Vec<u32> = rows
        .iter()
        .filter(|(_, state)| state != "done")
        .map(|(ep, _)| *ep)
        .collect();

    if missing_aired.is_empty() {
        return Ok(Vec::new());
    }

    let is_ongoing = total_season_episodes
        .map(|tot| tot > all_aired.len() as u32)
        .unwrap_or(false);

    if missing_aired.len() >= min_missing_for_pack {
        let has_covering_pack = candidates
            .iter()
            .any(|c| is_covering_pack(c, season, &all_aired, is_ongoing));
        if has_covering_pack {
            return Ok(vec![PlannedSeriesJob::Pack { season }]);
        }
    }

    Ok(missing_aired
        .into_iter()
        .map(|ep| PlannedSeriesJob::Single {
            season,
            episode: ep,
        })
        .collect())
}

/// Enqueue planned series jobs into the `jobs` table.
pub fn enqueue_series_jobs(
    db: &Db,
    clock: &dyn Clock,
    media_id: i64,
    planned_jobs: &[PlannedSeriesJob],
) -> Result<usize> {
    let now = clock.now();
    db.with(|c| {
        let tx = c.unchecked_transaction()?;
        for _job in planned_jobs {
            tx.execute(
                "INSERT INTO jobs (kind, media_kind, media_id, state, created_at, updated_at)
                 VALUES ('search', 'series', ?1, 'search', ?2, ?2)",
                rusqlite::params![media_id, now],
            )?;
        }
        tx.commit()?;
        Ok(planned_jobs.len())
    })
}

/// Plan and enqueue jobs for all seasons of a series.
pub fn plan_series(
    db: &Db,
    clock: &dyn Clock,
    media_id: i64,
    candidates: &[Candidate],
    today: &str,
) -> Result<usize> {
    let seasons: Vec<u32> = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT DISTINCT season FROM media_episodes WHERE media_id = ?1 AND season > 0 ORDER BY season",
        )?;
        let mapped = stmt.query_map([media_id], |r| r.get::<_, u32>(0))?;
        let mut list = Vec::new();
        for s in mapped {
            list.push(s?);
        }
        Ok(list)
    })?;

    let min_missing = DEFAULT_MIN_MISSING_FOR_PACK;

    let mut all_jobs = Vec::new();
    for season in seasons {
        let jobs = plan_season_jobs(db, media_id, season, candidates, min_missing, today, None)?;
        all_jobs.extend(jobs);
    }

    enqueue_series_jobs(db, clock, media_id, &all_jobs)
}

/// Convenience entry point using the injected clock's current date.
pub fn plan_series_media(db: &Db, clock: &dyn Clock, media_id: i64) -> Result<usize> {
    let today = unix_secs_to_date_string(clock.now());
    plan_series(db, clock, media_id, &[], &today)
}
