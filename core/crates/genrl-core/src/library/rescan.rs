use std::fs;
use std::path::Path;

use rusqlite::OptionalExtension;

use crate::db::Db;
use crate::error::Result;

use super::formatting::is_video_file;
use super::types::{RescanReport, MARKER_FILE};

/// Rescan the library root directory: adopts existing folders and indexes episodes.
pub fn rescan_library(db: &Db, library_folder: &Path) -> Result<RescanReport> {
    if !library_folder.exists() {
        return Ok(RescanReport::default());
    }

    let mut shows_adopted = 0usize;
    let mut episodes_found = 0usize;

    let entries = fs::read_dir(library_folder)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let marker = path.join(MARKER_FILE);
        let mut mal_id: Option<i64> = None;

        if marker.exists() {
            if let Ok(content) = fs::read_to_string(&marker) {
                if let Ok(id) = content.trim().parse::<i64>() {
                    mal_id = Some(id);
                }
            }
        }

        // If no marker, try matching by directory name against show titles
        if mal_id.is_none() {
            if let Some(dirname) = path.file_name().and_then(|n| n.to_str()) {
                let normalized = crate::names::normalize_alias(dirname);
                let matched_id: Option<i64> = db.with(|c| {
                    Ok(c.query_row(
                        "SELECT s.mal_id FROM shows s
                         LEFT JOIN show_names sn ON sn.mal_id = s.mal_id
                         WHERE sn.normalized = ?1 OR lower(s.title) = lower(?2)
                         LIMIT 1",
                        rusqlite::params![normalized, dirname],
                        |r| r.get(0),
                    )
                    .optional()?)
                })?;
                if let Some(id) = matched_id {
                    let _ = fs::write(&marker, format!("{id}\n"));
                    mal_id = Some(id);
                }
            }
        }

        if let Some(id) = mal_id {
            // Adopt show folder
            db.with(|c| {
                c.execute(
                    "UPDATE shows SET folder_path = ?1 WHERE mal_id = ?2",
                    rusqlite::params![path.display().to_string(), id],
                )?;
                Ok(())
            })?;
            shows_adopted += 1;

            // Scan video files for episodes
            if let Ok(files) = fs::read_dir(&path) {
                for file_entry in files.flatten() {
                    let file_path = file_entry.path();
                    if !file_path.is_file() || !is_video_file(&file_path) {
                        continue;
                    }
                    if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                        let parsed = crate::filename::parse_release(filename);
                        if let Some(ep_num) = parsed.episode {
                            let size = fs::metadata(&file_path).map(|m| m.len() as i64).unwrap_or(0);
                            let updated = db.with(|c| {
                                Ok(c.execute(
                                    "UPDATE episodes SET file_path = ?1, size = ?2, state = 'done'
                                     WHERE mal_id = ?3 AND number = ?4",
                                    rusqlite::params![file_path.display().to_string(), size, id, ep_num],
                                )?)
                            })?;
                            if updated > 0 {
                                episodes_found += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(RescanReport {
        shows_adopted,
        episodes_found,
    })
}
