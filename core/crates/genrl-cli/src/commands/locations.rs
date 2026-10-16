use anyhow::{bail, Result};
use clap::Subcommand;
use genrl_core::EventLog;
use std::path::{Path, PathBuf};

use super::common::{human_bytes, open_db};

#[derive(Subcommand, Debug)]
pub enum LocationsCommand {
    /// Get current locations report (download_dir, library_dir, same-filesystem status).
    Get {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Set download_dir or library_dir, or per-show folder override.
    Set {
        /// 'download_dir', 'library_dir', or 'folder_path'.
        key: String,
        /// Destination directory path.
        path: String,
        /// Optional MAL id for per-show folder override.
        #[arg(long)]
        mal_id: Option<i64>,
    },
    /// Live-check directory path for existence, writability, and free space.
    Check {
        /// Directory path to check.
        path: String,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Get folder path override for a specific show.
    GetShow {
        /// MAL show id.
        mal_id: i64,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Migrate existing library from source directory to target directory (explicit confirmed action).
    Migrate {
        /// Source directory.
        #[arg(long)]
        from: PathBuf,
        /// Target destination directory.
        #[arg(long)]
        to: PathBuf,
        /// Confirmed flag required to execute migration.
        #[arg(long)]
        confirm: bool,
    },
}

pub fn locations_get_cmd(data_dir: &Path, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let report = genrl_core::locations::get_locations(&db)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("=== Storage Locations ===");
        println!("Download Staging Directory:");
        println!("  Path       : {}", report.download_dir.path);
        println!("  Exists     : {}", report.download_dir.exists);
        println!("  Writable   : {}", report.download_dir.writable);
        println!(
            "  Free Space : {:.2} GiB",
            report.download_dir.free_space_gb.unwrap_or(0.0)
        );
        if let Some(err) = &report.download_dir.error {
            println!("  Warning    : {}", err);
        }
        println!();
        println!("Final Library Destination:");
        println!("  Path       : {}", report.library_dir.path);
        println!("  Exists     : {}", report.library_dir.exists);
        println!("  Writable   : {}", report.library_dir.writable);
        println!(
            "  Free Space : {:.2} GiB",
            report.library_dir.free_space_gb.unwrap_or(0.0)
        );
        if let Some(err) = &report.library_dir.error {
            println!("  Warning    : {}", err);
        }
        println!();
        println!("Filesystem Status:");
        println!("  Same Filesystem : {}", report.same_filesystem);
        println!("  Note            : {}", report.note);
    }
    Ok(())
}

pub fn locations_set_cmd(
    data_dir: &Path,
    key: &str,
    path: &str,
    mal_id: Option<i64>,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let log = EventLog::new(db.clone());
    let check = genrl_core::locations::set_location(&db, key, path, mal_id, &log)?;
    println!(
        "Updated location for '{}' to '{}' (valid: {}, writable: {})",
        if let Some(id) = mal_id {
            format!("show mal_id={id}")
        } else {
            key.to_string()
        },
        check.path,
        check.is_valid,
        check.writable
    );
    Ok(())
}

pub fn locations_check_cmd(path: &str, json: bool) -> Result<()> {
    let check = genrl_core::locations::check_directory(path, false);
    if json {
        println!("{}", serde_json::to_string_pretty(&check)?);
    } else {
        println!("Checking location: {}", check.path);
        println!("  Exists     : {}", check.exists);
        println!("  Writable   : {}", check.writable);
        println!(
            "  Free Space : {:.2} GiB",
            check.free_space_gb.unwrap_or(0.0)
        );
        println!("  Valid      : {}", check.is_valid);
        if let Some(err) = &check.error {
            println!("  Error      : {}", err);
        }
    }
    Ok(())
}

pub fn locations_get_show_cmd(data_dir: &Path, mal_id: i64, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let info = genrl_core::locations::get_show_folder(&db, mal_id)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Show '{}' (MAL #{})", info.title, info.mal_id);
        if let Some(folder) = &info.folder_path {
            println!("  Folder Override : {}", folder);
            if let Some(chk) = &info.check {
                println!("  Exists          : {}", chk.exists);
                println!("  Writable        : {}", chk.writable);
                println!(
                    "  Free Space      : {:.2} GiB",
                    chk.free_space_gb.unwrap_or(0.0)
                );
            }
        } else {
            println!("  Folder Override : None (uses library default)");
        }
    }
    Ok(())
}

pub fn locations_migrate_cmd(data_dir: &Path, from: &Path, to: &Path, confirm: bool) -> Result<()> {
    if !confirm {
        bail!("Library migration requires --confirm to avoid unintentional file moves.");
    }
    let db = open_db(data_dir)?;
    let log = EventLog::new(db.clone());
    let report = genrl_core::locations::move_existing_library(&db, from, to, &log)?;
    println!("Migration complete:");
    println!("  Shows moved : {}", report.shows_moved);
    println!("  Files moved : {}", report.files_moved);
    println!("  Bytes moved : {}", human_bytes(report.bytes_moved));
    if !report.errors.is_empty() {
        println!("  Errors ({}):", report.errors.len());
        for err in &report.errors {
            println!("    - {}", err);
        }
    }
    Ok(())
}
