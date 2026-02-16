// StoreService: CRUD + patch routing + run logging

use rusqlite::{params, Connection};

use crate::error::{GargoyleError, Result};
use crate::models::entity::{Entity, Source};
use crate::models::patch::{
    CreateClaimPayload, CreateEntityPayload, CreateRelationPayload, PatchOp, PatchResult, PatchSet,
    UpdateEntityPayload,
};
use crate::models::relation::Relation;
use crate::models::run::{Run, RunStatus};
use crate::patch;

/// Parse a lowercase source string from the database into the Source enum.
fn parse_source(s: &str) -> Result<Source> {
    match s {
        "manual" => Ok(Source::Manual),
        "clipboard" => Ok(Source::Clipboard),
        "web" => Ok(Source::Web),
        "import" => Ok(Source::Import),
        "agent" => Ok(Source::Agent),
        "template" => Ok(Source::Template),
        "bootstrap" => Ok(Source::Bootstrap),
        other => Err(GargoyleError::Schema(format!(
            "Unknown source value: '{}'",
            other
        ))),
    }
}

/// Parse a lowercase run status string from the database into the RunStatus enum.
fn parse_run_status(s: &str) -> Result<RunStatus> {
    match s {
        "pending" => Ok(RunStatus::Pending),
        "applied" => Ok(RunStatus::Applied),
        "rejected" => Ok(RunStatus::Rejected),
        "partial" => Ok(RunStatus::Partial),
        other => Err(GargoyleError::Schema(format!(
            "Unknown run status value: '{}'",
            other
        ))),
    }
}

/// Parse an Entity from a rusqlite Row.
///
/// Expected column order:
///   0: id, 1: entity_type, 2: category, 3: title, 4: body_md, 5: status,
///   6: priority, 7: due_at, 8: created_at, 9: updated_at, 10: source,
///   11: canonical_fields, 12: _schema_version, 13: deleted_at, 14: provenance_run_id
fn entity_from_row(row: &rusqlite::Row) -> rusqlite::Result<Entity> {
    let source_str: String = row.get(10)?;
    let canonical_fields_str: String = row.get(11)?;

    // We map rusqlite errors for source/canonical_fields parsing into
    // rusqlite::Error::FromSqlConversionFailure so the row mapper stays pure.
    let source = parse_source(&source_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            10,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
        )
    })?;

    let canonical_fields: serde_json::Value =
        serde_json::from_str(&canonical_fields_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                11,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?;

    Ok(Entity {
        id: row.get(0)?,
        entity_type: row.get(1)?,
        category: row.get(2)?,
        title: row.get(3)?,
        body_md: row.get(4)?,
        status: row.get(5)?,
        priority: row.get(6)?,
        due_at: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        source,
        canonical_fields,
        schema_version: row.get(12)?,
        deleted_at: row.get(13)?,
        provenance_run_id: row.get(14)?,
    })
}

/// Parse a Relation from a rusqlite Row.
///
/// Expected column order:
///   0: id, 1: from_id, 2: to_id, 3: relation_type, 4: weight,
///   5: confidence, 6: provenance_run_id, 7: created_at
fn relation_from_row(row: &rusqlite::Row) -> rusqlite::Result<Relation> {
    Ok(Relation {
        id: row.get(0)?,
        from_id: row.get(1)?,
        to_id: row.get(2)?,
        relation_type: row.get(3)?,
        weight: row.get(4)?,
        confidence: row.get(5)?,
        provenance_run_id: row.get(6)?,
        created_at: row.get(7)?,
    })
}

