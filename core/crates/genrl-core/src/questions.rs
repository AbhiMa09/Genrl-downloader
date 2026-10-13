//! Question objects: the Inbox.
//!
//! "Never guess silently": when the title match or the torrent choice is
//! uncertain, the app queues a question *instead of* downloading something
//! wrong. Questions never block other jobs — each one sits in the Inbox until
//! answered, and unanswered ones expire back to `missing` so a show cannot
//! silently hang forever.
//!
//! Each kind carries a typed payload (what the card shows) and an answer that
//! names a choice plus the "always do this" flag from the checkbox, which the
//! caller turns into a saved alias or rule — the mitigation for a wrong answer
//! repeating forever.

use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::Result;
use crate::models::{Question, QuestionKind};

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
    EntryChanged { mal_id: i64, title: String, detail: String },
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

/// Create, answer, expire and list the Inbox.
pub struct QuestionStore;

impl QuestionStore {
    /// Queue a question. The caller supplies `now` and the expiry, so the
    /// function stays pure time-wise and the tests never wait.
    pub fn ask(
        db: &Db,
        job_id: Option<i64>,
        mal_id: Option<i64>,
        payload: &Payload,
        now: i64,
        expires_in: Option<i64>,
    ) -> Result<i64> {
        let kind = payload.kind();
        let body = serde_json::to_string(payload)?;
        let id = db.with(|c| {
            c.execute(
                "INSERT INTO questions (job_id, mal_id, kind, payload, created_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    job_id,
                    mal_id,
                    kind.as_str(),
                    body,
                    now,
                    expires_in.map(|secs| now + secs),
                ],
            )?;
            Ok(c.last_insert_rowid())
        })?;
        Ok(id)
    }

    /// Every still-open question, oldest first (the Inbox reads in order).
    pub fn open(db: &Db, now: i64) -> Result<Vec<OpenQuestion>> {
        Self::query(db, "answer IS NULL AND (expires_at IS NULL OR expires_at > ?1)", [now])
    }

    /// Open questions for one show.
    pub fn for_show(db: &Db, mal_id: i64, now: i64) -> Result<Vec<OpenQuestion>> {
        Self::query(
            db,
            "answer IS NULL AND (expires_at IS NULL OR expires_at > ?1) AND mal_id = ?2",
            rusqlite::params![now, mal_id],
        )
    }

    /// Open questions of one kind.
    pub fn of_kind(db: &Db, kind: QuestionKind, now: i64) -> Result<Vec<OpenQuestion>> {
        Self::query(
            db,
            "answer IS NULL AND (expires_at IS NULL OR expires_at > ?1) AND kind = ?2",
            rusqlite::params![now, kind.as_str()],
        )
    }

    /// Record an answer. Answers overwrite, so re-answering is harmless.
    pub fn answer(db: &Db, id: i64, answer: &Answer) -> Result<()> {
        let answer = serde_json::to_string(answer)?;
        db.with(|c| {
            c.execute(
                "UPDATE questions SET answer = ?1 WHERE id = ?2",
                rusqlite::params![answer, id],
            )?;
            Ok(())
        })
    }

    /// One question by id, with its payload parsed.
    ///
    /// Applying an answer ([`crate::worker::apply_answer`]) needs the stored
    /// row — the Inbox answers by id, and the tests drive the same path.
    pub fn get(db: &Db, id: i64) -> Result<Option<Question>> {
        db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, job_id, mal_id, kind, payload, answer, created_at, expires_at
                 FROM questions WHERE id = ?1",
            )?;
            let mut rows = stmt.query_map([id], |r| {
                Ok(Question {
                    id: r.get(0)?,
                    job_id: r.get(1)?,
                    mal_id: r.get(2)?,
                    kind: QuestionKind::parse(r.get::<_, String>(3)?.as_str()),
                    payload: serde_json::from_str(r.get::<_, String>(4)?.as_str())
                        .unwrap_or(serde_json::Value::Null),
                    answer: r.get(5)?,
                    created_at: r.get(6)?,
                    expires_at: r.get(7)?,
                })
            })?;
            match rows.next().transpose()? {
                Some(question) => Ok(Some(question)),
                None => Ok(None),
            }
        })
    }

    /// Expire unanswered questions past their deadline and return them, so
    /// the caller can return the episodes to `missing`.
    pub fn expire(db: &Db, now: i64) -> Result<Vec<Question>> {
        let expired = db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT id, job_id, mal_id, kind, payload, answer, created_at, expires_at
                 FROM questions
                 WHERE answer IS NULL AND expires_at IS NOT NULL AND expires_at <= ?1",
            )?;
            let rows = stmt.query_map([now], |r| {
                Ok(Question {
                    id: r.get(0)?,
                    job_id: r.get(1)?,
                    mal_id: r.get(2)?,
                    kind: QuestionKind::parse(r.get::<_, String>(3)?.as_str()),
                    payload: serde_json::from_str(r.get::<_, String>(4)?.as_str())
                        .unwrap_or(serde_json::Value::Null),
                    answer: None,
                    created_at: r.get(6)?,
                    expires_at: r.get(7)?,
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })?;

        if !expired.is_empty() {
            let expired_answer = serde_json::to_string(&Answer {
                choice: Choice::Expired,
                always: false,
            })?;
            db.with(|c| {
                let tx = c.unchecked_transaction()?;
                for question in &expired {
                    tx.execute(
                        "UPDATE questions SET answer = ?1 WHERE id = ?2",
                        rusqlite::params![expired_answer, question.id],
                    )?;
                }
                tx.commit()?;
                Ok(())
            })?;
        }
        Ok(expired)
    }

    /// Remove a question outright (undo of a "never ask this again" answer).
    pub fn delete(db: &Db, id: i64) -> Result<bool> {
        db.with(|c| {
            let changed = c.execute("DELETE FROM questions WHERE id = ?1", [id])?;
            Ok(changed > 0)
        })
    }

    /// How many are waiting, for the Inbox badge.
    pub fn open_count(db: &Db, now: i64) -> Result<u64> {
        db.with(|c| {
            let count: i64 = c.query_row(
                "SELECT COUNT(*) FROM questions
                 WHERE answer IS NULL AND (expires_at IS NULL OR expires_at > ?1)",
                [now],
                |r| r.get(0),
            )?;
            Ok(count as u64)
        })
    }

    fn query(db: &Db, where_clause: &str, args: impl rusqlite::Params) -> Result<Vec<OpenQuestion>> {
        let sql = format!(
            "SELECT id, job_id, mal_id, payload, created_at, expires_at
             FROM questions WHERE {where_clause} ORDER BY id"
        );
        db.with(|c| {
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map(args, |r| {
                let payload: String = r.get(3)?;
                Ok(OpenQuestion {
                    id: r.get(0)?,
                    job_id: r.get(1)?,
                    mal_id: r.get(2)?,
                    payload: serde_json::from_str(&payload)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        ))?,
                    created_at: r.get(4)?,
                    expires_at: r.get(5)?,
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Db {
        Db::open_in_memory().unwrap()
    }

    fn now() -> i64 {
        1_789_227_900
    }

    fn review_payload() -> Payload {
        Payload::MatchReview {
            expected: ExpectedShow {
                mal_id: 42,
                title: "Kimi no Na wa".to_string(),
                english_title: Some("Your Name".to_string()),
                year: Some(2016),
            },
            options: vec![MatchOption {
                info_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                name: "[SubsPlease] Kimi no Na wa (2016) [BD1080p]".to_string(),
                magnet: "magnet:?xt=urn:btih:aaaa".to_string(),
                group: Some("SubsPlease".to_string()),
                resolution: Some("1080p".to_string()),
                seeders: 53,
                size_bytes: 1_600_000_000,
                signal: 90.0,
                ranking: 110.0,
                title_ratio: 1.0,
                why: vec!["title matches \"kimi no na wa\" (1.00)".to_string()],
            }],
            reason: "runner-up is only 12 points behind".to_string(),
        }
    }

    #[test]
    fn a_question_opens_gets_answered_and_stops_showing() {
        let db = db();
        db.with(|c| {
            c.execute(
                "INSERT INTO shows (mal_id, title, type, status) VALUES (42, 'Kimi no Na wa', 'movie', 'finished')",
                [],
            )?;
            Ok(())
        })
        .unwrap();
        let id = QuestionStore::ask(&db, None, Some(42), &review_payload(), now(), Some(72 * 3600))
            .unwrap();

        let open = QuestionStore::open(&db, now()).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].kind(), QuestionKind::MatchReview);
        match &open[0].payload {
            Payload::MatchReview { expected, options, .. } => {
                assert_eq!(expected.title, "Kimi no Na wa");
                assert_eq!(options.len(), 1);
                assert_eq!(options[0].group.as_deref(), Some("SubsPlease"));
            }
            other => panic!("wrong payload: {other:?}"),
        }
        assert_eq!(QuestionStore::open_count(&db, now()).unwrap(), 1);
        assert_eq!(QuestionStore::for_show(&db, 42, now()).unwrap().len(), 1);
        assert!(QuestionStore::for_show(&db, 43, now()).unwrap().is_empty());

        // Answer with the "always do this" box ticked.
        QuestionStore::answer(
            &db,
            id,
            &Answer {
                choice: Choice::UseThis {
                    info_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                },
                always: true,
            },
        )
        .unwrap();

        assert!(QuestionStore::open(&db, now()).unwrap().is_empty());
        assert_eq!(QuestionStore::open_count(&db, now()).unwrap(), 0);
    }

    #[test]
    fn unanswered_questions_expire_and_return_for_cleanup() {
        let db = db();
        // A real job id, so the question hangs off a queue row like in life.
        db.with(|c| {
            c.execute(
                "INSERT INTO shows (mal_id, title, type, status) VALUES (99, 'Show', 'tv', 'airing')",
                [],
            )?;
            c.execute(
                "INSERT INTO episodes (id, mal_id, number) VALUES (5, 99, 4)",
                [],
            )?;
            c.execute(
                "INSERT INTO jobs (id, episode_id, kind, state, created_at, updated_at)
                 VALUES (7, 5, 'search', 'needs_input', 0, 0)",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let payload = Payload::LowSeeds {
            seeders: 4,
            release: "[Group] Show - 05 (1080p)".to_string(),
            magnet: "magnet:?xt=urn:btih:bbbb".to_string(),
            auto_list: false,
        };
        let id = QuestionStore::ask(&db, Some(7), Some(99), &payload, now(), Some(3600)).unwrap();

        assert!(QuestionStore::expire(&db, now()).unwrap().is_empty(), "not yet");
        let expired = QuestionStore::expire(&db, now() + 3601).unwrap();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].id, id);
        assert_eq!(expired[0].job_id, Some(7));
        assert_eq!(expired[0].mal_id, Some(99), "the caller returns the episode to missing");

        // Expiring marks it answered, so it stops showing in the Inbox.
        assert!(QuestionStore::open(&db, now() + 3601).unwrap().is_empty());
        // And deleting is the undo behind "never ask me this again".
        assert!(QuestionStore::delete(&db, id).unwrap());
        assert!(!QuestionStore::delete(&db, id).unwrap());
    }

    #[test]
    fn payloads_round_trip_through_the_database() {
        let db = db();
        let payloads = vec![
            review_payload(),
            Payload::FileExists {
                episode: 5.0,
                existing: "/library/Show - S01E05.mkv".to_string(),
                candidate: "[Group] Show - 05 (1080p) v2".to_string(),
            },
            Payload::BatchOnly {
                release: "[Judas] Show [Batch]".to_string(),
                seeders: 12,
            },
            Payload::DiskSpace {
                free_gb: 3,
                floor_gb: 20,
            },
            Payload::AllFailed { candidates_tried: 3 },
            Payload::EntryChanged {
                mal_id: 5,
                title: "Show".to_string(),
                detail: "MAL merged the entry into another id".to_string(),
            },
        ];
        for (index, payload) in payloads.iter().enumerate() {
            let id =
                QuestionStore::ask(&db, None, None, payload, now(), None).unwrap();
            let open = QuestionStore::open(&db, now()).unwrap();
            let stored = open
                .iter()
                .find(|q| q.id == id)
                .expect("just asked");
            assert_eq!(&stored.payload, payload, "payload {index} survived the round trip");
        }
    }

    #[test]
    fn kinds_are_listable_for_the_inbox_filter_tabs() {
        let db = db();
        QuestionStore::ask(
            &db,
            None,
            None,
            &Payload::LowSeeds {
                seeders: 4,
                release: "r".to_string(),
                magnet: "m".to_string(),
                auto_list: true,
            },
            now(),
            None,
        )
        .unwrap();
        assert_eq!(QuestionStore::of_kind(&db, QuestionKind::LowSeeds, now()).unwrap().len(), 1);
        assert!(QuestionStore::of_kind(&db, QuestionKind::FileExists, now()).unwrap().is_empty());
    }
}
