use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Result};

use genrl_core::clock::{Clock, SystemClock};
use genrl_core::db::media_repo::{self, UpsertMedia};
use genrl_core::http::Gate;
use genrl_core::indexer::config::load_tmdb_key;
use genrl_core::indexer::fetch::GateFetch;
use genrl_core::indexer::types::MediaKind;
use genrl_core::metadata::tmdb::TmdbClient;
use genrl_core::metadata::MetadataProvider;
use genrl_core::planner::series::{
    plan_series_media, sync_tmdb_episodes, unix_secs_to_date_string,
};

use super::super::common::open_db;
use super::super::indexers::check_forbidden_key_args;
use super::types::*;

fn cap_overview(overview: String) -> String {
    overview.chars().take(OVERVIEW_CAP).collect()
}

fn overview_from_raw(raw: Option<&str>) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw?).ok()?;
    value.get("overview")?.as_str().map(str::to_string)
}

pub async fn handle_search(
    data_dir: &Path,
    query: String,
    kind: String,
    year: Option<u32>,
    json: bool,
) -> Result<()> {
    check_forbidden_key_args()?;
    let db = open_db(data_dir)?;
    let media_kind = parse_kind(&kind)?;
    let key = match load_tmdb_key() {
        Some(k) => k,
        None => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "error": "tmdb_not_configured",
                        "message": "TMDB API key is not configured"
                    }))?
                );
                return Ok(());
            } else {
                bail!("tmdb_not_configured: TMDB API key is not configured. Set it with `genrl settings set-tmdb-key`.");
            }
        }
    };

    let gate = Arc::new(Gate::new(db)?);
    let fetch = Arc::new(GateFetch::new(gate));
    let base_url = std::env::var("GENRL_TMDB_BASE_URL")
        .unwrap_or_else(|_| "https://api.themoviedb.org/3".to_string());
    let client = TmdbClient::with_base_url(key, base_url, fetch);

    let results = client.search(&query, media_kind, year).await?;
    let items: Vec<MediaSearchResult> = results
        .into_iter()
        .map(|item| {
            let truncated_overview = item.overview.map(|ov| {
                if ov.chars().count() > 300 {
                    let mut s: String = ov.chars().take(297).collect();
                    s.push_str("...");
                    s
                } else {
                    ov
                }
            });
            MediaSearchResult {
                tmdb_id: item.tmdb_id,
                title: item.title,
                year: item.year,
                poster_path: item.poster_path,
                overview: truncated_overview,
            }
        })
        .collect();

    if json {
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else if items.is_empty() {
        println!("No results found for '{query}'.");
    } else {
        for it in items {
            let y = it.year.map(|y| format!(" ({y})")).unwrap_or_default();
            println!("[{}] {}{}", it.tmdb_id, it.title, y);
            if let Some(ov) = it.overview {
                println!("    {ov}");
            }
        }
    }
    Ok(())
}

