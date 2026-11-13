use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};

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

use super::common::open_db;
use super::indexers::check_forbidden_key_args;

#[derive(Subcommand, Debug, Clone)]
pub enum MediaCommand {
    /// Search TMDB for movies or series.
    Search {
        query: String,
        #[arg(long, default_value = "movie")]
        kind: String,
        #[arg(long)]
        year: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Add a movie or series by TMDB ID.
    Add {
        tmdb_id: u32,
        #[arg(long, default_value = "movie")]
        kind: String,
        #[arg(long)]
        json: bool,
    },
    /// List added movies or series.
    List {
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Remove a media entry and its episodes, cancelling active jobs.
    Remove {
        media_id: i64,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaSearchResult {
    pub tmdb_id: u32,
    pub title: String,
    pub year: Option<u32>,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EpisodeProgress {
    pub wanted: usize,
    pub queued: usize,
    pub done: usize,
    pub unaired: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaListItem {
    pub id: i64,
    pub kind: String,
    pub title: String,
    pub year: Option<u32>,
    pub tmdb_id: Option<u32>,
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<EpisodeProgress>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaAddResult {
    pub media_id: i64,
    pub jobs_created: usize,
}

pub fn parse_kind(s: &str) -> Result<MediaKind> {
    match s.trim().to_ascii_lowercase().as_str() {
        "movie" | "movies" => Ok(MediaKind::Movie),
        "series" | "tv" => Ok(MediaKind::Series),
        _ => bail!("Unsupported media kind '{s}'. Supported: movie, series"),
    }
}

pub async fn media_cmd(data_dir: &Path, command: MediaCommand) -> Result<()> {
    check_forbidden_key_args()?;
    let db = open_db(data_dir)?;

    match command {
        MediaCommand::Search {
            query,
            kind,
            year,
            json,
        } => {
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
        }
        MediaCommand::Add {
            tmdb_id,
            kind,
            json,
        } => {
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
                    let m_id = media_repo::upsert(&db, &upsert_media, now)?;

                    // Check if an active job already exists for this movie
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
                    let m_id = media_repo::upsert(&db, &upsert_media, now)?;

                    // Sync episodes for all seasons > 0
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

                    // Check if active jobs already exist for this series
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
        }
        MediaCommand::List { kind, json } => {
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

                items.push(MediaListItem {
                    id: rec.id,
                    kind: rec.kind.as_str().to_string(),
                    title: rec.title,
                    year: rec.year,
                    tmdb_id: rec.tmdb_id,
                    status: rec.status,
                    progress,
                });
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else if items.is_empty() {
                println!("No media entries found.");
            } else {
                for it in items {
                    let y = it.year.map(|y| format!(" ({y})")).unwrap_or_default();
                    let prog_str = if let Some(p) = it.progress {
                        format!(
                            " [wanted: {}, queued: {}, done: {}, unaired: {}]",
                            p.wanted, p.queued, p.done, p.unaired
                        )
                    } else {
                        String::new()
                    };
                    println!("[{}] {} - {}{}{}", it.id, it.kind, it.title, y, prog_str);
                }
            }
        }
        MediaCommand::Remove { media_id, json } => {
            let clock = SystemClock;
            let now = clock.now();

            db.with(|c| {
                let tx = c.unchecked_transaction()?;
                // 1. Cancel unfinished jobs
                tx.execute(
                    "UPDATE jobs SET state = 'cancelled', updated_at = ?2 WHERE media_id = ?1 AND state NOT IN ('done', 'cancelled')",
                    rusqlite::params![media_id, now],
                )?;
                // 2. Disassociate jobs from media_id to respect foreign key on media(id)
                tx.execute(
                    "UPDATE jobs SET media_id = NULL WHERE media_id = ?1",
                    rusqlite::params![media_id],
                )?;
                // 3. Delete media_episodes
                tx.execute(
                    "DELETE FROM media_episodes WHERE media_id = ?1",
                    rusqlite::params![media_id],
                )?;
                // 4. Delete media row
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
        }
    }

    Ok(())
}
