//! The planner: turning the user's MAL list into episode rows and jobs.
//!
//! The list decides what is wanted (the design's season and watch-status
//! rules); this module turns that decision into rows. Three rules shape it:
//!
//! * **Nothing is added before the first sync is confirmed.** A first sync
//!   produces a [`PlanPreview`] — "this will queue 214 episodes across 31
//!   shows" — and waits for one confirmation, so a bad import cannot queue
//!   hundreds of episodes at once.
//! * **A daily cap on new torrents** keeps a bad sync from flooding the
//!   client; a first sync is capped tighter still.
//! * **Only episodes up to the aired count**, and re-checking weekly, so an
//!   airing show gains new episodes without anything being queued twice
//!   (`episodes` is unique on `(mal_id, number)` and jobs are idempotent).
//!
//! Until MAL OAuth lands (phase 5) the list comes from a JSON file, which is
//! also how the tests drive this module.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::matching::TargetInputs;
use crate::models::{AiringStatus, ListStatus, ShowType};
use crate::names::{NameSet, PRIORITY_MAIN};
use crate::models::NameSource;
use crate::season::{Overrides, SeasonChain, SeasonEntry, Want};
use crate::settings::Settings;

/// Re-check for new episodes this often (the design's weekly cadence).
pub const RECHECK_SECS: i64 = 7 * 24 * 3600;
/// One show on a list, MAL-shaped so the real sync can reuse the struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListEntry {
    pub mal_id: i64,
    pub title: String,
    #[serde(default)]
    pub english_title: Option<String>,
    pub list_status: ListStatus,
    #[serde(default)]
    pub watched_count: i64,
    pub total_episodes: Option<i64>,
    #[serde(default)]
    pub start_year: Option<i64>,
    #[serde(default)]
    pub auto: bool,
    #[serde(default)]
    pub episode_offset: i64,
    /// Titles and aliases to search; the main one is searched first.
    #[serde(default)]
    pub names: Vec<String>,
    /// Shows sharing a franchise are one season chain, ordered by `season`.
    #[serde(default)]
    pub franchise: i64,
    #[serde(default = "one_season")]
    pub season: i64,
}

fn one_season() -> i64 {
    1
}

impl ListEntry {
    /// Airing status is "finished" unless the entry says otherwise; the fake
    /// list can override it with `status` when a test needs a batch decision.
    fn airing_finished(&self) -> bool {
        // Without a real MAL feed there is no airing status to report, so a
        // fully-watched entry is treated as finished for batch decisions.
        self.total_episodes
            .map(|total| self.watched_count >= total)
            .unwrap_or(true)
    }
}

/// A list as the planner sees it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MalList {
    pub entries: Vec<ListEntry>,
}

impl MalList {
    /// Read a list from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let text = std::fs::read_to_string(path.as_ref())?;
        Ok(serde_json::from_str(&text)?)
    }

    /// Read entries directly from the database's `shows` and `mal_entries` tables.
    pub fn load_from_db(db: &Db) -> Result<Self> {
        db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT s.mal_id, s.title, s.english_title, m.list_status, m.watched_count,
                        s.total_episodes, s.start_year, s.auto, s.season, s.episode_offset
                 FROM shows s
                 JOIN mal_entries m ON s.mal_id = m.mal_id
                 ORDER BY s.title ASC"
            )?;
            let rows = stmt.query_map([], |r| {
                let mal_id = r.get::<_, i64>(0)?;
                let title = r.get::<_, String>(1)?;
                let english_title = r.get::<_, Option<String>>(2)?;
                let status_str = r.get::<_, String>(3)?;
                let watched_count = r.get::<_, i64>(4)?;
                let total_episodes = r.get::<_, Option<i64>>(5)?;
                let start_year = r.get::<_, Option<i64>>(6)?;
                let auto = r.get::<_, i64>(7)? != 0;
                let season = r.get::<_, Option<i64>>(8)?.unwrap_or(1);
                let episode_offset = r.get::<_, Option<i64>>(9)?.unwrap_or(0);

                let list_status = match status_str.as_str() {
                    "watching" => ListStatus::Watching,
                    "completed" => ListStatus::Completed,
                    "on_hold" => ListStatus::OnHold,
                    "dropped" => ListStatus::Dropped,
                    _ => ListStatus::PlanToWatch,
                };

                Ok(ListEntry {
                    mal_id,
                    title: title.clone(),
                    english_title,
                    list_status,
                    watched_count,
                    total_episodes,
                    start_year,
                    auto,
                    episode_offset,
                    names: vec![title],
                    franchise: mal_id,
                    season,
                })
            })?;
            let mut entries = Vec::new();
            for r in rows {
                entries.push(r?);
            }

            for entry in &mut entries {
                if let Ok(mut nstmt) = c.prepare("SELECT name FROM show_names WHERE mal_id = ?1") {
                    if let Ok(names) = nstmt.query_map([entry.mal_id], |nr| nr.get::<_, String>(0)) {
                        for name in names.flatten() {
                            if !entry.names.contains(&name) {
                                entry.names.push(name);
                            }
                        }
                    }
                }
            }

            Ok(MalList { entries })
        })
    }

    /// Group the entries into season chains, ordered by season.
    pub fn chains(&self) -> Vec<SeasonChain> {
        let mut franchises: Vec<i64> = self.entries.iter().map(|e| e.franchise).collect();
        franchises.sort_unstable();
        franchises.dedup();

        let mut out = Vec::new();
        for franchise in franchises {
            let mut entries: Vec<&ListEntry> = self
                .entries
                .iter()
                .filter(|e| e.franchise == franchise)
                .collect();
            entries.sort_by_key(|e| e.season);
            out.push(SeasonChain::new(
                entries.iter().map(|e| to_season_entry(e)).collect(),
            ));
        }
        out
    }
}

