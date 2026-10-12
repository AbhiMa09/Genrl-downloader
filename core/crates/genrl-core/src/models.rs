//! Rust shapes for the tables in the data model.
//!
//! Serde names are snake_case so the same enums serialize cleanly into the
//! `payload` JSON of a question and into settings rows.

use serde::{Deserialize, Serialize};

/// One row per anime. `mal_id` is the identity everything hangs off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Show {
    pub mal_id: i64,
    pub title: String,
    pub english_title: Option<String>,
    /// `tv`, `movie`, `ova`, `ona`, `special`.
    #[serde(rename = "type")]
    pub show_type: ShowType,
    pub status: AiringStatus,
    pub total_episodes: Option<i64>,
    pub start_year: Option<i64>,
    pub synopsis: Option<String>,
    pub poster_path: Option<String>,
    pub folder_path: Option<String>,
    /// On the automatic list: thin seeds download and get flagged instead of asking.
    pub auto: bool,
    /// Absolute vs per-season numbering (episode 27 of a long show is season 2 episode 3).
    pub episode_offset: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShowType {
    #[default]
    Tv,
    Movie,
    Ova,
    Ona,
    Special,
}

impl ShowType {
    /// Parse from a Jikan/MAL type string such as `"TV"`, `"Movie"`, `"OVA"`.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "movie" => ShowType::Movie,
            "ova" => ShowType::Ova,
            "ona" => ShowType::Ona,
            "special" => ShowType::Special,
            _ => ShowType::Tv,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiringStatus {
    Airing,
    #[default]
    Finished,
    NotYetAired,
}

impl AiringStatus {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "currently airing" | "airing" => AiringStatus::Airing,
            "not yet aired" | "upcoming" => AiringStatus::NotYetAired,
            _ => AiringStatus::Finished,
        }
    }
}

/// All titles and aliases used for matching.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShowName {
    pub id: i64,
    pub mal_id: i64,
    pub name: String,
    /// Lower-cased, punctuation stripped, season words removed, abbreviations expanded.
    pub normalized: String,
    pub source: NameSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameSource {
    Mal,
    Jikan,
    /// Manual alias added by the user (also written when a Match Review answer is saved).
    User,
}

impl NameSource {
    /// The `show_names.source` value (the column is CHECK-constrained).
    pub fn as_str(self) -> &'static str {
        match self {
            NameSource::Mal => "mal",
            NameSource::Jikan => "jikan",
            NameSource::User => "user",
        }
    }
}

/// Your MAL list as last synced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MalEntry {
    pub mal_id: i64,
    pub list_status: ListStatus,
    pub watched_count: i64,
    pub score: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListStatus {
    Watching,
    Completed,
    OnHold,
    Dropped,
    PlanToWatch,
}

impl ListStatus {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "watching" => ListStatus::Watching,
            "completed" => ListStatus::Completed,
            "on_hold" | "on-hold" | "onhold" => ListStatus::OnHold,
            "dropped" => ListStatus::Dropped,
            _ => ListStatus::PlanToWatch,
        }
    }
}

/// One row per episode wanted or owned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub id: i64,
    pub mal_id: i64,
    /// `f64` so OVAs numbered 6.5 and movies numbered 1 both fit.
    pub number: f64,
    pub state: EpisodeState,
    pub file_path: Option<String>,
    pub size: Option<i64>,
    pub torrent_id: Option<i64>,
    pub last_checked: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeState {
    Missing,
    Queued,
    Downloading,
    Done,
    Failed,
}

/// Candidates found and the one chosen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Torrent {
    pub id: i64,
    /// Lower-case hex SHA-1. Unique, so a torrent can never be queued twice.
    pub info_hash: String,
    pub name: String,
    pub magnet: String,
    pub seeders: i64,
    pub leechers: i64,
    /// Bytes.
    pub size: i64,
    pub group: Option<String>,
    pub resolution: Option<String>,
    pub source_url: Option<String>,
    pub score: f64,
    pub uploader: Option<String>,
    pub found_at: i64,
}

/// The work queue: one row per unit of work, persisted before the work runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub episode_id: i64,
    pub kind: JobKind,
    pub state: JobState,
    pub attempts: i64,
    pub next_run_at: Option<i64>,
    pub last_error: Option<String>,
    pub torrent_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Search,
    Add,
    Monitor,
    Finalize,
}

/// Main flow: `queued → searching → downloading → finalizing → done`.
/// Side states pause a job without blocking the rest of the queue; a stalled
/// torrent loops back to `searching` for the next candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Searching,
    Downloading,
    Finalizing,
    Done,
    /// Unclear match or thin seeds: waiting on a question in the Inbox.
    NeedsInput,
    /// A service is down (circuit breaker) or qBittorrent is unreachable.
    Blocked,
    Failed,
}

impl JobState {
    pub fn is_terminal(self) -> bool {
        matches!(self, JobState::Done)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            JobState::Queued => "queued",
            JobState::Searching => "searching",
            JobState::Downloading => "downloading",
            JobState::Finalizing => "finalizing",
            JobState::Done => "done",
            JobState::NeedsInput => "needs_input",
            JobState::Blocked => "blocked",
            JobState::Failed => "failed",
        }
    }
}

/// An Inbox item. Questions never block other jobs and expire back to `missing`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Question {
    pub id: i64,
    pub job_id: Option<i64>,
    pub mal_id: Option<i64>,
    pub kind: QuestionKind,
    pub payload: serde_json::Value,
    pub answer: Option<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    /// Match review: which of these candidates is the show?
    MatchReview,
    /// Only N seeders. Download anyway?
    LowSeeds,
    /// The episode already exists on disk.
    FileExists,
    /// Only a batch is available.
    BatchOnly,
    /// Free space is under the floor.
    DiskSpace,
    /// All candidates failed.
    AllFailed,
    /// MAL merged, split or renamed the entry.
    EntryChanged,
}

impl QuestionKind {
    /// The `questions.kind` value.
    pub fn as_str(self) -> &'static str {
        match self {
            QuestionKind::MatchReview => "match_review",
            QuestionKind::LowSeeds => "low_seeds",
            QuestionKind::FileExists => "file_exists",
            QuestionKind::BatchOnly => "batch_only",
            QuestionKind::DiskSpace => "disk_space",
            QuestionKind::AllFailed => "all_failed",
            QuestionKind::EntryChanged => "entry_changed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "low_seeds" => QuestionKind::LowSeeds,
            "file_exists" => QuestionKind::FileExists,
            "batch_only" => QuestionKind::BatchOnly,
            "disk_space" => QuestionKind::DiskSpace,
            "all_failed" => QuestionKind::AllFailed,
            "entry_changed" => QuestionKind::EntryChanged,
            _ => QuestionKind::MatchReview,
        }
    }
}

/// One line of the Activity log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: i64,
    pub at: i64,
    pub level: crate::events::Level,
    pub job_id: Option<i64>,
    pub mal_id: Option<i64>,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_states_round_trip() {
        for state in [
            JobState::Queued,
            JobState::Searching,
            JobState::Downloading,
            JobState::Finalizing,
            JobState::Done,
            JobState::NeedsInput,
            JobState::Blocked,
            JobState::Failed,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: JobState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, state);
        }
        assert_eq!(JobState::NeedsInput.as_str(), "needs_input");
    }

    #[test]
    fn mal_strings_parse() {
        assert_eq!(ShowType::parse("Movie"), ShowType::Movie);
        assert_eq!(AiringStatus::parse("Currently Airing"), AiringStatus::Airing);
        assert_eq!(ListStatus::parse("plan_to_watch"), ListStatus::PlanToWatch);
    }
}
