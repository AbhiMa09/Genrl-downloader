use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::matching::TargetInputs;
use crate::models::{AiringStatus, NameSource};
use crate::names::{NameSet, PRIORITY_MAIN};

use super::list::ListEntry;

/// Write a list entry's show row, keeping whatever the sync already knew.
pub fn upsert_show(db: &Db, entry: &ListEntry) -> Result<()> {
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, english_title, type, status, total_episodes,
                                start_year, auto, episode_offset, season)
             VALUES (?1, ?2, ?3, 'tv', 'finished', ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (mal_id) DO UPDATE SET
                title = excluded.title,
                english_title = COALESCE(excluded.english_title, shows.english_title),
                total_episodes = COALESCE(excluded.total_episodes, shows.total_episodes),
                start_year = COALESCE(excluded.start_year, shows.start_year),
                auto = excluded.auto,
                episode_offset = excluded.episode_offset,
                season = excluded.season",
            rusqlite::params![
                entry.mal_id,
                entry.title,
                entry.english_title,
                entry.total_episodes,
                entry.start_year,
                entry.auto as i64,
                entry.episode_offset,
                entry.season,
            ],
        )?;
        Ok(())
    })
}

/// A show's names: the entry's own names, main title first.
/// Non-destructive: if entry.names is empty, existing aliases are left untouched.
pub fn upsert_names(db: &Db, entry: &ListEntry) -> Result<()> {
    if entry.names.is_empty() {
        return db.with(|c| {
            let count: i64 = c.query_row(
                "SELECT COUNT(*) FROM show_names WHERE mal_id = ?1",
                [entry.mal_id],
                |r| r.get(0),
            )?;
            if count == 0 && !entry.title.is_empty() {
                let normalized = crate::names::normalize_alias(&entry.title);
                if !normalized.is_empty() {
                    c.execute(
                        "INSERT INTO show_names (mal_id, name, normalized, source, priority)
                         VALUES (?1, ?2, ?3, 'mal', ?4)
                         ON CONFLICT (mal_id, normalized) DO NOTHING",
                        rusqlite::params![entry.mal_id, entry.title, normalized, PRIORITY_MAIN],
                    )?;
                }
            }
            Ok(())
        });
    }

    let mut names: Vec<(String, NameSource, u8)> = Vec::new();
    for (index, name) in entry.names.iter().enumerate() {
        let priority = if index == 0 {
            PRIORITY_MAIN
        } else {
            index.min(u8::MAX as usize) as u8
        };
        names.push((name.clone(), NameSource::Mal, priority));
    }
    NameSet::replace_from_sync(db, entry.mal_id, &names)
}

/// The episode row, created once (`UNIQUE (mal_id, number)` makes a second
/// call a no-op, which is what makes the weekly planner safe to re-run).
pub fn ensure_episode(db: &Db, clock: &dyn Clock, mal_id: i64, number: f64) -> Result<i64> {
    let now = clock.now();
    db.with(|c| {
        let tx = c.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO episodes (mal_id, number, state, last_checked)
             VALUES (?1, ?2, 'missing', ?3)
             ON CONFLICT (mal_id, number) DO UPDATE SET last_checked = excluded.last_checked",
            rusqlite::params![mal_id, number, now],
        )?;
        // An DO UPDATE leaves last_insert_rowid() stale, so read the row back:
        // the conflict target is unique, so exactly one id answers.
        let id: i64 = tx.query_row(
            "SELECT id FROM episodes WHERE mal_id = ?1 AND number = ?2",
            rusqlite::params![mal_id, number],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(id)
    })
}

/// Mark an episode owned (the finalize step's outcome).
pub fn mark_done(db: &Db, episode_id: i64, file_path: &str, size: i64) -> Result<()> {
    db.with(|c| {
        c.execute(
            "UPDATE episodes SET state = 'done', file_path = ?2, size = ?3 WHERE id = ?1",
            rusqlite::params![episode_id, file_path, size],
        )?;
        Ok(())
    })
}

/// Mark an episode missing again (an expired question released it).
pub fn mark_missing(db: &Db, episode_id: i64) -> Result<()> {
    db.with(|c| {
        c.execute(
            "UPDATE episodes SET state = 'missing' WHERE id = ?1",
            [episode_id],
        )?;
        Ok(())
    })
}

/// Load a show's matching inputs (re-exported for the workers).
pub fn target_inputs(db: &Db, mal_id: i64) -> Result<TargetInputs> {
    TargetInputs::load(db, mal_id)
}

/// Airing status for a show the sync wrote.
pub fn airing_status_of(db: &Db, mal_id: i64) -> Result<AiringStatus> {
    db.with(|c| {
        Ok(c.query_row(
            "SELECT status FROM shows WHERE mal_id = ?1",
            [mal_id],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .map(|s| AiringStatus::parse(&s))
        .unwrap_or_default())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::models::ListStatus;
    use crate::planner::ListEntry;

    fn test_db() -> Db {
        let db = Db::open_in_memory().unwrap();
        db.with(|c| {
            c.execute(
                "INSERT INTO shows (mal_id, title, type, status) VALUES (100, 'Test Show', 'tv', 'airing')",
                [],
            )?;
            Ok(())
        })
        .unwrap();
        db
    }

    #[test]
    fn upsert_names_with_empty_list_does_not_shrink_existing_aliases() {
        let db = test_db();
        let mut entry = ListEntry {
            mal_id: 100,
            title: "Test Show".to_string(),
            english_title: Some("Test Show English".to_string()),
            list_status: ListStatus::Watching,
            watched_count: 0,
            total_episodes: Some(12),
            start_year: Some(2025),
            auto: true,
            episode_offset: 0,
            names: vec![
                "Test Show".to_string(),
                "Test Show English".to_string(),
                "Test Show Synonym".to_string(),
            ],
            franchise: 100,
            season: 1,
        };
        upsert_names(&db, &entry).unwrap();
        let set = crate::names::NameSet::load(&db, 100).unwrap();
        assert_eq!(set.entries.len(), 3);

        // Sync with empty names must not shrink existing aliases
        entry.names.clear();
        upsert_names(&db, &entry).unwrap();
        let set = crate::names::NameSet::load(&db, 100).unwrap();
        assert_eq!(set.entries.len(), 3);

        // Brand-new show with empty names seeds title
        db.with(|c| {
            c.execute(
                "INSERT INTO shows (mal_id, title, type, status) VALUES (101, 'Brand New', 'tv', 'airing')",
                [],
            )?;
            Ok(())
        })
        .unwrap();
        let new_entry = ListEntry {
            mal_id: 101,
            title: "Brand New".to_string(),
            english_title: None,
            list_status: ListStatus::Watching,
            watched_count: 0,
            total_episodes: Some(12),
            start_year: Some(2025),
            auto: true,
            episode_offset: 0,
            names: vec![],
            franchise: 101,
            season: 1,
        };
        upsert_names(&db, &new_entry).unwrap();
        let set2 = crate::names::NameSet::load(&db, 101).unwrap();
        assert_eq!(set2.entries.len(), 1);
        assert!(set2.contains("brand new"));
    }
}
