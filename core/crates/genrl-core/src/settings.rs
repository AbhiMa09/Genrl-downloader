//! App configuration.
//!
//! "Everything that has a default number is a setting." Values live in the
//! `settings` table as JSON so types round-trip, export to a JSON file, and
//! each page that uses one links back here.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::db::{now_unix, Db};
use crate::error::Result;

/// How finished files get into the library.
/// Default follows the open decision in the design: hardlink, so seeding continues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MoveMode {
    #[default]
    Hardlink,
    Move,
    Copy,
}

/// Folder icon method, per desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IconMethod {
    /// Detect the desktop from `XDG_CURRENT_DESKTOP`, fall back to `.directory`.
    #[default]
    Auto,
    /// Write `Icon=folder.png` into `.directory` (KDE and most file managers).
    Directory,
    /// `gio set -t string <folder> metadata::custom-icon` (GNOME/Nautilus).
    Gio,
    Off,
}

/// Plain poster, or the poster composited onto a folder shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IconStyle {
    #[default]
    Plain,
    FolderShape,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // --- Wizard / lifecycle -------------------------------------------------
    /// The first-run wizard has been completed once (reachable again from Settings).
    pub first_run_complete: bool,
    /// One switch pauses everything; starts off until the wizard turns it on.
    pub automation_on: bool,

    // --- Accounts -----------------------------------------------------------
    /// Your own MAL client id (PKCE, no client secret is ever stored).
    pub mal_client_id: String,
    pub qbit_url: String,
    pub qbit_username: String,
    /// Start `qbittorrent-nox` ourselves if it is installed but not running.
    pub qbit_start_if_installed: bool,
    /// A remote qBittorrent URL needs an explicit warning acknowledgment.
    pub qbit_remote_acknowledged: bool,

    // --- Folders and naming -------------------------------------------------
    pub library_folder: String,
    /// Where qBittorrent saves; never directly into the library.
    pub download_folder: String,
    /// `{title} - S01E01.mkv`
    pub naming_pattern: String,
    /// `Kimi no Na wa (2016).mkv`
    pub movie_naming_pattern: String,
    pub move_mode: MoveMode,
    /// Keep the torrent until it reaches this ratio, then optionally remove it.
    pub seed_ratio_target: f64,
    pub remove_when_finished: bool,

    // --- Queue limits -------------------------------------------------------
    /// Active downloads are capped so bandwidth and disk are not swamped.
    pub active_downloads: u32,
    /// Search jobs share the Nyaa budget, so they run at most this many at a time.
    pub search_concurrency: u32,
    /// A daily cap on new torrents added; stops a bad sync queueing hundreds.
    pub daily_torrent_cap: u32,
    /// The first sync is capped tighter: the rest wait for batch approval.
    pub first_sync_cap: u32,
    /// Pause the queue below this much free space.
    pub disk_floor_gb: u64,

    // --- Rate limits and caches (overridable) -------------------------------
    /// MAL list syncs at most this often.
    pub mal_sync_interval_min: u32,
    pub jikan_per_second: f64,
    pub jikan_per_minute: u32,
    /// Nyaa gets one request per this many seconds, globally.
    pub nyaa_interval_secs: f64,
    pub mal_cache_secs: u32,
    pub jikan_cache_secs: u32,
    pub nyaa_cache_secs: u32,

    // --- Matching and stalls ------------------------------------------------
    /// Fuzzy ratio a parsed title must reach to count as a name match.
    pub fuzzy_threshold: f64,
    /// Best score at or above this may proceed automatically…
    pub score_threshold: f64,
    /// …and only if it is at least this far ahead of the runner-up.
    pub score_margin: f64,
    /// Plan to watch entries may queue on their own. Off by default — 73
    /// planned shows must never queue at once; per-show rules still opt one in.
    pub auto_plan_to_watch: bool,
    /// No progress for this long marks the torrent stalled and tries the next candidate.
    pub stall_timeout_min: u32,
    /// Ask the user after this many failed candidates.
    pub max_failed_candidates: u32,
    /// Unanswered questions expire after this long and return the episode to missing.
    pub question_expiry_hours: u32,
    /// Size sanity range for a 24-minute episode; catches fakes and mislabeled batches.
    pub min_episode_mb: u64,
    pub max_episode_mb: u64,

    // --- Endpoints ----------------------------------------------------------
    pub nyaa_base_url: String,
    /// Mirror fallback list when the primary is down.
    pub nyaa_mirrors: Vec<String>,
    pub jikan_base_url: String,
    pub mal_base_url: String,

    // --- Appearance ---------------------------------------------------------
    pub folder_icon_method: IconMethod,
    pub icon_style: IconStyle,
    pub theme: String,

    // --- Notifications and autostart ---------------------------------------
    pub notify_finished: bool,
    pub notify_questions: bool,
    pub notify_failures: bool,
    pub autostart: bool,

    // --- Logging and caps ---------------------------------------------------
    /// Debug logging is an optional switch and rotates by size.
    pub debug_logging: bool,
    pub log_max_bytes: u64,
    pub log_keep: usize,
    /// Cap the event log, pruning oldest first.
    pub events_keep_rows: u64,
    /// Cap the response cache, pruning oldest first.
    pub http_cache_keep_rows: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            first_run_complete: false,
            automation_on: false,

            mal_client_id: String::new(),
            qbit_url: "http://127.0.0.1:8080".to_string(),
            qbit_username: "admin".to_string(),
            qbit_start_if_installed: true,
            qbit_remote_acknowledged: false,

            library_folder: String::new(),
            download_folder: String::new(),
            naming_pattern: "{title} - S{season:02}E{episode:02}".to_string(),
            movie_naming_pattern: "{title} ({year})".to_string(),
            move_mode: MoveMode::default(),
            seed_ratio_target: 1.0,
            remove_when_finished: false,

            active_downloads: 2,
            search_concurrency: 2,
            daily_torrent_cap: 30,
            first_sync_cap: 25,
            disk_floor_gb: 20,

            mal_sync_interval_min: 30,
            jikan_per_second: 2.0,
            jikan_per_minute: 60,
            nyaa_interval_secs: 2.0,
            mal_cache_secs: 1800,
            jikan_cache_secs: 7 * 24 * 3600,
            nyaa_cache_secs: 600,

            fuzzy_threshold: 0.92,
            score_threshold: 80.0,
            score_margin: 15.0,
            // Plan to watch is manual by default: 73 planned shows must never
            // queue at once. A per-show override still opts one in.
            auto_plan_to_watch: false,
            stall_timeout_min: 20,
            max_failed_candidates: 3,
            question_expiry_hours: 72,
            min_episode_mb: 100,
            max_episode_mb: 4096,

            nyaa_base_url: "https://nyaa.si".to_string(),
            nyaa_mirrors: Vec::new(),
            jikan_base_url: "https://api.jikan.moe/v4".to_string(),
            mal_base_url: "https://api.myanimelist.net/v2".to_string(),

            folder_icon_method: IconMethod::default(),
            icon_style: IconStyle::default(),
            theme: "dark".to_string(),

            notify_finished: true,
            notify_questions: true,
            notify_failures: true,
            autostart: false,

            debug_logging: false,
            log_max_bytes: 5 * 1024 * 1024,
            log_keep: 3,
            events_keep_rows: 20_000,
            http_cache_keep_rows: 5_000,
        }
    }
}

