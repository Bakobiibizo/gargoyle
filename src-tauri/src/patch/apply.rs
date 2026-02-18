use crate::error::{ErrorCode, GargoyleError, ValidationError};
use crate::models::patch::{AppliedOp, PatchOp, PatchResult, PatchSet};
use crate::patch::attach_artifact::execute_attach_artifact;
use crate::patch::create_claim::execute_create_claim;
use crate::patch::create_entity::execute_create_entity;
use crate::patch::create_relation::execute_create_relation;
use crate::patch::delete_relation::execute_delete_relation;
use crate::patch::merge_entities::execute_merge_entities;
use crate::patch::promote_claim_op::execute_promote_claim;
use crate::patch::propose_relation_type::execute_propose_relation_type;
use crate::patch::update_context_op::execute_update_context_with_run_id;
use crate::patch::update_entity::execute_update_entity;
use crate::schema::registry::SchemaRegistry;
use crate::services::dedup::DedupPipeline;
use crate::validation::{
    validate_create_claim, validate_create_entity, validate_create_relation,
    validate_update_entity, ValidationOutput,
};

/// Applies a PatchSet atomically within a database transaction.
///
/// Iterates over each op in the PatchSet, executes it, and records
/// the result. If any op fails, the entire transaction is rolled back
/// and the error is returned.
///
/// Each op is validated through the 4-step validation pipeline before
/// execution: schema validation, status transitions, lock checks, and
/// referential integrity.
pub fn apply_patch_set(
    conn: &rusqlite::Connection,
    patch_set: &PatchSet,
) -> crate::error::Result<PatchResult> {
    let mut result = PatchResult {
        applied: Vec::new(),
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    conn.execute_batch("BEGIN TRANSACTION")?;

    for (index, op) in patch_set.ops.iter().enumerate() {
        let apply_result = apply_single_op(conn, op, index, Some(&patch_set.run_id));

        match apply_result {
            Ok((applied_op, warnings)) => {
                result.applied.push(applied_op);
                result.warnings.extend(warnings);
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

/// Converts a non-empty `Vec<ValidationError>` into the first error wrapped as
/// `GargoyleError::Validation`. Returns `Ok(())` if the list is empty.
fn fail_on_validation_errors(errors: Vec<ValidationError>) -> crate::error::Result<()> {
    if let Some(first) = errors.into_iter().next() {
        Err(GargoyleError::Validation(first))
    } else {
        Ok(())
    }
}

/// Checks a `ValidationOutput` for hard errors. If there are any, returns the
/// first one as `GargoyleError::Validation`. Otherwise returns the list of
/// warnings for the caller to propagate.
fn check_validation_output(output: ValidationOutput) -> crate::error::Result<Vec<String>> {
    fail_on_validation_errors(output.errors)?;
    Ok(output.warnings)
}

/// Builds an `EntityLookup` closure backed by the given database connection.
///
/// For each entity ID the closure queries the `entities` table and returns
/// `Some((entity_type, deleted_at))` when found, or `None` when the entity
/// does not exist at all.
fn build_entity_lookup(
    conn: &rusqlite::Connection,
) -> impl Fn(&str) -> Option<(String, Option<String>)> + '_ {
    move |id: &str| {
        conn.query_row(
            "SELECT entity_type, deleted_at FROM entities WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok()
    }
}

/// Loads the list of approved custom relation type keys from the
/// `custom_relation_types` table. Returns an empty vec if the table is
/// empty or does not exist (defensive).
fn load_approved_custom_relation_types(conn: &rusqlite::Connection) -> Vec<String> {
    let mut stmt = match conn.prepare("SELECT type_key FROM custom_relation_types WHERE approved_at IS NOT NULL") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |row| row.get(0))
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

/// Applies a single PatchOp and returns the corresponding AppliedOp along
/// with any validation warnings.
///
/// Each op is run through the validation pipeline before being executed.
/// If validation fails, the first validation error is returned immediately
/// and the op is never executed.
fn apply_single_op(
    conn: &rusqlite::Connection,
    op: &PatchOp,
    op_index: usize,
    run_id: Option<&str>,
) -> crate::error::Result<(AppliedOp, Vec<String>)> {
    let registry = SchemaRegistry::global();
    let lookup = build_entity_lookup(conn);

    match op {
        PatchOp::CreateEntity(payload) => {
            // --- Validation pipeline ---
            let schema_version = resolve_current_schema_version(&payload.entity_type);
            let field_defs = registry
                .get_schema(&payload.entity_type, schema_version)
                .unwrap_or_default();
            let output = validate_create_entity(
                &payload.entity_type,
                &payload.canonical_fields,
                &field_defs,
                payload.status.as_deref(),
                &lookup,
            );
            let warnings = check_validation_output(output)?;

            // --- Execution ---
            let entity_id = execute_create_entity(conn, payload, run_id)?;
            Ok((
                AppliedOp {
                    op_index,
                    entity_id: Some(entity_id),
                    relation_id: None,
                    claim_id: None,
                },
                warnings,
            ))
        }
        PatchOp::UpdateEntity(payload) => {
            // Version-aware validation: check if entity needs migration before update
            check_entity_schema_version(conn, &payload.entity_id)?;

            // Read entity metadata needed for validation and execution
            let (entity_type, current_status, actual_updated_at): (
                String,
                Option<String>,
                String,
            ) = conn
                .query_row(
                    "SELECT entity_type, status, updated_at FROM entities WHERE id = ?1 AND deleted_at IS NULL",
                    rusqlite::params![payload.entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                        entity_type: "entity".to_string(),
                        id: payload.entity_id.clone(),
                    },
                    other => GargoyleError::Database(other),
                })?;

            let current_schema_version = resolve_current_schema_version(&entity_type);
            let field_defs = registry
                .get_schema(&entity_type, current_schema_version)
                .unwrap_or_default();

            // --- Validation pipeline ---
            let output = validate_update_entity(
                &entity_type,
                payload.canonical_fields.as_ref(),
                &field_defs,
                current_status.as_deref(),
                payload.status.as_deref(),
                payload.reason.as_deref(),
                &payload.expected_updated_at,
                &actual_updated_at,
                &lookup,
            );
            let warnings = check_validation_output(output)?;

            // --- Execution ---
            // NOTE: execute_update_entity also performs its own optimistic lock
            // check (defense-in-depth). The validation pipeline above already
            // validates expected_updated_at via validate_lock, but the execute
            // function retains its own check as a safety net.
            let _new_updated_at =
                execute_update_entity(conn, payload, current_schema_version)?;
            Ok((
                AppliedOp {
                    op_index,
                    entity_id: Some(payload.entity_id.clone()),
                    relation_id: None,
                    claim_id: None,
                },
                warnings,
            ))
        }
        PatchOp::CreateRelation(payload) => {
            // --- Validation pipeline ---
            let approved_custom_types = load_approved_custom_relation_types(conn);
            let errors = validate_create_relation(
                &payload.from_id,
                &payload.to_id,
                &payload.relation_type,
                &approved_custom_types,
                &lookup,
            );
            fail_on_validation_errors(errors)?;

            // --- Execution ---
            let relation_id = execute_create_relation(conn, payload)?;
            Ok((
                AppliedOp {
                    op_index,
                    entity_id: None,
                    relation_id: Some(relation_id),
                    claim_id: None,
                },
                Vec::new(),
            ))
        }
        PatchOp::CreateClaim(payload) => {
            // --- Validation pipeline ---
            let errors = validate_create_claim(
                &payload.evidence_entity_id,
                &lookup,
            );
            fail_on_validation_errors(errors)?;

            // --- Execution ---
            let claim_id = execute_create_claim(conn, payload)?;
            Ok((
                AppliedOp {
                    op_index,
                    entity_id: None,
                    relation_id: None,
                    claim_id: Some(claim_id),
                },
                Vec::new(),
            ))
        }
        PatchOp::DeleteRelation(payload) => {
            let relation_id = execute_delete_relation(conn, payload)?;
            Ok((
                AppliedOp {
                    op_index,
                    entity_id: None,
                    relation_id: Some(relation_id),
                    claim_id: None,
                },
                Vec::new(),
            ))
        }
        PatchOp::AttachArtifact(payload) => {
            let entity_id = execute_attach_artifact(conn, payload)?;
            Ok((
                AppliedOp {
                    op_index,
                    entity_id: Some(entity_id),
                    relation_id: None,
                    claim_id: None,
                },
                Vec::new(),
            ))
        }
        PatchOp::MergeEntities(payload) => {
            let entity_id = execute_merge_entities(conn, payload)?;
            Ok((
                AppliedOp {
                    op_index,
                    entity_id: Some(entity_id),
                    relation_id: None,
                    claim_id: None,
                },
                Vec::new(),
            ))
        }
        PatchOp::UpdateContext(payload) => {
            execute_update_context_with_run_id(conn, payload, run_id)?;
            Ok((
                AppliedOp {
                    op_index,
                    entity_id: None,
                    relation_id: None,
                    claim_id: None,
                },
                Vec::new(),
            ))
        }
        PatchOp::PromoteClaim(payload) => {
            let entity_id = execute_promote_claim(conn, payload)?;
            Ok((
                AppliedOp {
                    op_index,
                    entity_id: Some(entity_id),
                    relation_id: None,
                    claim_id: None,
                },
                Vec::new(),
            ))
        }
        PatchOp::ProposeRelationType(payload) => {
            execute_propose_relation_type(conn, payload)?;
            Ok((
                AppliedOp {
                    op_index,
                    entity_id: None,
                    relation_id: None,
                    claim_id: None,
                },
                Vec::new(),
            ))
        }
    }
}
