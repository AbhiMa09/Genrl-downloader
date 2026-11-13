use std::fmt;

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
            .finish()
    }
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
        Self::save_all(db, &all)
    }

    /// Remove an indexer config and its stored secret.
    pub fn remove(db: &Db, id: &str) -> Result<bool> {
        let mut all = Self::load_all(db)?;
        let initial_len = all.len();
        all.retain(|c| c.id != id);
        if all.len() != initial_len {
            Self::save_all(db, &all)?;
            delete_api_key(id);
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

fn load_api_key(id: &str) -> Option<String> {
    if let Ok(val) = std::env::var(env_key_var(id)) {
        if !val.trim().is_empty() {
            return Some(val);
        }
    }
    if std::env::var("GENRL_FORCE_NO_KEYRING").as_deref() == Ok("1") {
        return None;
    }
    let entry = keyring::Entry::new("genrl-downloader", &keyring_key(id)).ok()?;
    entry.get_password().ok()
}

fn save_api_key(id: &str, key: &str) {
    if std::env::var("GENRL_FORCE_NO_KEYRING").as_deref() == Ok("1") || key.is_empty() {
        return;
    }
    if let Ok(entry) = keyring::Entry::new("genrl-downloader", &keyring_key(id)) {
        let _ = entry.set_password(key);
    }
}

fn delete_api_key(id: &str) {
    if std::env::var("GENRL_FORCE_NO_KEYRING").as_deref() == Ok("1") {
        return;
    }
    if let Ok(entry) = keyring::Entry::new("genrl-downloader", &keyring_key(id)) {
        let _ = entry.delete_credential();
    }
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
}
