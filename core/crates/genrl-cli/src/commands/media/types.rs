use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use genrl_core::indexer::types::MediaKind;

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
    pub poster_path: Option<String>,
    pub overview: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaAddResult {
    pub media_id: i64,
    pub jobs_created: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaEpisodeItem {
    pub id: i64,
    pub media_id: i64,
    pub season: u32,
    pub episode: u32,
    pub air_date: Option<String>,
    pub state: String,
    pub file_path: Option<String>,
}

pub fn parse_kind(s: &str) -> Result<MediaKind> {
    match s.trim().to_ascii_lowercase().as_str() {
        "movie" | "movies" => Ok(MediaKind::Movie),
        "series" | "tv" => Ok(MediaKind::Series),
        _ => bail!("Unsupported media kind '{s}'. Supported: movie, series"),
    }
}

pub const OVERVIEW_CAP: usize = 1000;

/// TMDB poster paths are exposed only in the strict `/name.ext` shape so the
/// UI can never build an image URL from arbitrary stored data.
pub fn sanitize_poster_path(raw: Option<&str>) -> Option<String> {
    let path = raw?;
    let stem = path.strip_prefix('/').and_then(|rest| {
        rest.strip_suffix(".jpeg")
            .or_else(|| rest.strip_suffix(".jpg"))
            .or_else(|| rest.strip_suffix(".png"))
    })?;
    if stem.is_empty() {
        return None;
    }
    if !stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    Some(path.to_string())
}
