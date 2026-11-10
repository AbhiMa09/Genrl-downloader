use super::types::{Candidate, Rejection, Target};
use crate::settings::Settings;

/// Compute the covered missing and owned counts for a batch.
pub fn calculate_batch_coverage(
    effective_range: Option<(f64, f64)>,
    mapped_season_range: Option<(u32, u32)>,
    is_batch: bool,
    missing_episodes: &[f64],
    owned_episodes: &[f64],
    _is_large_backcatalog: bool,
) -> (u64, u64) {
    if let Some((from, to)) = effective_range {
        let missing = missing_episodes
            .iter()
            .filter(|&&ep| ep >= from && ep <= to)
            .count() as u64;
        let owned = owned_episodes
            .iter()
            .filter(|&&ep| ep >= from && ep <= to)
            .count() as u64;
        (missing, owned)
    } else if let Some((first, last)) = mapped_season_range {
        let from = first as f64;
        let to = last as f64;
        let missing = missing_episodes
            .iter()
            .filter(|&&ep| ep >= from && ep <= to)
            .count() as u64;
        let owned = owned_episodes
            .iter()
            .filter(|&&ep| ep >= from && ep <= to)
            .count() as u64;
        (missing, owned)
    } else if is_batch {
        (missing_episodes.len() as u64, owned_episodes.len() as u64)
    } else {
        (0, 0)
    }
}


/// Compute scaled size cap in bytes for a batch candidate.
pub fn compute_batch_size_cap(
    settings: &Settings,
    ep_count: u64,
    has_explicit_range: bool,
    missing_episodes_len: usize,
    covered_missing: u64,
    total_episodes: Option<i64>,
) -> u64 {
    let effective_missing = if missing_episodes_len > 0 {
        if has_explicit_range {
            covered_missing
        } else {
            missing_episodes_len as u64
        }
    } else {
        0
    };

    if has_explicit_range {
        let base_cap = settings
            .max_episode_mb
            .saturating_mul(ep_count)
            .saturating_mul(1024 * 1024)
            .max(settings.max_batch_gb.saturating_mul(1024 * 1024 * 1024));
        if effective_missing > 0 && effective_missing < ep_count {
            let missing_cap = settings
                .max_episode_mb
                .saturating_mul(effective_missing)
                .saturating_mul(1024 * 1024);
            base_cap.min(missing_cap)
        } else {
            base_cap
        }
    } else {
        let missing_count = if effective_missing > 0 {
            effective_missing
        } else {
            total_episodes.unwrap_or(0).max(0) as u64
        };
        let missing_budget = settings
            .max_episode_mb
            .saturating_mul(missing_count)
            .saturating_mul(1024 * 1024);
        missing_budget.max(settings.max_batch_gb.saturating_mul(1024 * 1024 * 1024))
    }
}

/// Validate candidate size against target and settings.
pub fn check_candidate_size(
    candidate: &Candidate,
    target: &Target,
    settings: &Settings,
    is_batch: bool,
    has_explicit_range: bool,
    ep_count: u64,
    covered_missing: u64,
) -> Result<(), Rejection> {
    let size_bytes = candidate.size_bytes;
    let size_mb = size_bytes.div_ceil(1024 * 1024);

    if is_batch {
        let scaled_cap_bytes = compute_batch_size_cap(
            settings,
            ep_count,
            has_explicit_range,
            target.missing_episodes.len(),
            covered_missing,
            target.total_episodes,
        );
        if size_bytes > scaled_cap_bytes {
            return Err(Rejection::SizeOutOfRange {
                size_mb,
                min_mb: settings.min_episode_mb,
                max_mb: scaled_cap_bytes / (1024 * 1024),
            });
        }
        let per_ep_known = has_explicit_range
            || (target.total_episodes.is_some_and(|t| t > 1) && candidate.release.season_range.is_some());
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

    Ok(())
}
