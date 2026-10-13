//! Location management and filesystem verification.
//!
//! Provides validation and live checking of directories (download_dir, library_dir,
//! and per-show folder overrides) including existence, write permissions, free space,
//! path traversal / root containment, and filesystem cross-device checks.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::{Error, Result};
use crate::events::EventLog;
use crate::settings::Settings;

/// Report on location status and validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocationCheck {
    pub path: String,
    pub exists: bool,
    pub writable: bool,
    pub free_space_bytes: Option<u64>,
    pub free_space_gb: Option<f64>,
    pub is_valid: bool,
    pub error: Option<String>,
}

/// Overall locations status for Settings page and CLI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocationsReport {
    pub download_dir: LocationCheck,
    pub library_dir: LocationCheck,
    pub same_filesystem: bool,
    pub note: String,
}

/// Information about a single show's folder override.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShowFolderInfo {
    pub mal_id: i64,
    pub title: String,
    pub folder_path: Option<String>,
    pub check: Option<LocationCheck>,
}

/// Report resulting from moving/migrating a library folder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MoveLibraryReport {
    pub shows_moved: usize,
    pub files_moved: usize,
    pub bytes_moved: u64,
    pub errors: Vec<String>,
}

/// Resolve `~` prefix to current user's HOME directory.
pub fn resolve_home_dir(path_str: &str) -> PathBuf {
    let trimmed = path_str.trim();
    if trimmed == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    } else if let Some(rest) = trimmed.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(trimmed)
}

/// Default download folder (~/Downloads/Genrl or relative to HOME).
pub fn default_download_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("Downloads").join("Genrl")
}

/// Default library folder (~/Videos/Anime or relative to HOME).
pub fn default_library_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("Videos").join("Anime")
}

/// Check whether two paths reside on the same filesystem.
pub fn are_on_same_filesystem(path_a: &Path, path_b: &Path) -> bool {
    #[cfg(unix)]
    {
        let dev_a = get_dev_for_path(path_a);
        let dev_b = get_dev_for_path(path_b);
        if let (Some(a), Some(b)) = (dev_a, dev_b) {
            return a == b;
        }
    }
    let _ = (path_a, path_b);
    false
}

#[cfg(unix)]
fn get_dev_for_path(p: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    let mut curr = p.to_path_buf();
    loop {
        if let Ok(meta) = fs::metadata(&curr) {
            return Some(meta.dev());
        }
        if !curr.pop() {
            break;
        }
    }
    None
}

/// Validate path for directory containment and traversal threats.
pub fn validate_path_safety(path: &Path, allowed_roots: Option<&[&Path]>) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();
    if path_str.trim().is_empty() {
        return Err(Error::Other("path cannot be empty".to_string()));
    }

    // Check for traversal components
    for comp in path.components() {
        if let std::path::Component::ParentDir = comp {
            return Err(Error::Other(format!(
                "path traversal ('..') is not permitted in location: {path_str}"
            )));
        }
    }

    // Resolve ~ if present
    let resolved = resolve_home_dir(&path_str);
    if !resolved.is_absolute() {
        return Err(Error::Other(format!(
            "location path must be absolute: {path_str}"
        )));
    }

    if let Some(roots) = allowed_roots {
        let is_allowed = roots.iter().any(|root| {
            let res_root = resolve_home_dir(&root.to_string_lossy());
            resolved.starts_with(&res_root)
        });
        if !is_allowed {
            return Err(Error::Other(format!(
                "path '{}' is outside allowed roots: {:?}",
                resolved.display(),
                roots
            )));
        }
    }

    Ok(resolved)
}

