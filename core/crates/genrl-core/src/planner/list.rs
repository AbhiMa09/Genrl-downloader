use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::Result;
use crate::models::{ListStatus, ShowType};
use crate::season::{SeasonChain, SeasonEntry};

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
    pub(crate) fn airing_finished(&self) -> bool {
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

pub(crate) fn to_season_entry(entry: &ListEntry) -> SeasonEntry {
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
