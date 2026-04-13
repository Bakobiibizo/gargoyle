use crate::error::Result;
use rusqlite::Connection;

const MIGRATION_001: &str = include_str!("../../migrations/001_initial_schema.sql");
const MIGRATION_002: &str = include_str!("../../migrations/002_wave1_entity_types.sql");
const MIGRATION_003: &str = include_str!("../../migrations/003_wave2_entity_types.sql");
const MIGRATION_004: &str = include_str!("../../migrations/004_wave3_entity_types.sql");
const MIGRATION_005: &str = include_str!("../../migrations/005_new_entity_types.sql");
const MIGRATION_006: &str = include_str!("../../migrations/006_chat_tables.sql");
const MIGRATION_007: &str = include_str!("../../migrations/007_templates_table.sql");
const MIGRATION_008: &str = include_str!("../../migrations/008_memory_tables.sql");

/// Check whether a table already has a given column.
fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&sql).unwrap();
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    names.iter().any(|n| n == column)
}

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(MIGRATION_001)?;
    conn.execute_batch(MIGRATION_002)?;
    conn.execute_batch(MIGRATION_003)?;
    conn.execute_batch(MIGRATION_004)?;
    conn.execute_batch(MIGRATION_005)?;

    // 005 addendum: add deleted_at to relations (idempotent, cannot use IF NOT EXISTS with ALTER TABLE)
    if !has_column(conn, "relations", "deleted_at") {
        conn.execute_batch("ALTER TABLE relations ADD COLUMN deleted_at TEXT;")?;
    }

    conn.execute_batch(MIGRATION_006)?;
    conn.execute_batch(MIGRATION_007)?;
    conn.execute_batch(MIGRATION_008)?;
    Ok(())
}
