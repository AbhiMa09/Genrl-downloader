use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::models::ListStatus;
use crate::settings::Settings;

use super::db_ops::{ensure_episode, upsert_names, upsert_show};
use super::list::{ListEntry, MalList};
use super::preview::{build_preview, store_pending, PlanOutcome, PlanPreview};

/// Re-check for new episodes this often (the design's weekly cadence).
pub const RECHECK_SECS: i64 = 7 * 24 * 3600;

/// Plan from a list. `force` ignores the weekly cadence.
pub fn plan(
    db: &Db,
    clock: &dyn Clock,
    list: &MalList,
    force: bool,
) -> Result<PlanOutcome> {
    let settings = Settings::load(db)?;
    let now = clock.now();

    // Weekly cadence: cheap to check, and it keeps "re-check for new ones"
    // from becoming "re-plan every launch".
    if !force {
        let last: Option<i64> = db.with(|c| {
            Ok(c
                .query_row(
                    "SELECT value FROM settings WHERE key = 'state.last_plan_at'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .ok()
                .and_then(|raw| serde_json::from_str::<i64>(&raw).ok()))
        })?;
        if let Some(last) = last {
            if now - last < RECHECK_SECS {
                return Ok(PlanOutcome::Preview(PlanPreview {
                    episodes: Vec::new(),
                    skipped: Vec::new(),
                    cap: 0,
                    over_cap: 0,
                    awaiting_confirmation: false,
                }));
            }
        }
    }

    let (preview, _) = build_preview(db, list, &settings)?;

    // The first sync waits for one confirmation; nothing is queued yet.
    if !settings.first_run_complete {
        store_pending(db, &preview)?;
        return Ok(PlanOutcome::Preview(preview));
    }

    let queued = apply_preview(db, clock, list, &preview)?;
    db.with(|c| {
        c.execute(
            "INSERT INTO settings (key, value) VALUES ('state.last_plan_at', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            rusqlite::params![serde_json::to_string(&now)?],
        )?;
        Ok(())
    })?;
    Ok(PlanOutcome::Applied { queued, preview })
}

/// Apply the stored first-sync preview, capped at the (tighter) first-sync cap.
pub fn confirm(db: &Db, clock: &dyn Clock, list: &MalList) -> Result<usize> {
    let pending: Option<PlanPreview> = db.with(|c| {
        Ok(c
            .query_row(
                "SELECT value FROM settings WHERE key = 'state.pending_plan'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok()))
    })?;
    let Some(preview) = pending else {
        return Ok(0);
    };
    let settings = Settings::load(db)?;

    // Confirming is the one confirmation the first sync needs; afterwards the
    // wizard sets first_run_complete and normal weekly planning takes over.
    let queued = apply_preview_capped(db, clock, list, &preview, settings.first_sync_cap as usize)?;
    db.with(|c| {
        c.execute("DELETE FROM settings WHERE key = 'state.pending_plan'", [])?;
        c.execute(
            "INSERT INTO settings (key, value) VALUES ('state.last_plan_at', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            rusqlite::params![serde_json::to_string(&clock.now())?],
        )?;
        Ok(())
    })?;
    Ok(queued)
}

/// Plan and queue an episode or entire season for a specific show directly.
pub fn plan_single(
    db: &Db,
    clock: &dyn Clock,
    mal_id: i64,
    target_episode: Option<f64>,
) -> Result<usize> {
    let entry = db.with(|c| {
        let entry = c.query_row(
            "SELECT s.mal_id, s.title, s.english_title, s.start_year, s.total_episodes, s.season, s.episode_offset
             FROM shows s WHERE s.mal_id = ?1",
            [mal_id],
            |r| {
                let id = r.get::<_, i64>(0)?;
                let title = r.get::<_, String>(1)?;
                let english = r.get::<_, Option<String>>(2)?;
                let year = r.get::<_, Option<i64>>(3)?;
                let total = r.get::<_, Option<i64>>(4)?;
                let season = r.get::<_, Option<i64>>(5)?.unwrap_or(1);
                let offset = r.get::<_, Option<i64>>(6)?.unwrap_or(0);
                Ok(ListEntry {
                    mal_id,
                    title: title.clone(),
                    english_title: english,
                    list_status: ListStatus::Watching,
                    watched_count: 0,
                    total_episodes: total,
                    start_year: year,
                    auto: true,
                    episode_offset: offset,
                    names: vec![title],
                    franchise: id,
                    season,
                })
            },
        )?;
        Ok(entry)
    })?;

    upsert_show(db, &entry)?;
    upsert_names(db, &entry)?;

    let eps_to_queue: Vec<f64> = if let Some(ep) = target_episode {
        vec![ep]
    } else {
        let total = entry.total_episodes.unwrap_or(1);
        let mut list = Vec::new();
        for i in 1..=total {
            let already: i64 = db.with(|c| {
                Ok(c.query_row(
                    "SELECT COUNT(*) FROM episodes WHERE mal_id = ?1 AND number = ?2 AND state = 'done'",
                    rusqlite::params![mal_id, i as f64],
                    |r| r.get(0),
                ).unwrap_or(0))
            })?;
            if already == 0 {
                list.push(i as f64);
            }
        }
        list
    };

    let rules = crate::rules::Rules::for_show(db, mal_id)?;
    let should_batch_season = target_episode.is_none()
        && eps_to_queue.len() > 1
        && rules.prefer_batch_when_finished
        && entry.airing_finished();

    let mut queued = 0;
    if should_batch_season {
        // Ensure all episode rows exist in database
        for ep in &eps_to_queue {
            ensure_episode(db, clock, mal_id, *ep)?;
        }
        // Enqueue one search job for the season (associated with first missing episode)
        if let Some(&first_ep) = eps_to_queue.first() {
            let episode_id = ensure_episode(db, clock, mal_id, first_ep)?;
            crate::jobs::enqueue_search(db, clock, episode_id)?;
            queued += 1;
        }
    } else {
        for ep in eps_to_queue {
            let episode_id = ensure_episode(db, clock, mal_id, ep)?;
            crate::jobs::enqueue_search(db, clock, episode_id)?;
            queued += 1;
        }
    }

    Ok(queued)
}

pub(crate) fn apply_preview(
    db: &Db,
    clock: &dyn Clock,
    list: &MalList,
    preview: &PlanPreview,
) -> Result<usize> {
    let settings = Settings::load(db)?;
    apply_preview_capped(db, clock, list, preview, settings.daily_torrent_cap as usize)
}

pub(crate) fn apply_preview_capped(
    db: &Db,
    clock: &dyn Clock,
    list: &MalList,
    preview: &PlanPreview,
    cap: usize,
) -> Result<usize> {
    // Shows and names first: matching needs the show to exist before a job
    // can search for it, and the names are the whole matching engine's input.
    for entry in &list.entries {
        upsert_show(db, entry)?;
        upsert_names(db, entry)?;
    }

    let now = clock.now();
    let since = crate::jobs::start_of_day(now);
    let added_today = crate::jobs::added_since(db, since)? as usize;
    let mut queued = 0usize;

    for planned in &preview.episodes {
        if added_today + queued >= cap {
            break;
        }
        let episode_id = ensure_episode(db, clock, planned.mal_id, planned.episode)?;
        crate::jobs::enqueue_search(db, clock, episode_id)?;
        queued += 1;
    }

    Ok(queued)
}
