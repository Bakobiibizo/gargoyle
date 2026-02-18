use rusqlite::params;

use crate::error::{ErrorCode, GargoyleError, ValidationError};
use crate::models::patch::MergeEntitiesPayload;

/// Merges a source entity into a target entity.
///
/// Steps:
/// 1. Confirmation gate -- if `confirmed` is not `true`, return a validation
///    error indicating that the caller must confirm the merge.
/// 2. Validate both source and target exist and are not soft-deleted.
/// 3. Move all relations from source to target (update from_id / to_id).
/// 4. Merge canonical_fields JSON (target wins on key conflict).
/// 5. Soft-delete the source entity.
/// 6. Create a `duplicate_of` relation from source to target.
///
/// Returns the target entity ID on success.
pub fn execute_merge_entities(
    conn: &rusqlite::Connection,
    payload: &MergeEntitiesPayload,
) -> crate::error::Result<String> {
    // --- 1. Confirmation gate ---
    if payload.confirmed != Some(true) {
        return Err(GargoyleError::Validation(ValidationError {
            code: ErrorCode::MissingRequiredField,
            field_path: "confirmed".to_string(),
            message: format!(
                "Merge of '{}' into '{}' requires explicit confirmation. \
                 Set confirmed: true to proceed.",
                payload.source_id, payload.target_id
            ),
            expected: Some("true".to_string()),
            actual: Some(
                payload
                    .confirmed
                    .map(|b| b.to_string())
                    .unwrap_or_else(|| "null".to_string()),
            ),
        }));
    }

    // --- 2. Validate source and target entities ---
    let (source_type, source_fields_str): (String, String) = conn
        .query_row(
            "SELECT entity_type, canonical_fields FROM entities WHERE id = ?1 AND deleted_at IS NULL",
            params![payload.source_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                entity_type: "entity".to_string(),
                id: payload.source_id.clone(),
            },
            other => GargoyleError::Database(other),
        })?;

    let (target_type, target_fields_str): (String, String) = conn
        .query_row(
            "SELECT entity_type, canonical_fields FROM entities WHERE id = ?1 AND deleted_at IS NULL",
            params![payload.target_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                entity_type: "entity".to_string(),
                id: payload.target_id.clone(),
            },
            other => GargoyleError::Database(other),
        })?;

    // Entities must be of the same type
    if source_type != target_type {
        return Err(GargoyleError::Validation(ValidationError {
            code: ErrorCode::InvalidFieldType,
            field_path: "entity_type".to_string(),
            message: format!(
                "Cannot merge entities of different types: source is '{}', target is '{}'",
                source_type, target_type
            ),
            expected: Some(target_type),
            actual: Some(source_type),
        }));
    }

    // --- 3. Move relations from source to target ---
    // Update relations where source is the from_id (skip self-referential)
    conn.execute(
        "UPDATE relations SET from_id = ?1 WHERE from_id = ?2 AND to_id != ?1",
        params![payload.target_id, payload.source_id],
    )?;

    // Update relations where source is the to_id (skip self-referential)
    conn.execute(
        "UPDATE relations SET to_id = ?1 WHERE to_id = ?2 AND from_id != ?1",
        params![payload.target_id, payload.source_id],
    )?;

    // Delete any relations that would now be self-referential
    // (i.e., relations that were between source and target)
    conn.execute(
        "DELETE FROM relations WHERE from_id = ?1 AND to_id = ?1",
        params![payload.target_id],
    )?;

    // --- 4. Merge canonical_fields (target wins on conflict) ---
    let source_fields: serde_json::Value =
        serde_json::from_str(&source_fields_str).unwrap_or(serde_json::Value::Object(Default::default()));
    let target_fields: serde_json::Value =
        serde_json::from_str(&target_fields_str).unwrap_or(serde_json::Value::Object(Default::default()));

    let merged = merge_json_objects(&source_fields, &target_fields);
    let merged_str = serde_json::to_string(&merged)?;

    let now = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    conn.execute(
        "UPDATE entities SET canonical_fields = ?1, updated_at = ?2 WHERE id = ?3",
        params![merged_str, now, payload.target_id],
    )?;

    // --- 5. Soft-delete source entity ---
    conn.execute(
        "UPDATE entities SET deleted_at = ?1 WHERE id = ?2",
        params![now, payload.source_id],
    )?;

    // --- 6. Create duplicate_of relation from source to target ---
    let relation_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO relations (id, from_id, to_id, relation_type, weight, created_at)
         VALUES (?1, ?2, ?3, 'duplicate_of', 1.0, ?4)",
        params![relation_id, payload.source_id, payload.target_id, now],
    )?;

    Ok(payload.target_id.clone())
}

/// Merges two JSON objects. Source fields are used as a base, then target
/// fields are overlaid -- target wins on any key conflict.
fn merge_json_objects(source: &serde_json::Value, target: &serde_json::Value) -> serde_json::Value {
    match (source, target) {
        (serde_json::Value::Object(src), serde_json::Value::Object(tgt)) => {
            let mut merged = src.clone();
            for (key, value) in tgt {
                merged.insert(key.clone(), value.clone());
            }
            serde_json::Value::Object(merged)
        }
        // If either isn't an object, target wins entirely
        _ => target.clone(),
    }
}
