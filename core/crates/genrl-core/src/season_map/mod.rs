//! Community season numbering mapping.
//!
//! Maps community release season numbers (e.g. "Season 33") to entry-local episode
//! ranges for a show (e.g. absolute episodes 1050..1093 for Detective Conan / MAL 235).

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeasonMapSource {
    Bundled,
    User,
}

impl SeasonMapSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            SeasonMapSource::Bundled => "bundled",
            SeasonMapSource::User => "user",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "user" => SeasonMapSource::User,
            _ => SeasonMapSource::Bundled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeasonMapEntry {
    pub mal_id: i64,
    pub season_no: u32,
    pub first_ep: u32,
    pub last_ep: u32,
    pub title: Option<String>,
    pub year: Option<i64>,
    pub source: SeasonMapSource,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
struct SeedSeason {
    season_no: u32,
    first_ep: u32,
    last_ep: u32,
    title: Option<String>,
    year: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SeedShow {
    mal_id: i64,
    #[allow(dead_code)]
    title: String,
    #[allow(dead_code)]
    source_url: String,
    #[allow(dead_code)]
    retrieval_date: String,
    seasons: Vec<SeedSeason>,
}

pub const BUNDLED_SEED_JSON: &str = include_str!("../db/seeds/season_map.json");

/// Populate the `season_map` table with bundled seed data using `INSERT OR IGNORE`.
/// Existing rows (especially user overrides) are preserved intact.
pub fn seed_bundled(conn: &rusqlite::Connection, now: i64) -> Result<usize> {
    let raw = BUNDLED_SEED_JSON.trim_start_matches('\u{feff}');
    let seeds: Vec<SeedShow> = serde_json::from_str(raw)?;
    let mut inserted = 0;
    let mut stmt = conn.prepare(
        "INSERT OR IGNORE INTO season_map
         (mal_id, season_no, first_ep, last_ep, title, year, source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'bundled', ?7)",
    )?;

    for show in seeds {
        for season in show.seasons {
            let count = stmt.execute(rusqlite::params![
                show.mal_id,
                season.season_no,
                season.first_ep,
                season.last_ep,
                season.title,
                season.year,
                now,
            ])?;
            inserted += count;
        }
    }
    Ok(inserted)
}

/// User answers UPSERT with `source = 'user'` and always win.
#[allow(clippy::too_many_arguments)]
pub fn upsert_user(
    conn: &rusqlite::Connection,
    mal_id: i64,
    season_no: u32,
    first_ep: u32,
    last_ep: u32,
    title: Option<&str>,
    year: Option<i64>,
    now: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO season_map
         (mal_id, season_no, first_ep, last_ep, title, year, source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'user', ?7)
         ON CONFLICT (mal_id, season_no) DO UPDATE SET
             first_ep = excluded.first_ep,
             last_ep = excluded.last_ep,
             title = COALESCE(excluded.title, season_map.title),
             year = COALESCE(excluded.year, season_map.year),
             source = 'user',
             created_at = excluded.created_at",
        rusqlite::params![mal_id, season_no, first_ep, last_ep, title, year, now],
    )?;
    Ok(())
}

/// Lookup a specific season for a show in `season_map`.
pub fn lookup(
    conn: &rusqlite::Connection,
    mal_id: i64,
    season_no: u32,
) -> Result<Option<SeasonMapEntry>> {
    let mut stmt = conn.prepare(
        "SELECT mal_id, season_no, first_ep, last_ep, title, year, source, created_at
         FROM season_map
         WHERE mal_id = ?1 AND season_no = ?2",
    )?;
    let mut rows = stmt.query(rusqlite::params![mal_id, season_no])?;
    if let Some(row) = rows.next()? {
        let source_str: String = row.get(6)?;
        Ok(Some(SeasonMapEntry {
            mal_id: row.get(0)?,
            season_no: row.get(1)?,
            first_ep: row.get(2)?,
            last_ep: row.get(3)?,
            title: row.get(4)?,
            year: row.get(5)?,
            source: SeasonMapSource::parse(&source_str),
            created_at: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

/// Get the latest mapped season (highest season_no) for a show.
pub fn latest_season(conn: &rusqlite::Connection, mal_id: i64) -> Result<Option<SeasonMapEntry>> {
    let mut stmt = conn.prepare(
        "SELECT mal_id, season_no, first_ep, last_ep, title, year, source, created_at
         FROM season_map
         WHERE mal_id = ?1
         ORDER BY season_no DESC
         LIMIT 1",
    )?;
    let mut rows = stmt.query(rusqlite::params![mal_id])?;
    if let Some(row) = rows.next()? {
        let source_str: String = row.get(6)?;
        Ok(Some(SeasonMapEntry {
            mal_id: row.get(0)?,
            season_no: row.get(1)?,
            first_ep: row.get(2)?,
            last_ep: row.get(3)?,
            title: row.get(4)?,
            year: row.get(5)?,
            source: SeasonMapSource::parse(&source_str),
            created_at: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

/// Load all season mappings for a show, sorted by `season_no ASC`.
pub fn all_for_show(conn: &rusqlite::Connection, mal_id: i64) -> Result<Vec<SeasonMapEntry>> {
    let mut stmt = conn.prepare(
        "SELECT mal_id, season_no, first_ep, last_ep, title, year, source, created_at
         FROM season_map
         WHERE mal_id = ?1
         ORDER BY season_no ASC",
    )?;
    let rows = stmt.query_map(rusqlite::params![mal_id], |row| {
        let source_str: String = row.get(6)?;
        Ok(SeasonMapEntry {
            mal_id: row.get(0)?,
            season_no: row.get(1)?,
            first_ep: row.get(2)?,
            last_ep: row.get(3)?,
            title: row.get(4)?,
            year: row.get(5)?,
            source: SeasonMapSource::parse(&source_str),
            created_at: row.get(7)?,
        })
    })?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

/// Ensure missing episode rows exist in `episodes` table for all mapped ranges.
pub fn ensure_episodes_exist(conn: &rusqlite::Connection, mal_id: i64) -> Result<usize> {
    let entries = all_for_show(conn, mal_id)?;
    let mut created = 0;
    let mut stmt = conn.prepare(
        "INSERT OR IGNORE INTO episodes (mal_id, number, state)
         VALUES (?1, ?2, 'missing')",
    )?;
    for entry in entries {
        for ep in entry.first_ep..=entry.last_ep {
            created += stmt.execute(rusqlite::params![mal_id, ep as f64])?;
        }
    }
    Ok(created)
}

/// Suggest an episode range for an unmapped season based on previous mappings.
pub fn suggest_range(
    conn: &rusqlite::Connection,
    mal_id: i64,
    ep_count: u32,
) -> Result<(u32, u32)> {
    let max_last: Option<u32> = conn
        .query_row(
            "SELECT MAX(last_ep) FROM season_map WHERE mal_id = ?1",
            rusqlite::params![mal_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();

    let count = if ep_count == 0 { 24 } else { ep_count };
    let first = max_last.map(|m| m + 1).unwrap_or(1);
    let last = first + count - 1;
    Ok((first, last))
}
