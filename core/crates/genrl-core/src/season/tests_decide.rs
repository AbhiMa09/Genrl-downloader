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
        auto: true,
    }
}

fn chain() -> SeasonChain {
    SeasonChain::new(vec![
        tv(100, "Show", ListStatus::Completed, 12, Some(12)),
        tv(200, "Show 2", ListStatus::Watching, 4, Some(12)),
    ])
}

fn settings() -> Settings {
    Settings::default()
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

#[test]
fn split_cour_chain_preserves_title_season_marker() {
    let chain = SeasonChain::new(vec![
        tv(1, "Franchise", ListStatus::Completed, 25, Some(25)),
        tv(
            2,
            "Franchise 2nd Season",
            ListStatus::Completed,
            13,
            Some(13),
        ),
        tv(
            3,
            "Franchise 2nd Season Part 2",
            ListStatus::Completed,
            12,
            Some(12),
        ),
        tv(
            4,
            "Franchise 3rd Season",
            ListStatus::Completed,
            16,
            Some(16),
        ),
        tv(5, "Franchise 4th Season", ListStatus::Watching, 0, Some(19)),
    ]);
    assert_eq!(chain.season_of(5), Some(4));
    assert_eq!(chain.season_of(3), Some(2));
    assert_eq!(chain.season_of(4), Some(3));
    let want = match chain.wanted(&settings(), Overrides::default()) {
        Want::Some(w) => w,
        _ => panic!("expected want"),
    };
    assert_eq!(want.season, 4);
    assert_eq!(want.episode, Some(1.0));
    let release = parse_release("[SubsPlease] Franchise S04E01 (1080p) [1234ABCD].mkv");
    assert_eq!(
        decide(&chain, &want, &release),
        Decision::Download {
            season: 4,
            episode: 1.0,
        }
    );
}

#[test]
fn season_title_marker_rules_and_roman_numerals() {
    use crate::filename::{extract_season_from_title, is_cour_or_part_split};

    // 1. Roman numerals (III, IV, II)
    assert_eq!(extract_season_from_title("Mob Psycho 100 III"), Some(3));
    assert_eq!(extract_season_from_title("Overlord IV"), Some(4));
    assert_eq!(extract_season_from_title("Kingdom III"), Some(3));
    assert_eq!(extract_season_from_title("Show II"), Some(2));

    // 2. Explicit season combined with Part 2
    assert_eq!(
        extract_season_from_title("Franchise 2nd Season Part 2"),
        Some(2)
    );
    assert_eq!(
        extract_season_from_title("Jujutsu Kaisen Season 2 Part 2"),
        Some(2)
    );
    assert!(!is_cour_or_part_split("Franchise 2nd Season Part 2"));

    // 3. Standalone Part 2 / Cour 2 (must not be parsed as Season 2)
    assert_eq!(extract_season_from_title("Sousou no Frieren Part 2"), None);
    assert_eq!(extract_season_from_title("Bleach - Cour 2"), None);
    assert!(is_cour_or_part_split("Sousou no Frieren Part 2"));
    assert!(is_cour_or_part_split("Bleach - Cour 2"));

    // 4. "Final Season" without integer
    assert_eq!(
        extract_season_from_title("Shingeki no Kyojin: The Final Season"),
        None
    );
    assert_eq!(extract_season_from_title("Final Season"), None);

    // 4b. Negative tests (words ending in Roman letters/characters)
    assert_eq!(extract_season_from_title("Hunter x Hunter"), None);
    assert_eq!(extract_season_from_title("Initial D"), None);
    assert_eq!(extract_season_from_title("Clannad"), None);
    assert_eq!(extract_season_from_title("Steins;Gate"), None);

    // 5. Chain with Roman numerals
    let mob_chain = SeasonChain::new(vec![
        tv(1, "Mob Psycho 100", ListStatus::Completed, 12, Some(12)),
        tv(2, "Mob Psycho 100 II", ListStatus::Completed, 13, Some(13)),
        tv(3, "Mob Psycho 100 III", ListStatus::Watching, 0, Some(12)),
    ]);
    assert_eq!(mob_chain.season_of(1), Some(1));
    assert_eq!(mob_chain.season_of(2), Some(2));
    assert_eq!(mob_chain.season_of(3), Some(3));

    // 6. Chain with Cour 2
    let bleach_chain = SeasonChain::new(vec![
        tv(
            10,
            "Bleach: Thousand-Year Blood War",
            ListStatus::Completed,
            13,
            Some(13),
        ),
        tv(
            20,
            "Bleach: Thousand-Year Blood War - Cour 2",
            ListStatus::Watching,
            0,
            Some(13),
        ),
    ]);
    assert_eq!(bleach_chain.season_of(10), Some(1));
    assert_eq!(bleach_chain.season_of(20), Some(1));

    // 7. Chain with Part 2 and Final Season
    let aot_chain = SeasonChain::new(vec![
        tv(100, "Attack on Titan", ListStatus::Completed, 25, Some(25)),
        tv(
            200,
            "Attack on Titan Season 2",
            ListStatus::Completed,
            12,
            Some(12),
        ),
        tv(
            300,
            "Attack on Titan Season 3",
            ListStatus::Completed,
            12,
            Some(12),
        ),
        tv(
            400,
            "Attack on Titan Season 3 Part 2",
            ListStatus::Completed,
            10,
            Some(10),
        ),
        tv(
            500,
            "Attack on Titan The Final Season",
            ListStatus::Completed,
            16,
            Some(16),
        ),
        tv(
            600,
            "Attack on Titan The Final Season Part 2",
            ListStatus::Watching,
            0,
            Some(12),
        ),
    ]);
    assert_eq!(aot_chain.season_of(100), Some(1));
    assert_eq!(aot_chain.season_of(200), Some(2));
    assert_eq!(aot_chain.season_of(300), Some(3));
    assert_eq!(aot_chain.season_of(400), Some(3)); // Part 2 inherits Season 3
    assert_eq!(aot_chain.season_of(500), Some(4)); // The Final Season is Season 4
    assert_eq!(aot_chain.season_of(600), Some(4)); // The Final Season Part 2 inherits Season 4
}
