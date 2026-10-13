//! The activity log.
//!
//! Every job transition and every question is written here with the show, job
//! and reason, so you can always see why the app chose a release. The table is
//! capped by size and pruned oldest-first.

use serde::{Deserialize, Serialize};

use crate::db::{now_unix, Db};
use crate::error::Result;
use crate::models::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "info" | "debug" => Some(Level::Info),
            "warn" | "warning" => Some(Level::Warn),
            "error" => Some(Level::Error),
            _ => None,
        }
    }
}

/// Filter for the Activity page.
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub level: Option<Level>,
    pub job_id: Option<i64>,
    pub mal_id: Option<i64>,
    /// Inclusive lower bound (Unix seconds).
    pub since: Option<i64>,
    /// Inclusive upper bound (Unix seconds).
    pub until: Option<i64>,
    /// Free-text search over the message.
    pub search: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Clone)]
pub struct EventLog {
    db: Db,
}

impl EventLog {
    pub fn new(db: Db) -> Self {
        EventLog { db }
    }

    /// Append one event. Control characters and newlines are stripped so a
    /// hostile title cannot inject extra lines into the log or the UI.
    pub fn record(
        &self,
        level: Level,
        job_id: Option<i64>,
        mal_id: Option<i64>,
        message: impl AsRef<str>,
    ) -> Result<i64> {
        self.record_at(level, job_id, mal_id, message, now_unix())
    }

    /// Append one event at a caller-supplied time (the queue injects its clock).
    pub fn record_at(
        &self,
        level: Level,
        job_id: Option<i64>,
        mal_id: Option<i64>,
        message: impl AsRef<str>,
        at: i64,
    ) -> Result<i64> {
        let id = self.db.with(|c| record_in(c, level, job_id, mal_id, message.as_ref(), at))?;

        // Keep the log bounded without hammering the table on every write.
        if id % 500 == 0 {
            if let Ok(keep) = crate::Settings::load(&self.db) {
                let _ = self.prune(keep.events_keep_rows);
            }
        }
        tracing::info!(
            target: "genrl_activity",
            job_id,
            mal_id,
            level = level.as_str(),
            "{}",
            sanitize(message.as_ref())
        );
        Ok(id)
    }

    pub fn info(&self, job_id: Option<i64>, mal_id: Option<i64>, message: impl AsRef<str>) -> Result<i64> {
        self.record(Level::Info, job_id, mal_id, message)
    }

    pub fn warn(&self, job_id: Option<i64>, mal_id: Option<i64>, message: impl AsRef<str>) -> Result<i64> {
        self.record(Level::Warn, job_id, mal_id, message)
    }

    pub fn error(&self, job_id: Option<i64>, mal_id: Option<i64>, message: impl AsRef<str>) -> Result<i64> {
        self.record(Level::Error, job_id, mal_id, message)
    }

