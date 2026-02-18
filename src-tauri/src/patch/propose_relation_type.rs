use rusqlite::params;

use crate::error::{ErrorCode, GargoyleError, ValidationError};
use crate::models::patch::ProposeRelationTypePayload;

/// Proposes a new custom relation type for approval (spec §2.6.2).
///
/// Validates:
///   - type_key must start with "custom:"
///   - type_key must not already exist in custom_relation_types
///
/// Inserts a row with approved_at = NULL (pending approval).
/// The approval step is handled by a separate command.
pub fn execute_propose_relation_type(
    conn: &rusqlite::Connection,
    payload: &ProposeRelationTypePayload,
) -> crate::error::Result<()> {
    // Validate: type_key must start with "custom:"
    if !payload.type_key.starts_with("custom:") {
        return Err(GargoyleError::Validation(ValidationError {
            code: ErrorCode::InvalidEnumValue,
            field_path: "type_key".to_string(),
            message: format!(
                "Custom relation type_key must start with 'custom:', got '{}'",
                payload.type_key
            ),
            expected: Some("custom:*".to_string()),
            actual: Some(payload.type_key.clone()),
        }));
    }

    // Validate: no duplicate
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM custom_relation_types WHERE type_key = ?1",
            params![payload.type_key],
            |row| {
                let count: i64 = row.get(0)?;
                Ok(count > 0)
            },
        )
        .map_err(GargoyleError::Database)?;

    if exists {
        return Err(GargoyleError::Schema(format!(
            "Custom relation type '{}' already exists",
            payload.type_key
        )));
    }

    // Serialize expected_from_types and expected_to_types as JSON
    let from_types_json = payload
        .expected_from_types
        .as_ref()
        .map(|v| serde_json::to_string(v))
        .transpose()?;

    let to_types_json = payload
        .expected_to_types
        .as_ref()
        .map(|v| serde_json::to_string(v))
        .transpose()?;

    // Insert with approved_at = NULL (pending approval)
    conn.execute(
        "INSERT INTO custom_relation_types (type_key, description, expected_from_types, expected_to_types, approved_at)
         VALUES (?1, ?2, ?3, ?4, NULL)",
        params![
            payload.type_key,
            payload.description,
            from_types_json,
            to_types_json,
        ],
    )?;

    Ok(())
}
