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
