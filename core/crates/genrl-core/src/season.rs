//! The season chain, and the pure decision function that says what a release
//! is good for.
//!
//! The MAL id is the identity of a show, and **each season is its own MAL
//! entry**. What to download follows from the user's list — the status, the
//! watched count and where the release's number sits against the entry's
//! episode count — never from the release name alone.
//!
//! Two pure functions do all of it, which is why the Phase 3 check can run as
//! fixtures with a fake list and no network:
//!
//! * [`SeasonChain::wanted`] — what the list wants next (or why it wants
//!   nothing): Completed entries are walked past, the first entry that still
//!   wants work decides. Fully-watched is skipped with a suggestion, never an
//!   error. On hold, dropped and Plan to watch are skipped unless switched on.
//! * [`decide`] — given that want and one parsed release: download, skip (with
//!   a reason), ask (queue a question) or not found. A season the chain does
//!   not have is *never* substituted with another, and an episode number
//!   beyond the entry's count is treated as absolute and mapped through the
//!   earlier seasons' cumulative counts.

use crate::filename::Release;
use crate::models::{ListStatus, ShowType};
use crate::settings::Settings;

/// One MAL entry in the chain: a season of the franchise.
#[derive(Debug, Clone, PartialEq)]
pub struct SeasonEntry {
    pub mal_id: i64,
    pub title: String,
    pub list_status: ListStatus,
    /// Episodes the user has watched on this entry.
    pub watched_count: i64,
    /// `None` while MAL does not know the count (unannounced or weird shows).
    pub total_episodes: Option<i64>,
    /// Absolute-numbering offset for this entry: 12 when releases of season 2
    /// are numbered from 13. `0` means releases are numbered per season.
    pub episode_offset: i64,
    pub show_type: ShowType,
    /// Finished airing: a batch counts for a finished show, not an airing one.
    pub fully_aired: bool,
}

/// The franchise ordered first season → last, built from MAL's sequel and
/// prequel relations.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SeasonChain {
    pub entries: Vec<SeasonEntry>,
}

/// Per-show switches the walk consults (from the rules table).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Overrides {
    /// Per-show override of the Plan to watch switch.
    pub auto_plan_to_watch: Option<bool>,
}

/// What the list wants next.
#[derive(Debug, Clone, PartialEq)]
pub struct Wanted {
    pub mal_id: i64,
    pub title: String,
    /// 1-based position in the chain (season 1 → 1).
    pub season: i64,
    /// `None` for a movie or another feature without episode numbers.
    pub episode: Option<f64>,
}

/// The answer to "what does this chain want?".
#[derive(Debug, Clone, PartialEq)]
pub enum Want {
    /// An episode to fetch.
    Some(Wanted),
    /// Nothing to do, with the reason for the Activity log.
    Skip {
        reason: String,
        /// Watched everything: the app suggests marking the entry Completed.
        fully_watched: bool,
    },
}

/// What to do with one parsed release.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// This release delivers the wanted episode of the right season.
    Download { season: i64, episode: f64 },
    /// Right show, wrong episode or state: keep looking.
    Skip { reason: String },
    /// The numbers do not fit the chain; the user must decide.
    Ask { reason: String },
    /// No release exists for the wanted season, and another season is never
    /// substituted for it.
    NotFound { reason: String },
}

impl SeasonChain {
    pub fn new(entries: Vec<SeasonEntry>) -> Self {
        SeasonChain { entries }
    }

