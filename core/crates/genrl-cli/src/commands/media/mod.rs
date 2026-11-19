pub mod handlers;
pub mod types;

use std::path::Path;

use anyhow::Result;
use clap::Subcommand;

pub use handlers::*;

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
    /// List episodes and download states for a series.
    Episodes {
        media_id: i64,
        #[arg(long)]
        season: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    /// Queue download for a series season pack or single episode.
    Download {
        media_id: i64,
        #[arg(long)]
        season: Option<u32>,
        #[arg(long)]
        episode: Option<u32>,
        #[arg(long)]
        json: bool,
    },
}

pub async fn media_cmd(data_dir: &Path, command: MediaCommand) -> Result<()> {
    match command {
        MediaCommand::Search {
            query,
            kind,
            year,
            json,
        } => handle_search(data_dir, query, kind, year, json).await,
        MediaCommand::Add {
            tmdb_id,
            kind,
            json,
        } => handle_add(data_dir, tmdb_id, kind, json).await,
        MediaCommand::List { kind, json } => handle_list(data_dir, kind, json).await,
        MediaCommand::Remove { media_id, json } => handle_remove(data_dir, media_id, json).await,
        MediaCommand::Episodes {
            media_id,
            season,
            json,
        } => handle_episodes(data_dir, media_id, season, json).await,
        MediaCommand::Download {
            media_id,
            season,
            episode,
            json,
        } => handle_download(data_dir, media_id, season, episode, json).await,
    }
}