fn to_season_entry(entry: &ListEntry) -> SeasonEntry {
    SeasonEntry {
        mal_id: entry.mal_id,
        title: entry.title.clone(),
        list_status: entry.list_status,
        watched_count: entry.watched_count,
        total_episodes: entry.total_episodes,
        episode_offset: entry.episode_offset,
        show_type: ShowType::Tv,
        fully_aired: entry.airing_finished(),
    }
}

/// What the planner intends to do, in one glance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedEpisode {
    pub mal_id: i64,
    pub title: String,
    pub season: i64,
    pub episode: f64,
}

/// Why something was skipped, for the preview and the Activity log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkipNote {
    pub mal_id: i64,
    pub title: String,
    pub reason: String,
}

/// The first-sync preview: everything, before anything is queued.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanPreview {
    pub episodes: Vec<PlannedEpisode>,
    pub skipped: Vec<SkipNote>,
    /// How many episodes the cap allows this run to queue.
    pub cap: usize,
    /// Episodes dropped because the cap was reached.
    pub over_cap: usize,
    /// True when this is the first sync and confirmation is still required.
    pub awaiting_confirmation: bool,
}

impl PlanPreview {
    /// The sentence the preview card shows.
    pub fn summary(&self) -> String {
        let shows = self
            .episodes
            .iter()
            .map(|e| e.mal_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        format!(
            "This will queue {} episode{} across {} show{}{}",
            self.episodes.len(),
            plural(self.episodes.len()),
            shows,
            plural(shows),
            if self.over_cap > 0 {
                format!(
                    "; {} more wait for the daily cap",
                    self.over_cap
                )
            } else {
                String::new()
            }
        )
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

/// What a plan run did.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanOutcome {
    /// A preview: nothing was created, confirmation is needed.
    Preview(PlanPreview),
    /// Episodes and jobs were created (possibly capped).
    Applied { queued: usize, preview: PlanPreview },
}

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

/// What `plan` would queue right now, without touching the database
/// (`genrl plan --dry-run`): the same walk, applied to nothing.
pub fn preview(db: &Db, list: &MalList) -> Result<PlanPreview> {
    let settings = Settings::load(db)?;
    Ok(build_preview(db, list, &settings)?.0)
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

    let mut queued = 0;
    for ep in eps_to_queue {
        let episode_id = ensure_episode(db, clock, mal_id, ep)?;
        crate::jobs::enqueue_search(db, clock, episode_id)?;
        queued += 1;
    }

    Ok(queued)
}

/// The stored first-sync preview, if any (the wizard shows it).
pub fn pending_preview(db: &Db) -> Result<Option<PlanPreview>> {
    db.with(|c| {
        Ok(c
            .query_row(
                "SELECT value FROM settings WHERE key = 'state.pending_plan'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok()))
    })
}

/// Throw the stored preview away (the user said "not now").
pub fn discard_preview(db: &Db) -> Result<()> {
    db.with(|c| {
        c.execute("DELETE FROM settings WHERE key = 'state.pending_plan'", [])?;
        Ok(())
    })
}

/// Walk every chain and decide, without touching the database.
fn build_preview(
    db: &Db,
    list: &MalList,
    settings: &Settings,
) -> Result<(PlanPreview, Vec<PlannedEpisode>)> {
    let mut episodes = Vec::new();
    let mut skipped = Vec::new();

    for chain in list.chains() {
        // Per-show overrides ride along with the chain's first entry.
        let overrides = chain
            .entries
            .first()
            .map(|e| crate::rules::Rules::for_show(db, e.mal_id).map(|r| Overrides {
                auto_plan_to_watch: r.auto_plan_to_watch,
            }))
            .transpose()?
            .unwrap_or_default();

        match chain.wanted(settings, overrides) {
            Want::Some(wanted) => {
                let already: i64 = db.with(|c| {
                    Ok(c
                        .query_row(
                            "SELECT COUNT(*) FROM episodes WHERE mal_id = ?1 AND number = ?2",
                            rusqlite::params![wanted.mal_id, wanted.episode.unwrap_or(1.0)],
                            |r| r.get(0),
                        )
                        .unwrap_or(0))
                })?;
                if already == 0 {
                    episodes.push(PlannedEpisode {
                        mal_id: wanted.mal_id,
                        title: wanted.title,
                        season: wanted.season,
                        episode: wanted.episode.unwrap_or(1.0),
                    });
                }
            }
            Want::Skip { reason, .. } => {
                for entry in &chain.entries {
                    skipped.push(SkipNote {
                        mal_id: entry.mal_id,
                        title: entry.title.clone(),
                        reason: reason.clone(),
                    });
                }
            }
        }
    }

    let cap = settings.daily_torrent_cap as usize;
    let over_cap = episodes.len().saturating_sub(cap);
    let preview = PlanPreview {
        episodes,
        skipped,
        cap,
        over_cap,
        awaiting_confirmation: !settings.first_run_complete,
    };
    let planned = preview.episodes.clone();
    Ok((preview, planned))
}

fn apply_preview(
    db: &Db,
    clock: &dyn Clock,
    list: &MalList,
    preview: &PlanPreview,
) -> Result<usize> {
    let settings = Settings::load(db)?;
    apply_preview_capped(db, clock, list, preview, settings.daily_torrent_cap as usize)
}

fn apply_preview_capped(
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

/// Write a list entry's show row, keeping whatever the sync already knew.
pub fn upsert_show(db: &Db, entry: &ListEntry) -> Result<()> {
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, english_title, type, status, total_episodes,
                                start_year, auto, episode_offset, season)
             VALUES (?1, ?2, ?3, 'tv', 'finished', ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (mal_id) DO UPDATE SET
                title = excluded.title,
                english_title = COALESCE(excluded.english_title, shows.english_title),
                total_episodes = COALESCE(excluded.total_episodes, shows.total_episodes),
                start_year = COALESCE(excluded.start_year, shows.start_year),
                auto = excluded.auto,
                episode_offset = excluded.episode_offset,
                season = excluded.season",
            rusqlite::params![
                entry.mal_id,
                entry.title,
                entry.english_title,
                entry.total_episodes,
                entry.start_year,
                entry.auto as i64,
                entry.episode_offset,
                entry.season,
            ],
        )?;
        Ok(())
    })
}

/// A show's names: the entry's own names, main title first.
pub fn upsert_names(db: &Db, entry: &ListEntry) -> Result<()> {
    let mut names: Vec<(String, NameSource, u8)> = Vec::new();
    if entry.names.is_empty() {
        names.push((entry.title.clone(), NameSource::Mal, PRIORITY_MAIN));
    } else {
        for (index, name) in entry.names.iter().enumerate() {
            let priority = if index == 0 { PRIORITY_MAIN } else { index.min(u8::MAX as usize) as u8 };
            names.push((name.clone(), NameSource::Mal, priority));
        }
    }
    NameSet::replace_from_sync(db, entry.mal_id, &names)
}

/// The episode row, created once (`UNIQUE (mal_id, number)` makes a second
/// call a no-op, which is what makes the weekly planner safe to re-run).
pub fn ensure_episode(db: &Db, clock: &dyn Clock, mal_id: i64, number: f64) -> Result<i64> {
    let now = clock.now();
    db.with(|c| {
        let tx = c.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO episodes (mal_id, number, state, last_checked)
             VALUES (?1, ?2, 'missing', ?3)
             ON CONFLICT (mal_id, number) DO UPDATE SET last_checked = excluded.last_checked",
            rusqlite::params![mal_id, number, now],
        )?;
        // An DO UPDATE leaves last_insert_rowid() stale, so read the row back:
        // the conflict target is unique, so exactly one id answers.
        let id: i64 = tx.query_row(
            "SELECT id FROM episodes WHERE mal_id = ?1 AND number = ?2",
            rusqlite::params![mal_id, number],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(id)
    })
}

/// Mark an episode owned (the finalize step's outcome).
pub fn mark_done(db: &Db, episode_id: i64, file_path: &str, size: i64) -> Result<()> {
    db.with(|c| {
        c.execute(
            "UPDATE episodes SET state = 'done', file_path = ?2, size = ?3 WHERE id = ?1",
            rusqlite::params![episode_id, file_path, size],
        )?;
        Ok(())
    })
}

/// Mark an episode missing again (an expired question released it).
pub fn mark_missing(db: &Db, episode_id: i64) -> Result<()> {
    db.with(|c| {
        c.execute(
            "UPDATE episodes SET state = 'missing' WHERE id = ?1",
            [episode_id],
        )?;
        Ok(())
    })
}

/// Load a show's matching inputs (re-exported for the workers).
pub fn target_inputs(db: &Db, mal_id: i64) -> Result<TargetInputs> {
    TargetInputs::load(db, mal_id)
}

/// Airing status for a show the sync wrote.
pub fn airing_status_of(db: &Db, mal_id: i64) -> Result<AiringStatus> {
    db.with(|c| {
        Ok(c
            .query_row(
                "SELECT status FROM shows WHERE mal_id = ?1",
                [mal_id],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .map(|s| AiringStatus::parse(&s))
            .unwrap_or_default())
    })
}

fn store_pending(db: &Db, preview: &PlanPreview) -> Result<()> {
    db.with(|c| {
        c.execute(
            "INSERT INTO settings (key, value) VALUES ('state.pending_plan', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            rusqlite::params![serde_json::to_string(preview)?],
        )?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock() -> crate::clock::FakeClock {
        crate::clock::FakeClock::new(1_789_227_900)
    }

    /// S1 finished, S2 four episodes in: the design's fixture chain.
    fn list() -> MalList {
        MalList {
            entries: vec![
                ListEntry {
                    mal_id: 100,
                    title: "Show".to_string(),
                    english_title: None,
                    list_status: ListStatus::Completed,
                    watched_count: 12,
                    total_episodes: Some(12),
                    start_year: Some(2024),
                    auto: false,
                    episode_offset: 0,
                    names: vec!["Show".to_string()],
                    franchise: 1,
                    season: 1,
                },
                ListEntry {
                    mal_id: 200,
                    title: "Show 2".to_string(),
                    english_title: Some("Show Two".to_string()),
                    list_status: ListStatus::Watching,
                    watched_count: 4,
                    total_episodes: Some(12),
                    start_year: Some(2025),
                    auto: true,
                    episode_offset: 0,
                    names: vec!["Show 2".to_string()],
                    franchise: 1,
                    season: 2,
                },
            ],
        }
    }

    fn first_sync_done(db: &Db) {
        let mut settings = Settings::load(db).unwrap();
        settings.first_run_complete = true;
        Settings::save(&settings, db).unwrap();
    }

    #[test]
    fn a_first_sync_previews_and_adds_nothing() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        let outcome = plan(&db, &clock, &list(), false).unwrap();

        let PlanOutcome::Preview(preview) = outcome else {
            panic!("the first sync must only preview");
        };
        assert!(preview.awaiting_confirmation);
        assert_eq!(preview.episodes.len(), 1, "only S2 wants an episode");
        assert_eq!(preview.episodes[0].episode, 5.0);
        assert_eq!(preview.episodes[0].season, 2);
        assert!(preview.summary().contains("1 episode across 1 show"));

        // Nothing written: no shows, no episodes, no jobs.
        let shows: i64 = db.with(|c| Ok(c.query_row("SELECT COUNT(*) FROM shows", [], |r| r.get(0))?)).unwrap();
        let episodes: i64 = db.with(|c| Ok(c.query_row("SELECT COUNT(*) FROM episodes", [], |r| r.get(0))?)).unwrap();
        let jobs: i64 = db.with(|c| Ok(c.query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))?)).unwrap();
        assert_eq!((shows, episodes, jobs), (0, 0, 0));

        // The preview waits in settings for the one confirmation.
        assert!(pending_preview(&db).unwrap().is_some());

        let queued = confirm(&db, &clock, &list()).unwrap();
        assert_eq!(queued, 1, "confirmation applies exactly what was previewed");
        assert!(pending_preview(&db).unwrap().is_none());
    }

    #[test]
    fn confirming_a_second_time_does_nothing() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        assert!(matches!(plan(&db, &clock, &list(), false).unwrap(), PlanOutcome::Preview(_)));
        assert_eq!(confirm(&db, &clock, &list()).unwrap(), 1);
        assert_eq!(confirm(&db, &clock, &list()).unwrap(), 0, "no preview left");
    }

    #[test]
    fn a_later_sync_applies_immediately_and_is_idempotent() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        first_sync_done(&db);

        let PlanOutcome::Applied { queued, .. } = plan(&db, &clock, &list(), false).unwrap() else {
            panic!("after the first sync, planning applies");
        };
        assert_eq!(queued, 1);

        // Same list again: the episode and the job already exist.
        let again = plan(&db, &clock, &list(), true).unwrap();
        assert!(
            matches!(&again, PlanOutcome::Applied { queued: 0, .. }),
            "the weekly re-check must be a no-op: {again:?}"
        );
        let jobs: i64 = db.with(|c| Ok(c.query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))?)).unwrap();
        assert_eq!(jobs, 1, "duplicate enqueue is a no-op");
    }

    #[test]
    fn the_weekly_cadence_holds_until_it_passes() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        first_sync_done(&db);

        assert!(matches!(plan(&db, &clock, &list(), false).unwrap(), PlanOutcome::Applied { .. }));
        // Mark S2E5 owned so the next plan has nothing new to offer.
        db.with(|c| {
            c.execute("UPDATE episodes SET state = 'done' WHERE number = 5", [])?;
            Ok(())
        })
        .unwrap();
        db.with(|c| {
            c.execute("UPDATE shows SET auto = 0 WHERE mal_id = 100", [])?;
            Ok(())
        })
        .unwrap();

        clock.advance(24 * 3600);
        let too_soon = plan(&db, &clock, &list(), false).unwrap();
        assert!(
            matches!(&too_soon, PlanOutcome::Preview(p) if p.episodes.is_empty() && !p.awaiting_confirmation),
            "inside a week the planner stays quiet: {too_soon:?}"
        );

        clock.advance(6 * 24 * 3600);
        let forced = plan(&db, &clock, &list(), true).unwrap();
        assert!(matches!(forced, PlanOutcome::Applied { .. }), "force ignores the cadence");
    }

    #[test]
    fn plan_to_watch_is_skipped_in_the_preview_with_a_reason() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        let mut planned = list();
        planned.entries[1].list_status = ListStatus::PlanToWatch;
        let outcome = plan(&db, &clock, &planned, false).unwrap();
        let PlanOutcome::Preview(preview) = outcome else {
            panic!("first sync previews");
        };
        assert!(preview.episodes.is_empty());
        assert_eq!(preview.skipped.len(), 2, "every entry in the chain is reported");
        assert!(preview.skipped[1].reason.contains("plan to watch"), "{:?}", preview.skipped);
    }

    #[test]
    fn only_episodes_up_to_the_aired_count_are_queued() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        first_sync_done(&db);
        let mut planned = list();
        // S2 has five aired; the user has watched none of them: only 1 is next.
        planned.entries[1].watched_count = 0;
        planned.entries[1].total_episodes = Some(5);
        let PlanOutcome::Applied { queued, preview } = plan(&db, &clock, &planned, false).unwrap() else {
            panic!("applies");
        };
        assert_eq!(queued, 1);
        assert_eq!(preview.episodes[0].episode, 1.0, "next unwatched, never the whole season");
    }

    #[test]
    fn the_daily_cap_holds() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        let mut settings = Settings::load(&db).unwrap();
        settings.first_run_complete = true;
        settings.daily_torrent_cap = 3;
        Settings::save(&settings, &db).unwrap();

        // Five shows each wanting one episode.
        let many = MalList {
            entries: (1..=5)
                .map(|i| ListEntry {
                    mal_id: i * 10,
                    title: format!("Show {i}"),
                    english_title: None,
                    list_status: ListStatus::Watching,
                    watched_count: 0,
                    total_episodes: Some(12),
                    start_year: Some(2025),
                    auto: false,
                    episode_offset: 0,
                    names: vec![format!("Show {i}")],
                    franchise: i * 10,
                    season: 1,
                })
                .collect(),
        };
        let PlanOutcome::Applied { queued, preview } = plan(&db, &clock, &many, false).unwrap() else {
            panic!("applies");
        };
        assert_eq!(queued, 3, "the cap holds");
        assert_eq!(preview.over_cap, 2, "the rest are shown as waiting");
    }

    #[test]
    fn a_fake_list_loads_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("list.json");
        std::fs::write(&path, serde_json::to_string_pretty(&list()).unwrap()).unwrap();
        let loaded = MalList::load(&path).unwrap();
        assert_eq!(loaded.chains().len(), 1);
        assert_eq!(loaded.chains()[0].len(), 2);
        let entries = &loaded.chains()[0].entries;
        assert_eq!(entries[0].mal_id, 100);
        assert_eq!(entries[1].mal_id, 200);
    }
}
