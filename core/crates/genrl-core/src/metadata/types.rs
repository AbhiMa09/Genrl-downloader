use serde::{Deserialize, Serialize};

use crate::indexer::types::MediaKind;

/// External identifiers across movie/TV databases.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExternalIds {
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<u32>,
    pub tvdb_id: Option<u32>,
}

/// Lightweight reference to a media item in the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaRef {
    pub id: i64,
    pub kind: MediaKind,
    pub tmdb_id: Option<u32>,
    pub title: String,
}

/// Common unified metadata extracted from TMDB or other providers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub kind: MediaKind,
    pub tmdb_id: u32,
    pub external_ids: ExternalIds,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<u32>,
    pub poster_path: Option<String>,
    pub status: Option<String>,
    pub total_seasons: Option<u32>,
    pub total_episodes: Option<u32>,
    pub raw_json: Option<String>,
}

/// Metadata for a single episode in a TV season.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpisodeMetadata {
    pub season: u32,
    pub episode: u32,
    pub title: String,
    pub air_date: Option<String>,
}
