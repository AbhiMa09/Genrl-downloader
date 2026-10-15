use super::*;
use rusqlite::Connection;

#[test]
fn test_validate_{{MODULE_NAME}}_params() {
    let invalid = {{ENTITY_NAME}}Params { name: "   ".to_string() };
    assert!(validate_{{MODULE_NAME}}_params(&invalid).is_err());

    let valid = {{ENTITY_NAME}}Params { name: "valid_name".to_string() };
    assert!(validate_{{MODULE_NAME}}_params(&valid).is_ok());
}

#[test]
fn test_db_{{MODULE_NAME}}_crud() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE {{MODULE_NAME}} (id INTEGER PRIMARY KEY, name TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        [],
    ).unwrap();

    let params = {{ENTITY_NAME}}Params { name: "test_entry".to_string() };
    let id = insert_{{MODULE_NAME}}(&conn, &params).unwrap();
    let retrieved = get_{{MODULE_NAME}}_by_id(&conn, id).unwrap();

    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "test_entry");
}
