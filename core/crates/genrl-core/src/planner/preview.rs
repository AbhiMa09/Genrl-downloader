use serde::{Deserialize, Serialize};

use crate::clock::Clock;
use crate::db::Db;

use crate::error::Result;
use crate::season::{Overrides, Want};
use crate::settings::Settings;

use super::list::MalList;

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
                format!("; {} more wait for the daily cap", self.over_cap)
            } else {
                String::new()
            }
        )
    }
}

pub(crate) fn plural(count: usize) -> &'static str {
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

/// What `plan` would queue right now, without touching the database
/// (`genrl plan --dry-run`): the same walk, applied to nothing.
pub fn preview(db: &Db, list: &MalList) -> Result<PlanPreview> {
    let settings = Settings::load(db)?;
    Ok(build_preview(db, list, &settings)?.0)
}

/// The stored first-sync preview, if any (the wizard shows it).
pub fn pending_preview(db: &Db) -> Result<Option<PlanPreview>> {
    db.with(|c| {
        Ok(c.query_row(
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

pub(crate) fn store_pending(db: &Db, preview: &PlanPreview) -> Result<()> {
    db.with(|c| {
        c.execute(
            "INSERT INTO settings (key, value) VALUES ('state.pending_plan', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            rusqlite::params![serde_json::to_string(preview)?],
        )?;
        Ok(())
    })
}

/// Walk every chain and decide, without touching the database.
pub(crate) fn build_preview(
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
            .map(|e| {
                crate::rules::Rules::for_show(db, e.mal_id).map(|r| Overrides {
                    auto_plan_to_watch: r.auto_plan_to_watch,
                })
            })
            .transpose()?
            .unwrap_or_default();

        match chain.wanted(settings, overrides) {
            Want::Some(wanted) => {
                let rating_info: Option<(Option<i64>, i64, Option<i64>)> = db.with(|c| {
                    Ok(c.query_row(
                        "SELECT m.score, m.watched_count, s.total_episodes
                         FROM mal_entries m
                         JOIN shows s ON s.mal_id = m.mal_id
                         WHERE m.mal_id = ?1",
                        [wanted.mal_id],
                        |r| {
                            Ok((
                                r.get::<_, Option<i64>>(0)?,
                                r.get::<_, i64>(1)?,
                                r.get::<_, Option<i64>>(2)?,
                            ))
                        },
                    )
                    .ok())
                })?;

                if let Some((Some(score), watched_count, total_episodes)) = rating_info {
                    if score > 0 {
                        let effective_total = total_episodes.filter(|&t| t > 0).or_else(|| {
                            db.with(|c| {
                                crate::season_map::latest_season(c, wanted.mal_id)
                                    .map(|opt| opt.map(|m| m.last_ep as i64))
                            })
                            .ok()
                            .flatten()
                        });

                        let is_fully_watched =
                            effective_total.is_some_and(|total| watched_count >= total);
                        if is_fully_watched {
                            skipped.push(SkipNote {
                                mal_id: wanted.mal_id,
                                title: wanted.title.clone(),
                                reason: format!("{} is fully watched and rated", wanted.title),
                            });
                            continue;
                        }

                        // Rated and partially watched: do not queue. Open inbox question if needed.
                        let now = crate::clock::SystemClock.now();
                        let existing =
                            crate::questions::QuestionStore::for_show(db, wanted.mal_id, now)?;
                        if !existing
                            .iter()
                            .any(|q| q.kind() == crate::models::QuestionKind::RatedShow)
                        {
                            let payload = crate::questions::Payload::RatedShow {
                                mal_id: wanted.mal_id,
                                title: wanted.title.clone(),
                                score,
                                watched_count,
                                total_episodes: effective_total,
                                reason: format!(
                                    "Rated {}/10 on MAL, watched {}/{} episodes",
                                    score,
                                    watched_count,
                                    effective_total
                                        .map(|t| t.to_string())
                                        .unwrap_or_else(|| "?".to_string()),
                                ),
                            };
                            crate::questions::QuestionStore::ask(
                                db,
                                None,
                                Some(wanted.mal_id),
                                &payload,
                                now,
                                None,
                            )?;
                        }

                        skipped.push(SkipNote {
                            mal_id: wanted.mal_id,
                            title: wanted.title.clone(),
                            reason: "rated and partially watched; awaiting user choice".to_string(),
                        });
                        continue;
                    }
                }

                let missing_count: i64 = db.with(|c| {
                    let _ = crate::season_map::ensure_episodes_exist(c, wanted.mal_id);
                    Ok(c.query_row(
                        "SELECT COUNT(*) FROM episodes WHERE mal_id = ?1 AND state = 'missing'",
                        [wanted.mal_id],
                        |r| r.get(0),
                    )
                    .unwrap_or(0))
                })?;
                let target_ep = if missing_count >= settings.pack_first_min_missing as i64 {
                    db.with(|c| {
                        Ok(c.query_row(
                            "SELECT number FROM episodes WHERE mal_id = ?1 AND state = 'missing' ORDER BY number ASC LIMIT 1",
                            [wanted.mal_id],
                            |r| r.get(0),
                        )
                        .ok())
                    })?
                    .unwrap_or(wanted.episode.unwrap_or(1.0))
                } else {
                    wanted.episode.unwrap_or(1.0)
                };

                let already: i64 = db.with(|c| {
                    Ok(c.query_row(
                        "SELECT COUNT(*) FROM episodes e
                         WHERE e.mal_id = ?1 AND e.number = ?2
                           AND (e.state = 'done' OR EXISTS (
                               SELECT 1 FROM jobs j WHERE j.episode_id = e.id AND j.state != 'done'
                           ))",
                        rusqlite::params![wanted.mal_id, target_ep],
                        |r| r.get(0),
                    )
                    .unwrap_or(0))
                })?;
                if already == 0 {
                    episodes.push(PlannedEpisode {
                        mal_id: wanted.mal_id,
                        title: wanted.title,
                        season: wanted.season,
                        episode: target_ep,
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
