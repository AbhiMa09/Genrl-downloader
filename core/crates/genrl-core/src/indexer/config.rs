use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::Result;
use crate::indexer::types::MediaKind;

const SETTINGS_KEY: &str = "indexers";

/// Configuration for an external indexer (e.g. Torznab via Prowlarr).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexerConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    /// API key reference or secret. Omitted when serialized to database or logs.
    #[serde(skip_serializing, default)]
    pub api_key: Option<String>,
    pub enabled: bool,
    pub media_kinds: Vec<MediaKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_test_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_test_at: Option<i64>,
}

impl fmt::Debug for IndexerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IndexerConfig")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field("enabled", &self.enabled)
            .field("media_kinds", &self.media_kinds)
            .field("last_test_status", &self.last_test_status)
            .field("last_test_at", &self.last_test_at)
            .finish()
    }
}

/// Validate indexer URLs: http or https only, no userinfo (user:pass@),
/// no fragment, no query string containing key-like params (strip query string and refuse key-like params), max length 2048.
pub fn validate_indexer_url(url_str: &str) -> Result<String> {
    let trimmed = url_str.trim();
    if trimmed.len() > 2048 {
        return Err(crate::error::Error::Other(
            "indexer URL exceeds maximum length of 2048 characters".into(),
        ));
    }
    let parsed = url::Url::parse(trimmed)
        .map_err(|e| crate::error::Error::Other(format!("invalid URL: {e}")))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(crate::error::Error::Other(
            "indexer URL must use http or https scheme".into(),
        ));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(crate::error::Error::Other(
            "indexer URL must not contain userinfo (username or password)".into(),
        ));
    }
    if parsed.fragment().is_some() {
        return Err(crate::error::Error::Other(
            "indexer URL must not contain URL fragments (#)".into(),
        ));
    }
    if parsed.query().is_some() {
        for (k, _) in parsed.query_pairs() {
            let lower = k.to_ascii_lowercase();
            if lower == "apikey" || lower == "api_key" || lower == "passkey" || lower == "key" {
                return Err(crate::error::Error::Other(
                    "indexer URL must not contain API key parameters in query string".into(),
                ));
            }
        }
    }
    let mut clean = parsed;
    clean.set_query(None);
    Ok(clean.to_string().trim_end_matches('/').to_string())
}