impl Settings {
    /// Read settings from the database, filling in defaults for anything new.
    pub fn load(db: &Db) -> Result<Self> {
        let mut map = BTreeMap::new();
        db.with(|c| {
            let mut stmt = c.prepare("SELECT key, value FROM settings")?;
            let rows = stmt.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (key, value) = row?;
                if key.starts_with(STATE_PREFIX) {
                    continue;
                }
                // Tolerate a hand-edited row that is not valid JSON: keep it as a string.
                let json = serde_json::from_str(&value)
                    .unwrap_or_else(|_| serde_json::Value::String(value.clone()));
                map.insert(key, json);
            }
            Ok(())
        })?;

        let value = serde_json::Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, v))
                .collect::<serde_json::Map<_, _>>(),
        );
        let settings: Settings = serde_json::from_value(value)?;
        Ok(settings)
    }

    /// Write every setting, and drop rows for fields that no longer exist
    /// (runtime state rows prefixed with `state.` are left alone).
    pub fn save(&self, db: &Db) -> Result<()> {
        let json = serde_json::to_value(self)?;
        let map = json
            .as_object()
            .ok_or_else(|| crate::Error::Other("settings did not serialize to an object".into()))?;

        db.with(|c| {
            let tx = c.unchecked_transaction()?;
            for (key, value) in map {
                tx.execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    rusqlite::params![key, value.to_string()],
                )?;
            }

            // Remove stale config keys, keeping runtime state rows.
            let keep: Vec<&String> = map.keys().collect();
            let mut sql = String::from(
                "DELETE FROM settings WHERE key NOT LIKE 'state.%'",
            );
            if !keep.is_empty() {
                sql.push_str(" AND key NOT IN (");
                for i in 0..keep.len() {
                    if i > 0 {
                        sql.push(',');
                    }
                    sql.push_str(&format!("?{}", i + 1));
                }
                sql.push(')');
            }
            let mut stmt = tx.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::ToSql> =
                keep.iter().map(|k| k as &dyn rusqlite::ToSql).collect();
            stmt.execute(rusqlite::params_from_iter(params))?;
            drop(stmt);
            tx.commit()?;
            Ok(())
        })?;
        Ok(())
    }

    /// Refuse obviously broken configuration instead of failing later.
    pub fn validate(&self) -> Result<()> {
        let bad = |msg: &str| Err(crate::Error::Other(msg.to_string()));
        if self.active_downloads == 0 {
            return bad("active_downloads must be at least 1");
        }
        if self.search_concurrency == 0 {
            return bad("search_concurrency must be at least 1");
        }
        if self.daily_torrent_cap == 0 {
            return bad("daily_torrent_cap must be at least 1");
        }
        if self.nyaa_interval_secs <= 0.0 {
            return bad("nyaa_interval_secs must be greater than 0");
        }
        if self.jikan_per_second <= 0.0 {
            return bad("jikan_per_second must be greater than 0");
        }
        if self.min_episode_mb >= self.max_episode_mb {
            return bad("min_episode_mb must be smaller than max_episode_mb");
        }
        if self.score_threshold > 100.0 {
            return bad("score_threshold is a score out of 100");
        }
        if !(0.0..=1.0).contains(&self.fuzzy_threshold) {
            return bad("fuzzy_threshold is a ratio between 0 and 1");
        }
        if !self.qbit_url.starts_with("http://") && !self.qbit_url.starts_with("https://") {
            return bad("qbit_url must start with http:// or https://");
        }
        for folder in [&self.library_folder, &self.download_folder] {
            if !folder.is_empty() && !Path::new(folder).is_absolute() {
                return bad(&format!("folder path must be absolute: {folder}"));
            }
        }
        if self
            .qbit_url
            .strip_prefix("https://")
            .or_else(|| self.qbit_url.strip_prefix("http://"))
            .map(|rest| {
                let host = rest.split(['/', ':']).next().unwrap_or("");
                host != "127.0.0.1" && host != "localhost" && host != "::1"
            })
            .unwrap_or(false)
            && !self.qbit_remote_acknowledged
        {
            return bad("remote qBittorrent URL needs explicit acknowledgment");
        }
        Ok(())
    }

    /// Pretty JSON for the export file.
    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Export to a JSON file (owner-only permissions — exports must not leak).
    pub fn export_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_json_pretty()?)?;
        set_owner_only(path);
        Ok(())
    }

    /// Parse an exported file (without touching the database).
    pub fn import_file(path: impl AsRef<Path>) -> Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let settings: Settings = serde_json::from_str(&raw)?;
        settings.validate()?;
        Ok(settings)
    }

    /// Load from a file, validate, and store into the database.
    pub fn import_into(path: impl AsRef<Path>, db: &Db) -> Result<Self> {
        let settings = Self::import_file(path)?;
        settings.save(db)?;
        Ok(settings)
    }
}

