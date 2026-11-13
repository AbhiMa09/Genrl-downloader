use crate::matching::release_parse::parse_media_release;
use crate::rules::Rules;
use crate::score::helpers::best_ratio;
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

fn normalize_for_title_match(raw: &str) -> (String, Vec<String>) {
    let lower = raw.to_lowercase();
    let replaced_and = lower.replace('&', " and ");
    let mut cleaned = String::with_capacity(replaced_and.len());
    for c in replaced_and.chars() {
        if c.is_alphanumeric() {
            cleaned.push(c);
        } else {
            cleaned.push(' ');
        }
    }
    let words: Vec<String> = cleaned
        .split_whitespace()
        .filter(|w| *w != "the")
        .filter(|w| {
            if w.len() == 4 && w.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(y) = w.parse::<u32>() {
                    return !(1900..=2099).contains(&y);
                }
            }
            true
        })
        .map(|w| w.to_string())
        .collect();
    let norm = words.join(" ");
    (norm, words)
}

fn extract_numbers_and_numerals(words: &[String]) -> Vec<String> {
    words
        .iter()
        .filter_map(|w| match w.as_str() {
            "1" | "i" | "one" => Some("1".to_string()),
            "2" | "ii" | "two" => Some("2".to_string()),
            "3" | "iii" | "three" => Some("3".to_string()),
            "4" | "iv" | "four" => Some("4".to_string()),
            "5" | "v" | "five" => Some("5".to_string()),
            "6" | "vi" | "six" => Some("6".to_string()),
            "7" | "vii" | "seven" => Some("7".to_string()),
            "8" | "viii" | "eight" => Some("8".to_string()),
            "9" | "ix" | "nine" => Some("9".to_string()),
            other if other.chars().all(|c| c.is_ascii_digit()) => Some(other.to_string()),
            _ => None,
        })
        .collect()
}

pub fn calculate_title_similarity(parsed: &str, wanted: &str) -> f64 {
    let (norm_p, words_p) = normalize_for_title_match(parsed);
    let (norm_w, words_w) = normalize_for_title_match(wanted);

    if norm_p.is_empty() || norm_w.is_empty() {
        return 0.0;
    }
    if norm_p == norm_w {
        return 1.0;
    }

    let nums_p = extract_numbers_and_numerals(&words_p);
    let nums_w = extract_numbers_and_numerals(&words_w);
    if (!nums_p.is_empty() || !nums_w.is_empty()) && nums_p != nums_w {
        return 0.0;
    }

    let set_p: std::collections::HashSet<&String> = words_p.iter().collect();
    let set_w: std::collections::HashSet<&String> = words_w.iter().collect();
    let inter = set_p.intersection(&set_w).count();
    let union = set_p.union(&set_w).count();
    let token_sim = if union > 0 {
        inter as f64 / union as f64
    } else {
        0.0
    };

    let str_sim = crate::filename::fuzzy_ratio(&norm_p, &norm_w);
    token_sim.max(str_sim)
}

pub fn best_title_similarity(parsed: &str, wanted_names: &[String]) -> (f64, String) {
    let mut best_sim = 0.0;
    let mut best_name = String::new();
    for wanted in wanted_names {
        let sim = calculate_title_similarity(parsed, wanted);
        if sim > best_sim {
            best_sim = sim;
            best_name = wanted.clone();
        }
    }
    (best_sim, best_name)
}

