use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;

use crate::db::Db;
use crate::error::Result;
use crate::models::ShowType;
use crate::settings::{MoveMode, Settings};

use super::formatting::{format_nfo, format_synopsis, is_placeholder_synopsis, sanitize_component};
use super::icons::create_folder_png;
use super::types::{FOLDER_ICON_FILE, MARKER_FILE, SYNOPSIS_FILE};

/// Move, copy, or hardlink a file into the library with chmod 644.
pub fn transfer_file(source: &Path, target: &Path, mode: MoveMode) -> Result<()> {
    if source == target {
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    match mode {
        MoveMode::Hardlink => {
            if let Err(_) = fs::hard_link(source, target) {
                // Cross-device link fallback: copy
                fs::copy(source, target)?;
            }
        }
        MoveMode::Move => {
            if let Err(_) = fs::rename(source, target) {
                // Cross-device rename fallback: copy then unlink
                fs::copy(source, target)?;
                let _ = fs::remove_file(source);
            }
        }
        MoveMode::Copy => {
            fs::copy(source, target)?;
        }
    }

    // Strip execute bits (chmod 644)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(target, fs::Permissions::from_mode(0o644));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Show Folder Resolution
// ---------------------------------------------------------------------------

/// Resolve or create the show folder in the library root.
pub fn resolve_show_folder(
    db: &Db,
    library_folder: &Path,
    mal_id: i64,
    title: &str,
    start_year: Option<i64>,
) -> Result<PathBuf> {
    // 1. Check database for existing folder_path
    let existing_path: Option<String> = db.with(|c| {
        Ok(c.query_row(
            "SELECT folder_path FROM shows WHERE mal_id = ?1",
            [mal_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?)
    })?
    .flatten();

    if let Some(path_str) = &existing_path {
        let p = PathBuf::from(path_str);
        if p.exists() {
            let marker = p.join(MARKER_FILE);
            if !marker.exists() {
                let _ = fs::write(&marker, format!("{mal_id}\n"));
            }
            return Ok(p);
        }
    }

    // 2. Scan immediate subdirectories for .mal_id marker (detect folder rename)
    if library_folder.exists() {
        if let Ok(entries) = fs::read_dir(library_folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let marker = path.join(MARKER_FILE);
                    if marker.exists() {
                        if let Ok(content) = fs::read_to_string(&marker) {
                            if let Ok(id) = content.trim().parse::<i64>() {
                                if id == mal_id {
                                    db.with(|c| {
                                        c.execute(
                                            "UPDATE shows SET folder_path = ?1 WHERE mal_id = ?2",
                                            rusqlite::params![path.display().to_string(), mal_id],
                                        )?;
                                        Ok(())
                                    })?;
                                    return Ok(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2b. If existing_path was set and not found in library_folder, it is an explicit override that must be created
    if let Some(path_str) = existing_path {
        let p = PathBuf::from(&path_str);
        fs::create_dir_all(&p)?;
        let marker = p.join(MARKER_FILE);
        if !marker.exists() {
            let _ = fs::write(&marker, format!("{mal_id}\n"));
        }
        return Ok(p);
    }

    // 3. Create a new show folder
    let clean_title = sanitize_component(title);
    let folder_name = match start_year {
        Some(year) => format!("{clean_title} ({year})"),
        None => clean_title,
    };

    let show_dir = library_folder.join(folder_name);
    fs::create_dir_all(&show_dir)?;
    let marker = show_dir.join(MARKER_FILE);
    fs::write(&marker, format!("{mal_id}\n"))?;

    db.with(|c| {
        c.execute(
            "UPDATE shows SET folder_path = ?1 WHERE mal_id = ?2",
            rusqlite::params![show_dir.display().to_string(), mal_id],
        )?;
        Ok(())
    })?;

    Ok(show_dir)
}

/// Rewrite synopsis.txt and movie.nfo/tvshow.nfo (plot, rating, year) for all existing folders of a show.
/// Called when shows.synopsis becomes ready or when metadata is updated.
pub fn rewrite_show_folder_metadata(db: &Db, mal_id: i64) -> Result<usize> {
    // 1. Fetch show details from database
    let show_data: Option<(
        String,
        Option<String>,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<f64>,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = db.with(|c| {
        Ok(c.query_row(
            "SELECT title, english_title, type, start_year, synopsis, poster_path, score, total_episodes, status, folder_path
             FROM shows WHERE mal_id = ?1",
            [mal_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<f64>>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                ))
            },
        )
        .optional()?)
    })?;

    let (title, english_title, show_type_str, start_year, synopsis, poster_path, score, total_episodes, status, folder_path) = match show_data {
        Some(d) => d,
        None => return Ok(0),
    };

    let syn = match synopsis {
        Some(s) if !s.trim().is_empty() && !is_placeholder_synopsis(&s) => s,
        _ => return Ok(0), // synopsis not ready yet
    };

    let show_type = ShowType::parse(&show_type_str);

    // Fetch Japanese title from show_names if present
    let japanese_title: Option<String> = db.with(|c| {
        Ok(c.query_row(
            "SELECT name FROM show_names WHERE mal_id = ?1 AND source = 'jikan' ORDER BY priority DESC LIMIT 1",
            [mal_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
    })?;

    // Load settings for library root
    let settings = Settings::load(db)?;
    let library_root = if settings.library_folder.trim().is_empty() {
        crate::locations::default_library_dir()
    } else {
        crate::locations::resolve_home_dir(&settings.library_folder)
    };

    // Collect all unique existing folders for this show
    let mut folders = HashSet::new();

    // 1) Folder path stored in shows table
    if let Some(ref fp) = folder_path {
        let p = PathBuf::from(fp);
        if p.is_dir() {
            folders.insert(p);
        }
    }

    // 2) Parent directory of any recorded episode file
    let ep_paths: Vec<String> = db.with(|c| {
        let mut stmt = c.prepare("SELECT DISTINCT file_path FROM episodes WHERE mal_id = ?1 AND file_path IS NOT NULL")?;
        let rows = stmt.query_map([mal_id], |r| r.get::<_, String>(0))?;
        let mut list = Vec::new();
        for r in rows.flatten() {
            if !r.trim().is_empty() {
                list.push(r);
            }
        }
        Ok(list)
    })?;

    for ep_p in ep_paths {
        let p = PathBuf::from(&ep_p);
        if let Some(parent) = p.parent() {
            if parent.is_dir() {
                folders.insert(parent.to_path_buf());
            }
        }
    }

    // 3) Any subfolder in library_root with matching .mal_id marker
    if library_root.is_dir() {
        if let Ok(entries) = fs::read_dir(&library_root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let marker = p.join(MARKER_FILE);
                    if marker.exists() {
                        if let Ok(content) = fs::read_to_string(&marker) {
                            if let Ok(id) = content.trim().parse::<i64>() {
                                if id == mal_id {
                                    folders.insert(p);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if folders.is_empty() {
        return Ok(0);
    }

    let now_date = format!(
        "Unix timestamp {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    let syn_formatted = format_synopsis(
        &title,
        english_title.as_deref(),
        japanese_title.as_deref(),
        &show_type_str,
        total_episodes,
        status.as_deref(),
        None,
        score,
        &[],
        Some(&syn),
        mal_id,
        &now_date,
    );

    let tag = if show_type == ShowType::Movie { "movie" } else { "tvshow" };
    let nfo_content = format_nfo(
        tag,
        &title,
        english_title.as_deref(),
        &syn,
        score,
        start_year,
        mal_id,
    );

    let mut updated_count = 0;
    for dir in &folders {
        // Rewrite synopsis.txt
        let synopsis_file = dir.join(SYNOPSIS_FILE);
        let _ = fs::write(&synopsis_file, &syn_formatted);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&synopsis_file, fs::Permissions::from_mode(0o644));
        }

        // Rewrite movie.nfo or tvshow.nfo
        let movie_nfo = dir.join("movie.nfo");
        let tvshow_nfo = dir.join("tvshow.nfo");
        let target_nfo = if movie_nfo.exists() {
            movie_nfo
        } else if tvshow_nfo.exists() {
            tvshow_nfo
        } else if show_type == ShowType::Movie {
            movie_nfo
        } else {
            tvshow_nfo
        };

        let _ = fs::write(&target_nfo, &nfo_content);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target_nfo, fs::Permissions::from_mode(0o644));
        }

        // Ensure marker exists
        let marker_file = dir.join(MARKER_FILE);
        if !marker_file.exists() {
            let _ = fs::write(&marker_file, format!("{mal_id}\n"));
        }

        // If poster_path is a local file and folder.png is missing, create it
        let folder_png = dir.join(FOLDER_ICON_FILE);
        if !folder_png.exists() {
            if let Some(ref post) = poster_path {
                let pb = Path::new(post);
                if pb.is_file() {
                    let _ = create_folder_png(pb, &folder_png, settings.icon_style);
                }
            }
        }

        updated_count += 1;
    }

    Ok(updated_count)
}
