use crate::db::Db;
use crate::error::Result;
use crate::models::{AiringStatus, ShowType};
use crate::names::NameSet;
use crate::questions::{ExpectedShow, MatchOption};
use crate::score::{self, Candidate, Rejection, Scored};

use super::search::{load_show, other_show_names};

/// Everything the scorer needs to know about the show, owned so nothing
/// borrows the database across an await.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetInputs {
    pub mal_id: i64,
    pub title: String,
    pub english_title: Option<String>,
    pub start_year: Option<i64>,
    pub show_type: ShowType,
    /// 1-based season within the chain (MAL season 2 → 2).
    pub season: i64,
    pub fully_aired: bool,
    /// On the automatic list: thin seeds download and get flagged instead of
    /// asking.
    pub auto: bool,
    /// The show's own normalized names, in search order.
    pub names: Vec<String>,
    /// Every other show's normalized names, to reject on.
    pub other_names: Vec<String>,
}

impl TargetInputs {
    /// Load a show's matching inputs: its names plus the names of every other
    /// show (the reject set).
    pub fn load(db: &Db, mal_id: i64) -> Result<Self> {
        let show = load_show(db, mal_id)?
            .ok_or_else(|| crate::Error::Other(format!("no show with MAL id {mal_id}")))?;
        let names = NameSet::load(db, mal_id)?.search_order();
        if names.is_empty() {
            return Err(crate::Error::Other(format!(
                "show {mal_id} has no names to search for"
            )));
        }
        let other_names = other_show_names(db, mal_id)?;
        Ok(TargetInputs {
            mal_id: show.mal_id,
            title: show.title,
            english_title: show.english_title,
            start_year: show.start_year,
            show_type: show.show_type,
            season: show.season,
            fully_aired: show.status == AiringStatus::Finished,
            auto: show.auto,
            names,
            other_names,
        })
    }

    /// Set the season this run is looking for (from the season chain).
    pub fn with_season(mut self, season: i64) -> Self {
        self.season = season;
        self
    }

    /// A `score::Target` for the wanted episode.
    pub fn score_target(&self, wanted_episode: Option<f64>) -> score::Target<'_> {
        score::Target {
            names: &self.names,
            other_names: &self.other_names,
            wanted_episode,
            show_type: self.show_type,
            season: self.season,
            start_year: self.start_year,
            fully_aired: self.fully_aired,
        }
    }

    /// The left-hand card of a Match Review question.
    pub fn expected_show(&self) -> ExpectedShow {
        ExpectedShow {
            mal_id: self.mal_id,
            title: self.title.clone(),
            english_title: self.english_title.clone(),
            year: self.start_year,
        }
    }
}

/// A candidate that passed the filters, with its scores.
#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub candidate: Candidate,
    pub scored: Scored,
}

impl Ranked {
    /// The Match Review card line for this candidate.
    pub fn to_option(&self) -> MatchOption {
        MatchOption {
            info_hash: self.candidate.info_hash.clone(),
            name: self.candidate.release.raw.clone(),
            magnet: self.candidate.magnet.clone(),
            group: self.candidate.release.group.clone(),
            resolution: self.candidate.release.resolution.clone(),
            seeders: self.candidate.seeders,
            size_bytes: self.candidate.size_bytes,
            signal: self.scored.signal,
            ranking: self.scored.ranking,
            title_ratio: self.scored.title_ratio,
            why: self.scored.notes.clone(),
        }
    }
}

/// Why the engine will not decide on its own.
#[derive(Debug, Clone, PartialEq)]
pub enum AskReason {
    /// No candidate reached the accept score.
    NoConfidentMatch { best: f64, needed: f64 },
    /// The runner-up is too close to call.
    TooClose { best: f64, runner_up: f64, margin: f64 },
    /// The winner is thin, and this show is not on the auto list.
    ThinSeeds { seeders: u32 },
}

impl std::fmt::Display for AskReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AskReason::NoConfidentMatch { best, needed } => write!(
                f,
                "best candidate scored {best:.0}, below the {needed:.0} needed to proceed"
            ),
            AskReason::TooClose { best, runner_up, margin } => write!(
                f,
                "runner-up scored {runner_up:.0} against the best {best:.0}, closer than the {margin:.0} margin"
            ),
            AskReason::ThinSeeds { seeders } => {
                write!(f, "only {seeders} seeders, which is below your auto-download line")
            }
        }
    }
}

/// What the engine says to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Download this one.
    Auto {
        chosen: Box<Ranked>,
        /// Thin seeds on an auto-list show: download, but flag it.
        thin: bool,
    },
    /// Not clear enough: pause the job and show a Match Review card.
    Ask { reason: AskReason, ranked: Vec<Ranked> },
    /// Nothing usable, with the reasons everything was dropped for.
    NotFound { rejections: Vec<(String, Rejection)> },
}
