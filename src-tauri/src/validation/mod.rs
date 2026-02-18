pub mod schema_validator;
pub mod status_validator;
pub mod lock_validator;
pub mod referential_validator;

use crate::error::ValidationError;
use crate::schema::field_def::FieldDef;
use serde_json::Value;

use self::lock_validator::validate_lock;
use self::referential_validator::{
    validate_entity_exists, validate_entity_refs, validate_relation_refs,
};
use self::schema_validator::validate_canonical_fields;
use self::status_validator::validate_status_transition;

/// The 4-step validation pipeline for Gargoyle patch operations.
///
/// Steps:
/// 1. Schema validation - canonical_fields against field definitions
/// 2. Status validation - state machine transition rules
/// 3. Lock validation   - optimistic concurrency control
/// 4. Referential validation - FK integrity, entity_ref types, soft-delete checks

/// Result of the validation pipeline, containing both hard errors and soft warnings.
///
/// - `errors`: hard validation failures that block the write.
/// - `warnings`: soft constraint notices (e.g., backward status transitions without reason,
///   skip transitions) that do NOT block the write but should be surfaced for audit.
pub struct ValidationOutput {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

/// Validate a create_entity operation.
///
/// Runs:
/// - Step 1: Schema validation (canonical_fields against field_defs)
/// - Step 2: Status validation (null -> new_status, if status provided)
/// - Step 4: Referential validation for EntityRef fields in canonical_fields
///
/// No lock validation (step 3) since this is a new entity.
///
/// The `lookup` parameter is generic so callers can pass closures that
/// borrow non-`'static` data (e.g. a database connection reference).
pub fn validate_create_entity<F>(
    entity_type: &str,
    canonical_fields: &Value,
    field_defs: &[FieldDef],
    status: Option<&str>,
    lookup: &F,
) -> ValidationOutput
where
    F: Fn(&str) -> Option<(String, Option<String>)>,
{
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Step 1: Schema validation
    errors.extend(validate_canonical_fields(canonical_fields, field_defs));

    // Step 2: Status validation (null -> new status)
    if let Some(new_status) = status {
        let status_result = validate_status_transition(entity_type, None, new_status, None);
        errors.extend(status_result.errors);
        warnings.extend(status_result.warnings);
    }

    // Step 4: Referential integrity for entity_ref fields
    errors.extend(validate_entity_refs(canonical_fields, field_defs, lookup));

    ValidationOutput { errors, warnings }
}

/// Validate an update_entity operation.
///
/// Runs:
/// - Step 1: Schema validation (if canonical_fields provided)
/// - Step 2: Status validation (if new status provided)
/// - Step 3: Lock validation (expected_updated_at vs actual)
/// - Step 4: Referential validation for EntityRef fields (if canonical_fields provided)
///
/// The `lookup` parameter is generic so callers can pass closures that
/// borrow non-`'static` data (e.g. a database connection reference).
pub fn validate_update_entity<F>(
    entity_type: &str,
    canonical_fields: Option<&Value>,
    field_defs: &[FieldDef],
    current_status: Option<&str>,
    new_status: Option<&str>,
    reason: Option<&str>,
    expected_updated_at: &str,
    actual_updated_at: &str,
    lookup: &F,
) -> ValidationOutput
where
    F: Fn(&str) -> Option<(String, Option<String>)>,
{
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Step 1: Schema validation (only if canonical_fields are being updated)
    if let Some(fields) = canonical_fields {
        errors.extend(validate_canonical_fields(fields, field_defs));
    }

    // Step 2: Status validation (only if a new status is provided)
    if let Some(new_s) = new_status {
        let status_result = validate_status_transition(
            entity_type,
            current_status,
            new_s,
            reason,
        );
        errors.extend(status_result.errors);
        warnings.extend(status_result.warnings);
    }

    // Step 3: Lock validation
    errors.extend(validate_lock(expected_updated_at, actual_updated_at));

    // Step 4: Referential integrity for entity_ref fields (only if canonical_fields provided)
    if let Some(fields) = canonical_fields {
        errors.extend(validate_entity_refs(fields, field_defs, lookup));
    }

    ValidationOutput { errors, warnings }
}

/// Validate a create_relation operation.
///
/// Runs:
/// - Step 4: Referential validation (from_id, to_id exist and are not deleted,
///   relation_type is approved if custom)
///
/// The `lookup` parameter is generic so callers can pass closures that
/// borrow non-`'static` data (e.g. a database connection reference).
pub fn validate_create_relation<F>(
    from_id: &str,
    to_id: &str,
    relation_type: &str,
    approved_custom_types: &[String],
    lookup: &F,
) -> Vec<ValidationError>
where
    F: Fn(&str) -> Option<(String, Option<String>)>,
{
    validate_relation_refs(from_id, to_id, relation_type, approved_custom_types, lookup)
}

/// Validate a create_claim operation.
///
/// Validates that the evidence_entity_id exists and is not soft-deleted.
///
/// The `lookup` parameter is generic so callers can pass closures that
/// borrow non-`'static` data (e.g. a database connection reference).
pub fn validate_create_claim<F>(
    evidence_entity_id: &str,
    lookup: &F,
) -> Vec<ValidationError>
where
    F: Fn(&str) -> Option<(String, Option<String>)>,
{
    let mut errors = Vec::new();
    validate_entity_exists(evidence_entity_id, "evidence_entity_id", lookup, &mut errors);
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::schema::field_def::{FieldDef, FieldType};
    use serde_json::json;

    fn mock_lookup(id: &str) -> Option<(String, Option<String>)> {
        match id {
            "metric-1" => Some(("metric".to_string(), None)),
            "experiment-1" => Some(("experiment".to_string(), None)),
            "result-1" => Some(("result".to_string(), None)),
            "deleted-1" => Some(("metric".to_string(), Some("2025-06-01T00:00:00Z".to_string()))),
            _ => None,
        }
    }

    fn metric_field_defs() -> Vec<FieldDef> {
        vec![
            FieldDef {
                name: "current_value".to_string(),
                field_type: FieldType::Number,
                required: false,
                description: None,
                added_in_version: None,
            },
            FieldDef {
                name: "target_value".to_string(),
                field_type: FieldType::Number,
                required: false,
                description: None,
                added_in_version: None,
            },
            FieldDef {
                name: "trend".to_string(),
                field_type: FieldType::Enum(vec![
                    "up".to_string(),
                    "down".to_string(),
                    "flat".to_string(),
                ]),
                required: false,
                description: None,
                added_in_version: None,
            },
            FieldDef {
                name: "data_source".to_string(),
                field_type: FieldType::String,
                required: false,
                description: None,
                added_in_version: None,
            },
            FieldDef {
                name: "related_experiment".to_string(),
                field_type: FieldType::EntityRef("experiment".to_string()),
                required: false,
                description: None,
                added_in_version: None,
            },
        ]
    }

    // --- validate_create_entity ---

    #[test]
    fn test_create_entity_valid() {
        let fields = json!({
            "current_value": 42.0,
            "trend": "up"
        });
        let output = validate_create_entity("metric", &fields, &metric_field_defs(), Some("active"), &mock_lookup);
        assert!(output.errors.is_empty(), "Expected no errors, got: {:?}", output.errors);
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_create_entity_invalid_schema_and_status() {
        let fields = json!({
            "current_value": "not_a_number",
            "trend": "sideways"
        });
        let output = validate_create_entity("metric", &fields, &metric_field_defs(), Some("bogus"), &mock_lookup);
        // schema errors: InvalidFieldType for current_value, InvalidEnumValue for trend
        // status error: InvalidStatusTransition for "bogus"
        assert_eq!(output.errors.len(), 3);
    }

    #[test]
    fn test_create_entity_no_status() {
        let fields = json!({});
        let output = validate_create_entity("metric", &fields, &metric_field_defs(), None, &mock_lookup);
        assert!(output.errors.is_empty());
    }

    #[test]
    fn test_create_entity_with_entity_ref_valid() {
        let fields = json!({
            "related_experiment": "experiment-1"
        });
        let output = validate_create_entity("metric", &fields, &metric_field_defs(), None, &mock_lookup);
        assert!(output.errors.is_empty());
    }

    #[test]
    fn test_create_entity_with_entity_ref_not_found() {
        let fields = json!({
            "related_experiment": "nonexistent"
        });
        let output = validate_create_entity("metric", &fields, &metric_field_defs(), None, &mock_lookup);
        assert_eq!(output.errors.len(), 1);
        assert!(matches!(output.errors[0].code, ErrorCode::EntityNotFound));
    }

    #[test]
    fn test_create_entity_with_entity_ref_wrong_type() {
        let fields = json!({
            "related_experiment": "metric-1"
        });
        let output = validate_create_entity("metric", &fields, &metric_field_defs(), None, &mock_lookup);
        assert_eq!(output.errors.len(), 1);
        assert!(matches!(output.errors[0].code, ErrorCode::EntityRefTypeMismatch));
    }

    // --- validate_update_entity ---

    #[test]
    fn test_update_entity_valid() {
        let fields = json!({ "current_value": 100.0 });
        let output = validate_update_entity(
            "metric",
            Some(&fields),
            &metric_field_defs(),
            Some("active"),
            Some("paused"),
            None,
            "2025-01-01T00:00:00Z",
            "2025-01-01T00:00:00Z",
            &mock_lookup,
        );
        assert!(output.errors.is_empty());
    }

    #[test]
    fn test_update_entity_lock_conflict() {
        let output = validate_update_entity(
            "metric",
            None,
            &metric_field_defs(),
            Some("active"),
            None,
            None,
            "2025-01-01T00:00:00Z",
            "2025-01-01T12:00:00Z",
            &mock_lookup,
        );
        assert_eq!(output.errors.len(), 1);
        assert!(matches!(output.errors[0].code, ErrorCode::LockConflict));
    }

    #[test]
    fn test_update_entity_backward_status_without_reason_succeeds_with_warning() {
        let output = validate_update_entity(
            "metric",
            None,
            &metric_field_defs(),
            Some("paused"),
            Some("active"),
            None,
            "2025-01-01T00:00:00Z",
            "2025-01-01T00:00:00Z",
            &mock_lookup,
        );
        // Backward transitions are now soft constraints -- no errors
        assert!(output.errors.is_empty(), "Expected no errors, got: {:?}", output.errors);
        // But there should be a warning
        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("Backward"));
    }

    #[test]
    fn test_update_entity_backward_status_with_reason() {
        let output = validate_update_entity(
            "metric",
            None,
            &metric_field_defs(),
            Some("paused"),
            Some("active"),
            Some("reactivating metric"),
            "2025-01-01T00:00:00Z",
            "2025-01-01T00:00:00Z",
            &mock_lookup,
        );
        assert!(output.errors.is_empty());
        // Informational warning about the backward transition
        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("reactivating metric"));
    }

