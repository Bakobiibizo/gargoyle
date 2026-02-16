use rusqlite::Connection;
use crate::error::Result;

pub fn create_connection(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
    ")?;
    Ok(conn)
}

pub fn create_memory_connection() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
    ")?;
    Ok(conn)
}
