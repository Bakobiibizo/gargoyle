use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::GargoyleError;
use crate::schema::registry::SchemaRegistry;
use crate::schema::types::{
    experiment::experiment_current_version,
    metric::metric_current_version,
    result::result_current_version,
};

/// Tracks the current schema version for each entity type.
/// Provides lookup and bump logic for schema evolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Maps entity type name to its current schema version number.
    versions: HashMap<String, i32>,
}

impl SchemaVersion {
    /// Creates a new SchemaVersion initialized with the current versions
    /// for all known entity types.
    pub fn new() -> Self {
        let mut versions = HashMap::new();
        versions.insert("metric".to_string(), metric_current_version());
        versions.insert("experiment".to_string(), experiment_current_version());
        versions.insert("result".to_string(), result_current_version());
        Self { versions }
    }

    /// Returns the current schema version for the given entity type,
    /// or None if the entity type is not recognized.
    pub fn current_version(&self, entity_type: &str) -> Option<i32> {
        self.versions.get(entity_type).copied()
    }

    /// Returns all known entity types and their current versions.
    pub fn all_versions(&self) -> &HashMap<String, i32> {
        &self.versions
    }

    /// Bumps the schema version for the given entity type by 1.
    /// Returns the new version number, or None if the entity type is unknown.
    pub fn bump(&mut self, entity_type: &str) -> Option<i32> {
        if let Some(version) = self.versions.get_mut(entity_type) {
            *version += 1;
            Some(*version)
        } else {
            None
        }
    }

    /// Sets the schema version for a given entity type to a specific value.
    /// This can be used to register new entity types or set a specific version.
    pub fn set_version(&mut self, entity_type: &str, version: i32) {
        self.versions.insert(entity_type.to_string(), version);
    }

    /// Returns true if the given entity type is known to the schema system.
    pub fn has_entity_type(&self, entity_type: &str) -> bool {
        self.versions.contains_key(entity_type)
    }

    /// Returns all known entity type names.
    pub fn entity_types(&self) -> Vec<String> {
        self.versions.keys().cloned().collect()
    }
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self::new()
    }
}

/// SchemaMigrator provides migration capabilities for entities whose
/// `_schema_version` is behind the current schema version in the registry.
///
/// It uses the global `SchemaRegistry` singleton to look up current versions
/// and performs in-place updates to bring stale entities up to date.
pub struct SchemaMigrator;

impl SchemaMigrator {
    /// Migrates a single entity to the current schema version.
    ///
    /// - If the entity is already at the current version, this is a no-op.
    /// - If the entity's `_schema_version < current`, bumps it to current and
    ///   updates `updated_at`.
    /// - Returns an error if the entity is not found or the entity type is unknown.
    pub fn migrate_entity(
        conn: &rusqlite::Connection,
        entity_id: &str,
    ) -> crate::error::Result<()> {
        // Read entity's current schema version and entity type
        let (entity_type, entity_version, _updated_at): (String, i32, String) = conn
            .query_row(
                "SELECT entity_type, _schema_version, updated_at FROM entities WHERE id = ?1 AND deleted_at IS NULL",
                params![entity_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                    entity_type: "entity".to_string(),
                    id: entity_id.to_string(),
                },
                other => GargoyleError::Database(other),
            })?;

        // Look up the current schema version from the registry
        let registry = SchemaRegistry::global();
        let current_version = registry
            .current_version(&entity_type)
            .ok_or_else(|| {
                GargoyleError::Schema(format!(
                    "Unknown entity type '{}' in schema registry",
                    entity_type
                ))
            })?;

        // If already current, no-op
        if entity_version >= current_version {
            return Ok(());
        }

        // Bump the schema version and update the timestamp
        let new_updated_at = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        conn.execute(
            "UPDATE entities SET _schema_version = ?1, updated_at = ?2 WHERE id = ?3",
            params![current_version, new_updated_at, entity_id],
        )?;

