//! Name sets per show: the design's Step 1, and where Step 2 searches from.
//!
//! Nyaa uploaders use romaji, English, abbreviations or typos, so one title is
//! never enough. From MAL and Jikan we collect the main title, English title,
//! Japanese title and all synonyms, normalize each (lowercase, punctuation
//! gone, season words removed, full-width converted, known abbreviations
//! expanded) and store them in `show_names` with a **priority**: Step 2 asks
//! Nyaa for the names in that order — romaji first, since most fansubs use
//! it — and stops as soon as one name returns enough good candidates, which
//! keeps the request count low.
//!
//! The user's manual aliases live in the same table with the lowest priority,
//! so a Match Review answer ("none of these") is remembered for next time.

use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::Result;
use crate::filename;
use crate::models::NameSource;

/// Search priority of the main title: everything is tried after this.
pub const PRIORITY_MAIN: u8 = 0;
/// English and Japanese titles: still likely to be what a fansub used.
pub const PRIORITY_TRANSLATED: u8 = 1;
/// Synonyms from MAL/Jikan.
pub const PRIORITY_SYNONYM: u8 = 2;
/// Aliases the user added by hand, including saved Match Review answers.
pub const PRIORITY_USER: u8 = 9;

/// One stored name: as written, as compared, and where it sits in the queue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NameEntry {
    pub name: String,
    pub normalized: String,
    pub priority: u8,
    pub source: NameSource,
}

/// Normalize a name the way matching compares them: shared title
/// normalization plus expansion of the abbreviations fansubs actually use.
///
/// `"Kimi no Na wa."` becomes `kimi no na wa`; `"Re:Zero"` becomes `re zero`
/// so it can never collide with `"Re:Creators"` — the sequels-sharing-a-prefix
/// trap against `show_names` of the *other* entry.
pub fn normalize_alias(input: &str) -> String {
    let base = filename::normalize(input);
    if let Some(expansion) = ABBREVIATIONS
        .iter()
        .find(|(short, _)| *short == base)
        .map(|(_, long)| *long)
    {
        return expansion.to_string();
    }
    base
}

/// Abbreviations fansubs use where the long form is a different string.
///
/// Deliberately short: an entry only belongs here when the short form is not a
/// prefix of the long one (`Re:Zero` → `re zero` needs no help). A wrong
/// expansion poisons matching for that show, so growth needs care.
const ABBREVIATIONS: &[(&str, &str)] = &[
    ("knk", "kumo no mukou yami nakutemo"),
    (
        "oregairu",
        "yahari ore no seishun love come wa machigatteiru",
    ),
    ("snk", "shingeki no kyojin"),
    ("aot", "shingeki no kyojin"),
    ("mha", "boku no hero academia"),
    ("opm", "one punch man"),
    ("fma", "fullmetal alchemist"),
    ("fmab", "fullmetal alchemist brotherhood"),
    ("hxh", "hunter x hunter"),
    ("nnt", "non non biyori"),
    ("kyoukai", "kyoukai no kanata"),
    ("kks", "kyoukai no kanata"),
];

/// A show's whole name set, in search order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NameSet {
    pub mal_id: i64,
    pub entries: Vec<NameEntry>,
}