/// Parse a Run from a rusqlite Row.
///
/// Expected column order:
///   0: run_id, 1: template_key, 2: template_version, 3: template_category,
///   4: inputs_snapshot, 5: outputs_snapshot, 6: patch_set, 7: status, 8: created_at
fn run_from_row(row: &rusqlite::Row) -> rusqlite::Result<Run> {
    let inputs_str: String = row.get(4)?;
    let outputs_str: String = row.get(5)?;
    let patch_set_str: String = row.get(6)?;
    let status_str: String = row.get(7)?;

    let inputs_snapshot: serde_json::Value =
        serde_json::from_str(&inputs_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?;

    let outputs_snapshot: serde_json::Value =
        serde_json::from_str(&outputs_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?;

    let patch_set: serde_json::Value =
        serde_json::from_str(&patch_set_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?;

    let status = parse_run_status(&status_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            7,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
        )
    })?;

    Ok(Run {
        run_id: row.get(0)?,
        template_key: row.get(1)?,
        template_version: row.get(2)?,
        template_category: row.get(3)?,
        inputs_snapshot,
        outputs_snapshot,
        patch_set,
        status,
        created_at: row.get(8)?,
    })
}

const ENTITY_SELECT_COLUMNS: &str =
    "id, entity_type, category, title, body_md, status, priority, due_at, \
     created_at, updated_at, source, canonical_fields, _schema_version, \
     deleted_at, provenance_run_id";

const RELATION_SELECT_COLUMNS: &str =
    "id, from_id, to_id, relation_type, weight, confidence, provenance_run_id, created_at";

const RUN_SELECT_COLUMNS: &str =
    "run_id, template_key, template_version, template_category, \
     inputs_snapshot, outputs_snapshot, patch_set, status, created_at";

pub struct StoreService;

impl StoreService {
    /// Apply a patch set with full validation.
    pub fn apply_patch_set(conn: &Connection, patch_set: &PatchSet) -> Result<PatchResult> {
        patch::apply_patch_set(conn, patch_set)
    }

    /// Get entity by ID (non-deleted only).
    pub fn get_entity(conn: &Connection, id: &str) -> Result<Entity> {
        let sql = format!(
            "SELECT {} FROM entities WHERE id = ?1 AND deleted_at IS NULL",
            ENTITY_SELECT_COLUMNS
        );

        conn.query_row(&sql, params![id], entity_from_row)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                    entity_type: "entity".to_string(),
                    id: id.to_string(),
                },
                other => GargoyleError::Database(other),
            })
    }

    /// List entities by type (non-deleted only).
    pub fn list_entities(
        conn: &Connection,
        entity_type: Option<&str>,
    ) -> Result<Vec<Entity>> {
        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            match entity_type {
                Some(et) => (
                    format!(
                        "SELECT {} FROM entities WHERE deleted_at IS NULL AND entity_type = ?1 ORDER BY created_at DESC",
                        ENTITY_SELECT_COLUMNS
                    ),
                    vec![Box::new(et.to_string())],
                ),
                None => (
                    format!(
                        "SELECT {} FROM entities WHERE deleted_at IS NULL ORDER BY created_at DESC",
                        ENTITY_SELECT_COLUMNS
                    ),
                    vec![],
                ),
            };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), entity_from_row)?;

        let mut entities = Vec::new();
        for row_result in rows {
            entities.push(row_result?);
        }
        Ok(entities)
    }

    /// Soft-delete an entity by setting deleted_at to the current timestamp.
    pub fn delete_entity(conn: &Connection, id: &str) -> Result<()> {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        let rows_affected = conn.execute(
            "UPDATE entities SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id],
        )?;

        if rows_affected == 0 {
            return Err(GargoyleError::NotFound {
                entity_type: "entity".to_string(),
                id: id.to_string(),
            });
        }

        Ok(())
    }

    /// Get relations for an entity (both directions).
    pub fn get_relations(conn: &Connection, entity_id: &str) -> Result<Vec<Relation>> {
        let sql = format!(
            "SELECT {} FROM relations WHERE from_id = ?1 OR to_id = ?1 ORDER BY created_at DESC",
            RELATION_SELECT_COLUMNS
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![entity_id], relation_from_row)?;

        let mut relations = Vec::new();
        for row_result in rows {
            relations.push(row_result?);
        }
        Ok(relations)
    }

    /// Log a template run by inserting it into the runs table.
    pub fn log_run(conn: &Connection, run: &Run) -> Result<()> {
        let inputs_str = serde_json::to_string(&run.inputs_snapshot)?;
        let outputs_str = serde_json::to_string(&run.outputs_snapshot)?;
        let patch_set_str = serde_json::to_string(&run.patch_set)?;
        let status_str = run.status.to_string();

        conn.execute(
            "INSERT INTO runs (run_id, template_key, template_version, template_category, \
             inputs_snapshot, outputs_snapshot, patch_set, status, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                run.run_id,
                run.template_key,
                run.template_version,
                run.template_category,
                inputs_str,
                outputs_str,
                patch_set_str,
                status_str,
                run.created_at,
            ],
        )?;

        Ok(())
    }

    /// Get run by ID.
    pub fn get_run(conn: &Connection, run_id: &str) -> Result<Run> {
        let sql = format!(
            "SELECT {} FROM runs WHERE run_id = ?1",
            RUN_SELECT_COLUMNS
        );

        conn.query_row(&sql, params![run_id], run_from_row)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                    entity_type: "run".to_string(),
                    id: run_id.to_string(),
                },
                other => GargoyleError::Database(other),
            })
    }

    /// List runs, optionally filtered by template_key.
    pub fn list_runs(
        conn: &Connection,
        template_key: Option<&str>,
    ) -> Result<Vec<Run>> {
        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            match template_key {
                Some(tk) => (
                    format!(
                        "SELECT {} FROM runs WHERE template_key = ?1 ORDER BY created_at DESC",
                        RUN_SELECT_COLUMNS
                    ),
                    vec![Box::new(tk.to_string())],
                ),
                None => (
                    format!(
                        "SELECT {} FROM runs ORDER BY created_at DESC",
                        RUN_SELECT_COLUMNS
                    ),
                    vec![],
                ),
            };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), run_from_row)?;

        let mut runs = Vec::new();
        for row_result in rows {
            runs.push(row_result?);
        }
        Ok(runs)
    }

    /// Create a full entity with validation (convenience wrapper around apply_patch_set).
    pub fn create_entity(
        conn: &Connection,
        payload: CreateEntityPayload,
    ) -> Result<PatchResult> {
        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateEntity(payload)],
            run_id: None,
        };
        patch::apply_patch_set(conn, &patch_set)
    }

    /// Update entity with validation (convenience wrapper).
    pub fn update_entity(
        conn: &Connection,
        payload: UpdateEntityPayload,
    ) -> Result<PatchResult> {
        let patch_set = PatchSet {
            ops: vec![PatchOp::UpdateEntity(payload)],
            run_id: None,
        };
        patch::apply_patch_set(conn, &patch_set)
    }

    /// Create a relation (convenience wrapper).
    pub fn create_relation(
        conn: &Connection,
        payload: CreateRelationPayload,
    ) -> Result<PatchResult> {
        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateRelation(payload)],
            run_id: None,
        };
        patch::apply_patch_set(conn, &patch_set)
    }

    /// Create a claim (convenience wrapper).
    pub fn create_claim(
        conn: &Connection,
        payload: CreateClaimPayload,
    ) -> Result<PatchResult> {
        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateClaim(payload)],
            run_id: None,
        };
        patch::apply_patch_set(conn, &patch_set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::create_memory_connection;
    use crate::db::migrations::run_migrations;
    use serde_json::json;

    /// Create a fresh in-memory database with all migrations applied.
    fn test_db() -> Connection {
        let conn = create_memory_connection().expect("Failed to create test DB");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    /// Helper: insert a test entity directly for setup purposes.
    fn insert_test_entity(
        conn: &Connection,
        id: &str,
        entity_type: &str,
        title: &str,
        source: &str,
        canonical_fields: &str,
    ) -> String {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at) \
             VALUES (?1, ?2, ?3, '', ?4, ?5, 1, ?6, ?6)",
            params![id, entity_type, title, source, canonical_fields, now],
        )
        .expect("Failed to insert test entity");
        // Also insert into FTS5 index so update/delete operations on FTS work correctly
        conn.execute(
            "INSERT INTO entities_fts(rowid, title, body_md) SELECT rowid, title, body_md FROM entities WHERE id = ?1",
            params![id],
        )
        .expect("Failed to insert test entity into FTS");
        now
    }

    /// Helper: insert a test relation directly for setup purposes.
    fn insert_test_relation(
        conn: &Connection,
        id: &str,
        from_id: &str,
        to_id: &str,
        relation_type: &str,
    ) {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "INSERT INTO relations (id, from_id, to_id, relation_type, weight, created_at) \
             VALUES (?1, ?2, ?3, ?4, 1.0, ?5)",
            params![id, from_id, to_id, relation_type, now],
        )
        .expect("Failed to insert test relation");
    }

    // ========================================================================
    // apply_patch_set
    // ========================================================================

    #[test]
    fn test_apply_patch_set_create_entity() {
        let conn = test_db();
        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "metric".to_string(),
                title: "Test Metric".to_string(),
                source: "manual".to_string(),
                canonical_fields: json!({"current_value": 42}),
                body_md: Some("A test metric".to_string()),
                status: Some("active".to_string()),
                category: None,
                priority: None,
            })],
            run_id: None,
        };

        let result = StoreService::apply_patch_set(&conn, &patch_set).unwrap();
        assert_eq!(result.applied.len(), 1);
        assert!(result.applied[0].entity_id.is_some());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_apply_patch_set_multiple_ops() {
        let conn = test_db();

        // Create two entities in one patch set
        let patch_set = PatchSet {
            ops: vec![
                PatchOp::CreateEntity(CreateEntityPayload {
                    entity_type: "metric".to_string(),
                    title: "Metric A".to_string(),
                    source: "manual".to_string(),
                    canonical_fields: json!({}),
                    body_md: None,
                    status: None,
                    category: None,
                    priority: None,
                }),
                PatchOp::CreateEntity(CreateEntityPayload {
                    entity_type: "experiment".to_string(),
                    title: "Experiment B".to_string(),
                    source: "agent".to_string(),
                    canonical_fields: json!({}),
                    body_md: None,
                    status: None,
                    category: None,
                    priority: None,
                }),
            ],
            run_id: None,
        };

        let result = StoreService::apply_patch_set(&conn, &patch_set).unwrap();
        assert_eq!(result.applied.len(), 2);
        assert_eq!(result.applied[0].op_index, 0);
        assert_eq!(result.applied[1].op_index, 1);
    }

    // ========================================================================
    // get_entity
    // ========================================================================

    #[test]
    fn test_get_entity_found() {
        let conn = test_db();
        insert_test_entity(
            &conn,
            "ent-1",
            "metric",
            "My Metric",
            "manual",
            r#"{"current_value": 100}"#,
        );

        let entity = StoreService::get_entity(&conn, "ent-1").unwrap();
        assert_eq!(entity.id, "ent-1");
        assert_eq!(entity.entity_type, "metric");
        assert_eq!(entity.title, "My Metric");
        assert_eq!(entity.source, Source::Manual);
        assert_eq!(entity.canonical_fields["current_value"], 100);
        assert_eq!(entity.schema_version, 1);
        assert!(entity.deleted_at.is_none());
    }

    #[test]
    fn test_get_entity_not_found() {
        let conn = test_db();
        let result = StoreService::get_entity(&conn, "nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            GargoyleError::NotFound { entity_type, id } => {
                assert_eq!(entity_type, "entity");
                assert_eq!(id, "nonexistent");
            }
            other => panic!("Expected NotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_get_entity_soft_deleted_not_returned() {
        let conn = test_db();
        insert_test_entity(
            &conn,
            "ent-del",
            "metric",
            "Deleted Metric",
            "manual",
            "{}",
        );
        // Soft-delete it
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "UPDATE entities SET deleted_at = ?1 WHERE id = ?2",
            params![now, "ent-del"],
        )
        .unwrap();

        let result = StoreService::get_entity(&conn, "ent-del");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GargoyleError::NotFound { .. }));
    }

    #[test]
    fn test_get_entity_parses_all_source_variants() {
        let conn = test_db();
        let sources = vec![
            "manual", "clipboard", "web", "import", "agent", "template", "bootstrap",
        ];
        let expected = vec![
            Source::Manual,
            Source::Clipboard,
            Source::Web,
            Source::Import,
            Source::Agent,
            Source::Template,
            Source::Bootstrap,
        ];

        for (i, src) in sources.iter().enumerate() {
            let id = format!("src-{}", i);
            insert_test_entity(&conn, &id, "metric", "Test", src, "{}");
            let entity = StoreService::get_entity(&conn, &id).unwrap();
            assert_eq!(entity.source, expected[i], "Source mismatch for '{}'", src);
        }
    }

    // ========================================================================
    // list_entities
    // ========================================================================

    #[test]
    fn test_list_entities_all() {
        let conn = test_db();
        insert_test_entity(&conn, "m-1", "metric", "Metric 1", "manual", "{}");
        insert_test_entity(&conn, "e-1", "experiment", "Exp 1", "manual", "{}");

        let all = StoreService::list_entities(&conn, None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_list_entities_by_type() {
        let conn = test_db();
        insert_test_entity(&conn, "m-1", "metric", "Metric 1", "manual", "{}");
        insert_test_entity(&conn, "m-2", "metric", "Metric 2", "manual", "{}");
        insert_test_entity(&conn, "e-1", "experiment", "Exp 1", "manual", "{}");

        let metrics = StoreService::list_entities(&conn, Some("metric")).unwrap();
        assert_eq!(metrics.len(), 2);
        assert!(metrics.iter().all(|e| e.entity_type == "metric"));

        let experiments = StoreService::list_entities(&conn, Some("experiment")).unwrap();
        assert_eq!(experiments.len(), 1);
    }

    #[test]
    fn test_list_entities_excludes_deleted() {
        let conn = test_db();
        insert_test_entity(&conn, "m-1", "metric", "Metric 1", "manual", "{}");
        insert_test_entity(&conn, "m-2", "metric", "Metric 2", "manual", "{}");

        // Soft-delete one
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "UPDATE entities SET deleted_at = ?1 WHERE id = 'm-2'",
            params![now],
        )
        .unwrap();

        let metrics = StoreService::list_entities(&conn, Some("metric")).unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].id, "m-1");
    }

    #[test]
    fn test_list_entities_empty() {
        let conn = test_db();
        let result = StoreService::list_entities(&conn, Some("metric")).unwrap();
        assert!(result.is_empty());
    }

    // ========================================================================
    // delete_entity
    // ========================================================================

    #[test]
    fn test_delete_entity_success() {
        let conn = test_db();
        insert_test_entity(&conn, "del-1", "metric", "To Delete", "manual", "{}");

        StoreService::delete_entity(&conn, "del-1").unwrap();

        // Should no longer be found
        let result = StoreService::get_entity(&conn, "del-1");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GargoyleError::NotFound { .. }));
    }

    #[test]
    fn test_delete_entity_not_found() {
        let conn = test_db();
        let result = StoreService::delete_entity(&conn, "nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GargoyleError::NotFound { .. }));
    }

    #[test]
    fn test_delete_entity_already_deleted() {
        let conn = test_db();
        insert_test_entity(&conn, "del-2", "metric", "Already Deleted", "manual", "{}");
        StoreService::delete_entity(&conn, "del-2").unwrap();

        // Deleting again should return NotFound
        let result = StoreService::delete_entity(&conn, "del-2");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GargoyleError::NotFound { .. }));
    }

    // ========================================================================
    // get_relations
    // ========================================================================

    #[test]
    fn test_get_relations_from_direction() {
        let conn = test_db();
        insert_test_entity(&conn, "a", "metric", "A", "manual", "{}");
        insert_test_entity(&conn, "b", "experiment", "B", "manual", "{}");
        insert_test_relation(&conn, "rel-1", "a", "b", "relates_to");

        let rels = StoreService::get_relations(&conn, "a").unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].from_id, "a");
        assert_eq!(rels[0].to_id, "b");
        assert_eq!(rels[0].relation_type, "relates_to");
    }

    #[test]
    fn test_get_relations_to_direction() {
        let conn = test_db();
        insert_test_entity(&conn, "a", "metric", "A", "manual", "{}");
        insert_test_entity(&conn, "b", "experiment", "B", "manual", "{}");
        insert_test_relation(&conn, "rel-1", "a", "b", "relates_to");

        // Query from B's perspective -- should still find the relation
        let rels = StoreService::get_relations(&conn, "b").unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].from_id, "a");
        assert_eq!(rels[0].to_id, "b");
    }

    #[test]
    fn test_get_relations_both_directions() {
        let conn = test_db();
        insert_test_entity(&conn, "a", "metric", "A", "manual", "{}");
        insert_test_entity(&conn, "b", "experiment", "B", "manual", "{}");
        insert_test_entity(&conn, "c", "result", "C", "manual", "{}");
        insert_test_relation(&conn, "rel-1", "a", "b", "relates_to");
        insert_test_relation(&conn, "rel-2", "c", "a", "derived_from");

        // A is from_id in rel-1, to_id in rel-2
        let rels = StoreService::get_relations(&conn, "a").unwrap();
        assert_eq!(rels.len(), 2);
    }

    #[test]
    fn test_get_relations_empty() {
        let conn = test_db();
        insert_test_entity(&conn, "lonely", "metric", "Lonely", "manual", "{}");
        let rels = StoreService::get_relations(&conn, "lonely").unwrap();
        assert!(rels.is_empty());
    }

    // ========================================================================
    // log_run / get_run / list_runs
    // ========================================================================

    fn make_test_run(run_id: &str, template_key: &str, status: RunStatus) -> Run {
        Run {
            run_id: run_id.to_string(),
            template_key: template_key.to_string(),
            template_version: "1.0".to_string(),
            template_category: "analytics".to_string(),
            inputs_snapshot: json!({"input_key": "value"}),
            outputs_snapshot: json!({"output_key": "result"}),
            patch_set: json!([]),
            status,
            created_at: chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string(),
        }
    }

    #[test]
    fn test_log_run_and_get_run() {
        let conn = test_db();
        let run = make_test_run("run-1", "metric_snapshot", RunStatus::Applied);

        StoreService::log_run(&conn, &run).unwrap();

        let fetched = StoreService::get_run(&conn, "run-1").unwrap();
        assert_eq!(fetched.run_id, "run-1");
        assert_eq!(fetched.template_key, "metric_snapshot");
        assert_eq!(fetched.template_version, "1.0");
        assert_eq!(fetched.template_category, "analytics");
        assert_eq!(fetched.inputs_snapshot["input_key"], "value");
        assert_eq!(fetched.outputs_snapshot["output_key"], "result");
        assert_eq!(fetched.status, RunStatus::Applied);
    }

    #[test]
    fn test_get_run_not_found() {
        let conn = test_db();
        let result = StoreService::get_run(&conn, "nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            GargoyleError::NotFound { entity_type, id } => {
                assert_eq!(entity_type, "run");
                assert_eq!(id, "nonexistent");
            }
            other => panic!("Expected NotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_list_runs_all() {
        let conn = test_db();
        let run1 = make_test_run("run-1", "metric_snapshot", RunStatus::Applied);
        let run2 = make_test_run("run-2", "experiment_runner", RunStatus::Pending);

        StoreService::log_run(&conn, &run1).unwrap();
        StoreService::log_run(&conn, &run2).unwrap();

        let all = StoreService::list_runs(&conn, None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_list_runs_by_template_key() {
        let conn = test_db();
        let run1 = make_test_run("run-1", "metric_snapshot", RunStatus::Applied);
        let run2 = make_test_run("run-2", "experiment_runner", RunStatus::Pending);
        let run3 = make_test_run("run-3", "metric_snapshot", RunStatus::Rejected);

        StoreService::log_run(&conn, &run1).unwrap();
        StoreService::log_run(&conn, &run2).unwrap();
        StoreService::log_run(&conn, &run3).unwrap();

        let metric_runs = StoreService::list_runs(&conn, Some("metric_snapshot")).unwrap();
        assert_eq!(metric_runs.len(), 2);
        assert!(metric_runs.iter().all(|r| r.template_key == "metric_snapshot"));

        let exp_runs = StoreService::list_runs(&conn, Some("experiment_runner")).unwrap();
        assert_eq!(exp_runs.len(), 1);
    }

    #[test]
    fn test_list_runs_empty() {
        let conn = test_db();
        let result = StoreService::list_runs(&conn, None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_log_run_all_status_variants() {
        let conn = test_db();
        let statuses = vec![
            RunStatus::Pending,
            RunStatus::Applied,
            RunStatus::Rejected,
            RunStatus::Partial,
        ];

        for (i, status) in statuses.iter().enumerate() {
            let run_id = format!("run-status-{}", i);
            let run = make_test_run(&run_id, "test", status.clone());
            StoreService::log_run(&conn, &run).unwrap();

            let fetched = StoreService::get_run(&conn, &run_id).unwrap();
            assert_eq!(fetched.status, *status);
        }
    }

    // ========================================================================
    // create_entity (convenience wrapper)
    // ========================================================================

    #[test]
    fn test_create_entity_convenience() {
        let conn = test_db();
        let payload = CreateEntityPayload {
            entity_type: "metric".to_string(),
            title: "Conversion Rate".to_string(),
            source: "manual".to_string(),
            canonical_fields: json!({"current_value": 0.75, "trend": "up"}),
            body_md: Some("Tracks conversion rate".to_string()),
            status: Some("active".to_string()),
            category: Some("growth".to_string()),
            priority: Some(2),
        };

        let result = StoreService::create_entity(&conn, payload).unwrap();
        assert_eq!(result.applied.len(), 1);
        let entity_id = result.applied[0].entity_id.as_ref().unwrap();

        // Verify it can be retrieved
        let entity = StoreService::get_entity(&conn, entity_id).unwrap();
        assert_eq!(entity.title, "Conversion Rate");
        assert_eq!(entity.entity_type, "metric");
        assert_eq!(entity.source, Source::Manual);
        assert_eq!(entity.body_md, "Tracks conversion rate");
        assert_eq!(entity.status, Some("active".to_string()));
        assert_eq!(entity.category, Some("growth".to_string()));
        assert_eq!(entity.priority, Some(2));
        assert_eq!(entity.canonical_fields["current_value"], 0.75);
        assert_eq!(entity.canonical_fields["trend"], "up");
    }

    // ========================================================================
    // update_entity (convenience wrapper)
    // ========================================================================

    #[test]
    fn test_update_entity_convenience() {
        let conn = test_db();
        let updated_at = insert_test_entity(
            &conn,
            "upd-1",
            "metric",
            "Old Title",
            "manual",
            r#"{"current_value": 10}"#,
        );

        let payload = UpdateEntityPayload {
            entity_id: "upd-1".to_string(),
            expected_updated_at: updated_at,
            title: Some("New Title".to_string()),
            body_md: None,
            status: None,
            canonical_fields: Some(json!({"current_value": 20})),
            category: None,
            priority: None,
            reason: None,
        };

        let result = StoreService::update_entity(&conn, payload).unwrap();
        assert_eq!(result.applied.len(), 1);
        assert_eq!(
            result.applied[0].entity_id.as_ref().unwrap(),
            "upd-1"
        );

        let entity = StoreService::get_entity(&conn, "upd-1").unwrap();
        assert_eq!(entity.title, "New Title");
        assert_eq!(entity.canonical_fields["current_value"], 20);
    }

    #[test]
    fn test_update_entity_lock_conflict() {
        let conn = test_db();
        insert_test_entity(&conn, "lock-1", "metric", "Title", "manual", "{}");

        let payload = UpdateEntityPayload {
            entity_id: "lock-1".to_string(),
            expected_updated_at: "1970-01-01T00:00:00.000Z".to_string(), // stale
            title: Some("Won't Work".to_string()),
            body_md: None,
            status: None,
            canonical_fields: None,
            category: None,
            priority: None,
            reason: None,
        };

        let result = StoreService::update_entity(&conn, payload);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GargoyleError::LockConflict { .. }));
    }

    // ========================================================================
    // create_relation (convenience wrapper)
    // ========================================================================

    #[test]
    fn test_create_relation_convenience() {
        let conn = test_db();
        insert_test_entity(&conn, "from-ent", "metric", "From", "manual", "{}");
        insert_test_entity(&conn, "to-ent", "experiment", "To", "manual", "{}");

        let payload = CreateRelationPayload {
            from_id: "from-ent".to_string(),
            to_id: "to-ent".to_string(),
            relation_type: "relates_to".to_string(),
            weight: Some(0.8),
            confidence: Some(0.9),
            provenance_run_id: None,
        };

        let result = StoreService::create_relation(&conn, payload).unwrap();
        assert_eq!(result.applied.len(), 1);
        let rel_id = result.applied[0].relation_id.as_ref().unwrap();

        // Verify the relation exists
        let rels = StoreService::get_relations(&conn, "from-ent").unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].id, *rel_id);
        assert_eq!(rels[0].relation_type, "relates_to");
        assert!((rels[0].weight - 0.8).abs() < f64::EPSILON);
        assert!((rels[0].confidence.unwrap() - 0.9).abs() < f64::EPSILON);
    }

    // ========================================================================
    // create_claim (convenience wrapper)
    // ========================================================================

    #[test]
    fn test_create_claim_convenience() {
        let conn = test_db();
        insert_test_entity(&conn, "evidence", "result", "Evidence", "manual", "{}");

        let payload = CreateClaimPayload {
            subject: "Conversion Rate".to_string(),
            predicate: "increased_by".to_string(),
            object: "15%".to_string(),
            confidence: 0.85,
            evidence_entity_id: "evidence".to_string(),
            provenance_run_id: None,
        };

        let result = StoreService::create_claim(&conn, payload).unwrap();
        assert_eq!(result.applied.len(), 1);
        assert!(result.applied[0].claim_id.is_some());
    }

    // ========================================================================
    // Integration: full workflow
    // ========================================================================

    #[test]
    fn test_full_workflow_create_list_get_delete() {
        let conn = test_db();

        // 1. Create entity
        let entity_id = "workflow-ent-1";
        insert_test_entity(
            &conn,
            entity_id,
            "metric",
            "DAU",
            "agent",
            r#"{"current_value": 1000}"#,
        );

        // 2. Verify entity is in the list
        let entities = StoreService::list_entities(&conn, Some("metric")).unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, entity_id);

        // 3. Verify get_entity
        let entity = StoreService::get_entity(&conn, entity_id).unwrap();
        assert_eq!(entity.title, "DAU");
        assert_eq!(entity.source, Source::Agent);
        assert_eq!(entity.canonical_fields["current_value"], 1000);

        // 4. Soft-delete
        StoreService::delete_entity(&conn, entity_id).unwrap();

        // 5. Verify gone from list
        let entities = StoreService::list_entities(&conn, Some("metric")).unwrap();
        assert!(entities.is_empty());

        // 6. Verify gone from get
        let get_result = StoreService::get_entity(&conn, entity_id);
        assert!(get_result.is_err());
        assert!(matches!(get_result.unwrap_err(), GargoyleError::NotFound { .. }));
    }

    #[test]
    fn test_full_workflow_create_update_get() {
        let conn = test_db();

        // 1. Create entity
        let updated_at = insert_test_entity(
            &conn,
            "wf-1",
            "metric",
            "Old Title",
            "manual",
            r#"{"current_value": 10}"#,
        );

        // 2. Update entity
        StoreService::update_entity(
            &conn,
            UpdateEntityPayload {
                entity_id: "wf-1".to_string(),
                expected_updated_at: updated_at,
                title: Some("New Title".to_string()),
                body_md: None,
                status: None,
                canonical_fields: Some(json!({"current_value": 20})),
                category: None,
                priority: None,
                reason: None,
            },
        )
        .unwrap();

        // 3. Verify updated values
        let updated = StoreService::get_entity(&conn, "wf-1").unwrap();
        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.canonical_fields["current_value"], 20);
    }

    #[test]
    fn test_full_workflow_with_run_logging() {
        let conn = test_db();

        // 1. Log a run
        let run = make_test_run("workflow-run-1", "analytics_refresh", RunStatus::Applied);
        StoreService::log_run(&conn, &run).unwrap();

        // 2. Create entity associated with run via patch set
        let patch_set = PatchSet {
            ops: vec![PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: "metric".to_string(),
                title: "Run-Created Metric".to_string(),
                source: "template".to_string(),
                canonical_fields: json!({}),
                body_md: None,
                status: None,
                category: None,
                priority: None,
            })],
            run_id: Some("workflow-run-1".to_string()),
        };

        let result = StoreService::apply_patch_set(&conn, &patch_set).unwrap();
        let entity_id = result.applied[0].entity_id.clone().unwrap();

        // 3. Verify entity has provenance_run_id
        let entity = StoreService::get_entity(&conn, &entity_id).unwrap();
        assert_eq!(entity.provenance_run_id.as_deref(), Some("workflow-run-1"));

        // 4. Verify run retrieval
        let fetched_run = StoreService::get_run(&conn, "workflow-run-1").unwrap();
        assert_eq!(fetched_run.template_key, "analytics_refresh");

        let runs = StoreService::list_runs(&conn, Some("analytics_refresh")).unwrap();
        assert_eq!(runs.len(), 1);
    }
}
