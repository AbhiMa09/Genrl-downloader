//! String matching and scoring helper functions.

use crate::filename::fuzzy_ratio;

pub fn best_ratio<S: AsRef<str>>(names: &[S], title: &str) -> (f64, Option<String>) {
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
pub fn best_own_ratio(names: &[String], title: &str) -> (f64, Option<String>) {
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

pub fn words(s: &str) -> Vec<&str> {
    s.split_whitespace().collect()
}

/// `needle`'s words appear as a contiguous run inside `haystack`'s words.
pub fn contains_words(haystack: &str, needle: &str) -> bool {
    let hay = words(haystack);
    let needle = words(needle);
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w == &needle[..])
}

/// `needle` occurs in `hay` only between word boundaries, so "kingdom" matches
/// "Kingdom S6" but not "Kingdoms of Ruin".
pub fn at_word_boundary(hay: &str, needle: &str) -> bool {
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
pub fn word_prefix_match(a: &str, b: &str) -> bool {
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
pub fn identity_words(s: &str) -> Vec<&str> {
    words(s)
        .into_iter()
        .filter(|w| {
            let body = w.strip_prefix('s').unwrap_or(w);
            body.is_empty() || !body.chars().all(|c| c.is_ascii_digit())
        })
        .collect()
}

pub fn resolution_rank(resolution: &str) -> Option<u32> {
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

/// Compute the covered missing and owned counts for a batch.
pub fn calculate_batch_coverage(
    effective_range: Option<(f64, f64)>,
    mapped_season_range: Option<(u32, u32)>,
    missing_episodes: &[f64],
    owned_episodes: &[f64],
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
    } else {
        (missing_episodes.len() as u64, owned_episodes.len() as u64)
    }
}

/// Compute scaled size cap in bytes for a batch candidate.
pub fn compute_batch_size_cap(
    settings: &crate::settings::Settings,
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