    /// Query events for the Activity page.
    pub fn query(&self, filter: &EventFilter) -> Result<Vec<Event>> {
        let mut sql = String::from(
            "SELECT id, at, level, job_id, mal_id, message FROM events WHERE 1 = 1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(level) = filter.level {
            sql.push_str(" AND level = ?");
            params.push(Box::new(level.as_str().to_string()));
        }
        if let Some(job_id) = filter.job_id {
            sql.push_str(" AND job_id = ?");
            params.push(Box::new(job_id));
        }
        if let Some(mal_id) = filter.mal_id {
            sql.push_str(" AND mal_id = ?");
            params.push(Box::new(mal_id));
        }
        if let Some(since) = filter.since {
            sql.push_str(" AND at >= ?");
            params.push(Box::new(since));
        }
        if let Some(until) = filter.until {
            sql.push_str(" AND at <= ?");
            params.push(Box::new(until));
        }
        if let Some(search) = &filter.search {
            sql.push_str(" AND message LIKE ? ESCAPE '\\'");
            params.push(Box::new(format!("%{}%", escape_like(search))));
        }
        sql.push_str(" ORDER BY at DESC, id DESC LIMIT ?");
        let limit = filter.limit.unwrap_or(200);
        params.push(Box::new(limit));

        self.db.with(|c| {
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                |r| {
                    Ok(Event {
                        id: r.get(0)?,
                        at: r.get(1)?,
                        level: Level::parse(&r.get::<_, String>(2)?).unwrap_or(Level::Info),
                        job_id: r.get(3)?,
                        mal_id: r.get(4)?,
                        message: r.get(5)?,
                    })
                },
            )?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    /// The most recent events, newest first.
    pub fn recent(&self, limit: u32) -> Result<Vec<Event>> {
        self.query(&EventFilter {
            limit: Some(limit),
            ..Default::default()
        })
    }

    /// Delete events older than `before` (Unix seconds). "Clear old entries".
    pub fn clear_before(&self, before: i64) -> Result<usize> {
        Ok(self.db.with(|c| {
            c.execute("DELETE FROM events WHERE at < ?1", [before])?;
            Ok(c.changes() as usize)
        })?)
    }

    /// Keep only the newest `keep` rows. Size cap, pruning oldest first.
    pub fn prune(&self, keep: u64) -> Result<usize> {
        Ok(self.db.with(|c| {
            c.execute(
                "DELETE FROM events WHERE id NOT IN (
                     SELECT id FROM events ORDER BY at DESC, id DESC LIMIT ?1
                 )",
                [keep as i64],
            )?;
            Ok(c.changes() as usize)
        })?)
    }

    pub fn count(&self) -> Result<u64> {
        self.db.with(|c| {
            let n: i64 = c.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?;
            Ok(n as u64)
        })
    }
}

/// One line per event: no newlines, no control or bidi characters.
/// Insert one event row on an existing connection.
///
/// The queue calls this *inside* its transition transaction, so the state
/// change and its Activity-log line land together or not at all. Kept `pub`
/// for that reason only.
pub fn record_in(
    conn: &rusqlite::Connection,
    level: Level,
    job_id: Option<i64>,
    mal_id: Option<i64>,
    message: &str,
    at: i64,
) -> Result<i64> {
    let message = sanitize(message);
    conn.execute(
        "INSERT INTO events (at, level, job_id, mal_id, message) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![at, level.as_str(), job_id, mal_id, message],
    )?;
    Ok(conn.last_insert_rowid())
}

fn sanitize(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    for ch in message.chars() {
        if ch == '\n' || ch == '\r' || ch == '\t' {
            out.push(' ');
        } else if ch.is_control() || ch.is_whitespace() && out.ends_with(' ') {
            // collapse runs of whitespace
        } else {
            out.push(ch);
        }
    }
    out.trim().to_string()
}

fn escape_like(s: &str) -> String {
    // Backslash first, or the escapes below would double-escape themselves.
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_reads_back() {
        let db = Db::open_in_memory().unwrap();
        let log = EventLog::new(db);
        log.info(Some(1), Some(21), "Chose [SubsPlease] 1080p for episode 7")
            .unwrap();
        log.warn(None, None, "nyaa circuit breaker open").unwrap();

        let events = log.recent(10).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "nyaa circuit breaker open", "newest first");
        assert_eq!(events[1].job_id, Some(1));
        assert_eq!(events[1].level, Level::Info);
    }

    #[test]
    fn strips_newlines_from_messages() {
        let db = Db::open_in_memory().unwrap();
        let log = EventLog::new(db);
        log.info(None, None, "Show\nName; rm -rf ~\tfooter").unwrap();
        let events = log.recent(1).unwrap();
        assert!(!events[0].message.contains('\n'));
        assert!(!events[0].message.contains('\t'));
        assert_eq!(events[0].message.matches("rm -rf").count(), 1);
    }

    #[test]
    fn filters_by_level_and_text() {
        let db = Db::open_in_memory().unwrap();
        let log = EventLog::new(db.clone());
        log.info(Some(5), None, "job started").unwrap();
        log.error(Some(6), None, "download failed").unwrap();

        let warns = log
            .query(&EventFilter {
                level: Some(Level::Error),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(warns.len(), 1);
        assert_eq!(warns[0].job_id, Some(6));

        let found = log
            .query(&EventFilter {
                search: Some("started".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn prunes_oldest_first() {
        let db = Db::open_in_memory().unwrap();
        let log = EventLog::new(db);
        for i in 0..10 {
            log.info(None, None, format!("event {i}")).unwrap();
        }
        let removed = log.prune(3).unwrap();
        assert_eq!(removed, 7);
        assert_eq!(log.count().unwrap(), 3);
    }

    #[test]
    fn clear_before_removes_old_entries() {
        let db = Db::open_in_memory().unwrap();
        let log = EventLog::new(db);
        log.info(None, None, "recent").unwrap();
        log.clear_before(now_unix() + 60).unwrap();
        assert_eq!(log.count().unwrap(), 0);
    }
}
