use crate::error::{ErrorCode, GargoyleError, ValidationError};
use crate::models::patch::{AppliedOp, PatchOp, PatchResult, PatchSet};
use crate::patch::create_claim::execute_create_claim;
use crate::patch::create_entity::execute_create_entity;
use crate::patch::create_relation::execute_create_relation;
use crate::patch::update_entity::execute_update_entity;
use crate::schema::registry::SchemaRegistry;
use crate::services::dedup::DedupPipeline;

/// Applies a PatchSet atomically within a database transaction.
///
/// Iterates over each op in the PatchSet, executes it, and records
/// the result. If any op fails, the entire transaction is rolled back
/// and the error is returned.
///
/// Validation wiring (schema registry, entity lookup, etc.) will be
/// added in a later phase.
pub fn apply_patch_set(
    conn: &rusqlite::Connection,
    patch_set: &PatchSet,
) -> crate::error::Result<PatchResult> {
    let mut result = PatchResult {
        applied: Vec::new(),
        errors: Vec::new(),
    };

    conn.execute_batch("BEGIN TRANSACTION")?;

    for (index, op) in patch_set.ops.iter().enumerate() {
        let apply_result = apply_single_op(conn, op, index, patch_set.run_id.as_deref());

        match apply_result {
            Ok(applied_op) => {
                result.applied.push(applied_op);
            }
            Err(e) => {
                // Rollback the entire transaction on any failure
                let _ = conn.execute_batch("ROLLBACK");
                return Err(e);
            }
        }
    }

    conn.execute_batch("COMMIT")?;

    // Post-commit: run dedup pipeline for any newly created entities.
    // This is non-blocking -- dedup failures are swallowed and never
    // prevent entity creation from succeeding.
    for applied_op in &result.applied {
        if let Some(ref entity_id) = applied_op.entity_id {
            // Only run dedup for CreateEntity ops (not updates).
            // We check the op type to avoid running dedup on updates.
            if let Some(PatchOp::CreateEntity(_)) = patch_set.ops.get(applied_op.op_index) {
                let _ = DedupPipeline::check_for_duplicates(conn, entity_id);
            }
        }
    }

    Ok(result)
}

/// Resolves the current schema version for the given entity type from the
/// global SchemaRegistry. Falls back to 1 if the entity type is unknown.
fn resolve_current_schema_version(entity_type: &str) -> i32 {
    SchemaRegistry::global()
        .current_version(entity_type)
        .unwrap_or(1)
}

/// Checks whether the entity's `_schema_version` in the database is behind the
/// current version in the schema registry. If so, returns a SchemaVersionMismatch
/// validation error indicating the entity needs migration before update.
fn check_entity_schema_version(
    conn: &rusqlite::Connection,
    entity_id: &str,
) -> crate::error::Result<()> {
    // Read entity's type and schema version
    let (entity_type, entity_version): (String, i32) = conn
        .query_row(
            "SELECT entity_type, _schema_version FROM entities WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![entity_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                entity_type: "entity".to_string(),
                id: entity_id.to_string(),
            },
            other => GargoyleError::Database(other),
        })?;

    let current_version = resolve_current_schema_version(&entity_type);

    if entity_version < current_version {
        return Err(GargoyleError::Validation(ValidationError {
            code: ErrorCode::SchemaVersionMismatch,
            field_path: "_schema_version".to_string(),
            message: format!(
                "Entity '{}' has schema version {} but current is {}. Migration required before update.",
                entity_id, entity_version, current_version
            ),
            expected: Some(current_version.to_string()),
            actual: Some(entity_version.to_string()),
        }));
    }

    Ok(())
}

/// Applies a single PatchOp and returns the corresponding AppliedOp.
fn apply_single_op(
    conn: &rusqlite::Connection,
    op: &PatchOp,
    op_index: usize,
    run_id: Option<&str>,
) -> crate::error::Result<AppliedOp> {
    match op {
        PatchOp::CreateEntity(payload) => {
            let entity_id = execute_create_entity(conn, payload, run_id)?;
            Ok(AppliedOp {
                op_index,
                entity_id: Some(entity_id),
                relation_id: None,
                claim_id: None,
            })
        }
        PatchOp::UpdateEntity(payload) => {
            // Version-aware validation: check if entity needs migration before update
            check_entity_schema_version(conn, &payload.entity_id)?;

            // Read entity type to resolve current schema version dynamically
            let entity_type: String = conn
                .query_row(
                    "SELECT entity_type FROM entities WHERE id = ?1 AND deleted_at IS NULL",
                    rusqlite::params![payload.entity_id],
                    |row| row.get(0),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                        entity_type: "entity".to_string(),
                        id: payload.entity_id.clone(),
                    },
                    other => GargoyleError::Database(other),
                })?;

            let current_schema_version = resolve_current_schema_version(&entity_type);
            let _new_updated_at =
                execute_update_entity(conn, payload, current_schema_version)?;
            Ok(AppliedOp {
                op_index,
                entity_id: Some(payload.entity_id.clone()),
                relation_id: None,
                claim_id: None,
            })
        }
        PatchOp::CreateRelation(payload) => {
            let relation_id = execute_create_relation(conn, payload)?;
            Ok(AppliedOp {
                op_index,
                entity_id: None,
                relation_id: Some(relation_id),
                claim_id: None,
            })
        }
        PatchOp::CreateClaim(payload) => {
            let claim_id = execute_create_claim(conn, payload)?;
            Ok(AppliedOp {
                op_index,
                entity_id: None,
                relation_id: None,
                claim_id: Some(claim_id),
            })
        }
    }
}