    #[test]
    fn test_update_entity_multiple_errors() {
        let fields = json!({ "current_value": "bad" });
        let output = validate_update_entity(
            "metric",
            Some(&fields),
            &metric_field_defs(),
            Some("paused"),
            Some("active"),
            None, // no reason for backward transition (soft constraint now)
            "old_timestamp",
            "new_timestamp", // lock conflict
            &mock_lookup,
        );
        // Errors: InvalidFieldType + LockConflict = 2
        // (backward transition without reason is now a warning, not an error)
        assert_eq!(output.errors.len(), 2, "Expected 2 errors, got: {:?}", output.errors);
        // Plus 1 warning for backward transition
        assert_eq!(output.warnings.len(), 1);
    }

    // --- validate_create_relation ---

    #[test]
    fn test_create_relation_valid() {
        let errors = validate_create_relation("metric-1", "experiment-1", "related_to", &[], &mock_lookup);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_create_relation_from_not_found() {
        let errors = validate_create_relation("nonexistent", "experiment-1", "related_to", &[], &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityNotFound));
    }

    #[test]
    fn test_create_relation_custom_type_approved() {
        let approved = vec!["custom:depends_on".to_string()];
        let errors = validate_create_relation("metric-1", "experiment-1", "custom:depends_on", &approved, &mock_lookup);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_create_relation_custom_type_not_approved() {
        let approved = vec!["custom:depends_on".to_string()];
        let errors = validate_create_relation("metric-1", "experiment-1", "custom:blocks", &approved, &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::RelationTypeNotApproved));
    }

    // --- validate_create_claim ---

    #[test]
    fn test_create_claim_valid() {
        let errors = validate_create_claim("result-1", &mock_lookup);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_create_claim_evidence_not_found() {
        let errors = validate_create_claim("nonexistent", &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityNotFound));
    }

    #[test]
    fn test_create_claim_evidence_deleted() {
        let errors = validate_create_claim("deleted-1", &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityDeleted));
    }
}