/// Check a directory path: existence, writability, free space.
pub fn check_directory(path_str: &str, create_if_missing: bool) -> LocationCheck {
    let resolved = resolve_home_dir(path_str);
    let resolved_str = resolved.to_string_lossy().to_string();

    if let Err(e) = validate_path_safety(&resolved, None) {
        return LocationCheck {
            path: resolved_str,
            exists: false,
            writable: false,
            free_space_bytes: None,
            free_space_gb: None,
            is_valid: false,
            error: Some(e.to_string()),
        };
    }

    if !resolved.exists() {
        if create_if_missing {
            if let Err(e) = fs::create_dir_all(&resolved) {
                return LocationCheck {
                    path: resolved_str,
                    exists: false,
                    writable: false,
                    free_space_bytes: None,
                    free_space_gb: None,
                    is_valid: false,
                    error: Some(format!("could not create directory: {e}")),
                };
            }
        } else {
            // Check if parent directory exists and is writable
            let mut parent = resolved.parent();
            while let Some(p) = parent {
                if p.exists() {
                    let writable = is_path_writable(p);
                    let free_space = fs2::available_space(p).ok();
                    let free_gb = free_space.map(|b| b as f64 / (1024.0 * 1024.0 * 1024.0));
                    return LocationCheck {
                        path: resolved_str,
                        exists: false,
                        writable,
                        free_space_bytes: free_space,
                        free_space_gb: free_gb,
                        is_valid: writable,
                        error: if writable {
                            None
                        } else {
                            Some("parent directory is not writable".to_string())
                        },
                    };
                }
                parent = p.parent();
            }

            return LocationCheck {
                path: resolved_str,
                exists: false,
                writable: false,
                free_space_bytes: None,
                free_space_gb: None,
                is_valid: false,
                error: Some("directory does not exist".to_string()),
            };
        }
    }

    let is_dir = resolved.is_dir();
    if !is_dir {
        return LocationCheck {
            path: resolved_str,
            exists: true,
            writable: false,
            free_space_bytes: None,
            free_space_gb: None,
            is_valid: false,
            error: Some("path exists but is not a directory".to_string()),
        };
    }

    let writable = is_path_writable(&resolved);
    let free_space = fs2::available_space(&resolved).ok();
    let free_gb = free_space.map(|b| b as f64 / (1024.0 * 1024.0 * 1024.0));

    LocationCheck {
        path: resolved_str,
        exists: true,
        writable,
        free_space_bytes: free_space,
        free_space_gb: free_gb,
        is_valid: writable,
        error: if writable {
            None
        } else {
            Some("directory exists but is not writable (permission denied)".to_string())
        },
    }
}

