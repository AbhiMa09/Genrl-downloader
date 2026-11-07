use crate::filename::fuzzy_ratio;
use crate::rules::Rules;
use crate::settings::Settings;

use super::types::{Candidate, Rejection, Scored, Target};

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

    if let Some(cat) = &candidate.category_id {
        if !cat.starts_with("1_") && !cat.is_empty() {
            return Err(Rejection::TypeMismatch {
                wanted: target.show_type,
                found: crate::models::ShowType::Special,
            });
        }
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

    // ------------------------------------------------------- size policy
    let is_batch = candidate.release.is_batch
        || (target.show_type == crate::models::ShowType::Tv
            && target.wanted_episode.is_some()
            && candidate.release.episode.is_none()
            && candidate.show_type != Some(crate::models::ShowType::Movie));
    let ep_count = if let Some((from, to)) = candidate.release.episode_range {
        ((to - from + 1.0) as u64).max(1)
    } else {
        // A pack of whole seasons holds one show's worth of episodes per
        // season it spans — `S01-S06` of a 13-episode season is 78, not 13.
        let seasons = candidate
            .release
            .season_range
            .map(|(from, to)| u64::from(to.saturating_sub(from) + 1))
            .unwrap_or(1);
        target.total_episodes.unwrap_or(1).max(1) as u64 * seasons
    };

    let size_bytes = candidate.size_bytes;
    let size_mb = size_bytes.div_ceil(1024 * 1024);

    if is_batch {
        // Cap a pack by a per-episode budget scaled to how many episodes it covers, so a
        // legitimate long-running range pack (e.g. 001-700) is not rejected just for being
        // large. max_batch_gb stays as the floor for small batches (no change for them),
        // and the per-episode check below still rejects per-episode bloat.
        let scaled_cap_bytes = settings
            .max_episode_mb
            .saturating_mul(ep_count)
            .saturating_mul(1024 * 1024)
            .max(settings.max_batch_gb.saturating_mul(1024 * 1024 * 1024));
        if size_bytes > scaled_cap_bytes {
            return Err(Rejection::SizeOutOfRange {
                size_mb,
                min_mb: settings.min_episode_mb,
                max_mb: scaled_cap_bytes / (1024 * 1024),
            });
        }
        // How many episodes the pack holds is only knowable when the show has a
        // fixed count or the release names its own range. A whole-season pack
        // for a show with no fixed count is judged by the size cap above alone.
        let per_ep_known = target.total_episodes.is_some()
            || candidate.release.episode_range.is_some()
            || candidate.release.season_range.is_none();
        if per_ep_known {
            let per_ep_bytes = size_bytes / ep_count;
            let per_ep_mb = per_ep_bytes.div_ceil(1024 * 1024);
            if per_ep_mb > settings.max_episode_mb || per_ep_mb < settings.min_episode_mb {
                return Err(Rejection::SizeOutOfRange {
                    size_mb: per_ep_mb,
                    min_mb: settings.min_episode_mb,
                    max_mb: settings.max_episode_mb,
                });
            }
        }
    } else if target.show_type == crate::models::ShowType::Movie {
        let max_movie_bytes = settings.max_batch_gb.saturating_mul(1024 * 1024 * 1024);
        if size_bytes > max_movie_bytes || size_mb < settings.min_episode_mb {
            return Err(Rejection::SizeOutOfRange {
                size_mb,
                min_mb: settings.min_episode_mb,
                max_mb: settings.max_batch_gb.saturating_mul(1024),
            });
        }
    } else {
        if size_mb > settings.max_episode_mb || size_mb < settings.min_episode_mb {
            return Err(Rejection::SizeOutOfRange {
                size_mb,
                min_mb: settings.min_episode_mb,
                max_mb: settings.max_episode_mb,
            });
        }
    }

    // ------------------------------------------------------- title & identity
    let title = &candidate.release.normalized_title;
    let (best_own, matched) = best_own_ratio(target.names, title);

    // A name our own title already carries is not evidence of a different show:
    // "Kingdom 6th Season" releases say "kingdom", and "Re:Zero … Starting Life
    // in Another World" says "another". Only names foreign to our own set may
    // reject — otherwise every sequel whose MAL name adds a season qualifier
    // rejects its own releases.
    let foreign: Vec<&String> = target
        .other_names
        .iter()
        .filter(|other| {
            let other_norm = other.trim().to_ascii_lowercase();
            !target
                .names
                .iter()
                .any(|own| contains_words(own, &other_norm))
        })
        .collect();
    let (best_other, other_match) = best_ratio(&foreign, title);

    if best_own < settings.fuzzy_threshold && best_other >= settings.fuzzy_threshold {
        return Err(Rejection::TitleIsAnotherShow {
            name: other_match.unwrap_or_default(),
            ratio: best_other,
        });
    }

    // A release whose raw name explicitly mentions another show (e.g. cross-tagged OVA)
    // is rejected if that other show is known — at a word boundary only, so
    // "kingdom" cannot reject "Kingdoms of Ruin" either.
    let raw_lower = candidate.release.raw.to_ascii_lowercase();
    for other in &foreign {
        let other_norm = other.trim().to_ascii_lowercase();
        if other_norm.len() >= 4 && at_word_boundary(&raw_lower, &other_norm) {
            return Err(Rejection::TitleIsAnotherShow {
                name: other.to_string(),
                ratio: 1.0,
            });
        }
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

    if let (Some(wanted), Some((from, to))) =
        (target.wanted_episode, candidate.release.episode_range)
    {
        if wanted < from || wanted > to {
            return Err(Rejection::WrongEpisode { wanted, found: to });
        }
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
    // A release without a season tag represents Season 1, which conflicts with Season 2+.
    // A pack that spans seasons (`S01-S06`) covers the one being asked for.
    let covers_season = candidate
        .release
        .season_range
        .map(|(from, to)| {
            let wanted = target.season;
            wanted >= i64::from(from) && wanted <= i64::from(to)
        })
        .unwrap_or(false);
    let found_season = candidate.release.season.map(|s| s as i64).unwrap_or(1);
    if !covers_season && found_season != target.season {
        return Err(Rejection::SeasonConflict {
            wanted: target.season,
            found: found_season,
        });
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
    let is_batch = candidate.release.is_batch
        || (target.show_type == crate::models::ShowType::Tv
            && target.wanted_episode.is_some()
            && candidate.release.episode.is_none()
            && candidate.show_type != Some(crate::models::ShowType::Movie));

    if is_batch && target.fully_aired {
        if target.wanted_episode.is_some() && rules.prefer_batch_when_finished {
            signal += 50.0;
            notes.push("season batch prioritized over single episodes (+50)".into());
        } else if target.wanted_episode.is_some() {
            signal += 30.0;
            notes.push("batch covering season episodes (+30)".into());
        } else {
            signal += 15.0;
            notes.push("batch for a show that has finished airing".into());
        }
    } else if is_batch {
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
    let seeder_points = 20.0 * ((candidate.seeders as f64 + 1.0).log10() / 1001.0f64.log10());
    ranking += seeder_points;
    notes.push(format!(
        "{} seeders (+{seeder_points:.1})",
        candidate.seeders
    ));

    if candidate.trusted && !candidate.remake {
        ranking += 15.0;
        notes.push("trusted, not a remake".into());
    }

    // Languages: exactly what was asked for wins; dual audio is still a plus.
    if !rules.preferred_languages.is_empty()
        && candidate.release.languages.iter().any(|l| {
            rules
                .preferred_languages
                .iter()
                .any(|p| p.eq_ignore_ascii_case(l))
        })
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

    // Episode-first mode: a multi-episode batch must not win over a single episode,
    // EXCEPT a large back-catalogue pack (many episodes) for a long-running series, which
    // the user wants grabbed first even while the show is still airing. Smaller stale
    // batches for a currently-airing cour are still deprioritized.
    let covered_span = candidate
        .release
        .episode_range
        .map(|(from, to)| to - from + 1.0)
        .unwrap_or(0.0);
    let is_large_backcatalog = covered_span >= 50.0;
    if candidate.release.is_batch
        && target.wanted_episode.is_some()
        && !(target.fully_aired && rules.prefer_batch_when_finished)
        && !is_large_backcatalog
    {
        ranking -= 50.0;
        notes.push("multi-episode batch deprioritized in episode-first mode (-50)".into());
    } else if is_batch {
        // Among packs that are a legitimate choice (a finished show with batch-preference,
        // or a whole-series feature), prefer the one covering the most episodes, log-scaled
        // so a "001-700" mega-pack outranks a single-cour batch for a long-running series.
        // Seeders are rewarded separately above, so coverage and health combine.
        let covered = candidate
            .release
            .episode_range
            .map(|(from, to)| (to - from + 1.0).max(1.0))
            .unwrap_or_else(|| target.total_episodes.unwrap_or(1).max(1) as f64);
        let coverage_bonus = 20.0 * (covered.min(1000.0).log10() / 1000.0f64.log10());
        if coverage_bonus > 0.0 {
            ranking += coverage_bonus;
            notes.push(format!(
                "pack covers {covered:.0} episodes (+{coverage_bonus:.1})"
            ));
        }
    }

    // Pure dub without explicit user preference should not outrank sub
    let is_dub = !candidate.release.dual_audio
        && (candidate
            .release
            .languages
            .iter()
            .any(|l| l.eq_ignore_ascii_case("dub") || l.eq_ignore_ascii_case("english dub"))
            || candidate.release.raw.to_ascii_lowercase().contains("dub"));
    if is_dub
        && !rules
            .preferred_languages
            .iter()
            .any(|p| p.to_ascii_lowercase().contains("dub"))
    {
        ranking -= 40.0;
        notes.push("pure dub deprioritized without preference (-40)".into());
    }

    Ok(Scored {
        signal,
        ranking,
        title_ratio: best_own,
        matched_name: matched,
        notes,
    })
}

fn best_ratio<S: AsRef<str>>(names: &[S], title: &str) -> (f64, Option<String>) {
    let mut best = 0.0;
    let mut best_name = None;
    for name in names {
        let ratio = fuzzy_ratio(name.as_ref(), title);
        if ratio > best {
            best = ratio;
            best_name = Some(name.as_ref().to_string());
        }
    }
    (best, best_name)
}

/// Best ratio against the show's *own* names, where being a whole-word prefix
/// of one of them counts as a full match.
///
/// A release title drops the season qualifier MAL keeps: "Kingdom" for
/// "Kingdom 6th Season", "… Isekai Seikatsu" for "… 4th Season". Edit distance
/// alone reads that as a mismatch (0.78), the show then fails the confidence
/// bar, and every candidate ends up asked about instead of taken.
fn best_own_ratio(names: &[String], title: &str) -> (f64, Option<String>) {
    let mut best = 0.0;
    let mut best_name = None;
    for name in names {
        let mut ratio = fuzzy_ratio(name, title);
        if ratio < 1.0 && word_prefix_match(name, title) {
            ratio = 1.0;
        }
        if ratio > best {
            best = ratio;
            best_name = Some(name.clone());
        }
    }
    (best, best_name)
}

fn words(s: &str) -> Vec<&str> {
    s.split_whitespace().collect()
}

/// `needle`'s words appear as a contiguous run inside `haystack`'s words.
fn contains_words(haystack: &str, needle: &str) -> bool {
    let hay = words(haystack);
    let needle = words(needle);
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w == &needle[..])
}

/// `needle` occurs in `hay` only between word boundaries, so "kingdom" matches
/// "Kingdom S6" but not "Kingdoms of Ruin".
fn at_word_boundary(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    hay.match_indices(needle).any(|(i, _)| {
        let before = hay[..i]
            .chars()
            .next_back()
            .map_or(true, |c| !c.is_alphanumeric());
        let after = hay[i + needle.len()..]
            .chars()
            .next()
            .map_or(true, |c| !c.is_alphanumeric());
        before && after
    })
}

/// One name is the same show as the other, once the season qualifier one of
/// them keeps is set aside: "kingdom" for "kingdom 6", "… seikatsu" for
/// "… seikatsu 4", "kingdom" for "kingdom s6". The short side must still be a
/// real word or two, so a stray token cannot certify a match.
fn word_prefix_match(a: &str, b: &str) -> bool {
    let (short, long) = if a.chars().count() <= b.chars().count() {
        (a, b)
    } else {
        (b, a)
    };
    let short_words = identity_words(short);
    if short_words.is_empty() || short.chars().count() < 6 {
        return false;
    }
    let long_words = identity_words(long);
    short_words.len() <= long_words.len() && long_words[..short_words.len()] == short_words[..]
}

/// The words that carry a title's identity: the season marker and the bare
/// season number dropped, since one side writes "S6" and the other "6th
/// Season". `normalize` already turned "6th" into "6" and removed "season".
fn identity_words(s: &str) -> Vec<&str> {
    words(s)
        .into_iter()
        .filter(|w| {
            let body = w.strip_prefix('s').unwrap_or(w);
            body.is_empty() || !body.chars().all(|c| c.is_ascii_digit())
        })
        .collect()
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
