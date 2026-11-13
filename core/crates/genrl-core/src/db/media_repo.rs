use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::Result;
use crate::indexer::types::MediaKind;

/// Full record stored in SQLite `media` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaRecord {
    pub id: i64,
    pub kind: MediaKind,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<u32>,
    pub tmdb_id: Option<u32>,
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<u32>,
    pub poster_path: Option<String>,
    pub status: Option<String>,
    pub total_seasons: Option<u32>,
    pub total_episodes: Option<u32>,
    pub raw_json: Option<String>,
    pub created_at: i64,
    pub updated_at: Option<i64>,
}

/// Parameters for creating or updating a media record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpsertMedia {
    pub kind: MediaKind,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<u32>,
    pub tmdb_id: Option<u32>,
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<u32>,
    pub poster_path: Option<String>,
    pub status: Option<String>,
    pub total_seasons: Option<u32>,
    pub total_episodes: Option<u32>,
    pub raw_json: Option<String>,
}

impl UpsertMedia {
    pub fn new(kind: MediaKind, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            ..Default::default()
        }
    }
}

/// Upsert a media record keyed on `(kind, tmdb_id)`.
/// Returns the record's primary key ID.
pub fn upsert(db: &Db, item: &UpsertMedia, now: i64) -> Result<i64> {
    db.with(|c| {
        c.execute(
            "INSERT INTO media (
                kind, title, original_title, year, tmdb_id, imdb_id, tvdb_id,
                poster_path, status, total_seasons, total_episodes, raw_json,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
            ON CONFLICT(kind, tmdb_id) DO UPDATE SET
                title = excluded.title,
                original_title = COALESCE(excluded.original_title, media.original_title),
                year = COALESCE(excluded.year, media.year),
                imdb_id = COALESCE(excluded.imdb_id, media.imdb_id),
                tvdb_id = COALESCE(excluded.tvdb_id, media.tvdb_id),
                poster_path = COALESCE(excluded.poster_path, media.poster_path),
                status = COALESCE(excluded.status, media.status),
                total_seasons = COALESCE(excluded.total_seasons, media.total_seasons),
                total_episodes = COALESCE(excluded.total_episodes, media.total_episodes),
                raw_json = COALESCE(excluded.raw_json, media.raw_json),
                updated_at = excluded.updated_at",
            params![
                item.kind.as_str(),
                item.title,
                item.original_title,
                item.year,
                item.tmdb_id,
                item.imdb_id,
                item.tvdb_id,
                item.poster_path,
                item.status,
                item.total_seasons,
                item.total_episodes,
                item.raw_json,
                now,
            ],
        )?;

        let id: i64 = if let Some(tmdb_id) = item.tmdb_id {
            c.query_row(
                "SELECT id FROM media WHERE kind = ?1 AND tmdb_id = ?2",
                params![item.kind.as_str(), tmdb_id],
                |r| r.get(0),
            )?
        } else {
            c.last_insert_rowid()
        };

        Ok(id)
    })
}

/// Find a media record by its database ID.
pub fn find_by_id(db: &Db, id: i64) -> Result<Option<MediaRecord>> {
    db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT id, kind, title, original_title, year, tmdb_id, imdb_id, tvdb_id,
                    poster_path, status, total_seasons, total_episodes, raw_json,
                    created_at, updated_at
             FROM media WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_media(row)?))
        } else {
            Ok(None)
        }
    })
}

/// Find a media record by media kind and TMDB ID.
pub fn find_by_tmdb_id(db: &Db, kind: MediaKind, tmdb_id: u32) -> Result<Option<MediaRecord>> {
    db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT id, kind, title, original_title, year, tmdb_id, imdb_id, tvdb_id,
                    poster_path, status, total_seasons, total_episodes, raw_json,
                    created_at, updated_at
             FROM media WHERE kind = ?1 AND tmdb_id = ?2",
        )?;
        let mut rows = stmt.query(params![kind.as_str(), tmdb_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_media(row)?))
        } else {
            Ok(None)
        }
    })
}

/// List all media records of a specific kind, newest first.
pub fn list_by_kind(db: &Db, kind: MediaKind) -> Result<Vec<MediaRecord>> {
    db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT id, kind, title, original_title, year, tmdb_id, imdb_id, tvdb_id,
                    poster_path, status, total_seasons, total_episodes, raw_json,
                    created_at, updated_at
             FROM media WHERE kind = ?1 ORDER BY id DESC",
        )?;
        let rows = stmt.query_map(params![kind.as_str()], row_to_media)?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    })
}

/// Remove a media record by ID.
pub fn delete(db: &Db, id: i64) -> Result<bool> {
    db.with(|c| {
        let count = c.execute("DELETE FROM media WHERE id = ?1", params![id])?;
        Ok(count > 0)
    })
}

fn row_to_media(row: &rusqlite::Row<'_>) -> rusqlite::Result<MediaRecord> {
    let kind_str: String = row.get(1)?;
    let kind = MediaKind::parse(&kind_str).unwrap_or(MediaKind::Anime);
    Ok(MediaRecord {
        id: row.get(0)?,
        kind,
        title: row.get(2)?,
        original_title: row.get(3)?,
        year: row.get(4)?,
        tmdb_id: row.get(5)?,
        imdb_id: row.get(6)?,
        tvdb_id: row.get(7)?,
        poster_path: row.get(8)?,
        status: row.get(9)?,
        total_seasons: row.get(10)?,
        total_episodes: row.get(11)?,
        raw_json: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_repo_upsert_idempotent() {
        let db = Db::open_in_memory().unwrap();
        let now = 1700000000;

        let mut item = UpsertMedia::new(MediaKind::Movie, "Dune: Part Two");
        item.tmdb_id = Some(693134);
        item.imdb_id = Some("tt15239678".to_string());
        item.year = Some(2024);

        let id1 = upsert(&db, &item, now).expect("first upsert");
        let found = find_by_id(&db, id1).expect("find").expect("present");
        assert_eq!(found.title, "Dune: Part Two");
        assert_eq!(found.tmdb_id, Some(693134));
        assert_eq!(found.imdb_id.as_deref(), Some("tt15239678"));
        assert_eq!(found.year, Some(2024));

        // Update title and add poster_path on second upsert
        item.title = "Dune: Part Two (Updated)".to_string();
        item.poster_path = Some("/path/to/poster.jpg".to_string());
        let id2 = upsert(&db, &item, now + 100).expect("second upsert");
        assert_eq!(id1, id2, "id must be identical on conflict update");

        let updated = find_by_id(&db, id2).expect("find").expect("present");
        assert_eq!(updated.title, "Dune: Part Two (Updated)");
        assert_eq!(updated.poster_path.as_deref(), Some("/path/to/poster.jpg"));
        assert_eq!(updated.imdb_id.as_deref(), Some("tt15239678")); // preserved
        assert_eq!(updated.created_at, now);
        assert_eq!(updated.updated_at, Some(now + 100));

        let list = list_by_kind(&db, MediaKind::Movie).expect("list");
        assert_eq!(list.len(), 1);

        assert!(delete(&db, id1).expect("delete"));
        assert!(find_by_id(&db, id1).expect("find").is_none());
    }
}
