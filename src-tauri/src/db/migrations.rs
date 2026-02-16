use rusqlite::Connection;
use crate::error::Result;

const MIGRATION_001: &str = include_str!("../../migrations/001_initial_schema.sql");

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(MIGRATION_001)?;
    Ok(())
}
