use crate::db::Db;
use crate::error::Result;
use crate::filename;
use crate::http::Gate;
use crate::models::{AiringStatus, Show, ShowType};
use crate::nyaa::{self, FeedItem, SearchQuery};
use crate::rules::Rules;
use crate::score::Candidate;
use crate::settings::Settings;

use super::decide::{decide_verdict, rank};
use super::types::{TargetInputs, Verdict};

/// Ask Nyaa once per name in priority order, stopping as soon as a name
/// returns enough candidates.
///
/// This is the request-count control from Step 2: the main (romaji) name is
/// asked first, and the translated and synonym names are only spent when the
/// first did not turn up enough.
pub async fn search_candidates(
    gate: &Gate,
    settings: &Settings,
    names: &[String],
    per_name: usize,
    stop_after: usize,
) -> Result<Vec<Candidate>> {
    let mut seen: Vec<String> = Vec::new();
    let mut found: Vec<Candidate> = Vec::new();

    for name in names {
        if found.len() >= stop_after {
            break;
        }
        let mut query = SearchQuery::new(name.as_str());
        query.min_seeders = 1; // dead torrents are never candidates
        query.limit = per_name.clamp(1, 1000);
        let items = nyaa::search(gate, settings, &query).await?;
        for item in items.iter() {
            if seen.contains(&item.info_hash) {
                continue;
            }
            seen.push(item.info_hash.clone());
            let release = filename::parse_release(&item.title);
            found.push(candidate_from(&release, item));
        }
    }

    // Best seeders first so the cap below keeps the healthiest candidates.
    found.sort_by(|a, b| b.seeders.cmp(&a.seeders));
    found.truncate(stop_after.max(1));
    Ok(found)
}

/// A parsed feed item as a scoring candidate.
pub fn candidate_from(release: &filename::Release, item: &FeedItem) -> Candidate {
    let mut candidate = Candidate::from_item(release, item);
    // Nyaa's uploader column is not in the RSS feed; the blocklist still works
    // against it once the torrent row is loaded.
    candidate.uploader = None;
    candidate
}

/// One end-to-end matching run for a wanted episode of a show.
pub async fn run(
    db: &Db,
    gate: &Gate,
    inputs: &TargetInputs,
    wanted_episode: Option<f64>,
    now: i64,
) -> Result<Verdict> {
    let settings = Settings::load(db)?;
    let rules = Rules::for_show(db, inputs.mal_id)?;
    let candidates =
        search_candidates(gate, &settings, &inputs.names, 50, 25).await?;
    let (ranked, rejected) = rank(candidates, inputs, wanted_episode, &rules, &settings, now);
    Ok(decide_verdict(ranked, rejected, inputs, &rules, &settings))
}

// ---------------------------------------------------------------------- db glue

pub(crate) fn load_show(db: &Db, mal_id: i64) -> Result<Option<Show>> {
    db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT mal_id, title, english_title, type, status, total_episodes, start_year,
                    synopsis, poster_path, folder_path, auto, episode_offset, season
             FROM shows WHERE mal_id = ?1",
        )?;
        let mut rows = stmt.query_map([mal_id], |r| {
            Ok(Show {
                mal_id: r.get(0)?,
                title: r.get(1)?,
                english_title: r.get(2)?,
                show_type: ShowType::parse(&r.get::<_, String>(3)?),
                status: AiringStatus::parse(&r.get::<_, String>(4)?),
                total_episodes: r.get(5)?,
                start_year: r.get(6)?,
                synopsis: r.get(7)?,
                poster_path: r.get(8)?,
                folder_path: r.get(9)?,
                auto: r.get::<_, i64>(10)? != 0,
                episode_offset: r.get(11)?,
                season: r.get(12)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    })
}

/// Every name of every *other* show: the reject set.
pub(crate) fn other_show_names(db: &Db, exclude_mal_id: i64) -> Result<Vec<String>> {
    db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT DISTINCT normalized FROM show_names WHERE mal_id != ?1",
        )?;
        let rows = stmt.query_map([exclude_mal_id], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    })
}
