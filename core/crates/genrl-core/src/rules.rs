//! Download preferences: group priorities, resolution, size limits, seed
//! thresholds and blocked uploaders — global, with per-show overrides.
//!
//! The `rules` table stores the global rows (`mal_id IS NULL`) and any explicit
//! per-show overrides. `Rules::for_show` overlays them, so a show only ever
//! stores what it changes. Saved answers ("always do this") and aliases land
//! here too, with undo, which is the mitigation for rule poisoning.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rules {
    /// Fansub groups in priority order, highest first.
    pub groups: Vec<String>,
    /// The resolution you asked for.
    pub preferred_resolution: String,
    /// Hard floor: anything below this is dropped.
    pub min_resolution: String,
    /// Codecs you excluded (e.g. `"hevc"`).
    pub codecs_blocked: Vec<String>,
    /// Uploader names that are never downloaded.
    pub blocked_uploaders: Vec<String>,
    /// Languages you asked for; empty means no preference.
    pub preferred_languages: Vec<String>,
    /// Size sanity range for a 24-minute episode. Catches fakes and mislabeled batches.
    pub min_size_mb: u64,
    pub max_size_mb: u64,
    /// At or above this many seeders, download automatically.
    pub seed_auto_threshold: u32,
    /// Between these seed counts the app asks (or flags, for auto-list shows).
    pub seed_ask_min: u32,
    pub seed_ask_max: u32,
    /// Prefer a batch when the show has finished airing.
    pub prefer_batch_when_finished: bool,
    /// Per-show override of the Plan to watch switch.
    pub auto_plan_to_watch: Option<bool>,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            groups: Vec::new(),
            preferred_resolution: "1080p".to_string(),
            min_resolution: "720p".to_string(),
            codecs_blocked: Vec::new(),
            blocked_uploaders: Vec::new(),
            preferred_languages: Vec::new(),
            min_size_mb: 100,
            max_size_mb: 4096,
            seed_auto_threshold: 10,
            seed_ask_min: 1,
            seed_ask_max: 9,
            prefer_batch_when_finished: true,
            auto_plan_to_watch: None,
        }
    }
}

impl Rules {
    /// Global rules, or defaults when nothing has been saved yet.
    pub fn global(db: &Db) -> Result<Self> {
        let map = Self::read(db, None)?;
        Ok(Self::from_map(map)?.unwrap_or_default())
    }

    /// Save the global rules (replacing every global row).
    pub fn save_global(db: &Db, rules: &Rules) -> Result<()> {
        rules.validate()?;
        Self::write_all(db, None, rules)
    }

    /// Global rules with the show's explicit overrides applied on top.
    ///
    /// Only the keys the show actually stores are overlaid, so an override of
    /// one field never resets the rest back to defaults.
    pub fn for_show(db: &Db, mal_id: i64) -> Result<Self> {
        let mut rules = Self::global(db)?;
        if let Some(overrides) = Self::read(db, Some(mal_id))? {
            let mut merged = serde_json::to_value(&rules)?;
            for (key, value) in &overrides {
                merged[key] = value.clone();
            }
            rules = serde_json::from_value(merged)?;
        }
        Ok(rules)
    }

    /// Set one override for a show. `value` is JSON-encoded, so types round-trip.
    pub fn set_override<T: Serialize>(db: &Db, mal_id: i64, key: &str, value: T) -> Result<()> {
        ensure_known_key(key)?;
        let value = serde_json::to_string(&value)?;
        db.with(|c| {
            c.execute(
                "INSERT INTO rules (mal_id, key, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT DO UPDATE SET value = excluded.value",
                rusqlite::params![Some(mal_id), key, value],
            )?;
            Ok(())
        })
    }

    /// Remove one override, falling back to the global rule (undo).
    pub fn clear_override(db: &Db, mal_id: i64, key: &str) -> Result<()> {
        db.with(|c| {
            c.execute(
                "DELETE FROM rules WHERE mal_id = ?1 AND key = ?2",
                rusqlite::params![mal_id, key],
            )?;
            Ok(())
        })
    }

