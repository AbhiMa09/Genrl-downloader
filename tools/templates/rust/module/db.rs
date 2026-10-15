use super::types::{{{ENTITY_NAME}}, {{ENTITY_NAME}}Params};
use anyhow::Result;
use rusqlite::{params, Connection};

/// Insert a new record for {{MODULE_NAME}}.
pub fn insert_{{MODULE_NAME}}(conn: &Connection, params: &{{ENTITY_NAME}}Params) -> Result<i64> {
    conn.execute(
        "INSERT INTO {{MODULE_NAME}} (name) VALUES (?1)",
        params![params.name],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Fetch a record by ID.
pub fn get_{{MODULE_NAME}}_by_id(conn: &Connection, id: i64) -> Result<Option<{{ENTITY_NAME}}>> {
    let mut stmt = conn.prepare("SELECT id, name, created_at FROM {{MODULE_NAME}} WHERE id = ?1")?;
    let mut rows = stmt.query(params![id])?;

    if let Some(row) = rows.next()? {
        Ok(Some({{ENTITY_NAME}} {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
        }))
    } else {
        Ok(None)
    }
}