fn is_path_writable(path: &Path) -> bool {
    let test_file = path.join(".genrl_write_test_tmp");
    match fs::write(&test_file, b"test") {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

/// Retrieve effective locations report based on current settings.
pub fn get_locations(db: &Db) -> Result<LocationsReport> {
    let settings = Settings::load(db)?;
    let dl_path = if settings.download_folder.trim().is_empty() {
        default_download_dir().display().to_string()
    } else {
        settings.download_folder.clone()
    };

    let lib_path = if settings.library_folder.trim().is_empty() {
        default_library_dir().display().to_string()
    } else {
        settings.library_folder.clone()
    };

    let download_check = check_directory(&dl_path, false);
    let library_check = check_directory(&lib_path, false);

    let path_a = resolve_home_dir(&dl_path);
    let path_b = resolve_home_dir(&lib_path);
    let same_fs = are_on_same_filesystem(&path_a, &path_b);

    let note = if same_fs {
        "Download folder and Library folder are on the SAME filesystem. Hardlinking is supported (seeding continues after filing)."
            .to_string()
    } else {
        "Download folder and Library folder are on DIFFERENT filesystems. Files will be moved/copied across devices (seeding stops after move)."
            .to_string()
    };

    Ok(LocationsReport {
        download_dir: download_check,
        library_dir: library_check,
        same_filesystem: same_fs,
        note,
    })
}

/// Set a location key (`download_dir` / `download_folder`, `library_dir` / `library_folder`, or per-show `folder_path`).
pub fn set_location(
    db: &Db,
    key: &str,
    val: &str,
    mal_id: Option<i64>,
    log: &EventLog,
) -> Result<LocationCheck> {
    let resolved = resolve_home_dir(val);
    let check = check_directory(&resolved.to_string_lossy(), true);
    if !check.is_valid {
        return Err(Error::Other(format!(
            "invalid location '{}': {}",
            val,
            check.error.as_deref().unwrap_or("validation failed")
        )));
    }

    let val_to_store = resolved.display().to_string();

    if let Some(id) = mal_id {
        db.with(|c| {
            c.execute(
                "UPDATE shows SET folder_path = ?1 WHERE mal_id = ?2",
                rusqlite::params![val_to_store, id],
            )?;
            Ok(())
        })?;
        let _ = log.info(
            None,
            Some(id),
            format!("Updated show folder override to '{}'", val_to_store),
        );
    } else {
        let mut settings = Settings::load(db)?;
        match key {
            "download_dir" | "download_folder" => {
                settings.download_folder = val_to_store.clone();
                settings.save(db)?;
                let _ = log.info(
                    None,
                    None,
                    format!("Updated download folder to '{}'", val_to_store),
                );
            }
            "library_dir" | "library_folder" => {
                settings.library_folder = val_to_store.clone();
                settings.save(db)?;
                let _ = log.info(
                    None,
                    None,
                    format!("Updated library folder to '{}'", val_to_store),
                );
            }
            other => {
                return Err(Error::Other(format!(
                    "unknown location key '{other}'; use 'download_dir' or 'library_dir'"
                )));
            }
        }
    }

    Ok(check)
}

/// Get folder path override for a specific show.
pub fn get_show_folder(db: &Db, mal_id: i64) -> Result<ShowFolderInfo> {
    let (title, folder_path): (String, Option<String>) = db.with(|c| {
        Ok(c.query_row(
            "SELECT title, folder_path FROM shows WHERE mal_id = ?1",
            [mal_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
    })?
    .ok_or_else(|| Error::Other(format!("show with MAL id {mal_id} not found")))?;

    let check = folder_path.as_deref().map(|p| check_directory(p, false));

    Ok(ShowFolderInfo {
        mal_id,
        title,
        folder_path,
        check,
    })
}

/// Explicit, confirmed migration: moves library contents from old location to new location.
pub fn move_existing_library(
    db: &Db,
    source_dir: &Path,
    target_dir: &Path,
    log: &EventLog,
) -> Result<MoveLibraryReport> {
    if !source_dir.exists() {
        return Err(Error::Other(format!(
            "source directory '{}' does not exist",
            source_dir.display()
        )));
    }
    fs::create_dir_all(target_dir)?;

    let mut report = MoveLibraryReport::default();
    let entries = fs::read_dir(source_dir)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = match path.file_name() {
                Some(n) => n,
                None => continue,
            };
            let dst_show_dir = target_dir.join(dir_name);
            let show_res = move_dir_recursive(&path, &dst_show_dir, &mut report);
            if let Err(e) = show_res {
                report.errors.push(format!("{}: {e}", path.display()));
            } else {
                report.shows_moved += 1;
                // Update folder_path in shows table if it matched the old path
                let old_str = path.display().to_string();
                let new_str = dst_show_dir.display().to_string();
                let _ = db.with(|c| {
                    c.execute(
                        "UPDATE shows SET folder_path = ?1 WHERE folder_path = ?2",
                        rusqlite::params![new_str, old_str],
                    )?;
                    Ok(())
                });
            }
        }
    }

    let _ = log.info(
        None,
        None,
        format!(
            "Migrated library from '{}' to '{}': {} shows moved, {} files, {} bytes",
            source_dir.display(),
            target_dir.display(),
            report.shows_moved,
            report.files_moved,
            report.bytes_moved
        ),
    );

    Ok(report)
}

fn move_dir_recursive(
    src: &Path,
    dst: &Path,
    report: &mut MoveLibraryReport,
) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_child = entry.path();
        let dst_child = dst.join(entry.file_name());

        if src_child.is_dir() {
            move_dir_recursive(&src_child, &dst_child, report)?;
        } else {
            let len = entry.metadata()?.len();
            if fs::rename(&src_child, &dst_child).is_err() {
                fs::copy(&src_child, &dst_child)?;
                fs::remove_file(&src_child)?;
            }
            report.files_moved += 1;
            report.bytes_moved += len;
        }
    }
    let _ = fs::remove_dir(src);
    Ok(())
}
