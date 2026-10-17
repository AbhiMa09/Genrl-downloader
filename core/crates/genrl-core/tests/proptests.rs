use genrl_core::filename::{is_metadata_bracket, normalize, parse_release};
use genrl_core::models::{ListStatus, ShowType};
use genrl_core::season::{decide, Decision, Overrides, SeasonChain, SeasonEntry, Want, Wanted};
use genrl_core::settings::Settings;
use proptest::prelude::*;

proptest! {
    /// 1. Arbitrary input never panics the release parser.
    #[test]
    fn parse_release_never_panics(s in "\\PC*") {
        let release = parse_release(&s);
        prop_assert_eq!(&release.raw, s.trim());
    }

    /// 2. Normalization is strictly idempotent: normalize(normalize(x)) == normalize(x).
    #[test]
    fn normalize_is_idempotent(s in "\\PC*") {
        let norm1 = normalize(&s);
        let norm2 = normalize(&norm1);
        prop_assert_eq!(norm1, norm2);
    }

    /// 3. Normalization never emits uppercase characters.
    #[test]
    fn normalize_produces_lowercase_only(s in "\\PC*") {
        let norm = normalize(&s);
        prop_assert_eq!(norm.to_lowercase(), norm);
    }

    /// 4. Structured release string property test.
    #[test]
    fn structured_release_preserves_episode(
        group in "[a-zA-Z0-9_-]{2,15}",
        title in "[a-zA-Z]{2,10}( [a-zA-Z]{2,10}){0,2}",
        ep in 1u32..=999u32,
    ) {
        prop_assume!(!is_metadata_bracket(&group));
        let g_lower = group.to_lowercase();
        prop_assume!(
            !g_lower.contains("4k")
                && !g_lower.contains("2k")
                && !g_lower.contains("1080")
                && !g_lower.contains("720")
                && !g_lower.contains("480")
                && !g_lower.contains("2160")
        );
        let filename = format!("[{group}] {title} - {ep:02} (1080p) [DEADBEEF].mkv");
        let parsed = parse_release(&filename);

        prop_assert_eq!(parsed.episode, Some(ep as f64));
        prop_assert_eq!(parsed.group.as_deref(), Some(group.as_str()));
        prop_assert_eq!(parsed.resolution.as_deref(), Some("1080p"));
    }

    /// 5. Season chain logic invariant: wanted episode is always watched + 1 when within total.
    #[test]
    fn season_chain_wanted_progress_invariant(
        watched in 0i64..100i64,
        delta in 1i64..50i64,
    ) {
        let total = watched + delta;
        let entry = SeasonEntry {
            mal_id: 101,
            title: "Proptest Show".to_string(),
            list_status: ListStatus::Watching,
            watched_count: watched,
            total_episodes: Some(total),
            episode_offset: 0,
            show_type: ShowType::Tv,
            fully_aired: true,
        };

        let chain = SeasonChain::new(vec![entry]);
        let settings = Settings::default();
        let want = chain.wanted(&settings, Overrides::default());

        match want {
            Want::Some(wanted) => {
                prop_assert_eq!(wanted.episode, Some((watched + 1) as f64));
            }
            Want::Skip { reason, .. } => {
                panic!("Expected Want::Some, got Skip: {reason}");
            }
        }
    }

    /// 6. Season decide function does not panic on any parsed release against valid Wanted.
    #[test]
    fn season_decide_never_panics(
        random_title in "\\PC*",
        wanted_ep in 1i64..100i64,
    ) {
        let release = parse_release(&random_title);
        let entry = SeasonEntry {
            mal_id: 101,
            title: "Arbitrary Show".to_string(),
            list_status: ListStatus::Watching,
            watched_count: wanted_ep - 1,
            total_episodes: Some(100),
            episode_offset: 0,
            show_type: ShowType::Tv,
            fully_aired: true,
        };
        let chain = SeasonChain::new(vec![entry]);
        let wanted = Wanted {
            mal_id: 101,
            title: "Arbitrary Show".to_string(),
            season: 1,
            episode: Some(wanted_ep as f64),
        };

        let decision = decide(&chain, &wanted, &release);
        let is_valid = matches!(
            decision,
            Decision::Download { .. }
                | Decision::Skip { .. }
                | Decision::Ask { .. }
                | Decision::NotFound { .. }
        );
        prop_assert!(is_valid);
    }
}
