use rusqlite::Connection;
use crate::error::Result;

const MIGRATION_001: &str = include_str!("../../migrations/001_initial_schema.sql");
const MIGRATION_002: &str = include_str!("../../migrations/002_wave1_entity_types.sql");
const MIGRATION_003: &str = include_str!("../../migrations/003_wave2_entity_types.sql");
const MIGRATION_004: &str = include_str!("../../migrations/004_wave3_entity_types.sql");

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(MIGRATION_001)?;
    conn.execute_batch(MIGRATION_002)?;
    conn.execute_batch(MIGRATION_003)?;
    conn.execute_batch(MIGRATION_004)?;
    Ok(())
}