impl NameSet {
    /// Load a show's names from the database, lowest priority first.
    pub fn load(db: &Db, mal_id: i64) -> Result<Self> {
        let entries = db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT name, priority, source FROM show_names
                 WHERE mal_id = ?1 ORDER BY priority, id",
            )?;
            let rows = stmt.query_map([mal_id], |r| {
                let name: String = r.get(0)?;
                let priority: u8 = r.get(1)?;
                let source: String = r.get(2)?;
                Ok(NameEntry {
                    normalized: normalize_alias(&name),
                    name,
                    priority,
                    source: parse_source(&source),
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })?;
        Ok(NameSet { mal_id, entries })
    }

    /// Add a name, ignoring duplicates (`UNIQUE (mal_id, normalized)`).
    ///
    /// Returns whether the set actually grew, so a sync can say "added 3
    /// names" truthfully and a re-run is a no-op.
    pub fn add(db: &Db, mal_id: i64, name: &str, source: NameSource, priority: u8) -> Result<bool> {
        let normalized = normalize_alias(name);
        if normalized.is_empty() {
            return Ok(false);
        }
        db.with(|c| {
            let changed = c.execute(
                "INSERT INTO show_names (mal_id, name, normalized, source, priority)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (mal_id, normalized) DO NOTHING",
                rusqlite::params![mal_id, name, normalized, source.as_str(), priority],
            )?;
            Ok(changed > 0)
        })
    }

    /// Replace every name of a show with this ordered list.
    ///
    /// Used by the MAL sync: names come back from the API in a known order
    /// (main, English, Japanese, synonyms), so the whole set is rewritten with
    /// the right priorities. User aliases are preserved — MAL does not own them.
    pub fn replace_from_sync(
        db: &Db,
        mal_id: i64,
        names: &[(String, NameSource, u8)],
    ) -> Result<()> {
        db.with(|c| {
            let tx = c.unchecked_transaction()?;
            tx.execute(
                "DELETE FROM show_names WHERE mal_id = ?1 AND source != 'user'",
                [mal_id],
            )?;
            for (name, source, priority) in names {
                let normalized = normalize_alias(name);
                if normalized.is_empty() {
                    continue;
                }
                tx.execute(
                    "INSERT INTO show_names (mal_id, name, normalized, source, priority)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT (mal_id, normalized) DO NOTHING",
                    rusqlite::params![mal_id, name, normalized, source.as_str(), priority],
                )?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    /// Remove one name (the undo behind a saved alias or rule).
    pub fn remove(db: &Db, mal_id: i64, name: &str) -> Result<bool> {
        let normalized = normalize_alias(name);
        db.with(|c| {
            let changed = c.execute(
                "DELETE FROM show_names WHERE mal_id = ?1 AND normalized = ?2",
                rusqlite::params![mal_id, normalized],
            )?;
            Ok(changed > 0)
        })
    }

    /// Every alias the user added by hand — shown in Show detail for undo.
    pub fn user_aliases(db: &Db, mal_id: i64) -> Result<Vec<String>> {
        db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT name FROM show_names WHERE mal_id = ?1 AND source = 'user'
                 ORDER BY priority, id",
            )?;
            let rows = stmt.query_map([mal_id], |r| r.get::<_, String>(0))?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    /// Normalized names in search order: romaji/main first, user last.
    pub fn search_order(&self) -> Vec<String> {
        let mut names: Vec<String> = self.entries.iter().map(|e| e.normalized.clone()).collect();
        names.dedup();
        names
    }

    /// Is this name one of ours?
    pub fn contains(&self, normalized: &str) -> bool {
        self.entries.iter().any(|e| e.normalized == normalized)
    }

    /// A one-line summary for the Activity log, e.g. `3 names (romaji, en)`.
    pub fn summary(&self) -> String {
        let kinds: Vec<&str> = self
            .entries
            .iter()
            .map(|e| match e.priority {
                PRIORITY_MAIN => "romaji",
                PRIORITY_TRANSLATED => "translated",
                PRIORITY_SYNONYM => "synonym",
                _ => "alias",
            })
            .fold(Vec::new(), |mut acc, kind| {
                if !acc.contains(&kind) {
                    acc.push(kind);
                }
                acc
            });
        format!("{} name(s) ({})", self.entries.len(), kinds.join(", "))
    }
}

fn parse_source(raw: &str) -> NameSource {
    match raw {
        "mal" => NameSource::Mal,
        "jikan" => NameSource::Jikan,
        _ => NameSource::User,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_with_show(mal_id: i64) -> Db {
        let db = Db::open_in_memory().unwrap();
        db.with(|c| {
            c.execute(
                "INSERT INTO shows (mal_id, title, type, status) VALUES (?1, ?2, 'tv', 'finished')",
                rusqlite::params![mal_id, "Show"],
            )?;
            Ok(())
        })
        .unwrap();
        db
    }

    #[test]
    fn normalization_collapses_the_ways_fansubs_write_names() {
        // The design's own example.
        assert_eq!(normalize_alias("Kimi no Na wa."), "kimi no na wa");
        // Full-width input and a stray ampersand.
        assert_eq!(normalize_alias("Ｒｅ：Ｚｅｒｏ"), "re zero");
        assert_eq!(normalize_alias("Re:Zero"), "re zero");
        // Season words go; the season number stays (it identifies the season).
        assert_eq!(normalize_alias("Frieren 2nd Season"), "frieren 2");
        // "Kimi no Todoke" is a *different* show and must stay different.
        assert_ne!(
            normalize_alias("Kimi no Na wa"),
            normalize_alias("Kimi no Todoke")
        );
    }

    #[test]
    fn known_abbreviations_expand() {
        assert_eq!(
            normalize_alias("Oregairu"),
            "yahari ore no seishun love come wa machigatteiru"
        );
        assert_eq!(normalize_alias("SnK"), "shingeki no kyojin");
        // Not in the table: normalization only, never invented.
        assert_eq!(normalize_alias("Some Show"), "some show");
    }

    #[test]
    fn names_round_trip_in_priority_order() {
        let db = db_with_show(10);
        assert!(NameSet::add(&db, 10, "Kimi no Na wa", NameSource::Mal, PRIORITY_MAIN).unwrap());
        assert!(NameSet::add(&db, 10, "Your Name", NameSource::Mal, PRIORITY_TRANSLATED).unwrap());
        assert!(NameSet::add(&db, 10, "君の名は。", NameSource::Mal, PRIORITY_SYNONYM).unwrap());
        // Adding the same name twice does not grow the set.
        assert!(!NameSet::add(&db, 10, "your name", NameSource::Mal, PRIORITY_MAIN).unwrap());

        let set = NameSet::load(&db, 10).unwrap();
        assert_eq!(
            set.search_order().first().map(String::as_str),
            Some("kimi no na wa")
        );
        assert!(set.contains("kimi no na wa"));
        assert!(!set.contains("kimi no todoke"));
        assert_eq!(set.summary(), "3 name(s) (romaji, translated, synonym)");
    }

    #[test]
    fn sync_rewrites_mal_names_but_keeps_user_aliases() {
        let db = db_with_show(21);
        NameSet::add(
            &db,
            21,
            "the name I know it by",
            NameSource::User,
            PRIORITY_USER,
        )
        .unwrap();

        let names = vec![
            (
                "Sousou no Frieren".to_string(),
                NameSource::Mal,
                PRIORITY_MAIN,
            ),
            (
                "Frieren: Beyond Journey's End".to_string(),
                NameSource::Jikan,
                PRIORITY_TRANSLATED,
            ),
        ];
        NameSet::replace_from_sync(&db, 21, &names).unwrap();

        let set = NameSet::load(&db, 21).unwrap();
        assert_eq!(
            set.search_order(),
            vec![
                "sousou no frieren".to_string(),
                "frieren beyond journey s end".to_string(),
                "name i know it by".to_string(), // "the" is a season word
            ]
        );
        assert_eq!(
            NameSet::user_aliases(&db, 21).unwrap(),
            vec!["the name I know it by".to_string()]
        );

        // A re-sync is idempotent.
        NameSet::replace_from_sync(&db, 21, &names).unwrap();
        assert_eq!(NameSet::load(&db, 21).unwrap().entries.len(), 3);
    }

    #[test]
    fn removals_are_per_show_and_normalized() {
        let db = db_with_show(30);
        db_with_show(31);
        NameSet::add(&db, 30, "Re:Zero", NameSource::User, PRIORITY_USER).unwrap();
        NameSet::add(&db, 30, "Re:Creators", NameSource::User, PRIORITY_USER).unwrap();

        // Removing by a differently-spelled name still hits the stored row.
        assert!(NameSet::remove(&db, 30, "re zero").unwrap());
        assert!(!NameSet::remove(&db, 30, "never there").unwrap());
        let set = NameSet::load(&db, 30).unwrap();
        assert_eq!(set.search_order(), vec!["re creators"]);
    }
}
