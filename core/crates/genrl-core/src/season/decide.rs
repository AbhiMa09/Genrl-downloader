use crate::filename::Release;

use super::types::{Decision, SeasonChain, Wanted};

/// What to do with one parsed release for one wanted episode.
///
/// The release is *judged*, never repaired: reads strictly for the want, and
/// anything it cannot map into the chain becomes an `Ask` rather than a guess.
pub fn decide(chain: &SeasonChain, wanted: &Wanted, release: &Release) -> Decision {
    let entry = match chain.entry(wanted.mal_id) {
        Some(entry) => entry,
        None => {
            return Decision::NotFound {
                reason: format!("no chain entry for {}", wanted.mal_id),
            }
        }
    };
    let wanted_season = wanted.season;

    // ------------------------------------------------------------- the season
    // A marker must name the wanted season. Anything else — including a season
    // the chain does not even have — is not found, never substituted.
    if let Some(found) = release.season {
        let found = i64::from(found);
        if found != wanted_season {
            return Decision::NotFound {
                reason: format!(
                    "release is for season {found}, but {} wants season {wanted_season}",
                    wanted.title
                ),
            };
        }
    }
    // No marker means "the active season": the one the walk landed on.

    // A movie or other feature wants no episode number.
    let Some(w) = wanted.episode else {
        return match release.episode {
            // Movies listed as "1 of 1" are the movie.
            None | Some(1.0) => Decision::Download {
                season: wanted_season,
                episode: 1.0,
            },
            Some(found) => Decision::Ask {
                reason: format!(
                    "episode-numbered release ({found}) for a feature without episodes"
                ),
            },
        };
    };

    // A batch carries no number; it either covers a finished show or it asks.
    if release.is_batch && release.episode.is_none() {
        return if entry.fully_aired {
            Decision::Download {
                season: wanted_season,
                episode: w,
            }
        } else {
            Decision::Ask {
                reason: "only a batch is available and the show is still airing".to_string(),
            }
        };
    }

    let found = match release.episode {
        Some(found) => found,
        // No number and not a batch: no way to tell what it delivers.
        None => {
            return Decision::Ask {
                reason: "release has no episode number".to_string(),
            }
        }
    };

    // The numbers the release may legitimately carry for our want: the
    // per-season number, and any offset the show is known to use.
    let mut accepted = vec![w];
    if entry.episode_offset != 0 {
        accepted.push(w + entry.episode_offset as f64);
    }
    if accepted
        .iter()
        .any(|want| (want - found).abs() < f64::EPSILON)
    {
        return Decision::Download {
            season: wanted_season,
            episode: w,
        };
    }

    // Already watched (or simply an earlier unwatched episode): keep looking.
    if found < w {
        if found as i64 <= entry.watched_count {
            return Decision::Skip {
                reason: format!("episode {found} is already watched"),
            };
        }
        return Decision::Skip {
            reason: format!("episode {found} is earlier than the wanted episode {w}"),
        };
    }

    // found > w. Beyond the entry's own count the number must be absolute:
    // subtract the earlier seasons' cumulative counts and see where it lands.
    match entry.total_episodes {
        Some(total) if found as i64 > total => {
            let Some(cumulative) = chain.episodes_before(wanted_season) else {
                return Decision::Ask {
                    reason: format!(
                        "episode {found} looks absolute, but an earlier season's episode count is unknown"
                    ),
                };
            };
            let mapped = found as i64 - cumulative;
            if mapped == w as i64 {
                return Decision::Download {
                    season: wanted_season,
                    episode: w,
                };
            }
            if (1..=total).contains(&mapped) {
                return Decision::Skip {
                    reason: format!("absolute episode {found} is episode {mapped}, not {w}"),
                };
            }
            Decision::Ask {
                reason: format!("episode {found} does not fit the season chain"),
            }
        }
        Some(_) => Decision::Skip {
            reason: format!("episode {found} is later than the wanted episode {w}"),
        },
        // The count is unknown: refuse to guess.
        None => Decision::Ask {
            reason: format!(
                "episode {found} is beyond the wanted {w} and the entry has no episode count"
            ),
        },
    }
}
