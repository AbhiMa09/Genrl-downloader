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

use crate::error::Result;
use crate::filename;
use crate::http::Gate;
use crate::models::{AiringStatus, Show, ShowType};
use crate::names::NameSet;
use crate::nyaa::{self, FeedItem, SearchQuery};
use crate::questions::{ExpectedShow, MatchOption};
use crate::rules::Rules;
use crate::score::{self, Candidate, Rejection, Scored, SeedHealth};
use crate::settings::Settings;
use crate::db::Db;

/// Everything the scorer needs to know about the show, owned so nothing
/// borrows the database across an await.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetInputs {
    pub mal_id: i64,
    pub title: String,
    pub english_title: Option<String>,
    pub start_year: Option<i64>,
    pub show_type: ShowType,
    /// 1-based season within the chain (MAL season 2 → 2).
    pub season: i64,
    pub fully_aired: bool,
    /// On the automatic list: thin seeds download and get flagged instead of
    /// asking.
    pub auto: bool,
    /// The show's own normalized names, in search order.
    pub names: Vec<String>,
    /// Every other show's normalized names, to reject on.
    pub other_names: Vec<String>,
}

impl TargetInputs {
    /// Load a show's matching inputs: its names plus the names of every other
    /// show (the reject set).
    pub fn load(db: &Db, mal_id: i64) -> Result<Self> {
        let show = load_show(db, mal_id)?
            .ok_or_else(|| crate::Error::Other(format!("no show with MAL id {mal_id}")))?;
        let names = NameSet::load(db, mal_id)?.search_order();
        if names.is_empty() {
            return Err(crate::Error::Other(format!(
                "show {mal_id} has no names to search for"
            )));
        }
        let other_names = other_show_names(db, mal_id)?;
        Ok(TargetInputs {
            mal_id: show.mal_id,
            title: show.title,
            english_title: show.english_title,
            start_year: show.start_year,
            show_type: show.show_type,
            season: show.season,
            fully_aired: show.status == AiringStatus::Finished,
            auto: show.auto,
            names,
            other_names,
        })
    }

    /// Set the season this run is looking for (from the season chain).
    pub fn with_season(mut self, season: i64) -> Self {
        self.season = season;
        self
    }

    /// A `score::Target` for the wanted episode.
    pub fn score_target(&self, wanted_episode: Option<f64>) -> score::Target<'_> {
        score::Target {
            names: &self.names,
            other_names: &self.other_names,
            wanted_episode,
            show_type: self.show_type,
            season: self.season,
            start_year: self.start_year,
            fully_aired: self.fully_aired,
        }
    }

    /// The left-hand card of a Match Review question.
    pub fn expected_show(&self) -> ExpectedShow {
        ExpectedShow {
            mal_id: self.mal_id,
            title: self.title.clone(),
            english_title: self.english_title.clone(),
            year: self.start_year,
        }
    }
}

/// A candidate that passed the filters, with its scores.
#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub candidate: Candidate,
    pub scored: Scored,
}

impl Ranked {
    /// The Match Review card line for this candidate.
    pub fn to_option(&self) -> MatchOption {
        MatchOption {
            info_hash: self.candidate.info_hash.clone(),
            name: self.candidate.release.raw.clone(),
            magnet: self.candidate.magnet.clone(),
            group: self.candidate.release.group.clone(),
            resolution: self.candidate.release.resolution.clone(),
            seeders: self.candidate.seeders,
            size_bytes: self.candidate.size_bytes,
            signal: self.scored.signal,
            ranking: self.scored.ranking,
            title_ratio: self.scored.title_ratio,
            why: self.scored.notes.clone(),
        }
    }
}

/// Why the engine will not decide on its own.
#[derive(Debug, Clone, PartialEq)]
pub enum AskReason {
    /// No candidate reached the accept score.
    NoConfidentMatch { best: f64, needed: f64 },
    /// The runner-up is too close to call.
    TooClose { best: f64, runner_up: f64, margin: f64 },
    /// The winner is thin, and this show is not on the auto list.
    ThinSeeds { seeders: u32 },
}

