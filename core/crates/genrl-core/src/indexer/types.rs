use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Media categories supported by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Anime,
    Movie,
    Series,
}

impl MediaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MediaKind::Anime => "anime",
            MediaKind::Movie => "movie",
            MediaKind::Series => "series",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "anime" => Some(MediaKind::Anime),
            "movie" | "movies" => Some(MediaKind::Movie),
            "series" | "drama" | "dramas" | "tv" => Some(MediaKind::Series),
            _ => None,
        }
    }
}

impl std::fmt::Display for MediaKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Unified query sent to an indexer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub kind: MediaKind,
    pub season: Option<u32>,
    pub episode: Option<f64>,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<u32>,
    pub tvdb_id: Option<u32>,
}

impl SearchQuery {
    pub fn new(text: impl Into<String>, kind: MediaKind) -> Self {
        Self {
            text: text.into(),
            kind,
            season: None,
            episode: None,
            imdb_id: None,
            tmdb_id: None,
            tvdb_id: None,
        }
    }
}

/// Download source: magnet link or remote .torrent URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "url")]
pub enum DownloadSource {
    Magnet(String),
    TorrentUrl(String),
}

impl DownloadSource {
    pub fn url(&self) -> &str {
        match self {
            DownloadSource::Magnet(s) => s,
            DownloadSource::TorrentUrl(s) => s,
        }
    }
}

/// One torrent release candidate from an indexer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Release {
    pub title: String,
    pub indexer_id: String,
    pub size_bytes: u64,
    pub seeders: u32,
    pub leechers: u32,
    pub download: DownloadSource,
    pub info_hash: Option<String>,
    pub categories: Vec<i32>,
    pub published_date: Option<String>,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default)]
    pub remake: bool,
}

/// Torznab capabilities for an indexer.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Caps {
    pub tv_available: bool,
    pub movie_available: bool,
    pub tv_params: Vec<String>,
    pub movie_params: Vec<String>,
    pub categories: Vec<i32>,
}

/// Indexer errors.
#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("http error: {0}")]
    Http(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("remote error {code}: {message}")]
    Remote { code: i32, message: String },

    #[error("indexer disabled")]
    Disabled,

    #[error("request timed out")]
    Timeout,

    #[error("circuit breaker open for {0}")]
    BreakerOpen(String),

    #[error(transparent)]
    Core(#[from] crate::error::Error),
}

impl From<IndexerError> for crate::error::Error {
    fn from(err: IndexerError) -> Self {
        match err {
            IndexerError::Core(e) => e,
            IndexerError::BreakerOpen(service) => crate::error::Error::BreakerOpen {
                service,
                retry_in_secs: 60,
            },
            IndexerError::Timeout => crate::error::Error::Timeout {
                url: "indexer".into(),
                timeout_ms: 10000,
            },
            other => crate::error::Error::Other(other.to_string()),
        }
    }
}
