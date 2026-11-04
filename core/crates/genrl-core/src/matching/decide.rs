use crate::rules::Rules;
use crate::score::{self, Candidate, Rejection, SeedHealth};
use crate::settings::Settings;

use super::types::{AskReason, Ranked, TargetInputs, Verdict};

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
    for mut candidate in candidates {
        if !candidate.release.is_batch
            && target.show_type == crate::models::ShowType::Tv
            && target.total_episodes.unwrap_or(0) > 1
            && candidate.release.episode.is_none()
            && candidate.show_type != Some(crate::models::ShowType::Movie)
        {
            candidate.release.is_batch = true;
        }
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
    // Candidate limit is applied AFTER episode and season matching
    if ranked.len() > 20 {
        ranked.truncate(20);
    }
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
        return Verdict::NotFound {
            rejections: rejected,
        };
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

    let mut best = best;
    if let Some(runner_up) = ranked.get(1) {
        let batch_prefers_winner = best.candidate.release.is_batch
            && !runner_up.candidate.release.is_batch
            && rules.prefer_batch_when_finished;

        if !batch_prefers_winner
            && best.scored.signal - runner_up.scored.signal < settings.score_margin
        {
            // Collect all candidates within score_margin of best
            let tied: Vec<&Ranked> = ranked
                .iter()
                .filter(|r| best.scored.signal - r.scored.signal < settings.score_margin)
                .collect();

            let mut preferred_pick = None;
            for pref in &rules.groups {
                let pref_norm = pref.trim().to_ascii_lowercase();
                if pref_norm.is_empty() {
                    continue;
                }
                if let Some(cand) = tied.iter().find(|r| {
                    r.scored.signal >= settings.score_threshold
                        && r.candidate
                            .release
                            .group
                            .as_deref()
                            .map(|g| g.trim().to_ascii_lowercase() == pref_norm)
                            .unwrap_or(false)
                }) {
                    preferred_pick = Some((*cand).clone());
                    break;
                }
            }

            if let Some(picked) = preferred_pick {
                best = picked;
            } else if best.scored.signal >= settings.score_threshold
                && runner_up.scored.signal >= settings.score_threshold
            {
                // When every tied option is a confident match (signal >= score_threshold),
                // treat them as all the right show and auto-select the highest-ranked
                // (top of `ranked`, already sorted by total score: seeders, quality, size).
            } else {
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
    }

    let mut ranked = ranked;
    if let Some(pos) = ranked
        .iter()
        .position(|r| r.candidate.info_hash == best.candidate.info_hash)
    {
        if pos != 0 {
            let item = ranked.remove(pos);
            ranked.insert(0, item);
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
                reasons.insert(
                    0,
                    (best.candidate.release.raw.clone(), Rejection::NoSeeders),
                );
                reasons
            },
        },
        // Thin on a manual add asks; on the auto list it downloads, flagged.
        SeedHealth::Thin if !inputs.auto => Verdict::Ask {
            reason: AskReason::ThinSeeds {
                seeders: best.candidate.seeders,
                chosen: Box::new(best.clone()),
            },
            ranked,
        },
        SeedHealth::Healthy | SeedHealth::Thin => Verdict::Auto {
            chosen: Box::new(best),
            thin: health == SeedHealth::Thin,
        },
    }
}