    /// Every override a show has, for the Rules page.
    pub fn overrides(db: &Db, mal_id: i64) -> Result<BTreeMap<String, String>> {
        db.with(|c| {
            let mut stmt =
                c.prepare("SELECT key, value FROM rules WHERE mal_id = ?1 ORDER BY key")?;
            let rows = stmt.query_map([mal_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
            let mut out = BTreeMap::new();
            for row in rows {
                let (k, v) = row?;
                out.insert(k, v);
            }
            Ok(out)
        })
    }

    /// Remove every override for a show.
    pub fn clear_all_overrides(db: &Db, mal_id: i64) -> Result<()> {
        db.with(|c| {
            c.execute("DELETE FROM rules WHERE mal_id = ?1", [mal_id])?;
            Ok(())
        })
    }

    fn read(db: &Db, mal_id: Option<i64>) -> Result<Option<serde_json::Map<String, serde_json::Value>>> {
        db.with(|c| {
            let mut stmt =
                c.prepare("SELECT key, value FROM rules WHERE mal_id IS ?1 ORDER BY key")?;
            let rows = stmt.query_map(rusqlite::params![mal_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            let mut map = serde_json::Map::new();
            for row in rows {
                let (key, value) = row?;
                let parsed = serde_json::from_str(&value)
                    .unwrap_or_else(|_| serde_json::Value::String(value.clone()));
                map.insert(key, parsed);
            }
            Ok(if map.is_empty() { None } else { Some(map) })
        })
    }

    fn from_map(map: Option<serde_json::Map<String, serde_json::Value>>) -> Result<Option<Rules>> {
        match map {
            None => Ok(None),
            Some(map) => Ok(Some(serde_json::from_value(
                serde_json::Value::Object(map),
            )?)),
        }
    }

    fn write_all(db: &Db, mal_id: Option<i64>, rules: &Rules) -> Result<()> {
        let value = serde_json::to_value(rules)?;
        let map = value
            .as_object()
            .ok_or_else(|| crate::Error::Other("rules did not serialize".into()))?;
        db.with(|c| {
            let tx = c.unchecked_transaction()?;
            tx.execute("DELETE FROM rules WHERE mal_id IS ?1", rusqlite::params![mal_id])?;
            for (key, val) in map {
                tx.execute(
                    "INSERT INTO rules (mal_id, key, value) VALUES (?1, ?2, ?3)",
                    rusqlite::params![mal_id, key, val.to_string()],
                )?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.seed_ask_min > self.seed_ask_max {
            return Err(crate::Error::Other(
                "seed_ask_min must not exceed seed_ask_max".into(),
            ));
        }
        if self.seed_auto_threshold < self.seed_ask_max {
            return Err(crate::Error::Other(
                "auto-download threshold must be at or above the top of the ask range".into(),
            ));
        }
        if self.min_size_mb >= self.max_size_mb {
            return Err(crate::Error::Other(
                "min_size_mb must be smaller than max_size_mb".into(),
            ));
        }
        Ok(())
    }
}

fn ensure_known_key(key: &str) -> Result<()> {
    const KEYS: &[&str] = &[
        "groups",
        "preferred_resolution",
        "min_resolution",
        "codecs_blocked",
        "blocked_uploaders",
        "preferred_languages",
        "min_size_mb",
        "max_size_mb",
        "seed_auto_threshold",
        "seed_ask_min",
        "seed_ask_max",
        "prefer_batch_when_finished",
        "auto_plan_to_watch",
    ];
    if KEYS.contains(&key) {
        Ok(())
    } else {
        Err(crate::Error::Other(format!("unknown rule key: {key}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_show(db: &Db, mal_id: i64, title: &str) {
        db.with(|c| {
            c.execute(
                "INSERT INTO shows (mal_id, title) VALUES (?1, ?2)",
                rusqlite::params![mal_id, title],
            )?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn defaults_match_the_design() {
        let r = Rules::default();
        assert_eq!(r.preferred_resolution, "1080p");
        assert_eq!(r.seed_auto_threshold, 10);
        assert_eq!(r.seed_ask_min, 1);
        assert_eq!(r.seed_ask_max, 9);
        assert_eq!(r.min_size_mb, 100);
        assert_eq!(r.max_size_mb, 4096);
        assert!(r.prefer_batch_when_finished);
    }

    #[test]
    fn global_round_trips() {
        let db = Db::open_in_memory().unwrap();
        let mut r = Rules::default();
        r.groups = vec!["SubsPlease".into(), "Erai-raws".into()];
        r.blocked_uploaders = vec!["some_uploader".into()];
        Rules::save_global(&db, &r).unwrap();
        assert_eq!(Rules::global(&db).unwrap(), r);
    }

    #[test]
    fn show_overrides_overlay_global() {
        let db = Db::open_in_memory().unwrap();
        insert_show(&db, 123, "Show A");
        insert_show(&db, 456, "Show B");

        let mut g = Rules::default();
        g.groups = vec!["SubsPlease".into()];
        Rules::save_global(&db, &g).unwrap();

        Rules::set_override(&db, 123, "min_resolution", "1080p").unwrap();
        Rules::set_override(&db, 123, "seed_auto_threshold", 20u32).unwrap();

        let show = Rules::for_show(&db, 123).unwrap();
        assert_eq!(show.min_resolution, "1080p");
        assert_eq!(show.seed_auto_threshold, 20);
        assert_eq!(show.groups, vec!["SubsPlease"], "unset keys fall back to global");

        let other = Rules::for_show(&db, 456).unwrap();
        assert_eq!(other.min_resolution, "720p");

        assert_eq!(
            Rules::overrides(&db, 123).unwrap().len(),
            2,
            "overrides are listed for undo"
        );

        Rules::clear_override(&db, 123, "min_resolution").unwrap();
        assert_eq!(Rules::for_show(&db, 123).unwrap().min_resolution, "720p");
    }

    #[test]
    fn rejects_unknown_rule_keys() {
        let db = Db::open_in_memory().unwrap();
        insert_show(&db, 1, "Show");
        assert!(Rules::set_override(&db, 1, "evil_key", 1).is_err());
    }

    #[test]
    fn validation_checks_seed_ranges() {
        let mut r = Rules::default();
        r.seed_ask_min = 20;
        r.seed_ask_max = 5;
        assert!(r.validate().is_err());
    }
}
