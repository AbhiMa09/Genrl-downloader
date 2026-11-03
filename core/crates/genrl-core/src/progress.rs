//! Live progress of long-running work (MAL sync, metadata), so the app can
//! show real counts and a time estimate instead of a spinner.
//!
//! Each task keeps one row in the settings state, overwritten as it moves and
//! removed when it ends. Reporting is best effort: a progress write that fails
//! never fails the work it describes.

use crate::db::Db;
use crate::settings::{read_state, write_state};

/// The MAL list sync.
pub const MAL_SYNC: &str = "mal_sync";
/// Metadata resolution started from the CLI (the setup wizard).
pub const METADATA: &str = "metadata";

fn key(task: &str) -> String {
    format!("progress.{task}")
}

/// Record where a task is: `total` is 0 while it isn't known yet.
pub fn report(db: &Db, task: &str, phase: &str, done: usize, total: usize, started_at: i64) {
    let value = serde_json::json!({
        "phase": phase,
        "done": done,
        "total": total,
        "started_at": started_at,
    });
    let _ = write_state(db, &key(task), &value.to_string());
}

/// The task ended (well or not): forget its progress.
pub fn finish(db: &Db, task: &str) {
    let full = format!("{}{}", crate::settings::STATE_PREFIX, key(task));
    let _ = db.with(|c| {
        c.execute("DELETE FROM settings WHERE key = ?1", [full])?;
        Ok(())
    });
}

/// Every task in progress, keyed by task name.
pub fn read_all(db: &Db) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for task in [MAL_SYNC, METADATA] {
        if let Ok(Some(raw)) = read_state(db, &key(task)) {
            if let Ok(value) = serde_json::from_str(&raw) {
                out.insert(task.to_string(), value);
            }
        }
    }
    serde_json::Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_then_forgets() {
        let db = Db::open_in_memory().unwrap();
        report(&db, MAL_SYNC, "relations", 3, 10, 100);
        let all = read_all(&db);
        assert_eq!(all[MAL_SYNC]["done"], 3);
        assert_eq!(all[MAL_SYNC]["total"], 10);
        finish(&db, MAL_SYNC);
        assert!(read_all(&db).get(MAL_SYNC).is_none());
    }
}
