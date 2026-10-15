use crate::db::{now_unix, Db};
use crate::error::Result;

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