pub async fn handle_add(data_dir: &Path, tmdb_id: u32, kind: String, json: bool) -> Result<()> {
    check_forbidden_key_args()?;
    let db = open_db(data_dir)?;
    let media_kind = parse_kind(&kind)?;
    let key = match load_tmdb_key() {
        Some(k) => k,
        None => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "error": "tmdb_not_configured",
                        "message": "TMDB API key is not configured"
                    }))?
                );
                return Ok(());
            } else {
                bail!("tmdb_not_configured: TMDB API key is not configured. Set it with `genrl settings set-tmdb-key`.");
            }
        }
    };

    let gate = Arc::new(Gate::new(db.clone())?);
    let fetch = Arc::new(GateFetch::new(gate));
    let base_url = std::env::var("GENRL_TMDB_BASE_URL")
        .unwrap_or_else(|_| "https://api.themoviedb.org/3".to_string());
    let client = TmdbClient::with_base_url(key, base_url, fetch);

    let clock = SystemClock;
    let now = clock.now();

    let (media_id, jobs_created) = match media_kind {
        MediaKind::Movie => {
            let movie_details = client.get_movie_details(tmdb_id).await?;
            let parsed_year = movie_details
                .release_date
                .as_deref()
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse::<u32>().ok());

            let mut upsert_media = UpsertMedia::new(MediaKind::Movie, &movie_details.title);
            upsert_media.original_title = movie_details.original_title;
            upsert_media.year = parsed_year;
            upsert_media.tmdb_id = Some(tmdb_id);
            upsert_media.poster_path = movie_details.poster_path;
            upsert_media.raw_json = movie_details
                .overview
                .map(|ov| serde_json::json!({ "overview": cap_overview(ov) }).to_string());
            let m_id = media_repo::upsert(&db, &upsert_media, now)?;

            let active_jobs: i64 = db.with(|c| {
                let count: i64 = c.query_row(
                    "SELECT count(*) FROM jobs WHERE media_kind = 'movie' AND media_id = ?1 AND state NOT IN ('done', 'cancelled')",
                    [m_id],
                    |r| r.get(0),
                )?;
                Ok(count)
            })?;

            let jobs_created = if active_jobs == 0 {
                db.with(|c| {
                    c.execute(
                        "INSERT INTO jobs (kind, media_kind, media_id, state, created_at, updated_at) VALUES ('search', 'movie', ?1, 'search', ?2, ?2)",
                        rusqlite::params![m_id, now],
                    )?;
                    Ok(1usize)
                })?
            } else {
                0
            };

            (m_id, jobs_created)
        }
        MediaKind::Series => {
            let tv_details = client.get_tv_details(tmdb_id).await?;
            let parsed_year = tv_details
                .first_air_date
                .as_deref()
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse::<u32>().ok());

            let mut upsert_media = UpsertMedia::new(MediaKind::Series, &tv_details.name);
            upsert_media.original_title = tv_details.original_name;
            upsert_media.year = parsed_year;
            upsert_media.tmdb_id = Some(tmdb_id);
            upsert_media.poster_path = tv_details.poster_path;
            upsert_media.status = tv_details.status;
            upsert_media.raw_json = tv_details
                .overview
                .map(|ov| serde_json::json!({ "overview": cap_overview(ov) }).to_string());
            let m_id = media_repo::upsert(&db, &upsert_media, now)?;

            let today = unix_secs_to_date_string(now);
            for season in tv_details.seasons {
                if season.season_number > 0 {
                    if let Ok(episodes) = client
                        .get_season_episodes(tmdb_id, season.season_number)
                        .await
                    {
                        let _ = sync_tmdb_episodes(&db, m_id, &episodes, &today);
                    }
                }
            }

            let active_jobs: i64 = db.with(|c| {
                let count: i64 = c.query_row(
                    "SELECT count(*) FROM jobs WHERE media_kind = 'series' AND media_id = ?1 AND state NOT IN ('done', 'cancelled')",
                    [m_id],
                    |r| r.get(0),
                )?;
                Ok(count)
            })?;

            let jobs_created = if active_jobs == 0 {
                plan_series_media(&db, &clock, m_id)?
            } else {
                0
            };

            (m_id, jobs_created)
        }
        MediaKind::Anime => bail!("Anime media is managed via MAL list import"),
    };

    let res = MediaAddResult {
        media_id,
        jobs_created,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!(
            "Media added: id={}, jobs_created={}",
            res.media_id, res.jobs_created
        );
    }
    Ok(())
}

pub async fn handle_list(data_dir: &Path, kind: Option<String>, json: bool) -> Result<()> {
    check_forbidden_key_args()?;
    let db = open_db(data_dir)?;
    let filter_kind = kind.as_deref().map(parse_kind).transpose()?;
    let records = match filter_kind {
        Some(k) => media_repo::list_by_kind(&db, k)?,
        None => {
            let mut list = media_repo::list_by_kind(&db, MediaKind::Movie)?;
            list.extend(media_repo::list_by_kind(&db, MediaKind::Series)?);
            list
        }
    };

    let mut items = Vec::new();
    for rec in records {
        let progress = if rec.kind == MediaKind::Series {
            let prog = db.with(|c| {
                let mut stmt = c.prepare(
                    "SELECT state, count(*) FROM media_episodes WHERE media_id = ?1 GROUP BY state",
                )?;
                let rows = stmt.query_map([rec.id], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as usize))
                })?;
                let mut wanted = 0;
                let mut queued = 0;
                let mut done = 0;
                let mut unaired = 0;
                for r in rows {
                    let (state, count) = r?;
                    match state.as_str() {
                        "wanted" => wanted += count,
                        "queued" => queued += count,
                        "done" => done += count,
                        "unaired" => unaired += count,
                        _ => {}
                    }
                }
                Ok(EpisodeProgress {
                    wanted,
                    queued,
                    done,
                    unaired,
                })
            })?;
            Some(prog)
        } else {
            None
        };

        let poster_path = sanitize_poster_path(rec.poster_path.as_deref());
        let overview = overview_from_raw(rec.raw_json.as_deref());

        items.push(MediaListItem {
            id: rec.id,
            kind: rec.kind.to_string(),
            title: rec.title,
            year: rec.year,
            tmdb_id: rec.tmdb_id,
            status: rec.status,
            progress,
            poster_path,
            overview,
        });
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        for it in items {
            let y = it.year.map(|y| format!(" ({y})")).unwrap_or_default();
            let st = it.status.as_deref().unwrap_or("unknown");
            println!("[{}] {} [{}]{}", it.id, it.title, st, y);
            if let Some(prog) = it.progress {
                println!(
                    "    Episodes: {} done, {} queued, {} wanted, {} unaired",
                    prog.done, prog.queued, prog.wanted, prog.unaired
                );
            }
        }
    }
    Ok(())
}

