use crate::filename::fuzzy_ratio;
use crate::models::ShowType;
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
