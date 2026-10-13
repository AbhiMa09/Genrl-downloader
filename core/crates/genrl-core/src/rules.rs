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
    /// How finished files get into the library (hardlink vs move vs copy).
    pub move_mode: Option<crate::settings::MoveMode>,
    /// Keep the torrent until it reaches this ratio.
    pub seed_ratio_target: Option<f64>,
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
            move_mode: None,
            seed_ratio_target: None,
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
        if let Some(ratio) = self.seed_ratio_target {
            if ratio < 0.0 {
                return Err(crate::Error::Other(
                    "seed_ratio_target must be non-negative".into(),
                ));
            }
        }
        Ok(())
    }
}

pub fn ensure_known_key(key: &str) -> Result<()> {
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
        "move_mode",
        "seed_ratio_target",
    ];
    if KEYS.contains(&key) {
        Ok(())
    } else {
        Err(crate::Error::Other(format!("unknown rule key: {key}")))
    }
}

/// Retrieve rules for global (`mal_id: None`) or specific show (`mal_id: Some(id)`).
pub fn get_rules(db: &Db, mal_id: Option<i64>) -> Result<Rules> {
    match mal_id {
        Some(id) => Rules::for_show(db, id),
        None => Rules::global(db),
    }
}

/// Update a single rule key and value, validating against full effective rules,
/// and logging the event in the activity log.
pub fn update_rule(
    db: &Db,
    mal_id: Option<i64>,
    key: &str,
    value: serde_json::Value,
    log: &crate::events::EventLog,
) -> Result<()> {
    ensure_known_key(key)?;
    let current = get_rules(db, mal_id)?;
    let mut obj = serde_json::to_value(&current)?;
    if let Some(map) = obj.as_object_mut() {
        map.insert(key.to_string(), value.clone());
    }
    let updated: Rules = serde_json::from_value(obj)?;
    updated.validate()?;

    let val_str = serde_json::to_string(&value)?;
    db.with(|c| {
        c.execute(
            "INSERT INTO rules (mal_id, key, value) VALUES (?1, ?2, ?3)
             ON CONFLICT DO UPDATE SET value = excluded.value",
            rusqlite::params![mal_id, key, val_str],
        )?;
        Ok(())
    })?;

    let target_desc = match mal_id {
        Some(id) => format!("show #{id}"),
        None => "global".to_string(),
    };
    let _ = log.info(
        None,
        mal_id,
        format!("Rule updated for {target_desc}: {key} = {val_str}"),
    );
    Ok(())
}

/// Reset a rule key (or all rules if `key: None`), logging the event.
pub fn reset_rule(
    db: &Db,
    mal_id: Option<i64>,
    key: Option<&str>,
    log: &crate::events::EventLog,
) -> Result<()> {
    if let Some(k) = key {
        ensure_known_key(k)?;
    }
    db.with(|c| {
        match (mal_id, key) {
            (Some(id), Some(k)) => {
                c.execute(
                    "DELETE FROM rules WHERE mal_id = ?1 AND key = ?2",
                    rusqlite::params![id, k],
                )?;
            }
            (Some(id), None) => {
                c.execute("DELETE FROM rules WHERE mal_id = ?1", [id])?;
            }
            (None, Some(k)) => {
                c.execute("DELETE FROM rules WHERE mal_id IS NULL AND key = ?1", [k])?;
            }
            (None, None) => {
                c.execute("DELETE FROM rules WHERE mal_id IS NULL", [])?;
            }
        }
        Ok(())
    })?;

    let target_desc = match mal_id {
        Some(id) => format!("show #{id}"),
        None => "global".to_string(),
    };
    let key_desc = key.unwrap_or("all rules");
    let _ = log.info(
        None,
        mal_id,
        format!("Rule reset for {target_desc}: {key_desc} restored to default"),
    );
    Ok(())
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

    #[test]
    fn editable_rules_commands_work_and_log_events() {
        let db = Db::open_in_memory().unwrap();
        let log = crate::events::EventLog::new(db.clone());
        insert_show(&db, 999, "Frieren");

        // Initial default
        let r = get_rules(&db, None).unwrap();
        assert_eq!(r.preferred_resolution, "1080p");

        // Update global rule
        update_rule(&db, None, "preferred_resolution", serde_json::json!("720p"), &log).unwrap();
        update_rule(&db, None, "seed_ratio_target", serde_json::json!(2.5), &log).unwrap();
        let updated = get_rules(&db, None).unwrap();
        assert_eq!(updated.preferred_resolution, "720p");
        assert_eq!(updated.seed_ratio_target, Some(2.5));

        // Invalid update fails validation
        let invalid = update_rule(&db, None, "seed_ask_min", serde_json::json!(50), &log);
        assert!(invalid.is_err(), "must reject invalid seed ask min > max");

        // Override on show
        update_rule(&db, Some(999), "preferred_resolution", serde_json::json!("2160p"), &log).unwrap();
        let show_rules = get_rules(&db, Some(999)).unwrap();
        assert_eq!(show_rules.preferred_resolution, "2160p");
        assert_eq!(show_rules.seed_ratio_target, Some(2.5), "inherits global ratio target");

        // Reset show override
        reset_rule(&db, Some(999), Some("preferred_resolution"), &log).unwrap();
        let show_rules_after = get_rules(&db, Some(999)).unwrap();
        assert_eq!(show_rules_after.preferred_resolution, "720p");

        // Reset global
        reset_rule(&db, None, None, &log).unwrap();
        let default_rules = get_rules(&db, None).unwrap();
        assert_eq!(default_rules.preferred_resolution, "1080p");
        assert_eq!(default_rules.seed_ratio_target, None);

        // Events logged
        assert!(log.count().unwrap() >= 5);
    }
}