pub async fn handle_remove(data_dir: &Path, media_id: i64, json: bool) -> Result<()> {
    check_forbidden_key_args()?;
    let db = open_db(data_dir)?;
    let clock = SystemClock;
    let now = clock.now();

    db.with(|c| {
        let tx = c.unchecked_transaction()?;
        tx.execute(
            "UPDATE jobs SET state = 'cancelled', updated_at = ?2 WHERE media_id = ?1 AND state NOT IN ('done', 'cancelled')",
            rusqlite::params![media_id, now],
        )?;
        tx.execute(
            "UPDATE jobs SET media_id = NULL WHERE media_id = ?1",
            rusqlite::params![media_id],
        )?;
        tx.execute(
            "DELETE FROM media_episodes WHERE media_id = ?1",
            rusqlite::params![media_id],
        )?;
        tx.execute(
            "DELETE FROM media WHERE id = ?1",
            rusqlite::params![media_id],
        )?;
        tx.commit()?;
        Ok(())
    })?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "media_id": media_id,
            }))?
        );
    } else {
        println!("Media removed: {media_id}");
    }
    Ok(())
}

pub async fn handle_episodes(
    data_dir: &Path,
    media_id: i64,
    season: Option<u32>,
    json: bool,
) -> Result<()> {
    check_forbidden_key_args()?;
    let db = open_db(data_dir)?;
    let episodes: Vec<MediaEpisodeItem> = db.with(|c| {
        let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match season {
            Some(s) => (
                "SELECT id, media_id, season, episode, air_date, state, file_path
                 FROM media_episodes WHERE media_id = ?1 AND season = ?2 ORDER BY season, episode"
                    .into(),
                vec![Box::new(media_id), Box::new(s)],
            ),
            None => (
                "SELECT id, media_id, season, episode, air_date, state, file_path
                 FROM media_episodes WHERE media_id = ?1 ORDER BY season, episode"
                    .into(),
                vec![Box::new(media_id)],
            ),
        };
        let mut stmt = c.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |r| {
            Ok(MediaEpisodeItem {
                id: r.get(0)?,
                media_id: r.get(1)?,
                season: r.get(2)?,
                episode: r.get(3)?,
                air_date: r.get(4)?,
                state: r.get(5)?,
                file_path: r.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for ep in rows {
            out.push(ep?);
        }
        Ok(out)
    })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&episodes)?);
    } else {
        for ep in &episodes {
            let date = ep.air_date.as_deref().unwrap_or("unknown date");
            println!(
                "S{:02}E{:02} [{}] ({date})",
                ep.season, ep.episode, ep.state
            );
        }
    }
    Ok(())
}

pub async fn handle_download(
    data_dir: &Path,
    media_id: i64,
    season: Option<u32>,
    episode: Option<u32>,
    json: bool,
) -> Result<()> {
    check_forbidden_key_args()?;
    let db = open_db(data_dir)?;
    let clock = SystemClock;
    let now = clock.now();
    let season_num = season.unwrap_or(1);
    let jobs_created = db.with(|c| {
        let tx = c.unchecked_transaction()?;
        if let Some(ep) = episode {
            tx.execute(
                "INSERT INTO jobs (kind, media_kind, media_id, season, episode, state, created_at, updated_at)
                 VALUES ('search', 'series', ?1, ?2, ?3, 'search', ?4, ?4)",
                rusqlite::params![media_id, season_num, ep, now],
            )?;
        } else {
            tx.execute(
                "INSERT INTO jobs (kind, media_kind, media_id, season, episode, state, created_at, updated_at)
                 VALUES ('search', 'series', ?1, ?2, NULL, 'search', ?3, ?3)",
                rusqlite::params![media_id, season_num, now],
            )?;
        }
        tx.commit()?;
        Ok(1usize)
    })?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "media_id": media_id,
                "season": season_num,
                "episode": episode,
                "jobs_created": jobs_created,
            }))?
        );
    } else {
        match episode {
            Some(ep) => println!("Queued single episode download for S{season_num:02}E{ep:02}"),
            None => println!("Queued season pack download for Season {season_num}"),
        }
    }
    Ok(())
}