    /// The chain, ordered, as long as the entries are given in order.
    pub fn len(&self) -> i64 {
        self.entries.len() as i64
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entry(&self, mal_id: i64) -> Option<&SeasonEntry> {
        self.entries.iter().find(|e| e.mal_id == mal_id)
    }

    /// 1-based season number of an entry, or `None` when it is not ours.
    pub fn season_of(&self, mal_id: i64) -> Option<i64> {
        self.entries
            .iter()
            .position(|e| e.mal_id == mal_id)
            .map(|i| i as i64 + 1)
    }

    /// The entry at a 1-based season number.
    pub fn by_season(&self, season: i64) -> Option<&SeasonEntry> {
        if season < 1 || season as usize > self.entries.len() {
            return None;
        }
        self.entries.get(season as usize - 1)
    }

    /// Cumulative episode count of every entry *before* this season, or `None`
    /// when any earlier entry's count is unknown (absolute numbering is then
    /// unanswerable and the app asks instead of guessing).
    fn episodes_before(&self, season: i64) -> Option<i64> {
        let mut total = 0;
        for entry in &self.entries[..(season.max(1) - 1).max(0) as usize] {
            total += entry.total_episodes?;
        }
        Some(total)
    }

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

/// What to do with one parsed release for one wanted episode.
///
/// The release is *judged*, never repaired: reads strictly for the want, and
/// anything it cannot map into the chain becomes an `Ask` rather than a guess.
pub fn decide(chain: &SeasonChain, wanted: &Wanted, release: &Release) -> Decision {
    let entry = match chain.entry(wanted.mal_id) {
        Some(entry) => entry,
        None => {
            return Decision::NotFound {
                reason: format!("no chain entry for {}", wanted.mal_id),
            }
        }
    };
    let wanted_season = wanted.season;

    // ------------------------------------------------------------- the season
    // A marker must name the wanted season. Anything else — including a season
    // the chain does not even have — is not found, never substituted.
    if let Some(found) = release.season {
        let found = i64::from(found);
        if found != wanted_season {
            return Decision::NotFound {
                reason: format!(
                    "release is for season {found}, but {} wants season {wanted_season}",
                    wanted.title
                ),
            };
        }
    }
    // No marker means "the active season": the one the walk landed on.

    // A movie or other feature wants no episode number.
    let Some(w) = wanted.episode else {
        return match release.episode {
            // Movies listed as "1 of 1" are the movie.
            None | Some(1.0) => Decision::Download {
                season: wanted_season,
                episode: 1.0,
            },
            Some(found) => Decision::Ask {
                reason: format!("episode-numbered release ({found}) for a feature without episodes"),
            },
        };
    };

    // A batch carries no number; it either covers a finished show or it asks.
    if release.is_batch && release.episode.is_none() {
        return if entry.fully_aired {
            Decision::Download {
                season: wanted_season,
                episode: w,
            }
        } else {
            Decision::Ask {
                reason: "only a batch is available and the show is still airing".to_string(),
            }
        };
    }

    let found = match release.episode {
        Some(found) => found,
        // No number and not a batch: no way to tell what it delivers.
        None => {
            return Decision::Ask {
                reason: "release has no episode number".to_string(),
            }
        }
    };

    // The numbers the release may legitimately carry for our want: the
    // per-season number, and any offset the show is known to use.
    let mut accepted = vec![w];
    if entry.episode_offset != 0 {
        accepted.push(w + entry.episode_offset as f64);
    }
    if accepted.iter().any(|want| (want - found).abs() < f64::EPSILON) {
        return Decision::Download {
            season: wanted_season,
            episode: w,
        };
    }

    // Already watched (or simply an earlier unwatched episode): keep looking.
    if found < w {
        if found as i64 <= entry.watched_count {
            return Decision::Skip {
                reason: format!("episode {found} is already watched"),
            };
        }
        return Decision::Skip {
            reason: format!("episode {found} is earlier than the wanted episode {w}"),
        };
    }

    // found > w. Beyond the entry's own count the number must be absolute:
    // subtract the earlier seasons' cumulative counts and see where it lands.
    match entry.total_episodes {
        Some(total) if found as i64 > total => {
            let Some(cumulative) = chain.episodes_before(wanted_season) else {
                return Decision::Ask {
                    reason: format!(
                        "episode {found} looks absolute, but an earlier season's episode count is unknown"
                    ),
                };
            };
            let mapped = found as i64 - cumulative;
            if mapped == w as i64 {
                return Decision::Download {
                    season: wanted_season,
                    episode: w,
                };
            }
            if (1..=total).contains(&mapped) {
                return Decision::Skip {
                    reason: format!("absolute episode {found} is episode {mapped}, not {w}"),
                };
            }
            Decision::Ask {
                reason: format!("episode {found} does not fit the season chain"),
            }
        }
        Some(_) => Decision::Skip {
            reason: format!("episode {found} is later than the wanted episode {w}"),
        },
        // The count is unknown: refuse to guess.
        None => Decision::Ask {
            reason: format!(
                "episode {found} is beyond the wanted {w} and the entry has no episode count"
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filename::parse_release;

    fn tv(mal_id: i64, title: &str, status: ListStatus, watched: i64, total: Option<i64>) -> SeasonEntry {
        SeasonEntry {
            mal_id,
            title: title.to_string(),
            list_status: status,
            watched_count: watched,
            total_episodes: total,
            episode_offset: 0,
            show_type: ShowType::Tv,
            fully_aired: true,
        }
    }

    /// The chain the design's fixtures use: season 1 finished (12 episodes),
    /// season 2 airing through 12 so far.
    fn chain() -> SeasonChain {
        SeasonChain::new(vec![
            tv(100, "Show", ListStatus::Completed, 12, Some(12)),
            tv(200, "Show 2", ListStatus::Watching, 4, Some(12)),
        ])
    }

    fn settings() -> Settings {
        Settings::default()
    }

    // ------------------------------------------------------------- Phase 3 fixtures

    #[test]
    fn s1_completed_plus_s2_watching_gives_s2_next() {
        let want = chain().wanted(&settings(), Overrides::default());
        assert_eq!(
            want,
            Want::Some(Wanted {
                mal_id: 200,
                title: "Show 2".to_string(),
                season: 2,
                episode: Some(5.0),
            })
        );
    }

    #[test]
    fn no_season_marker_resolves_to_the_active_season() {
        let chain = chain();
        let Want::Some(want) = chain.wanted(&settings(), Overrides::default()) else {
            panic!("expected a want");
        };
        // "Show - 05": no marker at all.
        let release = parse_release("[SubsPlease] Show - 05 (1080p) [ABC12345].mkv");
        assert_eq!(release.season, None);
        assert_eq!(release.episode, Some(5.0));
        assert_eq!(
            decide(&chain, &want, &release),
            Decision::Download { season: 2, episode: 5.0 }
        );
    }

    #[test]
    fn absolute_numbering_becomes_season_two_episode_five() {
        let chain = chain();
        let Want::Some(want) = chain.wanted(&settings(), Overrides::default()) else {
            panic!("expected a want");
        };
        // "- 17" with season 1 holding 12 episodes: absolute, 17 - 12 = 5.
        let release = parse_release("[SubsPlease] Show - 17 (1080p) [ABC12345].mkv");
        assert_eq!(release.episode, Some(17.0));
        assert_eq!(
            decide(&chain, &want, &release),
            Decision::Download { season: 2, episode: 5.0 }
        );
    }

    #[test]
    fn absolute_numbering_that_lands_elsewhere_is_skipped() {
        let chain = chain();
        let Want::Some(want) = chain.wanted(&settings(), Overrides::default()) else {
            panic!("expected a want");
        };
        // 16 - 12 = 4, which is the episode the user already watched.
        let release = parse_release("[SubsPlease] Show - 16 (1080p) [ABC12345].mkv");
        assert_eq!(
            decide(&chain, &want, &release),
            Decision::Skip {
                reason: "absolute episode 16 is episode 4, not 5".to_string()
            }
        );
    }

    #[test]
    fn watched_episodes_are_skipped() {
        let chain = chain();
        let Want::Some(want) = chain.wanted(&settings(), Overrides::default()) else {
            panic!("expected a want");
        };
        let release = parse_release("[SubsPlease] Show 2nd Season - 04 (1080p) [ABC12345].mkv");
        assert_eq!(
            decide(&chain, &want, &release),
            Decision::Skip {
                reason: "episode 4 is already watched".to_string()
            }
        );
    }

    #[test]
    fn a_missing_season_is_not_found_never_another_season() {
        let chain = chain();
        let Want::Some(want) = chain.wanted(&settings(), Overrides::default()) else {
            panic!("expected a want");
        };
        // Only season 1 exists for this want, and the only release is season 3.
        let release = parse_release("[SubsPlease] Show S03E05 (1080p) [ABC12345].mkv");
        match decide(&chain, &want, &release) {
            Decision::NotFound { .. } => {}
            other => panic!("expected NotFound, got {other:?}"),
        }

        // A season 1 release for a season 2 want is likewise never substituted.
        let release = parse_release("[SubsPlease] Show - S01E05 (1080p) [ABC12345].mkv");
        assert!(matches!(
            decide(&chain, &want, &release),
            Decision::NotFound { .. }
        ));
    }

    #[test]
    fn plan_to_watch_is_skipped_by_default() {
        let chain = SeasonChain::new(vec![
            tv(100, "Show", ListStatus::Completed, 12, Some(12)),
            tv(300, "Show 2", ListStatus::PlanToWatch, 0, Some(12)),
        ]);
        let Want::Skip { reason, fully_watched } = chain.wanted(&settings(), Overrides::default()) else {
            panic!("expected a skip");
        };
        assert!(reason.contains("plan to watch"), "{reason}");
        assert!(!fully_watched);

        // The global switch (or a per-show override) opts it in.
        let mut on = settings();
        on.auto_plan_to_watch = true;
        assert!(matches!(
            chain.wanted(&on, Overrides::default()),
            Want::Some(_)
        ));
        assert!(matches!(
            chain.wanted(&settings(), Overrides { auto_plan_to_watch: Some(true) }),
            Want::Some(_)
        ));
    }

    #[test]
    fn fully_watched_suggests_completing_rather_than_erroring() {
        let chain = SeasonChain::new(vec![tv(
            100,
            "Show",
            ListStatus::Watching,
            12,
            Some(12),
        )]);
        let Want::Skip { reason, fully_watched } = chain.wanted(&settings(), Overrides::default()) else {
            panic!("expected a skip");
        };
        assert!(fully_watched, "the suggestion to mark Completed");
        assert!(reason.contains("fully watched"), "{reason}");
    }

    #[test]
    fn watching_starts_after_the_watched_count() {
        // A missing watched count is treated as 0.
        let chain = SeasonChain::new(vec![SeasonEntry {
            list_status: ListStatus::Watching,
            watched_count: 0,
            ..tv(100, "Show", ListStatus::Watching, 0, Some(12))
        }]);
        assert!(matches!(
            chain.wanted(&settings(), Overrides::default()),
            Want::Some(w) if w.episode == Some(1.0)
        ));
    }

    #[test]
    fn on_hold_and_dropped_stop_the_walk() {
        for status in [ListStatus::OnHold, ListStatus::Dropped] {
            let chain = SeasonChain::new(vec![
                tv(100, "Show", ListStatus::Completed, 12, Some(12)),
                tv(200, "Show 2", status, 3, Some(12)),
            ]);
            assert!(matches!(
                chain.wanted(&settings(), Overrides::default()),
                Want::Skip { .. }
            ));
        }
    }

    // ------------------------------------------------------------------ decision

    #[test]
    fn a_per_show_offset_is_one_of_the_numbers_tried() {
        // Uploaders number this show's releases 12 ahead of the MAL episodes.
        let chain = SeasonChain::new(vec![SeasonEntry {
            episode_offset: 12,
            ..tv(100, "Show", ListStatus::Watching, 4, Some(12))
        }]);
        let want = Wanted {
            mal_id: 100,
            title: "Show".to_string(),
            season: 1,
            episode: Some(5.0),
        };
        let release = parse_release("[SubsPlease] Show - 17 (1080p) [ABC12345].mkv");
        assert_eq!(
            decide(&chain, &want, &release),
            Decision::Download { season: 1, episode: 5.0 }
        );
    }

    #[test]
    fn an_episode_beyond_the_count_asks_rather_than_guessing() {
        // Unknown earlier counts: absolute numbering is unanswerable.
        let chain = SeasonChain::new(vec![
            tv(100, "Show", ListStatus::Completed, 12, None),
            tv(200, "Show 2", ListStatus::Watching, 4, Some(12)),
        ]);
        let want = Wanted {
            mal_id: 200,
            title: "Show 2".to_string(),
            season: 2,
            episode: Some(5.0),
        };
        let release = parse_release("[SubsPlease] Show - 30 (1080p) [ABC12345].mkv");
        assert!(matches!(
            decide(&chain, &want, &release),
            Decision::Ask { .. }
        ));
    }

    #[test]
    fn a_batch_for_a_finished_show_is_taken() {
        let chain = SeasonChain::new(vec![tv(100, "Show", ListStatus::Watching, 4, Some(12))]);
        let want = Wanted {
            mal_id: 100,
            title: "Show".to_string(),
            season: 1,
            episode: Some(5.0),
        };
        let release = parse_release("[Judas] Show - 01-12 [Batch] [BD1080p]");
        assert_eq!(
            decide(&chain, &want, &release),
            Decision::Download { season: 1, episode: 5.0 }
        );
    }

    #[test]
    fn a_batch_while_still_airing_asks() {
        let chain = SeasonChain::new(vec![SeasonEntry {
            fully_aired: false,
            ..tv(100, "Show", ListStatus::Watching, 4, Some(12))
        }]);
        let want = Wanted {
            mal_id: 100,
            title: "Show".to_string(),
            season: 1,
            episode: Some(5.0),
        };
        let release = parse_release("[Judas] Show - 01-04 [Batch] [BD1080p]");
        assert!(matches!(
            decide(&chain, &want, &release),
            Decision::Ask { .. }
        ));
    }

    #[test]
    fn movies_are_one_episode_and_refuse_episode_numbers() {
        let chain = SeasonChain::new(vec![SeasonEntry {
            show_type: ShowType::Movie,
            total_episodes: Some(1),
            ..tv(500, "Kimi no Na wa", ListStatus::Watching, 0, Some(1))
        }]);
        let want = Wanted {
            mal_id: 500,
            title: "Kimi no Na wa".to_string(),
            season: 1,
            episode: None,
        };
        let release = parse_release("[Group] Kimi no Na wa (2016) [BD1080p]");
        assert_eq!(decide(&chain, &want, &release), Decision::Download { season: 1, episode: 1.0 });

        let release = parse_release("[Group] Kimi no Na wa - 03 (1080p) [ABC12345].mkv");
        assert!(matches!(
            decide(&chain, &want, &release),
            Decision::Ask { .. }
        ));
    }

    #[test]
    fn a_season_marker_matching_the_want_is_accepted() {
        let chain = chain();
        let want = Wanted {
            mal_id: 200,
            title: "Show 2".to_string(),
            season: 2,
            episode: Some(5.0),
        };
        let release = parse_release("[SubsPlease] Show S02E05 (1080p) [ABC12345].mkv");
        assert_eq!(
            decide(&chain, &want, &release),
            Decision::Download { season: 2, episode: 5.0 }
        );
    }
}
