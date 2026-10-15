use crate::models::{ListStatus, ShowType};

/// One MAL entry in the chain: a season of the franchise.
#[derive(Debug, Clone, PartialEq)]
pub struct SeasonEntry {
    pub mal_id: i64,
    pub title: String,
    pub list_status: ListStatus,
    /// Episodes the user has watched on this entry.
    pub watched_count: i64,
    /// `None` while MAL does not know the count (unannounced or weird shows).
    pub total_episodes: Option<i64>,
    /// Absolute-numbering offset for this entry: 12 when releases of season 2
    /// are numbered from 13. `0` means releases are numbered per season.
    pub episode_offset: i64,
    pub show_type: ShowType,
    /// Finished airing: a batch counts for a finished show, not an airing one.
    pub fully_aired: bool,
}

/// The franchise ordered first season → last, built from MAL's sequel and
/// prequel relations.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SeasonChain {
    pub entries: Vec<SeasonEntry>,
}

/// Per-show switches the walk consults (from the rules table).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Overrides {
    /// Per-show override of the Plan to watch switch.
    pub auto_plan_to_watch: Option<bool>,
}

/// What the list wants next.
#[derive(Debug, Clone, PartialEq)]
pub struct Wanted {
    pub mal_id: i64,
    pub title: String,
    /// 1-based position in the chain (season 1 → 1).
    pub season: i64,
    /// `None` for a movie or another feature without episode numbers.
    pub episode: Option<f64>,
}

/// The answer to "what does this chain want?".
#[derive(Debug, Clone, PartialEq)]
pub enum Want {
    /// An episode to fetch.
    Some(Wanted),
    /// Nothing to do, with the reason for the Activity log.
    Skip {
        reason: String,
        /// Watched everything: the app suggests marking the entry Completed.
        fully_watched: bool,
    },
}

/// What to do with one parsed release.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// This release delivers the wanted episode of the right season.
    Download { season: i64, episode: f64 },
    /// Right show, wrong episode or state: keep looking.
    Skip { reason: String },
    /// The numbers do not fit the chain; the user must decide.
    Ask { reason: String },
    /// No release exists for the wanted season, and another season is never
    /// substituted for it.
    NotFound { reason: String },
}

impl SeasonChain {
    pub fn new(entries: Vec<SeasonEntry>) -> Self {
        SeasonChain { entries }
    }

    /// The chain, ordered, as long as the entries are given in order.
    pub fn len(&self) -> i64 {
        self.entries.len() as i64
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entry(&self, mal_id: i64) -> Option<&SeasonEntry> {
        self.entries.iter().find(|e| e.mal_id == mal_id)
    }

    /// 1-based season number of an entry, or `None` when it is not ours.
    pub fn season_of(&self, mal_id: i64) -> Option<i64> {
        self.entries
            .iter()
            .position(|e| e.mal_id == mal_id)
            .map(|i| i as i64 + 1)
    }

    /// The entry at a 1-based season number.
    pub fn by_season(&self, season: i64) -> Option<&SeasonEntry> {
        if season < 1 || season as usize > self.entries.len() {
            return None;
        }
        self.entries.get(season as usize - 1)
    }

    /// Cumulative episode count of every entry *before* this season, or `None`
    /// when any earlier entry's count is unknown (absolute numbering is then
    /// unanswerable and the app asks instead of guessing).
    pub(crate) fn episodes_before(&self, season: i64) -> Option<i64> {
        let mut total = 0;
        for entry in &self.entries[..(season.max(1) - 1).max(0) as usize] {
            total += entry.total_episodes?;
        }
        Some(total)
    }
}
