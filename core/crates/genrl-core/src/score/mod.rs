//! Torrent selection: hard filters, the accept score, and ranking.
//!
//! Three stages, in order:
//!
//! 1. **Hard filters** — wrong episode, wrong type, below the resolution
//!    floor, excluded codec, out-of-range size, zero seeders, blocked
//!    uploader: reject outright, no points awarded.
//! 2. **Signal score** (max 105, minus 20 for a wrong year) — the design's
//!    accept table: title match 50, episode match 30, type match 10, batch
//!    when fully aired 15. Compared against `Settings::score_threshold`
//!    (default 80) to decide "download" versus "ask".
//! 3. **Ranking** — how to order the candidates that passed: group priority,
//!    resolution, seeders on a log scale, trusted/re-release, dual audio,
//!    recency. Ranking never rescues a candidate the signal score doubts.

pub mod evaluate;
pub mod types;

pub use evaluate::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filename::{parse_release, Release};
    use crate::models::ShowType;
    use crate::nyaa::FeedItem;
    use crate::rules::Rules;
    use crate::settings::Settings;

    fn target() -> (Vec<String>, Vec<String>) {
        (
            vec!["sousou no frieren".to_string()],
            vec!["kimi no todoke".to_string()],
        )
    }

    fn frieren_target<'a>(names: &'a [String], others: &'a [String]) -> Target<'a> {
        Target {
            names,
            other_names: others,
            wanted_episode: Some(28.0),
            show_type: ShowType::Tv,
            season: 1,
            start_year: Some(2023),
            fully_aired: true,
        }
    }

    fn candidate(name: &str, seeders: u32) -> (Release, Candidate) {
        let release = parse_release(name);
        let item = FeedItem {
            title: name.to_string(),
            link: "https://nyaa.si/download/1.torrent".into(),
            guid: "https://nyaa.si/view/1".into(),
            published_at: 1_789_227_900,
            seeders,
            leechers: 1,
            downloads: 10,
            info_hash: "ef3e7ad1b12bdd9fc341691d8866cd1fa8374a4b".into(),
            category_id: "1_2".into(),
            category: "Anime - English-translated".into(),
            size_bytes: 1_600_000_000,
            size_text: "1.5 GiB".into(),
            trusted: true,
            remake: false,
        };
        let candidate = Candidate::from_item(&release, &item);
        (release, candidate)
    }

    fn now() -> i64 {
        1_789_227_900
    }

    #[test]
    fn a_clean_single_episode_scores_above_the_threshold() {
        let (names, others) = target();
        let t = frieren_target(&names, &others);
        let (release, mut candidate) =
            candidate("[SubsPlease] Sousou no Frieren - 28 (1080p) [ABCD1234].mkv", 53);
        candidate.show_type = Some(ShowType::Tv);
        let rules = Rules {
            groups: vec!["SubsPlease".into(), "Erai-raws".into()],
            ..Rules::default()
        };
        let settings = Settings::default();

        let scored = evaluate(&candidate, &t, &rules, &settings, now()).unwrap();
        // title 50 + episode 30 + type 10 = 90 ≥ 80.
        assert_eq!(scored.signal, 90.0, "{scored:?}");
        assert!(scored.signal >= settings.score_threshold);
        assert_eq!(scored.matched_name.as_deref(), Some("sousou no frieren"));
        // Ranking: preferred group 40 + preferred resolution 25 + seeds + trusted + fresh.
        assert!(scored.ranking > 80.0, "{:?}", scored.ranking);
        assert!(release.group.is_some());
    }

    #[test]
    fn a_wrong_episode_is_rejected() {
        let (names, others) = target();
        let t = frieren_target(&names, &others);
        let (_release, candidate) =
            candidate("[SubsPlease] Sousou no Frieren - 07 (1080p) [ABCD1234].mkv", 53);
        let err = evaluate(&candidate, &t, &Rules::default(), &Settings::default(), now())
            .unwrap_err();
        assert_eq!(
            err,
            Rejection::WrongEpisode {
                wanted: 28.0,
                found: 7.0
            }
        );
    }

    #[test]
    fn a_title_belonging_to_another_show_is_rejected() {
        let (names, others) = target();
        let t = frieren_target(&names, &others);
        let (_release, candidate) =
            candidate("[SubsPlease] Kimi no Todoke - 28 (1080p) [ABCD1234].mkv", 53);
        let err = evaluate(&candidate, &t, &Rules::default(), &Settings::default(), now())
            .unwrap_err();
        match err {
            Rejection::TitleIsAnotherShow { name, .. } => assert_eq!(name, "kimi no todoke"),
            other => panic!("expected TitleIsAnotherShow, got {other:?}"),
        }
    }

    #[test]
    fn an_unrelated_title_scores_too_low_to_act_on() {
        // Not another show's name either — just not ours. Must not auto-download.
        let names = vec!["sousou no frieren".to_string()];
        let others: Vec<String> = vec![];
        let t = frieren_target(&names, &others);
        let (_release, candidate) =
            candidate("[SubsPlease] Some Random Show - 28 (1080p) [ABCD1234].mkv", 53);
        let settings = Settings::default();
        let scored = evaluate(&candidate, &t, &Rules::default(), &settings, now()).unwrap();
        assert!(
            scored.signal < settings.score_threshold,
            "no confident match: {}",
            scored.signal
        );
    }

    #[test]
    fn season_conflict_and_type_mismatch_reject() {
        let names = vec!["sousou no frieren".to_string()];
        let others: Vec<String> = vec![];
        let t = Target {
            season: 2,
            ..frieren_target(&names, &others)
        };

        // Season 1 release for a season 2 entry.
        let (_r, c) = candidate(
            "[SubsPlease] Sousou no Frieren - S01E07 (1080p) [ABCD1234].mkv",
            53,
        );
        // wanted_episode 28 vs 7 fires first; set the episode to match instead.
        let t = Target {
            wanted_episode: Some(7.0),
            ..t
        };
        let err = evaluate(&c, &t, &Rules::default(), &Settings::default(), now()).unwrap_err();
        assert_eq!(
            err,
            Rejection::SeasonConflict {
                wanted: 2,
                found: 1
            }
        );

        // Type: a Movie candidate for a TV entry.
        let (_r, mut c) = candidate("[Group] Sousou no Frieren (2023) [BD1080p]", 53);
        c.show_type = Some(ShowType::Movie);
        let t = Target {
            wanted_episode: None,
            season: 1,
            ..t
        };
        let err = evaluate(&c, &t, &Rules::default(), &Settings::default(), now()).unwrap_err();
        assert_eq!(
            err,
            Rejection::TypeMismatch {
                wanted: ShowType::Tv,
                found: ShowType::Movie
            }
        );
    }

    #[test]
    fn hard_filters_reject_before_scoring() {
        let names = vec!["sousou no frieren".to_string()];
        let others: Vec<String> = vec![];
        let t = frieren_target(&names, &others);
        let settings = Settings::default();

        // Zero seeders.
        let (_r, c) = candidate("[SubsPlease] Sousou no Frieren - 28 (1080p) [ABCD1234].mkv", 0);
        assert_eq!(
            evaluate(&c, &t, &Rules::default(), &settings, now()).unwrap_err(),
            Rejection::NoSeeders
        );

        // Below the resolution floor.
        let rules = Rules::default(); // floor 720p
        let (_r, c) = candidate("[SubsPlease] Sousou no Frieren - 28 (480p) [ABCD1234].mkv", 53);
        assert!(matches!(
            evaluate(&c, &t, &rules, &settings, now()).unwrap_err(),
            Rejection::ResolutionBelowFloor { .. }
        ));

        // Blocked codec.
        let rules = Rules {
            codecs_blocked: vec!["hevc".into()],
            ..Rules::default()
        };
        let (_r, c) = candidate("[SubsPlease] Sousou no Frieren - 28 (1080p HEVC).mkv", 53);
        assert_eq!(
            evaluate(&c, &t, &rules, &settings, now()).unwrap_err(),
            Rejection::CodecBlocked {
                codec: "hevc".into()
            }
        );

        // Size outside the sanity range (the item above is 1.5 GiB ≈ 1536 MB).
        let rules = Rules {
            max_size_mb: 1024,
            ..Rules::default()
        };
        let (_r, c) = candidate("[SubsPlease] Sousou no Frieren - 28 (1080p).mkv", 53);
        assert_eq!(
            evaluate(&c, &t, &rules, &settings, now()).unwrap_err(),
            Rejection::SizeOutOfRange {
                size_mb: 1525,
                min_mb: 100,
                max_mb: 1024
            }
        );

        // Blocked uploader.
        let rules = Rules {
            blocked_uploaders: vec!["BadActor".into()],
            ..Rules::default()
        };
        let (_r, mut c) = candidate("[SubsPlease] Sousou no Frieren - 28 (1080p).mkv", 53);
        c.uploader = Some("badactor".into());
        assert_eq!(
            evaluate(&c, &t, &rules, &settings, now()).unwrap_err(),
            Rejection::UploaderBlocked {
                uploader: "badactor".into()
            }
        );
    }

    #[test]
    fn ranking_orders_what_passed() {
        let names = vec!["sousou no frieren".to_string()];
        let others: Vec<String> = vec![];
        let t = frieren_target(&names, &others);
        let settings = Settings::default();
        let rules = Rules {
            groups: vec!["SubsPlease".into()],
            ..Rules::default()
        };

        // Preferred group at 1080p, many seeders…
        let (_r, good) = candidate("[SubsPlease] Sousou no Frieren - 28 (1080p).mkv", 500);
        // …versus an unknown group at the floor resolution with thin seeds.
        let (_r, thin) = candidate("[UnknownGroup] Sousou no Frieren - 28 (720p).mkv", 2);

        let good = evaluate(&good, &t, &rules, &settings, now()).unwrap();
        let thin = evaluate(&thin, &t, &rules, &settings, now()).unwrap();
        assert_eq!(good.signal, thin.signal, "same accept score");
        assert!(
            good.ranking > thin.ranking + 30.0,
            "ranking must put the preferred release first: {} vs {}",
            good.ranking,
            thin.ranking
        );
    }

    #[test]
    fn batches_and_years_follow_the_design() {
        let names = vec!["sousou no frieren".to_string()];
        let others: Vec<String> = vec![];
        let settings = Settings::default();

        // Batch for a finished show: +15, and no episode points (none given).
        let t = Target {
            wanted_episode: None,
            fully_aired: true,
            show_type: ShowType::Movie,
            start_year: Some(2023),
            ..frieren_target(&names, &others)
        };
        let (_r, c) = candidate("[Judas] Sousou no Frieren - 01-28 [Batch] [BD 1080p]", 53);
        let scored = evaluate(&c, &t, &Rules::default(), &settings, now()).unwrap();
        assert_eq!(scored.signal, 95.0, "{scored:?}"); // title 50 + feature 30 + batch 15

        // Same batch, still airing: no batch points.
        let t = Target {
            fully_aired: false,
            show_type: ShowType::Tv,
            wanted_episode: Some(28.0),
            ..t
        };
        let (_r, c) = candidate("[Judas] Sousou no Frieren - 01-28 [Batch] [BD 1080p]", 53);
        let scored = evaluate(&c, &t, &Rules::default(), &settings, now()).unwrap();
        assert_eq!(scored.signal, 50.0, "{scored:?}"); // title only

        // A remake two years off loses twenty points.
        let t = Target {
            start_year: Some(2023),
            wanted_episode: None,
            show_type: ShowType::Movie,
            fully_aired: true,
            ..t
        };
        let (_r, c) = candidate("[Group] Sousou no Frieren (2025) [BD1080p]", 53);
        let scored = evaluate(&c, &t, &Rules::default(), &settings, now()).unwrap();
        assert_eq!(scored.signal, 60.0, "50 + 30 - 20 year: {scored:?}");
    }

    #[test]
    fn seed_health_three_buckets() {
        let rules = Rules::default();
        assert_eq!(seed_health(0, &rules), SeedHealth::Dead);
        assert_eq!(seed_health(5, &rules), SeedHealth::Thin);
        assert_eq!(seed_health(10, &rules), SeedHealth::Healthy);
        assert_eq!(seed_health(999, &rules), SeedHealth::Healthy);
    }

    #[test]
    fn season_batch_prioritized_over_single_and_fails_over_on_zero_seeders() {
        let names = vec!["9 ruler s crown".to_string(), "9 nine ruler s crown".to_string()];
        let others: Vec<String> = vec![];
        let settings = Settings::default();
        let rules = Rules::default(); // prefer_batch_when_finished = true

        let t = Target {
            wanted_episode: Some(1.0),
            fully_aired: true,
            show_type: ShowType::Tv,
            season: 1,
            start_year: Some(2025),
            ..frieren_target(&names, &others)
        };

        // 1. Healthy Season Batch
        let (_r, mut batch_cand) = candidate("[EMBER] 9: Ruler's Crown (2025) (Season 1) [1080p] [Batch]", 14);
        batch_cand.show_type = Some(ShowType::Tv);
        let batch_scored = evaluate(&batch_cand, &t, &rules, &settings, now()).unwrap();
        assert_eq!(batch_scored.signal, 110.0); // 50 title + 10 type + 50 batch

        // 2. Single Episode
        let (_r, mut single_cand) = candidate("[Valhalla-Society] 9-nine- Ruler's Crown S01E01 [1080p]", 20);
        single_cand.show_type = Some(ShowType::Tv);
        let single_scored = evaluate(&single_cand, &t, &rules, &settings, now()).unwrap();
        assert_eq!(single_scored.signal, 90.0); // 50 title + 10 type + 30 episode

        // Batch signal (110) beats single episode (90) by 20 points, exceeding score_margin (15)!
        assert!(batch_scored.signal - single_scored.signal >= settings.score_margin);

        // 3. Batch with 0 seeders fails hard filter and is rejected with NoSeeders
        let (_r, dead_batch) = candidate("[EMBER] 9: Ruler's Crown (2025) (Season 1) [1080p] [Batch]", 0);
        let err = evaluate(&dead_batch, &t, &rules, &settings, now()).unwrap_err();
        assert_eq!(err, Rejection::NoSeeders);
    }
}
