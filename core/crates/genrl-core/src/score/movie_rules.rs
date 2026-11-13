use crate::matching::release_parse::parse_media_release;
use crate::rules::Rules;
use crate::score::helpers::{best_own_ratio, best_ratio};
use crate::score::types::{Candidate, Rejection, Scored, Target};
use crate::settings::Settings;

fn resolution_rank(res: &str) -> u32 {
    let lower = res.to_ascii_lowercase();
    if lower == "2160p" || lower == "4k" || lower == "uhd" {
        4
    } else if lower == "1080p" {
        3
    } else if lower == "720p" {
        2
    } else if lower == "480p" {
        1
    } else {
        0
    }
}

/// Evaluate a candidate against a movie or TV drama series target.
pub fn evaluate_media(
    candidate: &Candidate,
    target: &Target,
    rules: &Rules,
    settings: &Settings,
    _now: i64,
) -> Result<Scored, Rejection> {
    let mut notes = Vec::new();

    // ------------------------------------------------------------- 1. Hard filters
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
                uploader: uploader.clone(),
            });
        }
    }

    let parsed = parse_media_release(&candidate.release.raw);

    // Codec block
    if let Some(codec) = &parsed.codec {
        if rules
            .codecs_blocked
            .iter()
            .any(|c| c.eq_ignore_ascii_case(codec))
        {
            return Err(Rejection::CodecBlocked {
                codec: codec.clone(),
            });
        }
    }

    // Resolution floor
    if let Some(res) = &parsed.resolution {
        let res_r = resolution_rank(res);
        let floor_r = resolution_rank(&rules.min_resolution);
        if res_r > 0 && floor_r > 0 && res_r < floor_r {
            return Err(Rejection::ResolutionBelowFloor {
                resolution: res.clone(),
                floor: rules.min_resolution.clone(),
            });
        }
    }

    // ------------------------------------------------------------- 2. Signal score
    let mut signal = 0.0;

    // Title match
    let (best_other, other_match) = best_ratio(target.other_names, &parsed.title);
    let (best_own, matched) = best_own_ratio(target.names, &parsed.title);

    if best_other >= 0.85 && best_other > best_own {
        return Err(Rejection::TitleIsAnotherShow {
            name: other_match.unwrap_or_default(),
            ratio: best_other,
        });
    }

    if best_own >= settings.fuzzy_threshold {
        signal += 50.0;
        notes.push(format!(
            "title matches {:?} ({best_own:.2})",
            matched.clone().unwrap_or_default()
        ));
    } else {
        notes.push(format!(
            "title match below threshold ({best_own:.2} < {:.2})",
            settings.fuzzy_threshold
        ));
    }

    // Exact year match bonus or mismatch penalty
    if let (Some(rel_year), Some(tgt_year)) = (parsed.year, target.start_year) {
        if rel_year as i64 == tgt_year {
            signal += 15.0;
            notes.push("exact year match bonus (+15.0)".into());
        } else if (rel_year as i64 - tgt_year).abs() > 1 {
            signal -= 20.0;
            notes.push(format!(
                "year {rel_year} differs from target {tgt_year} (-20.0)"
            ));
        }
    }

    // Season match
    if target.season > 0 {
        let covers_season = if let Some((from, to)) = parsed.season_range {
            target.season >= from as i64 && target.season <= to as i64
        } else if let Some(s) = parsed.season {
            s as i64 == target.season
        } else {
            target.season == 1
        };

        if !covers_season {
            return Err(Rejection::SeasonConflict {
                wanted: target.season,
                found: parsed.season.unwrap_or(1) as i64,
            });
        }
    }

    // Episode / Feature match
    match (target.wanted_episode, parsed.episode) {
        (None, None) => {
            signal += 30.0;
            notes.push("feature / season pack: no episodes expected (+30.0)".into());
        }
        (None, Some(found)) => {
            if !parsed.is_batch {
                return Err(Rejection::EpisodeOnAFeature { found });
            }
            signal += 30.0;
            notes.push("batch release for feature/series (+30.0)".into());
        }
        (Some(wanted), Some(found)) => {
            if (wanted - found).abs() <= f64::EPSILON {
                signal += 30.0;
                notes.push(format!("episode {found} matches (+30.0)"));
            } else if parsed.is_batch {
                signal += 30.0;
                notes.push("batch covering wanted episode (+30.0)".into());
            } else {
                return Err(Rejection::WrongEpisode { wanted, found });
            }
        }
        (Some(_wanted), None) => {
            if parsed.is_batch {
                signal += 30.0;
                notes.push("batch covering wanted episode (+30.0)".into());
            } else {
                notes.push("no episode number found".into());
            }
        }
    }

    // Type agreement
    signal += 10.0;
    notes.push("type matches (+10.0)".into());

    // ------------------------------------------------------------- 3. Ranking score
    let mut ranking = 0.0;

    // Source quality ranking
    let (src_pts, src_note) = match parsed.source.as_deref() {
        Some("remux") => (45.0, Some("Remux (+45.0)")),
        Some("bluray") => (40.0, Some("BluRay (+40.0)")),
        Some("web-dl") => (30.0, Some("WEB-DL (+30.0)")),
        Some("webrip") | Some("web") => (20.0, Some("WEBRip (+20.0)")),
        Some("hdtv") | Some("hdrip") => (10.0, Some("HDTV (+10.0)")),
        _ => (0.0, None),
    };
    ranking += src_pts;
    if let Some(note) = src_note {
        notes.push(note.into());
    }

    // CAM / TS / TC heavy penalty: always ranks far below any real source
    if parsed.is_cam {
        ranking -= 100.0;
        notes.push("CAM/TS/TC release penalty (-100.0)".into());
    }

    // Hardcoded subs penalty (user prefers softsub)
    if parsed.is_hardcoded_subs {
        ranking -= 30.0;
        notes.push("hardcoded subs penalty (-30.0)".into());
    }

    // Proper / Repack bonus
    if parsed.is_proper || parsed.is_repack {
        ranking += 5.0;
        notes.push("proper/repack (+5.0)".into());
    }

    // Resolution preference
    if let Some(res) = &parsed.resolution {
        if res.eq_ignore_ascii_case(&rules.preferred_resolution) {
            ranking += 25.0;
            notes.push(format!("{res} is preferred resolution (+25.0)"));
        } else {
            ranking += 10.0;
            notes.push(format!("{res} resolution (+10.0)"));
        }
    }

    // Group priority
    if let Some(group) = &parsed.group {
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

    // Seeders on a log scale (max ~20.0)
    let seeder_points = 20.0 * ((candidate.seeders as f64 + 1.0).log10() / 1001.0f64.log10());
    ranking += seeder_points;
    notes.push(format!(
        "{} seeders (+{seeder_points:.1})",
        candidate.seeders
    ));

    Ok(Scored {
        signal,
        ranking,
        title_ratio: best_own,
        matched_name: matched,
        notes,
    })
}
