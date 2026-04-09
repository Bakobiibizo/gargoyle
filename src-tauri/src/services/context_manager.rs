use rusqlite::{params, Connection};

use crate::error::Result;
use crate::models::operational_context::OperationalContext;

pub struct ContextManager;

impl ContextManager {
    /// Get a context value by key.
    ///
    /// Returns `Ok(None)` if the key does not exist.
    pub fn get(conn: &Connection, key: &str) -> Result<Option<OperationalContext>> {
        let mut stmt = conn.prepare(
            "SELECT context_id, context_key, context_value, updated_at, updated_by_run_id
             FROM operational_context
             WHERE context_key = ?1",
        )?;

        let mut rows = stmt.query_map(params![key], |row| {
            let value_str: String = row.get(2)?;
            Ok(OperationalContext {
                context_id: row.get(0)?,
                context_key: row.get(1)?,
                context_value: serde_json::from_str(&value_str).unwrap_or(serde_json::Value::Null),
                updated_at: row.get(3)?,
                updated_by_run_id: row.get(4)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Set a context value (upsert).
    ///
    /// If the key already exists, its value, timestamp, and run_id are updated.
    /// If the key does not exist, a new row is inserted with a fresh UUID.
    pub fn set(
        conn: &Connection,
        key: &str,
        value: &serde_json::Value,
        run_id: Option<&str>,
    ) -> Result<()> {
        let context_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let value_str = serde_json::to_string(value)?;

        conn.execute(
            "INSERT INTO operational_context (context_id, context_key, context_value, updated_at, updated_by_run_id)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(context_key) DO UPDATE SET
               context_value = excluded.context_value,
               updated_at = excluded.updated_at,
               updated_by_run_id = excluded.updated_by_run_id",
            params![context_id, key, value_str, now, run_id],
        )?;

        Ok(())
    }

    /// Delete a context key.
    ///
    /// This is a hard delete. No error is raised if the key does not exist.
    pub fn delete(conn: &Connection, key: &str) -> Result<()> {
        conn.execute(
            "DELETE FROM operational_context WHERE context_key = ?1",
            params![key],
        )?;
        Ok(())
    }

    /// List all context entries, ordered by key.
    pub fn list(conn: &Connection) -> Result<Vec<OperationalContext>> {
        let mut stmt = conn.prepare(
            "SELECT context_id, context_key, context_value, updated_at, updated_by_run_id
             FROM operational_context
             ORDER BY context_key",
        )?;

        let rows = stmt.query_map([], |row| {
            let value_str: String = row.get(2)?;
            Ok(OperationalContext {
                context_id: row.get(0)?,
                context_key: row.get(1)?,
                context_value: serde_json::from_str(&value_str).unwrap_or(serde_json::Value::Null),
                updated_at: row.get(3)?,
                updated_by_run_id: row.get(4)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::create_memory_connection;
    use crate::db::migrations::run_migrations;
    use serde_json::json;

    /// Create an in-memory DB with the full schema applied.
    fn setup_db() -> Connection {
        let conn = create_memory_connection().expect("Failed to create in-memory connection");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    #[test]
    fn test_set_and_get() {
        let conn = setup_db();

        let value = json!({"threshold": 0.85, "enabled": true});
        ContextManager::set(&conn, "alert.threshold", &value, None).unwrap();

        let ctx = ContextManager::get(&conn, "alert.threshold").unwrap();
        assert!(ctx.is_some(), "Key should exist after set");

        let ctx = ctx.unwrap();
        assert_eq!(ctx.context_key, "alert.threshold");
        assert_eq!(ctx.context_value, value);
        assert!(ctx.updated_by_run_id.is_none());
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let conn = setup_db();

        let ctx = ContextManager::get(&conn, "does.not.exist").unwrap();
        assert!(ctx.is_none(), "Non-existent key should return None");
    }

    #[test]
    fn test_set_upsert_overwrites() {
        let conn = setup_db();

        let v1 = json!("first");
        let v2 = json!("second");

        ContextManager::set(&conn, "my.key", &v1, None).unwrap();
        ContextManager::set(&conn, "my.key", &v2, Some("run-123")).unwrap();

        let ctx = ContextManager::get(&conn, "my.key").unwrap().unwrap();
        assert_eq!(ctx.context_value, v2, "Value should be overwritten");
        assert_eq!(
            ctx.updated_by_run_id,
            Some("run-123".to_string()),
            "Run ID should be updated"
        );
    }

    #[test]
    fn test_set_with_run_id() {
        let conn = setup_db();

        let value = json!(42);
        ContextManager::set(&conn, "counter", &value, Some("run-abc")).unwrap();

        let ctx = ContextManager::get(&conn, "counter").unwrap().unwrap();
        assert_eq!(ctx.updated_by_run_id, Some("run-abc".to_string()));
    }

    #[test]
    fn test_delete() {
        let conn = setup_db();

        ContextManager::set(&conn, "to.delete", &json!("bye"), None).unwrap();
        assert!(ContextManager::get(&conn, "to.delete").unwrap().is_some());

        ContextManager::delete(&conn, "to.delete").unwrap();
        assert!(
            ContextManager::get(&conn, "to.delete").unwrap().is_none(),
            "Key should be gone after delete"
        );
    }

    #[test]
    fn test_delete_nonexistent_is_ok() {
        let conn = setup_db();
        // Should not error on missing key
        let result = ContextManager::delete(&conn, "ghost.key");
        assert!(
            result.is_ok(),
            "Deleting a non-existent key should not error"
        );
    }

    #[test]
    fn test_list_empty() {
        let conn = setup_db();

        let list = ContextManager::list(&conn).unwrap();
        assert!(list.is_empty(), "Fresh DB should have no context entries");
    }

    #[test]
    fn test_list_ordered_by_key() {
        let conn = setup_db();

        // Insert in non-alphabetical order
        ContextManager::set(&conn, "zebra", &json!("z"), None).unwrap();
        ContextManager::set(&conn, "alpha", &json!("a"), None).unwrap();
        ContextManager::set(&conn, "middle", &json!("m"), None).unwrap();

        let list = ContextManager::list(&conn).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].context_key, "alpha");
        assert_eq!(list[1].context_key, "middle");
        assert_eq!(list[2].context_key, "zebra");
    }

    #[test]
    fn test_list_reflects_mutations() {
        let conn = setup_db();

        ContextManager::set(&conn, "a", &json!(1), None).unwrap();
        ContextManager::set(&conn, "b", &json!(2), None).unwrap();
        ContextManager::set(&conn, "c", &json!(3), None).unwrap();

        assert_eq!(ContextManager::list(&conn).unwrap().len(), 3);

        ContextManager::delete(&conn, "b").unwrap();
        let list = ContextManager::list(&conn).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].context_key, "a");
        assert_eq!(list[1].context_key, "c");
    }

    #[test]
    fn test_complex_json_value() {
        let conn = setup_db();

        let complex = json!({
            "nested": {
                "array": [1, 2, 3],
                "obj": {"key": "value"}
            },
            "null_field": null,
            "bool": false,
            "number": 3.14
        });

        ContextManager::set(&conn, "complex", &complex, None).unwrap();
        let ctx = ContextManager::get(&conn, "complex").unwrap().unwrap();
        assert_eq!(
            ctx.context_value, complex,
            "Complex JSON should roundtrip perfectly"
        );
    }

    #[test]
    fn test_updated_at_is_populated() {
        let conn = setup_db();

        ContextManager::set(&conn, "ts.test", &json!("value"), None).unwrap();
        let ctx = ContextManager::get(&conn, "ts.test").unwrap().unwrap();

        // The updated_at should be a non-empty ISO-8601 string
        assert!(!ctx.updated_at.is_empty(), "updated_at should be populated");
        assert!(
            ctx.updated_at.contains('T'),
            "updated_at should look like an ISO-8601 timestamp, got: {}",
            ctx.updated_at
        );
    }

    #[test]
    fn test_context_id_is_uuid() {
        let conn = setup_db();

        ContextManager::set(&conn, "uuid.test", &json!("x"), None).unwrap();
        let ctx = ContextManager::get(&conn, "uuid.test").unwrap().unwrap();

        // UUIDs have the form xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        assert_eq!(
            ctx.context_id.len(),
            36,
            "context_id should be a UUID string"
        );
        assert_eq!(
            ctx.context_id.chars().filter(|c| *c == '-').count(),
            4,
            "UUID should have 4 dashes"
        );
    }
}
