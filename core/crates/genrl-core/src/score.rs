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

use std::fmt;

use crate::filename::{fuzzy_ratio, Release};
use crate::models::ShowType;
use crate::nyaa::FeedItem;
use crate::rules::Rules;
use crate::settings::Settings;

/// Seed health from the configured thresholds: dead, thin, healthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedHealth {
    Dead,
    Thin,
    Healthy,
}

pub fn seed_health(seeders: u32, rules: &Rules) -> SeedHealth {
    if seeders == 0 {
        SeedHealth::Dead
    } else if seeders >= rules.seed_auto_threshold {
        SeedHealth::Healthy
    } else {
        SeedHealth::Thin
    }
}

/// A candidate that survived fetching and parsing.
///
/// Fully owned: the parsed release is kept (cloned) alongside the feed facts,
/// so callers never juggle self-referential lifetimes.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub release: Release,
    /// Info hash, lower-case hex: the identity a question answer refers to.
    pub info_hash: String,
    /// Magnet link, ready for qBittorrent.
    pub magnet: String,
    pub seeders: u32,
    pub leechers: u32,
    pub trusted: bool,
    pub remake: bool,
    pub size_bytes: u64,
    pub published_at: i64,
    pub uploader: Option<String>,
    /// Known when the caller matched the name against a typed entry; `None`
    /// means "unknown", which scores nothing but never rejects.
    pub show_type: Option<ShowType>,
}

impl Candidate {
    /// Build from a Nyaa feed item plus its parsed release name.
    pub fn from_item(release: &Release, item: &FeedItem) -> Self {
        Candidate {
            release: release.clone(),
            info_hash: item.info_hash.trim().to_lowercase(),
            magnet: item.magnet(),
            seeders: item.seeders,
            leechers: item.leechers,
            trusted: item.trusted,
            remake: item.remake,
            size_bytes: item.size_bytes,
            published_at: item.published_at,
            uploader: None,
            show_type: None,
        }
    }
}

/// What we are looking for: the show's names, the episode, its type.
#[derive(Debug, Clone)]
pub struct Target<'a> {
    /// Normalized names and aliases of *this* show (`show_names.normalized`).
    pub names: &'a [String],
    /// Normalized names that belong to other MAL entries, used to reject a
    /// candidate that clearly names a different show.
    pub other_names: &'a [String],
    /// `None` for movies and other features without episode numbers.
    pub wanted_episode: Option<f64>,
    pub show_type: ShowType,
    /// Which season of the show the entry is (1-based; MAL season 2 → 2).
    pub season: i64,
    pub start_year: Option<i64>,
    pub fully_aired: bool,
}

/// Why a candidate was rejected outright.
#[derive(Debug, Clone, PartialEq)]
pub enum Rejection {
    NoSeeders,
    ResolutionBelowFloor { resolution: String, floor: String },
    CodecBlocked { codec: String },
    SizeOutOfRange {
        size_mb: u64,
        min_mb: u64,
        max_mb: u64,
    },
    UploaderBlocked { uploader: String },
    WrongEpisode { wanted: f64, found: f64 },
    EpisodeOnAFeature { found: f64 },
    TitleIsAnotherShow { name: String, ratio: f64 },
    TypeMismatch { wanted: ShowType, found: ShowType },
    SeasonConflict { wanted: i64, found: i64 },
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::NoSeeders => write!(f, "no seeders"),
            Rejection::ResolutionBelowFloor {
                resolution,
                floor,
            } => write!(f, "resolution {resolution} is below your floor of {floor}"),
            Rejection::CodecBlocked { codec } => write!(f, "codec {codec} is blocked"),
            Rejection::SizeOutOfRange {
                size_mb,
                min_mb,
                max_mb,
            } => write!(
                f,
                "size {size_mb} MB is outside your {min_mb}–{max_mb} MB range"
            ),
            Rejection::UploaderBlocked { uploader } => write!(f, "uploader {uploader} is blocked"),
            Rejection::WrongEpisode { wanted, found } => {
                write!(f, "episode {found} is not the wanted episode {wanted}")
            }
            Rejection::EpisodeOnAFeature { found } => {
                write!(f, "episode-numbered {found} for a feature without episodes")
            }
            Rejection::TitleIsAnotherShow { name, ratio } => write!(
                f,
                "title matches a different show ({name}, ratio {ratio:.2})"
            ),
            Rejection::TypeMismatch { wanted, found } => {
                write!(f, "type {found:?} does not match {wanted:?}")
            }
            Rejection::SeasonConflict { wanted, found } => {
                write!(f, "season {found} conflicts with wanted season {wanted}")
            }
        }
    }
}

