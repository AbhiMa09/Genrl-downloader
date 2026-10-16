use serde::{Deserialize, Serialize};

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
                let already: i64 = db.with(|c| {
                    Ok(c.query_row(
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