        Ok(())
    }

    /// Migrates all entities of a given type that have a stale `_schema_version`.
    ///
    /// Returns the count of entities that were migrated.
    pub fn migrate_all_entities(
        conn: &rusqlite::Connection,
        entity_type: &str,
    ) -> crate::error::Result<usize> {
        let registry = SchemaRegistry::global();
        let current_version = registry
            .current_version(entity_type)
            .ok_or_else(|| {
                GargoyleError::Schema(format!(
                    "Unknown entity type '{}' in schema registry",
                    entity_type
                ))
            })?;

        // Find all stale entity IDs
        let mut stmt = conn.prepare(
            "SELECT id FROM entities WHERE entity_type = ?1 AND _schema_version < ?2 AND deleted_at IS NULL",
        )?;
        let stale_ids: Vec<String> = stmt
            .query_map(params![entity_type, current_version], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;

        let count = stale_ids.len();

        // Migrate each stale entity
        for entity_id in &stale_ids {
            let new_updated_at = chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string();
            conn.execute(
                "UPDATE entities SET _schema_version = ?1, updated_at = ?2 WHERE id = ?3",
                params![current_version, new_updated_at, entity_id],
            )?;
        }

        Ok(count)
    }

    /// Finds all entities of a given type that have a stale `_schema_version`.
    ///
    /// Returns a list of `(entity_id, current_schema_version)` pairs for stale entities.
    pub fn find_stale_entities(
        conn: &rusqlite::Connection,
        entity_type: &str,
    ) -> crate::error::Result<Vec<(String, i32)>> {
        let registry = SchemaRegistry::global();
        let current_version = registry
            .current_version(entity_type)
            .ok_or_else(|| {
                GargoyleError::Schema(format!(
                    "Unknown entity type '{}' in schema registry",
                    entity_type
                ))
            })?;

        let mut stmt = conn.prepare(
            "SELECT id, _schema_version FROM entities WHERE entity_type = ?1 AND _schema_version < ?2 AND deleted_at IS NULL",
        )?;
        let results: Vec<(String, i32)> = stmt
            .query_map(params![entity_type, current_version], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<std::result::Result<Vec<(String, i32)>, _>>()?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_has_all_entity_types() {
        let sv = SchemaVersion::new();
        assert!(sv.has_entity_type("metric"));
        assert!(sv.has_entity_type("experiment"));
        assert!(sv.has_entity_type("result"));
        assert!(!sv.has_entity_type("nonexistent"));
    }

    #[test]
    fn test_current_versions_are_1() {
        let sv = SchemaVersion::new();
        assert_eq!(sv.current_version("metric"), Some(1));
        assert_eq!(sv.current_version("experiment"), Some(1));
        assert_eq!(sv.current_version("result"), Some(1));
    }

    #[test]
    fn test_unknown_entity_type_returns_none() {
        let sv = SchemaVersion::new();
        assert_eq!(sv.current_version("widget"), None);
    }

    #[test]
    fn test_bump_increments_version() {
        let mut sv = SchemaVersion::new();
        assert_eq!(sv.bump("metric"), Some(2));
        assert_eq!(sv.current_version("metric"), Some(2));
        assert_eq!(sv.bump("metric"), Some(3));
        assert_eq!(sv.current_version("metric"), Some(3));
    }

    #[test]
    fn test_bump_unknown_returns_none() {
        let mut sv = SchemaVersion::new();
        assert_eq!(sv.bump("widget"), None);
    }

    #[test]
    fn test_set_version() {
        let mut sv = SchemaVersion::new();
        sv.set_version("metric", 5);
        assert_eq!(sv.current_version("metric"), Some(5));
    }

    #[test]
    fn test_set_version_new_type() {
        let mut sv = SchemaVersion::new();
        sv.set_version("widget", 1);
        assert!(sv.has_entity_type("widget"));
        assert_eq!(sv.current_version("widget"), Some(1));
    }

    #[test]
    fn test_entity_types_list() {
        let sv = SchemaVersion::new();
        let types = sv.entity_types();
        assert_eq!(types.len(), 3);
        assert!(types.contains(&"metric".to_string()));
        assert!(types.contains(&"experiment".to_string()));
        assert!(types.contains(&"result".to_string()));
    }

    #[test]
    fn test_default_same_as_new() {
        let sv1 = SchemaVersion::new();
        let sv2 = SchemaVersion::default();
        assert_eq!(sv1.all_versions(), sv2.all_versions());
    }

    // ========================================================================
    // SchemaMigrator tests
    // ========================================================================

    fn test_db() -> rusqlite::Connection {
        let conn = crate::db::connection::create_memory_connection().unwrap();
        crate::db::migrations::run_migrations(&conn).unwrap();
        conn
    }

    /// Helper: insert a test entity with a specific schema version.
    fn insert_entity_with_version(
        conn: &rusqlite::Connection,
        id: &str,
        entity_type: &str,
        schema_version: i32,
    ) -> String {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at) \
             VALUES (?1, ?2, ?3, '', 'manual', '{}', ?4, ?5, ?5)",
            params![id, entity_type, format!("Test {}", id), schema_version, now],
        )
        .expect("Failed to insert test entity");
        now
    }

    #[test]
    fn test_migrate_entity_already_current() {
        let conn = test_db();
        // Entity at v1, registry at v1 -> no-op
        let original_updated_at = insert_entity_with_version(&conn, "ent-current", "metric", 1);

        SchemaMigrator::migrate_entity(&conn, "ent-current").unwrap();

        // Verify version is still 1 and updated_at did NOT change
        let (version, updated_at): (i32, String) = conn
            .query_row(
                "SELECT _schema_version, updated_at FROM entities WHERE id = ?1",
                params!["ent-current"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(version, 1);
        assert_eq!(updated_at, original_updated_at);
    }

    #[test]
    fn test_migrate_entity_stale() {
        let conn = test_db();
        // Manually set entity version to 0 (stale)
        insert_entity_with_version(&conn, "ent-stale", "metric", 0);

        SchemaMigrator::migrate_entity(&conn, "ent-stale").unwrap();

        // Verify version is now 1 (current)
        let version: i32 = conn
            .query_row(
                "SELECT _schema_version FROM entities WHERE id = ?1",
                params!["ent-stale"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn test_migrate_entity_not_found() {
        let conn = test_db();
        let result = SchemaMigrator::migrate_entity(&conn, "nonexistent");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GargoyleError::NotFound { .. }
        ));
    }

    #[test]
    fn test_migrate_all_entities() {
        let conn = test_db();
        // Create 3 entities: 2 stale (v0), 1 current (v1)
        insert_entity_with_version(&conn, "stale-1", "metric", 0);
        insert_entity_with_version(&conn, "stale-2", "metric", 0);
        insert_entity_with_version(&conn, "current-1", "metric", 1);

        let count = SchemaMigrator::migrate_all_entities(&conn, "metric").unwrap();
        assert_eq!(count, 2);

        // All should now be at v1
        let stale = SchemaMigrator::find_stale_entities(&conn, "metric").unwrap();
        assert!(stale.is_empty());
    }

    #[test]
    fn test_find_stale_entities_none() {
        let conn = test_db();
        // All entities at current version
        insert_entity_with_version(&conn, "ok-1", "metric", 1);
        insert_entity_with_version(&conn, "ok-2", "metric", 1);

        let stale = SchemaMigrator::find_stale_entities(&conn, "metric").unwrap();
        assert!(stale.is_empty());
    }

    #[test]
    fn test_find_stale_entities_some() {
        let conn = test_db();
        // 2 stale, 1 current
        insert_entity_with_version(&conn, "stale-a", "experiment", 0);
        insert_entity_with_version(&conn, "stale-b", "experiment", 0);
        insert_entity_with_version(&conn, "ok-a", "experiment", 1);

        let stale = SchemaMigrator::find_stale_entities(&conn, "experiment").unwrap();
        assert_eq!(stale.len(), 2);

        let stale_ids: Vec<&str> = stale.iter().map(|(id, _)| id.as_str()).collect();
        assert!(stale_ids.contains(&"stale-a"));
        assert!(stale_ids.contains(&"stale-b"));

        // All stale entities should have version 0
        for (_, ver) in &stale {
            assert_eq!(*ver, 0);
        }
    }

    #[test]
    fn test_migration_updates_timestamp() {
        let conn = test_db();
        let original_updated_at = insert_entity_with_version(&conn, "ts-ent", "metric", 0);

        // Small sleep to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        SchemaMigrator::migrate_entity(&conn, "ts-ent").unwrap();

        let new_updated_at: String = conn
            .query_row(
                "SELECT updated_at FROM entities WHERE id = ?1",
                params!["ts-ent"],
                |row| row.get(0),
            )
            .unwrap();

        assert_ne!(
            original_updated_at, new_updated_at,
            "updated_at should change after migration"
        );
    }
}
