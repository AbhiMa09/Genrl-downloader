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
            let video_files: Vec<_> = fs::read_dir(&path)
                .map(|files| {
                    files
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.is_file() && is_video_file(p))
                        .collect()
                })
                .unwrap_or_default();

            for file_path in &video_files {
                if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                    let parsed = crate::filename::parse_release(filename);
                    let ep_num = parsed.episode.or_else(|| {
                        let show_type: Option<String> = db
                            .with(|c| {
                                Ok(c.query_row(
                                    "SELECT type FROM shows WHERE mal_id = ?1",
                                    [id],
                                    |r| r.get(0),
                                )
                                .ok())
                            })
                            .ok()
                            .flatten();
                        if show_type.as_deref() == Some("movie") || video_files.len() == 1 {
                            Some(1.0)
                        } else {
                            None
                        }
                    });
                    if let Some(ep_num) = ep_num {
                        let size = fs::metadata(file_path).map(|m| m.len() as i64).unwrap_or(0);
                        let updated = db.with(|c| {
                            Ok(c.execute(
                                "INSERT INTO episodes (mal_id, number, state, file_path, size)
                                 VALUES (?1, ?2, 'done', ?3, ?4)
                                 ON CONFLICT (mal_id, number) DO UPDATE SET
                                     state = 'done',
                                     file_path = excluded.file_path,
                                     size = excluded.size",
                                rusqlite::params![
                                    id,
                                    ep_num,
                                    file_path.display().to_string(),
                                    size
                                ],
                            )?)
                        })?;
                        if updated > 0 {
                            episodes_found += 1;
                            let _ = db.with(|c| {
                                c.execute(
                                    "UPDATE jobs SET state = 'done'
                                     WHERE episode_id IN (
                                         SELECT id FROM episodes WHERE mal_id = ?1 AND number = ?2
                                     ) AND state != 'done'",
                                    rusqlite::params![id, ep_num],
                                )?;
                                Ok(())
                            });
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