impl std::fmt::Display for AskReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AskReason::NoConfidentMatch { best, needed } => write!(
                f,
                "best candidate scored {best:.0}, below the {needed:.0} needed to proceed"
            ),
            AskReason::TooClose { best, runner_up, margin } => write!(
                f,
                "runner-up scored {runner_up:.0} against the best {best:.0}, closer than the {margin:.0} margin"
            ),
            AskReason::ThinSeeds { seeders } => {
                write!(f, "only {seeders} seeders, which is below your auto-download line")
            }
        }
    }
}

/// What the engine says to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Download this one.
    Auto {
        chosen: Box<Ranked>,
        /// Thin seeds on an auto-list show: download, but flag it.
        thin: bool,
    },
    /// Not clear enough: pause the job and show a Match Review card.
    Ask { reason: AskReason, ranked: Vec<Ranked> },
    /// Nothing usable, with the reasons everything was dropped for.
    NotFound { rejections: Vec<(String, Rejection)> },
}

/// Score every candidate, sort by what matters, and keep the passes and the
/// rejections with their reasons.
pub fn rank(
    candidates: Vec<Candidate>,
    inputs: &TargetInputs,
    wanted_episode: Option<f64>,
    rules: &Rules,
    settings: &Settings,
    now: i64,
) -> (Vec<Ranked>, Vec<(String, Rejection)>) {
    let target = inputs.score_target(wanted_episode);
    let mut ranked: Vec<Ranked> = Vec::new();
    let mut rejected: Vec<(String, Rejection)> = Vec::new();
    for candidate in candidates {
        match score::evaluate(&candidate, &target, rules, settings, now) {
            Ok(scored) => ranked.push(Ranked { candidate, scored }),
            Err(rejection) => rejected.push((candidate.release.raw.clone(), rejection)),
        }
    }
    // Ranking only orders the candidates that passed; it never rescues one.
    ranked.sort_by(|a, b| {
        b.scored
            .total()
            .partial_cmp(&a.scored.total())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    (ranked, rejected)
}

/// The Step 4 decision, with the seed-health buckets folded in.
///
/// `rejected` rides along so a `NotFound` verdict can name why everything was
/// dropped — the Activity log sentence the design asks for.
pub fn decide_verdict(
    ranked: Vec<Ranked>,
    rejected: Vec<(String, Rejection)>,
    inputs: &TargetInputs,
    rules: &Rules,
    settings: &Settings,
) -> Verdict {
    let Some(best) = ranked.first().cloned() else {
        return Verdict::NotFound { rejections: rejected };
    };

    if best.scored.signal < settings.score_threshold {
        return Verdict::Ask {
            reason: AskReason::NoConfidentMatch {
                best: best.scored.signal,
                needed: settings.score_threshold,
            },
            ranked,
        };
    }

    if let Some(runner_up) = ranked.get(1) {
        let batch_prefers_winner = best.candidate.release.is_batch
            && !runner_up.candidate.release.is_batch
            && rules.prefer_batch_when_finished;

        if !batch_prefers_winner && best.scored.signal - runner_up.scored.signal < settings.score_margin {
            return Verdict::Ask {
                reason: AskReason::TooClose {
                    best: best.scored.signal,
                    runner_up: runner_up.scored.signal,
                    margin: settings.score_margin,
                },
                ranked,
            };
        }
    }

    // Seed health decides whether a good match is *allowed* to download.
    // (Zero seeders never reaches this point — it is a hard filter in
    // `score::evaluate` — but the branch stays so the rule reads in one place.)
    let health = score::seed_health(best.candidate.seeders, rules);
    match health {
        SeedHealth::Dead => Verdict::NotFound {
            rejections: {
                let mut reasons = rejected;
                reasons.insert(0, (best.candidate.release.raw.clone(), Rejection::NoSeeders));
                reasons
            },
        },
        // Thin on a manual add asks; on the auto list it downloads, flagged.
        SeedHealth::Thin if !inputs.auto => Verdict::Ask {
            reason: AskReason::ThinSeeds {
                seeders: best.candidate.seeders,
            },
            ranked,
        },
        SeedHealth::Healthy | SeedHealth::Thin => Verdict::Auto {
            chosen: Box::new(best),
            thin: health == SeedHealth::Thin,
        },
    }
}

/// Ask Nyaa once per name in priority order, stopping as soon as a name
/// returns enough candidates.
///
/// This is the request-count control from Step 2: the main (romaji) name is
/// asked first, and the translated and synonym names are only spent when the
/// first did not turn up enough.
pub async fn search_candidates(
    gate: &Gate,
    settings: &Settings,
    names: &[String],
    per_name: usize,
    stop_after: usize,
) -> Result<Vec<Candidate>> {
    let mut seen: Vec<String> = Vec::new();
    let mut found: Vec<Candidate> = Vec::new();

    for name in names {
        if found.len() >= stop_after {
            break;
        }
        let mut query = SearchQuery::new(name.as_str());
        query.min_seeders = 1; // dead torrents are never candidates
        query.limit = per_name.clamp(1, 1000);
        let items = nyaa::search(gate, settings, &query).await?;
        for item in items.iter() {
            if seen.contains(&item.info_hash) {
                continue;
            }
            seen.push(item.info_hash.clone());
            let release = filename::parse_release(&item.title);
            found.push(candidate_from(&release, item));
        }
    }

    // Best seeders first so the cap below keeps the healthiest candidates.
    found.sort_by(|a, b| b.seeders.cmp(&a.seeders));
    found.truncate(stop_after.max(1));
    Ok(found)
}

/// A parsed feed item as a scoring candidate.
pub fn candidate_from(release: &filename::Release, item: &FeedItem) -> Candidate {
    let mut candidate = Candidate::from_item(release, item);
    // Nyaa's uploader column is not in the RSS feed; the blocklist still works
    // against it once the torrent row is loaded.
    candidate.uploader = None;
    candidate
}

/// One end-to-end matching run for a wanted episode of a show.
pub async fn run(
    db: &Db,
    gate: &Gate,
    inputs: &TargetInputs,
    wanted_episode: Option<f64>,
    now: i64,
) -> Result<Verdict> {
    let settings = Settings::load(db)?;
    let rules = Rules::for_show(db, inputs.mal_id)?;
    let candidates =
        search_candidates(gate, &settings, &inputs.names, 50, 25).await?;
    let (ranked, rejected) = rank(candidates, inputs, wanted_episode, &rules, &settings, now);
    Ok(decide_verdict(ranked, rejected, inputs, &rules, &settings))
}

// ---------------------------------------------------------------------- db glue

fn load_show(db: &Db, mal_id: i64) -> Result<Option<Show>> {
    db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT mal_id, title, english_title, type, status, total_episodes, start_year,
                    synopsis, poster_path, folder_path, auto, episode_offset, season
             FROM shows WHERE mal_id = ?1",
        )?;
        let mut rows = stmt.query_map([mal_id], |r| {
            Ok(Show {
                mal_id: r.get(0)?,
                title: r.get(1)?,
                english_title: r.get(2)?,
                show_type: ShowType::parse(&r.get::<_, String>(3)?),
                status: AiringStatus::parse(&r.get::<_, String>(4)?),
                total_episodes: r.get(5)?,
                start_year: r.get(6)?,
                synopsis: r.get(7)?,
                poster_path: r.get(8)?,
                folder_path: r.get(9)?,
                auto: r.get::<_, i64>(10)? != 0,
                episode_offset: r.get(11)?,
                season: r.get(12)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    })
}

/// Every name of every *other* show: the reject set.
fn other_show_names(db: &Db, exclude_mal_id: i64) -> Result<Vec<String>> {
    db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT DISTINCT normalized FROM show_names WHERE mal_id != ?1",
        )?;
        let rows = stmt.query_map([exclude_mal_id], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NameSource;
    use crate::names::{self, PRIORITY_MAIN, PRIORITY_USER};

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