/// Runtime values (last sync time, cached versions) share the settings table
/// but never the settings struct, so exporting config never leaks state.
pub const STATE_PREFIX: &str = "state.";

/// Read a runtime state value.
pub fn read_state(db: &Db, key: &str) -> Result<Option<String>> {
    let full = format!("{STATE_PREFIX}{key}");
    db.with(|c| {
        let mut stmt = c.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query([&full])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    })
}

/// Write a runtime state value.
pub fn write_state(db: &Db, key: &str, value: &str) -> Result<()> {
    let full = format!("{STATE_PREFIX}{key}");
    db.with(|c| {
        c.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![full, value],
        )?;
        Ok(())
    })
}

/// Read a runtime state value as a Unix timestamp.
pub fn read_state_time(db: &Db, key: &str) -> Result<Option<i64>> {
    Ok(read_state(db, key)?.and_then(|v| v.parse().ok()))
}

/// Convenience: record "now" under a state key.
pub fn write_state_now(db: &Db, key: &str) -> Result<()> {
    write_state(db, key, &now_unix().to_string())
}

#[cfg(unix)]
fn set_owner_only(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_owner_only(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_design() {
        let s = Settings::default();
        assert_eq!(s.active_downloads, 2);
        assert_eq!(s.daily_torrent_cap, 30);
        assert_eq!(s.search_concurrency, 2);
        assert_eq!(s.stall_timeout_min, 20);
        assert_eq!(s.fuzzy_threshold, 0.92);
        assert_eq!(s.score_threshold, 80.0);
        assert_eq!(s.score_margin, 15.0);
        assert_eq!(s.min_episode_mb, 100);
        assert_eq!(s.max_episode_mb, 4096);
        assert_eq!(s.mal_sync_interval_min, 30);
        assert_eq!(s.jikan_per_second, 2.0);
        assert_eq!(s.jikan_per_minute, 60);
        assert_eq!(s.nyaa_interval_secs, 2.0);
        assert_eq!(s.jikan_cache_secs, 7 * 24 * 3600);
        assert_eq!(s.nyaa_cache_secs, 600);
        assert_eq!(s.max_failed_candidates, 3);
        assert_eq!(s.move_mode, MoveMode::Hardlink);
        assert!(!s.automation_on, "automation starts off until the wizard");
        assert!(s.qbit_start_if_installed);
    }

    #[test]
    fn round_trips_through_the_database() {
        let db = Db::open_in_memory().unwrap();
        let mut s = Settings::default();
        s.library_folder = "/mnt/anime".into();
        s.active_downloads = 4;
        s.daily_torrent_cap = 12;
        s.first_run_complete = true;
        s.save(&db).unwrap();

        let loaded = Settings::load(&db).unwrap();
        assert_eq!(loaded, s);
    }

    #[test]
    fn missing_rows_fall_back_to_defaults() {
        let db = Db::open_in_memory().unwrap();
        let loaded = Settings::load(&db).unwrap();
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn export_and_import_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut s = Settings::default();
        s.nyaa_base_url = "https://mirror.example".into();
        s.export_file(&path).unwrap();
        let back = Settings::import_file(&path).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn save_drops_stale_keys_but_keeps_state() {
        let db = Db::open_in_memory().unwrap();
        let mut s = Settings::default();
        s.active_downloads = 5;
        s.save(&db).unwrap();
        write_state(&db, "last_mal_sync_at", "1234").unwrap();

        s.active_downloads = 2;
        s.save(&db).unwrap();

        assert_eq!(Settings::load(&db).unwrap().active_downloads, 2);
        assert_eq!(read_state(&db, "last_mal_sync_at").unwrap().as_deref(), Some("1234"));
    }

    #[test]
    fn validation_rejects_bad_config() {
        let mut s = Settings::default();
        s.active_downloads = 0;
        assert!(s.validate().is_err());

        let mut s = Settings::default();
        s.library_folder = "relative/path".into();
        assert!(s.validate().is_err());

        let mut s = Settings::default();
        s.qbit_url = "http://192.168.1.5:8080".into();
        assert!(s.validate().is_err(), "remote URL must be acknowledged");
        s.qbit_remote_acknowledged = true;
        assert!(s.validate().is_ok());
    }
}