/// Evaluate a candidate against a movie or TV drama series target.
pub fn evaluate_media(
    candidate: &Candidate,
    target: &Target,
    rules: &Rules,
    _settings: &Settings,
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

    // Reject low quality sources outright (CAM, HDCAM, TS, HDTS, TELESYNC, TC, TELECINE, SCR)
    if parsed.is_cam {
        let src = parsed.source.unwrap_or_else(|| "cam".to_string());
        return Err(Rejection::LowQualitySource { source: src });
    }

    // Size sanity check
    if candidate.size_bytes > 0 {
        let size_mb = candidate.size_bytes / (1024 * 1024);
        if target.media_kind == Some(crate::indexer::types::MediaKind::Movie) {
            let min_mb = rules.movie_min_size_mb;
            let max_mb = rules.movie_max_size_mb;
            if size_mb < min_mb || size_mb > max_mb {
                return Err(Rejection::SizeOutOfRange {
                    size_mb,
                    min_mb,
                    max_mb,
                });
            }
        } else {
            // Series / TV
            if parsed.is_batch {
                let ep_count = parsed
                    .episode_range
                    .map(|(from, to)| (to - from + 1.0).max(1.0) as u64)
                    .or(target.total_episodes.map(|t| t.max(1) as u64));

                if let Some(count) = ep_count {
                    let min_mb = rules.episode_min_size_mb.saturating_mul(count);
                    let max_mb = rules.episode_max_size_mb.saturating_mul(count);
                    if size_mb < min_mb || size_mb > max_mb {
                        return Err(Rejection::SizeOutOfRange {
                            size_mb,
                            min_mb,
                            max_mb,
                        });
                    }
                } else {
                    let min_mb = rules.episode_min_size_mb;
                    if size_mb < min_mb {
                        return Err(Rejection::SizeOutOfRange {
                            size_mb,
                            min_mb,
                            max_mb: u64::MAX,
                        });
                    }
                }
            } else {
                let min_mb = rules.episode_min_size_mb;
                let max_mb = rules.episode_max_size_mb;
                if size_mb < min_mb || size_mb > max_mb {
                    return Err(Rejection::SizeOutOfRange {
                        size_mb,
                        min_mb,
                        max_mb,
                    });
                }
            }
        }
    }

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

    // Title match: normalized comparison (lowercase, strip punctuation, "the", "&"/"and", year, common separators)
    let (best_sim, matched_title) = best_title_similarity(&parsed.title, target.names);
    if best_sim < 0.85 {
        return Err(Rejection::TitleMismatch {
            parsed: parsed.title.clone(),
            wanted: target.names.first().cloned().unwrap_or_default(),
            similarity: best_sim,
        });
    }

    let (best_other, other_match) = best_ratio(target.other_names, &parsed.title);
    if best_other >= 0.85 && best_other > best_sim {
        return Err(Rejection::TitleIsAnotherShow {
            name: other_match.unwrap_or_default(),
            ratio: best_other,
        });
    }

    // ------------------------------------------------------------- 2. Signal score
    let mut signal = 0.0;

    signal += 50.0;
    notes.push(format!("title matches {matched_title:?} ({best_sim:.2})"));

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
    if target.media_kind == Some(crate::indexer::types::MediaKind::Movie)
        || target.show_type == crate::models::ShowType::Movie
    {
        if let Some(found) = parsed.episode {
            if !parsed.is_batch {
                return Err(Rejection::EpisodeOnAFeature { found });
            }
        }
        signal += 30.0;
        notes.push("feature: no episode expected (+30.0)".into());
    } else {
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
                } else if parsed.is_batch || parsed.episode_range.is_some() {
                    if let Some((from, to)) = parsed.episode_range {
                        if wanted >= from && wanted <= to {
                            signal += 30.0;
                            notes.push(format!(
                                "batch range {from}-{to} covers episode {wanted} (+30.0)"
                            ));
                        } else {
                            return Err(Rejection::WrongEpisode { wanted, found: to });
                        }
                    } else {
                        signal += 30.0;
                        notes.push("batch covering wanted episode (+30.0)".into());
                    }
                } else {
                    return Err(Rejection::WrongEpisode { wanted, found });
                }
            }
            (Some(wanted), None) => {
                if parsed.is_batch || parsed.episode_range.is_some() {
                    if let Some((from, to)) = parsed.episode_range {
                        if wanted >= from && wanted <= to {
                            signal += 30.0;
                            notes.push(format!(
                                "batch range {from}-{to} covers episode {wanted} (+30.0)"
                            ));
                        } else {
                            return Err(Rejection::WrongEpisode { wanted, found: to });
                        }
                    } else {
                        signal += 30.0;
                        notes.push("batch covering wanted episode (+30.0)".into());
                    }
                } else {
                    return Err(Rejection::WrongEpisode { wanted, found: 0.0 });
                }
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

    // Hardcoded subs penalty (only when user prefers softsub)
    if parsed.is_hardcoded_subs && rules.prefer_softsub {
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
        title_ratio: best_sim,
        matched_name: Some(matched_title),
        notes,
    })
}