/// A candidate that passed every filter, with its two scores.
#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    /// The accept table: title/episode/type/year/batch (max 105).
    pub signal: f64,
    /// Ordering among the passing candidates (max ~125).
    pub ranking: f64,
    /// Best title ratio found, for the explanation line.
    pub title_ratio: f64,
    /// The show name the title matched, if any.
    pub matched_name: Option<String>,
    /// Human-readable reasons, for the Activity log and `genrl search`.
    pub notes: Vec<String>,
}

impl Scored {
    pub fn total(&self) -> f64 {
        self.signal + self.ranking
    }
}

/// Evaluate one candidate against one target.
pub fn evaluate(
    candidate: &Candidate,
    target: &Target,
    rules: &Rules,
    settings: &Settings,
    now: i64,
) -> Result<Scored, Rejection> {
    let mut notes = Vec::new();

    // ---------------------------------------------------------------- stage 1
    if candidate.seeders == 0 {
        return Err(Rejection::NoSeeders);
    }

    if let Some(uploader) = &candidate.uploader {
        if rules
            .blocked_uploaders
            .iter()
            .any(|b| b.eq_ignore_ascii_case(uploader))
        {
            return Err(Rejection::UploaderBlocked {
                uploader: uploader.to_string(),
            });
        }
    }

    if let Some(resolution) = &candidate.release.resolution {
        if let (Some(got), Some(floor)) = (
            resolution_rank(resolution),
            resolution_rank(&rules.min_resolution),
        ) {
            if got < floor {
                return Err(Rejection::ResolutionBelowFloor {
                    resolution: resolution.clone(),
                    floor: rules.min_resolution.clone(),
                });
            }
        }
    }

    if let Some(codec) = &candidate.release.codec {
        if rules
            .codecs_blocked
            .iter()
            .any(|b| b.eq_ignore_ascii_case(codec))
        {
            return Err(Rejection::CodecBlocked {
                codec: codec.clone(),
            });
        }
    }

    // Size sanity: strict for a single episode; batches and movies are whole
    // works and get headroom (the library step re-checks free space for real).
    let size_mb = candidate.size_bytes / 1_048_576;
    let ceiling = if candidate.release.is_batch || target.show_type == ShowType::Movie {
        rules.max_size_mb * 20
    } else {
        rules.max_size_mb
    };
    if size_mb < rules.min_size_mb || size_mb > ceiling {
        return Err(Rejection::SizeOutOfRange {
            size_mb,
            min_mb: rules.min_size_mb,
            max_mb: ceiling,
        });
    }

    // ------------------------------------------------------- title & identity
    let title = &candidate.release.normalized_title;
    let (best_own, matched) = best_ratio(target.names, title);
    let (best_other, other_match) = best_ratio(target.other_names, title);

    if best_own < settings.fuzzy_threshold && best_other >= settings.fuzzy_threshold {
        return Err(Rejection::TitleIsAnotherShow {
            name: other_match.unwrap_or_default(),
            ratio: best_other,
        });
    }

    // ------------------------------------------------------------ stage 2: signals
    let mut signal = 0.0;

    if best_own >= settings.fuzzy_threshold {
        signal += 50.0;
        notes.push(format!(
            "title matches {:?} ({best_own:.2})",
            matched.clone().unwrap_or_default()
        ));
    } else {
        notes.push(format!(
            "no name match at {:.2} (need {:.2})",
            best_own, settings.fuzzy_threshold
        ));
    }

    // Episode: both absent means a feature without episodes — agreement.
    match (target.wanted_episode, candidate.release.episode) {
        (None, None) => {
            signal += 30.0;
            notes.push("no episodes expected, none found".into());
        }
        (None, Some(found)) => {
            if !candidate.release.is_batch {
                return Err(Rejection::EpisodeOnAFeature { found });
            }
        }
        (Some(wanted), Some(found)) => {
            if (wanted - found).abs() > f64::EPSILON && !candidate.release.is_batch {
                return Err(Rejection::WrongEpisode { wanted, found });
            }
            if (wanted - found).abs() <= f64::EPSILON {
                signal += 30.0;
                notes.push(format!("episode {found} matches"));
            } else {
                notes.push("batch release covering season episodes".into());
            }
        }
        (Some(_), None) => notes.push("no episode number (batch or feature)".into()),
    }

    // Type: unknown never rejects; a known mismatch does.
    if let Some(found) = candidate.show_type {
        if found != target.show_type {
            return Err(Rejection::TypeMismatch {
                wanted: target.show_type,
                found,
            });
        }
        signal += 10.0;
        notes.push(format!("type {found:?} matches"));
    }

    // Season/part conflict is a reject, a match earns nothing extra.
    if let Some(found) = candidate.release.season {
        let found = found as i64;
        if found != target.season {
            return Err(Rejection::SeasonConflict {
                wanted: target.season,
                found,
            });
        }
    }

    // Year: a remake or an unrelated same-title work differs by more than one.
    if let (Some(ours), Some(theirs)) = (candidate.release.year, target.start_year) {
        if (ours - theirs).abs() > 1 {
            signal -= 20.0;
            notes.push(format!(
                "year {ours} differs from the show's {theirs} by more than one"
            ));
        }
    }

    // A batch / season release prioritization.
    // When looking for a wanted episode in a finished show and prefer_batch_when_finished is enabled,
    // healthy season batches are prioritized (+50) over single episodes (+30).
    // If the batch has 0 seeders, it is rejected by the zero-seeder check and healthy
    // single episodes are selected instead.
    if candidate.release.is_batch && target.fully_aired {
        if target.wanted_episode.is_some() && rules.prefer_batch_when_finished {
            signal += 50.0;
            notes.push("season batch prioritized over single episodes (+50)".into());
        } else {
            signal += 15.0;
            notes.push("batch for a show that has finished airing".into());
        }
    } else if candidate.release.is_batch {
        notes.push("batch, but the show is still airing".into());
    }

    // --------------------------------------------------------- stage 3: ranking
    let mut ranking = 0.0;

    // Group priority: the first configured group is worth the most.
    if let Some(group) = &candidate.release.group {
        if let Some(index) = rules
            .groups
            .iter()
            .position(|g| g.eq_ignore_ascii_case(group))
        {
            let points = 40.0 - 5.0 * index as f64;
            ranking += points;
            notes.push(format!("{group} is priority group #{}", index + 1));
        }
    }

    // Resolution: exact preference beats "above the floor".
    match &candidate.release.resolution {
        Some(r) if r.eq_ignore_ascii_case(&rules.preferred_resolution) => {
            ranking += 25.0;
            notes.push(format!("{r} is your preferred resolution"));
        }
        Some(_) => ranking += 10.0,
        None => {}
    }

    // Seeders on a log scale: 500 seeders are worth more than 5, but not 100×.
    let seeder_points =
        20.0 * ((candidate.seeders as f64 + 1.0).log10() / 1001.0f64.log10());
    ranking += seeder_points;
    notes.push(format!("{} seeders (+{seeder_points:.1})", candidate.seeders));

    if candidate.trusted && !candidate.remake {
        ranking += 15.0;
        notes.push("trusted, not a remake".into());
    }

    // Languages: exactly what was asked for wins; dual audio is still a plus.
    if !rules.preferred_languages.is_empty()
        && candidate
            .release
            .languages
            .iter()
            .any(|l| rules.preferred_languages.iter().any(|p| p.eq_ignore_ascii_case(l)))
    {
        ranking += 15.0;
        notes.push("has the subtitle language you asked for".into());
    } else if candidate.release.dual_audio {
        ranking += 10.0;
        notes.push("dual audio".into());
    }

    // Newer is better: half-life of a week, and a v2 fix counts as fresh.
    let age_days = (now - candidate.published_at).max(0) as f64 / 86_400.0;
    let mut recency = 10.0 * 0.5f64.powf(age_days / 7.0);
    if candidate.release.version.unwrap_or(1) >= 2 {
        recency = recency.max(10.0);
        notes.push("v2 or newer fix".into());
    }
    ranking += recency;

    Ok(Scored {
        signal,
        ranking,
        title_ratio: best_own,
        matched_name: matched,
        notes,
    })
}

fn best_ratio(names: &[String], title: &str) -> (f64, Option<String>) {
    let mut best = 0.0;
    let mut best_name = None;
    for name in names {
        let ratio = fuzzy_ratio(name, title);
        if ratio > best {
            best = ratio;
            best_name = Some(name.clone());
        }
    }
    (best, best_name)
}

fn resolution_rank(resolution: &str) -> Option<u32> {
    match resolution.trim().to_ascii_lowercase().as_str() {
        "480p" => Some(1),
        "576p" => Some(2),
        "720p" => Some(3),
        "1080p" => Some(4),
        "1440p" => Some(5),
        "2160p" | "4k" => Some(6),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filename::parse_release;

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