impl IndexerConfig {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: Option<String>,
        media_kinds: Vec<MediaKind>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            base_url: base_url.into(),
            api_key,
            enabled: true,
            media_kinds,
            last_test_status: None,
            last_test_at: None,
        }
    }

    /// Load all configured indexers from database settings, populating API keys from keyring/env.
    pub fn load_all(db: &Db) -> Result<Vec<Self>> {
        let json_str: Option<String> = db.with(|c| {
            let mut stmt = c.prepare("SELECT value FROM settings WHERE key = ?1")?;
            let mut rows = stmt.query(rusqlite::params![SETTINGS_KEY])?;
            if let Some(row) = rows.next()? {
                Ok(Some(row.get(0)?))
            } else {
                Ok(None)
            }
        })?;

        let mut configs: Vec<IndexerConfig> = match json_str {
            Some(s) => serde_json::from_str(&s).unwrap_or_default(),
            None => Vec::new(),
        };

        for cfg in &mut configs {
            cfg.api_key = load_api_key(&cfg.id);
        }

        Ok(configs)
    }

    /// Save the complete list of indexer configs to database settings and persist their API keys.
    pub fn save_all(db: &Db, configs: &[Self]) -> Result<()> {
        let json = serde_json::to_string(configs)?;
        db.with(|c| {
            c.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![SETTINGS_KEY, json],
            )?;
            Ok(())
        })?;

        for cfg in configs {
            if let Some(key) = &cfg.api_key {
                save_api_key(&cfg.id, key);
            }
        }

        Ok(())
    }

    /// Add or update a single indexer config.
    pub fn save(db: &Db, config: &Self) -> Result<()> {
        let mut all = Self::load_all(db)?;
        if let Some(pos) = all.iter().position(|c| c.id == config.id) {
            all[pos] = config.clone();
        } else {
            all.push(config.clone());
        }
        Self::save_all(db, &all)?;
        let _ = crate::indexer::torznab_caps::delete_cached_caps(db, &config.id);
        Ok(())
    }

    /// Remove an indexer config and its stored secret.
    pub fn remove(db: &Db, id: &str) -> Result<bool> {
        let mut all = Self::load_all(db)?;
        let initial_len = all.len();
        all.retain(|c| c.id != id);
        if all.len() != initial_len {
            Self::save_all(db, &all)?;
            delete_api_key(id);
            let _ = crate::indexer::torznab_caps::delete_cached_caps(db, id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn env_key_var(id: &str) -> String {
    format!("GENRL_INDEXER_{}_KEY", id.to_uppercase().replace('-', "_"))
}

fn keyring_key(id: &str) -> String {
    format!("indexer_key_{id}")
}

fn secrets_file_path() -> PathBuf {
    if let Ok(path) = std::env::var("GENRL_SECRETS_FILE") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    dirs::config_dir()
        .map(|d| d.join("genrl-downloader"))
        .or_else(|| dirs::data_dir().map(|d| d.join("genrl-downloader")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("secrets.json")
}

fn read_secrets_file() -> HashMap<String, String> {
    let path = secrets_file_path();
    if let Ok(bytes) = std::fs::read(path) {
        serde_json::from_slice(&bytes).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

fn write_secrets_file(map: &HashMap<String, String>) {
    let path = secrets_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_vec_pretty(map) {
        if std::fs::write(&path, json).is_ok() {
            let _ = crate::platform::set_file_permissions(&path, 0o600);
        }
    }
}

pub fn load_api_key(id: &str) -> Option<String> {
    if let Ok(val) = std::env::var(env_key_var(id)) {
        if !val.trim().is_empty() {
            return Some(val);
        }
    }
    if std::env::var("GENRL_FORCE_NO_KEYRING").as_deref() != Ok("1") {
        if let Ok(entry) = keyring::Entry::new("genrl-downloader", &keyring_key(id)) {
            if let Ok(pw) = entry.get_password() {
                if !pw.is_empty() {
                    return Some(pw);
                }
            }
        }
    }
    read_secrets_file().get(&keyring_key(id)).cloned()
}

pub fn save_api_key(id: &str, key: &str) {
    if key.is_empty() {
        return;
    }
    let mut saved_to_keyring = false;
    if std::env::var("GENRL_FORCE_NO_KEYRING").as_deref() != Ok("1") {
        if let Ok(entry) = keyring::Entry::new("genrl-downloader", &keyring_key(id)) {
            if entry.set_password(key).is_ok() {
                saved_to_keyring = true;
            }
        }
    }
    if !saved_to_keyring {
        let mut map = read_secrets_file();
        map.insert(keyring_key(id), key.to_string());
        write_secrets_file(&map);
    }
}

pub fn delete_api_key(id: &str) {
    if std::env::var("GENRL_FORCE_NO_KEYRING").as_deref() != Ok("1") {
        if let Ok(entry) = keyring::Entry::new("genrl-downloader", &keyring_key(id)) {
            let _ = entry.delete_credential();
        }
    }
    let mut map = read_secrets_file();
    if map.remove(&keyring_key(id)).is_some() {
        write_secrets_file(&map);
    }
}

pub fn load_tmdb_key() -> Option<String> {
    if let Ok(val) = std::env::var("GENRL_TMDB_KEY") {
        if !val.trim().is_empty() {
            return Some(val.trim().to_string());
        }
    }
    if let Ok(val) = std::env::var("TMDB_API_KEY") {
        if !val.trim().is_empty() {
            return Some(val.trim().to_string());
        }
    }
    load_api_key("tmdb")
}

pub fn save_tmdb_key(key: &str) {
    save_api_key("tmdb", key);
}

pub fn delete_tmdb_key() {
    delete_api_key("tmdb");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexer_config_debug_redacts_api_key() {
        let cfg = IndexerConfig::new(
            "prowlarr_main",
            "Prowlarr Main",
            "http://127.0.0.1:9696",
            Some("SUPER_SECRET_KEY_12345".to_string()),
            vec![MediaKind::Movie, MediaKind::Series],
        );
        let debug_str = format!("{cfg:?}");
        assert!(!debug_str.contains("SUPER_SECRET_KEY_12345"));
        assert!(debug_str.contains("[REDACTED]"));
    }

    #[test]
    fn test_indexer_config_serialization_omits_api_key() {
        let cfg = IndexerConfig::new(
            "prowlarr_main",
            "Prowlarr Main",
            "http://127.0.0.1:9696",
            Some("SUPER_SECRET_KEY_12345".to_string()),
            vec![MediaKind::Movie],
        );
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(!json.contains("SUPER_SECRET_KEY_12345"));
        assert!(!json.contains("api_key"));
    }

    #[test]
    fn test_indexer_config_roundtrip_db() {
        std::env::set_var("GENRL_FORCE_NO_KEYRING", "1");
        std::env::set_var("GENRL_INDEXER_PROWLARR_KEY", "env_secret_key");

        let db = Db::open_in_memory().unwrap();
        let cfg = IndexerConfig::new(
            "prowlarr",
            "Prowlarr",
            "http://127.0.0.1:9696",
            Some("env_secret_key".to_string()),
            vec![MediaKind::Movie, MediaKind::Series],
        );

        IndexerConfig::save(&db, &cfg).expect("save config");
        let loaded = IndexerConfig::load_all(&db).expect("load configs");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "prowlarr");
        assert_eq!(loaded[0].name, "Prowlarr");
        assert_eq!(loaded[0].base_url, "http://127.0.0.1:9696");
        assert_eq!(loaded[0].api_key, Some("env_secret_key".to_string()));

        IndexerConfig::remove(&db, "prowlarr").expect("remove config");
        let empty = IndexerConfig::load_all(&db).expect("load empty");
        assert!(empty.is_empty());

        std::env::remove_var("GENRL_INDEXER_PROWLARR_KEY");
        std::env::remove_var("GENRL_FORCE_NO_KEYRING");
    }

    #[test]
    fn test_validate_indexer_url_rules() {
        // Valid http/https
        assert_eq!(
            validate_indexer_url("http://127.0.0.1:9696/1/api").unwrap(),
            "http://127.0.0.1:9696/1/api"
        );
        assert_eq!(
            validate_indexer_url("https://prowlarr.myhome.org/").unwrap(),
            "https://prowlarr.myhome.org"
        );

        // Strips safe query params
        assert_eq!(
            validate_indexer_url("http://localhost:9696/api?mode=search").unwrap(),
            "http://localhost:9696/api"
        );

        // Reject non-http/https
        assert!(validate_indexer_url("ftp://example.com/api").is_err());
        assert!(validate_indexer_url("file:///etc/passwd").is_err());

        // Reject userinfo
        assert!(validate_indexer_url("http://admin:pass@localhost:9696").is_err());

        // Reject fragments
        assert!(validate_indexer_url("http://localhost:9696/api#fragment").is_err());

        // Reject query string with apikey / api_key / passkey / key
        assert!(validate_indexer_url("http://localhost:9696/api?apikey=12345").is_err());
        assert!(validate_indexer_url("http://localhost:9696/api?api_key=12345").is_err());
        assert!(validate_indexer_url("http://localhost:9696/api?passkey=12345").is_err());
        assert!(validate_indexer_url("http://localhost:9696/api?KEY=12345").is_err());

        // Reject exceeds 2048 chars
        let long_path = "a".repeat(2050);
        let long_url = format!("http://localhost:9696/{long_path}");
        assert!(validate_indexer_url(&long_url).is_err());
    }
}
