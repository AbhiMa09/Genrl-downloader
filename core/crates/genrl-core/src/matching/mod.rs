//! The matching engine: the app resolves every show to a MAL id first, then
//! searches Nyaa with names that belong to that id.
//!
//! Nyaa uploaders use romaji, English, abbreviations or typos, so one title is
//! never enough. The engine:
//!
//! 1. builds the show's **name set** ([`names::NameSet`]) and reads every
//!    *other* show's names as the reject set, so `Kimi no Todoke` can never
//!    pass for `Kimi no Na wa`;
//! 2. searches Nyaa **once per name in priority order** (romaji first),
//!    stopping as soon as a name returns enough good candidates, which keeps
//!    the request count low;
//! 3. scores each result with [`score::evaluate`] against the show;
//! 4. **decides**: best score at or above the threshold *and* far enough ahead
//!    of the runner-up proceeds automatically; otherwise the job pauses with a
//!    Match Review card and the top candidates, why each scored as it did;
//!    a healthy-enough torrent still has to clear the seed-health buckets.
//!
//! The network step is the only async part; scoring and the decision are pure
//! functions, which is what the fixture suite tests.

pub mod decide;
pub mod search;
pub mod types;

pub use decide::*;
pub use search::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::filename;
    use crate::models::NameSource;
    use crate::names::{self, PRIORITY_MAIN, PRIORITY_USER};
    use crate::nyaa::FeedItem;
    use crate::rules::Rules;
    use crate::score::{Candidate, Rejection};
    use crate::settings::Settings;

    fn now() -> i64 {
        1_789_227_900
    }

    fn settings() -> Settings {
        Settings::default()
    }

    fn rules() -> Rules {
        Rules::default()
    }

    /// A show row plus its names, the way a MAL sync would leave them.
    fn seed_show(
        db: &Db,
        mal_id: i64,
        title: &str,
        show_type: &str,
        auto: bool,
        names: &[(&str, NameSource, u8)],
    ) {
        db.with(|c| {
            c.execute(
                "INSERT INTO shows (mal_id, title, english_title, type, status, total_episodes, start_year, auto)
                 VALUES (?1, ?2, ?3, ?4, 'finished', 1, 2016, ?5)",
                rusqlite::params![mal_id, title, format!("English {title}"), show_type, auto as i64],
            )?;
            Ok(())
        })
        .unwrap();
        for (name, source, priority) in names {
            names::NameSet::add(db, mal_id, name, *source, *priority).unwrap();
        }
    }

    fn item(title: &str, seeders: u32) -> FeedItem {
        FeedItem {
            title: title.to_string(),
            link: format!("https://nyaa.si/download/{seeders}.torrent"),
            guid: format!("https://nyaa.si/view/{seeders}"),
            published_at: now(),
            seeders,
            leechers: 2,
            downloads: 100,
            info_hash: format!("{:0>40x}", seeders),
            category_id: "1_2".to_string(),
            category: "Anime - English-translated".to_string(),
            size_bytes: 1_600_000_000,
            size_text: "1.5 GiB".to_string(),
            trusted: true,
            remake: false,
        }
    }

    fn candidates(titles: &[(&str, u32)]) -> Vec<Candidate> {
        titles
            .iter()
            .map(|(title, seeders)| {
                let release = filename::parse_release(title);
                candidate_from(&release, &item(title, *seeders))
            })
            .collect()
    }

    // ------------------------------------------------- the design's sequels trap

    #[test]
    fn kimi_no_na_wa_and_kimi_no_todoke_are_different_shows() {
        let db = Db::open_in_memory().unwrap();
        seed_show(
            &db,
            1,
            "Kimi no Na wa",
            "movie",
            false,
            &[("Kimi no Na wa", NameSource::Mal, PRIORITY_MAIN)],
        );
        seed_show(
            &db,
            2,
            "Kimi no Todoke",
            "tv",
            false,
            &[("Kimi no Todoke", NameSource::Mal, PRIORITY_MAIN)],
        );

        let inputs = TargetInputs::load(&db, 1).unwrap();
        assert_eq!(inputs.names, vec!["kimi no na wa"]);
        assert!(
            inputs.other_names.contains(&"kimi no todoke".to_string()),
            "the other show's names are the reject set: {:?}",
            inputs.other_names
        );

        // A Kimi no Todoke release is rejected outright, not scored low.
        let (ranked, rejected) = rank(
            candidates(&[("[SubsPlease] Kimi no Todoke - 01 (1080p) [ABCD1234].mkv", 80)]),
            &inputs,
            Some(1.0),
            &rules(),
            &settings(),
            now(),
        );
        assert!(ranked.is_empty(), "never a candidate for this show");
        assert_eq!(
            rejected[0].1,
            Rejection::TitleIsAnotherShow {
                name: "kimi no todoke".to_string(),
                ratio: 1.0
            }
        );
        assert!(matches!(
            decide_verdict(ranked, rejected, &inputs, &rules(), &settings()),
            Verdict::NotFound { .. }
        ));
    }

    #[test]
    fn a_confident_match_proceeds_without_asking() {
        let db = Db::open_in_memory().unwrap();
        seed_show(
            &db,
            1,
            "Kimi no Na wa",
            "movie",
            false,
            &[("Kimi no Na wa", NameSource::Mal, PRIORITY_MAIN)],
        );
        let inputs = TargetInputs::load(&db, 1).unwrap();
        assert!(inputs.fully_aired);

        let (ranked, rejected) = rank(
            candidates(&[("[SubsPlease] Kimi no Na wa (2016) [BD1080p].mkv", 53)]),
            &inputs,
            None,
            &rules(),
            &settings(),
            now(),
        );
        match decide_verdict(ranked, rejected, &inputs, &rules(), &settings()) {
            Verdict::Auto { chosen, thin } => {
                assert!(!thin);
                assert_eq!(chosen.scored.signal, 80.0, "title 50 + feature 30");
                assert!(chosen.scored.signal >= settings().score_threshold);
                // The card for a question — or the queue's add step — needs
                // the torrent's identity, not just its score.
                assert_eq!(chosen.candidate.info_hash.len(), 40);
                assert!(chosen.candidate.magnet.starts_with("magnet:?xt=urn:btih:"));
                let option = chosen.to_option();
                assert_eq!(option.info_hash, chosen.candidate.info_hash);
                assert_eq!(option.seeders, 53);
            }
            other => panic!("expected Auto, got {other:?}"),
        }
    }

    #[test]
    fn two_good_candidates_ask_which_one() {
        let db = Db::open_in_memory().unwrap();
        seed_show(
            &db,
            1,
            "Kimi no Na wa",
            "movie",
            false,
            &[("Kimi no Na wa", NameSource::Mal, PRIORITY_MAIN)],
        );
        let inputs = TargetInputs::load(&db, 1).unwrap();

        // Same show, same feature, two groups: both score 80, so the margin
        // of 15 cannot separate them and the app asks.
        let (ranked, rejected) = rank(
            candidates(&[
                ("[SubsPlease] Kimi no Na wa (2016) [BD1080p].mkv", 53),
                ("[Erai-raws] Kimi no Na wa (2016) [BD720p].mkv", 12),
            ]),
            &inputs,
            None,
            &rules(),
            &settings(),
            now(),
        );
        assert_eq!(ranked.len(), 2);
        assert!((ranked[0].scored.signal - ranked[1].scored.signal).abs() < f64::EPSILON);
        match decide_verdict(ranked, rejected, &inputs, &rules(), &settings()) {
            Verdict::Ask { reason, ranked } => {
                assert!(matches!(reason, AskReason::TooClose { .. }), "{reason}");
                assert_eq!(ranked.len(), 2, "the card shows both");
            }
            other => panic!("expected Ask, got {other:?}"),
        }
    }

    #[test]
    fn re_zero_and_re_creators_never_confuse_each_other() {
        let db = Db::open_in_memory().unwrap();
        seed_show(
            &db,
            10,
            "Re:Zero",
            "tv",
            false,
            &[("Re:Zero", NameSource::Mal, PRIORITY_MAIN)],
        );
        seed_show(
            &db,
            11,
            "Re:Creators",
            "tv",
            false,
            &[("Re:Creators", NameSource::Mal, PRIORITY_MAIN)],
        );
        let inputs = TargetInputs::load(&db, 10).unwrap();
        assert_eq!(inputs.names, vec!["re zero"]);

        let (ranked, rejected) = rank(
            candidates(&[("[SubsPlease] Re:Creators - 05 (1080p) [ABCD1234].mkv", 200)]),
            &inputs,
            Some(5.0),
            &rules(),
            &settings(),
            now(),
        );
        assert!(ranked.is_empty());
        match &rejected[0].1 {
            Rejection::TitleIsAnotherShow { name, .. } => assert_eq!(name, "re creators"),
            other => panic!("expected another-show rejection, got {other:?}"),
        }
    }

    #[test]
    fn a_user_alias_saved_from_a_match_review_is_searched() {
        let db = Db::open_in_memory().unwrap();
        seed_show(
            &db,
            20,
            "Shingeki no Kyojin",
            "tv",
            false,
            &[("Shingeki no Kyojin", NameSource::Mal, PRIORITY_MAIN)],
        );
        // The user answered "none of these" once and typed the name they knew.
        names::NameSet::add(&db, 20, "Attack on Titan", NameSource::User, PRIORITY_USER).unwrap();

        let set = names::NameSet::load(&db, 20).unwrap();
        // The user alias is searched last, after the romaji name.
        assert_eq!(set.search_order(), vec!["shingeki no kyojin", "attack on titan"]);
        assert_eq!(
            names::NameSet::user_aliases(&db, 20).unwrap(),
            vec!["Attack on Titan".to_string()]
        );

        // A release under the alias still matches the show.
        let inputs = TargetInputs::load(&db, 20).unwrap();
        let (ranked, rejected) = rank(
            candidates(&[("[SubsPlease] Attack on Titan - 05 (1080p) [ABCD1234].mkv", 53)]),
            &inputs,
            Some(5.0),
            &rules(),
            &settings(),
            now(),
        );
        assert_eq!(rejected.len(), 0);
        match decide_verdict(ranked, rejected, &inputs, &rules(), &settings()) {
            Verdict::Auto { chosen, .. } => {
                assert_eq!(chosen.scored.matched_name.as_deref(), Some("attack on titan"));
            }
            other => panic!("expected Auto through the alias, got {other:?}"),
        }
    }

    #[test]
    fn thin_seeds_ask_unless_the_show_is_on_the_auto_list() {
        let db = Db::open_in_memory().unwrap();
        seed_show(
            &db,
            1,
            "Kimi no Na wa",
            "movie",
            true,
            &[("Kimi no Na wa", NameSource::Mal, PRIORITY_MAIN)],
        );
        let inputs = TargetInputs::load(&db, 1).unwrap();
        assert!(inputs.auto);

        let (ranked, rejected) = rank(
            candidates(&[("[SubsPlease] Kimi no Na wa (2016) [BD1080p].mkv", 4)]),
            &inputs,
            None,
            &rules(),
            &settings(),
            now(),
        );
        match decide_verdict(ranked, rejected, &inputs, &rules(), &settings()) {
            Verdict::Auto { thin, .. } => assert!(thin, "flagged, not asked"),
            other => panic!("expected Auto with a flag, got {other:?}"),
        }

        // Off the auto list the same 4 seeders pause the job as a question.
        db.with(|c| {
            c.execute("UPDATE shows SET auto = 0 WHERE mal_id = 1", [])?;
            Ok(())
        })
        .unwrap();
        let manual = TargetInputs::load(&db, 1).unwrap();
        let (ranked, rejected) = rank(
            candidates(&[("[SubsPlease] Kimi no Na wa (2016) [BD1080p].mkv", 4)]),
            &manual,
            None,
            &rules(),
            &settings(),
            now(),
        );
        match decide_verdict(ranked, rejected, &manual, &rules(), &settings()) {
            Verdict::Ask { reason, .. } => {
                assert!(matches!(reason, AskReason::ThinSeeds { seeders: 4 }))
            }
            other => panic!("expected Ask, got {other:?}"),
        }
    }

    #[test]
    fn zero_seeders_are_never_auto_downloaded() {
        let db = Db::open_in_memory().unwrap();
        seed_show(
            &db,
            1,
            "Kimi no Na wa",
            "movie",
            false,
            &[("Kimi no Na wa", NameSource::Mal, PRIORITY_MAIN)],
        );
        let inputs = TargetInputs::load(&db, 1).unwrap();
        let (ranked, rejected) = rank(
            candidates(&[("[SubsPlease] Kimi no Na wa (2016) [BD1080p].mkv", 0)]),
            &inputs,
            None,
            &rules(),
            &settings(),
            now(),
        );
        // Zero seeders is a hard reject, so nothing is ranked.
        assert!(ranked.is_empty());
        let verdict = decide_verdict(ranked, rejected, &inputs, &rules(), &settings());
        match verdict {
            Verdict::NotFound { rejections } => {
                assert_eq!(rejections[0].1, Rejection::NoSeeders);
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn a_weak_match_never_proceeds() {
        let db = Db::open_in_memory().unwrap();
        seed_show(
            &db,
            1,
            "Kimi no Na wa",
            "movie",
            false,
            &[("Kimi no Na wa", NameSource::Mal, PRIORITY_MAIN)],
        );
        let inputs = TargetInputs::load(&db, 1).unwrap();
        let (ranked, rejected) = rank(
            candidates(&[("[SubsPlease] Some Unrelated Show (2016) [BD1080p].mkv", 53)]),
            &inputs,
            None,
            &rules(),
            &settings(),
            now(),
        );
        match decide_verdict(ranked, rejected, &inputs, &rules(), &settings()) {
            Verdict::Ask { reason, ranked } => {
                assert!(matches!(reason, AskReason::NoConfidentMatch { .. }));
                assert_eq!(ranked.len(), 1, "the card still shows what was found");
            }
            other => panic!("expected Ask, got {other:?}"),
        }
    }

    #[test]
    fn nothing_found_is_not_found_with_reasons() {
        let db = Db::open_in_memory().unwrap();
        seed_show(
            &db,
            1,
            "Kimi no Na wa",
            "movie",
            false,
            &[("Kimi no Na wa", NameSource::Mal, PRIORITY_MAIN)],
        );
        // The other show has to exist for its names to become the reject set.
        seed_show(
            &db,
            2,
            "Kimi no Todoke",
            "tv",
            false,
            &[("Kimi no Todoke", NameSource::Mal, PRIORITY_MAIN)],
        );
        let inputs = TargetInputs::load(&db, 1).unwrap();
        let (ranked, rejections) = rank(
            candidates(&[("[SubsPlease] Kimi no Todoke - 01 (1080p) [ABCD1234].mkv", 80)]),
            &inputs,
            Some(1.0),
            &rules(),
            &settings(),
            now(),
        );
        assert!(ranked.is_empty());
        assert_eq!(rejections.len(), 1);
        match decide_verdict(ranked, rejections, &inputs, &rules(), &settings()) {
            Verdict::NotFound { rejections } => {
                assert!(matches!(rejections[0].1, Rejection::TitleIsAnotherShow { .. }));
            }
            other => panic!("expected NotFound with reasons, got {other:?}"),
        }
    }

    #[test]
    fn the_expected_show_card_carries_the_identity() {
        let db = Db::open_in_memory().unwrap();
        seed_show(
            &db,
            1,
            "Kimi no Na wa",
            "movie",
            false,
            &[("Kimi no Na wa", NameSource::Mal, PRIORITY_MAIN)],
        );
        let inputs = TargetInputs::load(&db, 1).unwrap();
        let expected = inputs.expected_show();
        assert_eq!(expected.title, "Kimi no Na wa");
        assert_eq!(expected.english_title.as_deref(), Some("English Kimi no Na wa"));
        assert_eq!(expected.year, Some(2016));
    }

    #[test]
    fn a_show_without_names_cannot_be_matched() {
        let db = Db::open_in_memory().unwrap();
        db.with(|c| {
            c.execute(
                "INSERT INTO shows (mal_id, title, type, status) VALUES (1, 'No Names', 'tv', 'airing')",
                [],
            )?;
            Ok(())
        })
        .unwrap();
        // One source of truth: no name set, no search — never a guess.
        assert!(TargetInputs::load(&db, 1).is_err());
        assert!(TargetInputs::load(&db, 999).is_err());
    }
}
