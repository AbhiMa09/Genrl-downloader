//! Response cache in SQLite (`http_cache`).
//!
//! Keeps the app working when MAL, Jikan or Nyaa is down: stale-while-revalidate
//! with optional ETags, so MAL sync can send `If-None-Match` and get a cheap 304.

use rusqlite::OptionalExtension;

use crate::db::{now_unix, Db};
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub body: Vec<u8>,
    pub etag: Option<String>,
    pub fetched_at: i64,
    pub ttl: i64,
}

impl CacheEntry {
    pub fn is_fresh(&self, now: i64) -> bool {
        now < self.fetched_at + self.ttl
    }

    pub fn age(&self, now: i64) -> i64 {
        (now - self.fetched_at).max(0)
    }
}

/// Look up a cached response regardless of freshness. `None` when missing or
/// already deleted as expired.
pub fn get(db: &Db, service: &str, key: &str) -> Result<Option<CacheEntry>> {
    db.with(|c| {
        let row = c
            .query_row(
                "SELECT body, etag, fetched_at, ttl FROM http_cache
                  WHERE service = ?1 AND key = ?2",
                rusqlite::params![service, key],
                |r| {
                    Ok(CacheEntry {
                        body: r.get(0)?,
                        etag: r.get(1)?,
                        fetched_at: r.get(2)?,
                        ttl: r.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    })
}

/// Store a response.
pub fn put(
    db: &Db,
    service: &str,
    key: &str,
    body: &[u8],
    etag: Option<&str>,
    ttl_secs: u64,
) -> Result<()> {
    db.with(|c| {
        c.execute(
            "INSERT INTO http_cache (service, key, body, etag, fetched_at, ttl)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(service, key) DO UPDATE SET
                 body = excluded.body,
                 etag = excluded.etag,
                 fetched_at = excluded.fetched_at,
                 ttl = excluded.ttl",
            rusqlite::params![service, key, body, etag, now_unix(), ttl_secs as i64],
        )?;
        Ok(())
    })
}

/// Refresh the clock after a `304 Not Modified` without re-downloading the body.
pub fn touch(db: &Db, service: &str, key: &str, ttl_secs: u64) -> Result<()> {
    db.with(|c| {
        c.execute(
            "UPDATE http_cache SET fetched_at = ?3, ttl = ?4 WHERE service = ?1 AND key = ?2",
            rusqlite::params![service, key, now_unix(), ttl_secs as i64],
        )?;
        Ok(())
    })
}

/// Keep only the newest `keep` entries: the cache is capped by size, oldest first.
pub fn prune(db: &Db, keep: u64) -> Result<usize> {
    db.with(|c| {
        c.execute(
            "DELETE FROM http_cache WHERE (service, key) NOT IN (
                 SELECT service, key FROM http_cache ORDER BY fetched_at DESC LIMIT ?1
             )",
            [keep as i64],
        )?;
        Ok(c.changes() as usize)
    })
}

/// Drop expired rows (called on startup and periodically).
pub fn purge_expired(db: &Db) -> Result<usize> {
    db.with(|c| {
        c.execute("DELETE FROM http_cache WHERE fetched_at + ttl < ?1", [now_unix()])?;
        Ok(c.changes() as usize)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_reads_back() {
        let db = Db::open_in_memory().unwrap();
        put(&db, "jikan", "/anime/1", b"{}", Some("\"abc\""), 60).unwrap();
        let entry = get(&db, "jikan", "/anime/1").unwrap().unwrap();
        assert_eq!(entry.body, b"{}");
        assert_eq!(entry.etag.as_deref(), Some("\"abc\""));
        assert!(entry.is_fresh(now_unix()));
    }

    #[test]
    fn missing_entries_are_none() {
        let db = Db::open_in_memory().unwrap();
        assert!(get(&db, "jikan", "/nope").unwrap().is_none());
    }

    #[test]
    fn touch_refreshes_without_changing_the_body() {
        let db = Db::open_in_memory().unwrap();
        put(&db, "mal", "/me/list", b"body", None, 0).unwrap();
        touch(&db, "mal", "/me/list", 1800).unwrap();
        let entry = get(&db, "mal", "/me/list").unwrap().unwrap();
        assert_eq!(entry.body, b"body");
        assert!(entry.is_fresh(now_unix()), "304 refresh extends freshness");
    }

    #[test]
    fn prune_keeps_newest() {
        let db = Db::open_in_memory().unwrap();
        for i in 0..10 {
            put(&db, "nyaa", &format!("q{i}"), b"x", None, 600).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let removed = prune(&db, 3).unwrap();
        assert_eq!(removed, 7);
    }
}
