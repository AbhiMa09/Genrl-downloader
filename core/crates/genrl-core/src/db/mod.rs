//! SQLite storage: one database, WAL mode, versioned migrations.
//!
//! Everything the app knows lives here so a crash or a restart costs nothing:
//! on startup the core re-opens this database and reconciles against qBittorrent
//! instead of trusting memory.

mod migrations;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::error::{Error, Result};

/// The database handle. Cheap to clone; all clones share one connection.
#[derive(Clone)]
pub struct Db {
    inner: Arc<Mutex<Connection>>,
    path: Option<PathBuf>,
}

impl std::fmt::Debug for Db {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Db").field("path", &self.path).finish()
    }
}

impl Db {
    /// Open (creating if needed) the database at `path`, harden its permissions
    /// and bring it up to the latest schema version.
    ///
    /// Permissions follow the threat model: data folder `0700`, database `0600`,
    /// and the app refuses to start if either is group- or world-writable.
    pub fn open(path: impl AsRef<Path>) -> Result<Db> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
            harden_dir(parent)?;
        }

        let existed = path.exists();
        let conn = Connection::open(path)?;
        if !existed {
            set_owner_only(path)?;
        }
        harden_file(path)?;

        Self::init(conn, Some(path.to_path_buf()))
    }

    /// An in-memory database with the same schema. Used by tests.
    pub fn open_in_memory() -> Result<Db> {
        Self::init(Connection::open_in_memory()?, None)
    }

    fn init(conn: Connection, path: Option<PathBuf>) -> Result<Db> {
        // WAL keeps the UI reading while the core writes; NORMAL sync is the
        // usual durability/throughput trade-off for WAL.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5_000)?;

        let db = Db {
            inner: Arc::new(Mutex::new(conn)),
            path,
        };
        db.migrate()?;
        Ok(db)
    }

    /// Path of the database file, if it is not in-memory.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Run a closure against the connection.
    ///
    /// The mutex is never poisoned deliberately: a panicking writer would
    /// otherwise take the whole app down with it, so we recover the handle.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = self.lock();
        f(&conn)
    }

    /// Run a fallible `rusqlite` closure against the connection.
    pub fn with_rusqlite<T>(
        &self,
        f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T> {
        let conn = self.lock();
        f(&conn).map_err(Into::into)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn migrate(&self) -> Result<()> {
        let mut conn = self.lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                 version    INTEGER PRIMARY KEY,
                 applied_at INTEGER NOT NULL
             );",
        )?;

        let current: i64 = conn
            .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);

        let latest = migrations::MIGRATIONS
            .iter()
            .map(|m| m.version)
            .max()
            .unwrap_or(0);

        if current > latest {
            return Err(Error::Other(format!(
                "database schema version {current} is newer than this build understands ({latest})"
            )));
        }

        for migration in migrations::MIGRATIONS {
            if migration.version <= current {
                continue;
            }
            let tx = conn.transaction()?;
            tx.execute_batch(migration.sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![migration.version, now_unix()],
            )?;
            tx.commit()?;
            tracing::info!(version = migration.version, "applied schema migration");
        }
        Ok(())
    }

    /// Current schema version.
    pub fn schema_version(&self) -> Result<i64> {
        self.with(|c| {
            c.query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |r| r.get(0),
            )
            .map_err(Into::into)
        })
    }
}

/// Seconds since the Unix epoch. One clock for every timestamp column.
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(unix)]
fn harden_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if !path.exists() {
        return Ok(());
    }
    check_not_writable(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(unix)]
fn harden_file(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    check_not_writable(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(unix)]
fn set_owner_only(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// Refuse group/world-writable data — another user on the machine could
/// rewrite the database or drop a symlink in its place.
#[cfg(unix)]
fn check_not_writable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path)?.permissions().mode();
    if mode & 0o022 != 0 {
        return Err(Error::InsecurePermissions {
            path: path.display().to_string(),
            mode: mode & 0o777,
        });
    }
    Ok(())
}

#[cfg(not(unix))]
fn harden_dir(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn harden_file(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_db_migrates_to_latest() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.schema_version().unwrap() >= 1);
    }

    #[test]
    fn migrations_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("genrl.db");
        let first = Db::open(&path).unwrap();
        let v1 = first.schema_version().unwrap();
        drop(first);
        let second = Db::open(&path).unwrap();
        assert_eq!(second.schema_version().unwrap(), v1);
    }

    #[test]
    fn core_tables_exist() {
        let db = Db::open_in_memory().unwrap();
        db.with(|c| {
            for table in [
                "shows",
                "show_names",
                "mal_entries",
                "episodes",
                "torrents",
                "jobs",
                "questions",
                "rules",
                "http_cache",
                "events",
                "settings",
            ] {
                let n: i64 = c.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |r| r.get(0),
                )?;
                assert_eq!(n, 1, "missing table {table}");
            }
            Ok(())
        })
        .unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn refuses_world_writable_database() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("genrl.db");
        std::fs::write(&path, b"").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();
        let err = Db::open(&path).unwrap_err();
        assert!(matches!(err, Error::InsecurePermissions { .. }), "{err:?}");
    }

    #[cfg(unix)]
    #[test]
    fn database_is_owner_only_after_open() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("genrl.db");
        let db = Db::open(&path).unwrap();
        drop(db);
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
