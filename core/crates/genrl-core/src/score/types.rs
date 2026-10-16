use std::fmt;

use crate::filename::Release;
use crate::models::ShowType;
use crate::nyaa::FeedItem;
use crate::rules::Rules;

/// Seed health from the configured thresholds: dead, thin, healthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedHealth {
    Dead,
    Thin,
    Healthy,
}

pub fn seed_health(seeders: u32, rules: &Rules) -> SeedHealth {
    if seeders == 0 {
        SeedHealth::Dead
    } else if seeders >= rules.seed_auto_threshold {
        SeedHealth::Healthy
    } else {
        SeedHealth::Thin
    }
}

/// A candidate that survived fetching and parsing.
///
/// Fully owned: the parsed release is kept (cloned) alongside the feed facts,
/// so callers never juggle self-referential lifetimes.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub release: Release,
    /// Info hash, lower-case hex: the identity a question answer refers to.
    pub info_hash: String,
    /// Magnet link, ready for qBittorrent.
    pub magnet: String,
    pub seeders: u32,
    pub leechers: u32,
    pub trusted: bool,
    pub remake: bool,
    pub size_bytes: u64,
    pub published_at: i64,
    pub uploader: Option<String>,
    /// Known when the caller matched the name against a typed entry; `None`
    /// means "unknown", which scores nothing but never rejects.
    pub show_type: Option<ShowType>,
}

impl Candidate {
    /// Build from a Nyaa feed item plus its parsed release name.
    pub fn from_item(release: &Release, item: &FeedItem) -> Self {
        Candidate {
            release: release.clone(),
            info_hash: item.info_hash.trim().to_lowercase(),
            magnet: item.magnet(),
            seeders: item.seeders,
            leechers: item.leechers,
            trusted: item.trusted,
            remake: item.remake,
            size_bytes: item.size_bytes,
            published_at: item.published_at,
            uploader: None,
            show_type: None,
        }
    }
}

/// What we are looking for: the show's names, the episode, its type.
#[derive(Debug, Clone)]
pub struct Target<'a> {
    /// Normalized names and aliases of *this* show (`show_names.normalized`).
    pub names: &'a [String],
    /// Normalized names that belong to other MAL entries, used to reject a
    /// candidate that clearly names a different show.
    pub other_names: &'a [String],
    /// `None` for movies and other features without episode numbers.
    pub wanted_episode: Option<f64>,
    pub show_type: ShowType,
    /// Which season of the show the entry is (1-based; MAL season 2 → 2).
    pub season: i64,
    pub start_year: Option<i64>,
    pub fully_aired: bool,
}

/// Why a candidate was rejected outright.
#[derive(Debug, Clone, PartialEq)]
pub enum Rejection {
    NoSeeders,
    ResolutionBelowFloor {
        resolution: String,
        floor: String,
    },
    CodecBlocked {
        codec: String,
    },
    SizeOutOfRange {
        size_mb: u64,
        min_mb: u64,
        max_mb: u64,
    },
    UploaderBlocked {
        uploader: String,
    },
    WrongEpisode {
        wanted: f64,
        found: f64,
    },
    EpisodeOnAFeature {
        found: f64,
    },
    TitleIsAnotherShow {
        name: String,
        ratio: f64,
    },
    TypeMismatch {
        wanted: ShowType,
        found: ShowType,
    },
    SeasonConflict {
        wanted: i64,
        found: i64,
    },
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::NoSeeders => write!(f, "no seeders"),
            Rejection::ResolutionBelowFloor { resolution, floor } => {
                write!(f, "resolution {resolution} is below your floor of {floor}")
            }
            Rejection::CodecBlocked { codec } => write!(f, "codec {codec} is blocked"),
            Rejection::SizeOutOfRange {
                size_mb,
                min_mb,
                max_mb,
            } => write!(
                f,
                "size {size_mb} MB is outside your {min_mb}–{max_mb} MB range"
            ),
            Rejection::UploaderBlocked { uploader } => write!(f, "uploader {uploader} is blocked"),
            Rejection::WrongEpisode { wanted, found } => {
                write!(f, "episode {found} is not the wanted episode {wanted}")
            }
            Rejection::EpisodeOnAFeature { found } => {
                write!(f, "episode-numbered {found} for a feature without episodes")
            }
            Rejection::TitleIsAnotherShow { name, ratio } => write!(
                f,
                "title matches a different show ({name}, ratio {ratio:.2})"
            ),
            Rejection::TypeMismatch { wanted, found } => {
                write!(f, "type {found:?} does not match {wanted:?}")
            }
            Rejection::SeasonConflict { wanted, found } => {
                write!(f, "season {found} conflicts with wanted season {wanted}")
            }
        }
    }
}

/// A candidate that passed every filter, with its two scores.
#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    /// The accept table: title/episode/type/year/batch (max 105).
    pub signal: f64,
    /// Ordering among the passing candidates (max ~125).
    pub ranking: f64,
    /// Best title ratio found, for the explanation line.
    pub title_ratio: f64,
    /// The show name the title matched, if any.
    pub matched_name: Option<String>,
    /// Human-readable reasons, for the Activity log and `genrl search`.
    pub notes: Vec<String>,
}

impl Scored {
    pub fn total(&self) -> f64 {
        self.signal + self.ranking
    }
}
