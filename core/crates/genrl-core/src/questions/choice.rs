use serde::{Deserialize, Serialize};
use crate::models::QuestionKind;
use super::payload::Payload;

/// The choice a user makes on a card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "choice", rename_all = "snake_case")]
pub enum Choice {
    /// Match Review: use this candidate.
    UseThis { info_hash: String },
    /// Match Review: none of these — the title becomes a user alias telling
    /// the engine what to look for instead.
    NoneOfThese,
    /// Match Review: look again with the same names.
    SearchAgain,
    /// The release's parsed title is really this show: save it as an alias.
    Alias { name: String },

    /// Few seeders: take it anyway.
    Download,
    /// Few seeders: check again in 24 hours.
    Wait,
    /// Few seeders: pick another candidate.
    PickAnother,

    /// File exists: keep what is on disk.
    Keep,
    /// File exists: replace with the new version.
    Replace,
    /// File exists: keep both.
    KeepBoth,

    /// Batch: take the whole batch.
    Batch,
    /// Batch: wait for singles.
    WaitForSingles,

    /// Disk: pause the queue until there is room.
    Pause,
    /// Disk: work in a different folder.
    ChangeFolder,

    /// All failed: try again later.
    RetryLater,
    /// All failed: open the manual search page.
    ManualSearch,
    /// All failed: skip this episode.
    SkipEpisode,

    /// Nothing done: the question expired unanswered.
    Expired,
}

/// A stored answer: the choice, plus the "always do this" checkbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    pub choice: Choice,
    /// Remember the choice as a rule or alias so it never asks twice.
    #[serde(default)]
    pub always: bool,
}

/// A question waiting for an answer, with its payload parsed.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenQuestion {
    pub id: i64,
    pub job_id: Option<i64>,
    pub mal_id: Option<i64>,
    pub payload: Payload,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

impl OpenQuestion {
    pub fn kind(&self) -> QuestionKind {
        self.payload.kind()
    }
}
