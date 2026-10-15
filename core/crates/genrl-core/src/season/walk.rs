use crate::models::{ListStatus, ShowType};
use crate::settings::Settings;

use super::types::{Overrides, SeasonChain, SeasonEntry, Want, Wanted};

impl SeasonChain {
    /// Walk the list and say what it wants next.
    ///
    /// Completed entries are walked past. The walk stops at the first entry
    /// that still wants an answer: Watching (its next unwatched episode), Plan
    /// to watch (only with the switch on — off by default so 73 planned shows
    /// never queue at once), or a status that skips. On hold and dropped stop
    /// the walk, because watching is in order.
    pub fn wanted(&self, settings: &Settings, overrides: Overrides) -> Want {
        let plan_allowed = overrides
            .auto_plan_to_watch
            .unwrap_or(settings.auto_plan_to_watch);

        for entry in &self.entries {
            match entry.list_status {
                // Fully watched behaves like Completed, but the app mentions
                // the suggestion rather than silently moving on.
                ListStatus::Completed => continue,
                ListStatus::OnHold => {
                    return Want::Skip {
                        reason: format!("{} is on hold", entry.title),
                        fully_watched: false,
                    }
                }
                ListStatus::Dropped => {
                    return Want::Skip {
                        reason: format!("{} is dropped", entry.title),
                        fully_watched: false,
                    }
                }
                ListStatus::PlanToWatch => {
                    if !plan_allowed {
                        return Want::Skip {
                            reason: format!(
                                "{} is plan to watch and Plan to watch is manual by default",
                                entry.title
                            ),
                            fully_watched: false,
                        };
                    }
                    if let Some(total) = entry.total_episodes {
                        if entry.watched_count >= total {
                            continue;
                        }
                    }
                    return Want::Some(self.wanted_from(entry, entry.watched_count + 1));
                }
                ListStatus::Watching => {
                    if let Some(total) = entry.total_episodes {
                        if entry.watched_count >= total {
                            // Nothing left on this entry: walk on to the next
                            // season, and say why in the log.
                            return Want::Skip {
                                reason: format!(
                                    "{} is fully watched — consider marking it Completed",
                                    entry.title
                                ),
                                fully_watched: true,
                            };
                        }
                    }
                    return Want::Some(self.wanted_from(entry, entry.watched_count + 1));
                }
            }
        }

        Want::Skip {
            reason: "nothing left to download in this chain".to_string(),
            fully_watched: false,
        }
    }

    fn wanted_from(&self, entry: &SeasonEntry, episode: i64) -> Wanted {
        Wanted {
            mal_id: entry.mal_id,
            title: entry.title.clone(),
            season: self.season_of(entry.mal_id).unwrap_or(1),
            episode: match entry.show_type {
                // A movie or feature is one "episode" in the episodes table.
                ShowType::Movie | ShowType::Ova | ShowType::Ona | ShowType::Special => {
                    Some(episode as f64)
                }
                ShowType::Tv => Some(episode as f64),
            },
        }
    }
}
