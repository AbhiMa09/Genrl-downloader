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

pub mod decide;
pub mod types;
pub mod walk;

pub use decide::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filename::parse_release;
    use crate::models::{ListStatus, ShowType};
    use crate::settings::Settings;

    fn tv(
        mal_id: i64,
        title: &str,
        status: ListStatus,
        watched: i64,
        total: Option<i64>,
    ) -> SeasonEntry {
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
            Decision::Download {
                season: 2,
                episode: 5.0
            }
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
            Decision::Download {
                season: 2,
                episode: 5.0
            }
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
        let Want::Skip {
            reason,
            fully_watched,
        } = chain.wanted(&settings(), Overrides::default())
        else {
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
            chain.wanted(
                &settings(),
                Overrides {
                    auto_plan_to_watch: Some(true)
                }
            ),
            Want::Some(_)
        ));
    }

    #[test]
    fn fully_watched_suggests_completing_rather_than_erroring() {
        let chain = SeasonChain::new(vec![tv(100, "Show", ListStatus::Watching, 12, Some(12))]);
        let Want::Skip {
            reason,
            fully_watched,
        } = chain.wanted(&settings(), Overrides::default())
        else {
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
            Decision::Download {
                season: 1,
                episode: 5.0
            }
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
            Decision::Download {
                season: 1,
                episode: 5.0
            }
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
        assert_eq!(
            decide(&chain, &want, &release),
            Decision::Download {
                season: 1,
                episode: 1.0
            }
        );

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
            Decision::Download {
                season: 2,
                episode: 5.0
            }
        );
    }
}
