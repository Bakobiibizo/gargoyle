pub mod generators;

use rusqlite::Connection;
use gargoyle_lib::db::connection::create_memory_connection;
use gargoyle_lib::db::migrations::run_migrations;

/// Create a fresh in-memory database with all migrations applied
pub fn test_db() -> Connection {
    let conn = create_memory_connection().expect("Failed to create test DB");
    run_migrations(&conn).expect("Failed to run migrations");
    conn
}

/// Insert a test entity directly (bypasses validation for test setup)
pub fn insert_test_entity(
    conn: &Connection,
    id: &str,
    entity_type: &str,
    title: &str,
    source: &str,
    canonical_fields: &str,
) -> String {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    conn.execute(
        "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
         VALUES (?1, ?2, ?3, '', ?4, ?5, 1, ?6, ?6)",
        rusqlite::params![id, entity_type, title, source, canonical_fields, now],
    ).expect("Failed to insert test entity");
    // Also insert into FTS5 index so update/delete operations on FTS work correctly
    conn.execute(
        "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = ?1",
        rusqlite::params![id],
    ).expect("Failed to insert test entity into FTS");
    now
}

/// Insert a test entity and return (id, updated_at)
pub fn insert_test_metric(conn: &Connection, id: &str, title: &str) -> String {
    insert_test_entity(conn, id, "metric", title, "manual", r#"{"current_value": 100}"#)
}

/// Insert a test experiment
pub fn insert_test_experiment(conn: &Connection, id: &str, title: &str) -> String {
    insert_test_entity(conn, id, "experiment", title, "manual", r#"{"hypothesis": "test", "primary_metric": "conversion_rate"}"#)
}

/// Insert a test result entity
pub fn insert_test_result(conn: &Connection, id: &str, title: &str) -> String {
    insert_test_entity(conn, id, "result", title, "manual", r#"{"outcome": "test"}"#)
}

/// Soft-delete an entity
pub fn soft_delete_entity(conn: &Connection, id: &str) {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    conn.execute(
        "UPDATE entities SET deleted_at = ?1 WHERE id = ?2",
        rusqlite::params![now, id],
    ).expect("Failed to soft-delete entity");
}

/// Read entity updated_at
pub fn get_updated_at(conn: &Connection, id: &str) -> String {
    conn.query_row(
        "SELECT updated_at FROM entities WHERE id = ?1",
        rusqlite::params![id],
        |row| row.get(0),
    ).expect("Failed to get updated_at")
}

/// Get entity by ID
pub fn get_entity_row(conn: &Connection, id: &str) -> Option<(String, String, String, Option<String>, String, i32)> {
    conn.query_row(
        "SELECT id, entity_type, title, status, canonical_fields, _schema_version FROM entities WHERE id = ?1",
        rusqlite::params![id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    ).ok()
}
