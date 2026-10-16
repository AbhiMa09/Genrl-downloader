use crate::models::QuestionKind;
use serde::{Deserialize, Serialize};

/// The show the app expected, on the left of a Match Review card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpectedShow {
    pub mal_id: i64,
    pub title: String,
    pub english_title: Option<String>,
    pub year: Option<i64>,
}

/// One candidate on the right of a Match Review card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchOption {
    pub info_hash: String,
    pub name: String,
    pub magnet: String,
    pub group: Option<String>,
    pub resolution: Option<String>,
    pub seeders: u32,
    pub size_bytes: u64,
    /// The accept-table score (title/episode/type/batch/year), 0–105.
    pub signal: f64,
    /// Ordering among the passing candidates.
    pub ranking: f64,
    /// Best title ratio found, shown as the score bar.
    pub title_ratio: f64,
    /// One line per reason the app scored it this way.
    pub why: Vec<String>,
}

/// What a question card shows, per kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    /// Which of these is the show?
    MatchReview {
        expected: ExpectedShow,
        options: Vec<MatchOption>,
        /// Why the engine could not decide on its own.
        reason: String,
    },
    /// Only N seeders. Download anyway?
    LowSeeds {
        seeders: u32,
        release: String,
        magnet: String,
        /// Auto-list shows download and get flagged instead of asking.
        auto_list: bool,
    },
    /// The episode already exists on disk.
    FileExists {
        episode: f64,
        existing: String,
        candidate: String,
    },
    /// Only a batch is available.
    BatchOnly { release: String, seeders: u32 },
    /// Free space is under the floor.
    DiskSpace { free_gb: u64, floor_gb: u64 },
    /// All candidates failed.
    AllFailed { candidates_tried: u32 },
    /// MAL merged, split or renamed the entry.
    EntryChanged {
        mal_id: i64,
        title: String,
        detail: String,
    },
    /// A service needs attention: one notice, not one per job.
    ServiceNotice { service: String, detail: String },
}

impl Payload {
    pub fn kind(&self) -> QuestionKind {
        match self {
            Payload::MatchReview { .. } => QuestionKind::MatchReview,
            Payload::LowSeeds { .. } => QuestionKind::LowSeeds,
            Payload::FileExists { .. } => QuestionKind::FileExists,
            Payload::BatchOnly { .. } => QuestionKind::BatchOnly,
            Payload::DiskSpace { .. } => QuestionKind::DiskSpace,
            Payload::AllFailed { .. } => QuestionKind::AllFailed,
            Payload::EntryChanged { .. } => QuestionKind::EntryChanged,
            Payload::ServiceNotice { .. } => QuestionKind::ServiceNotice,
        }
    }
}
