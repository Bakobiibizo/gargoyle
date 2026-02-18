// Step 4: FK + entity_ref type + soft-delete checks

use crate::config::GargoyleConfig;
use crate::error::{ErrorCode, ValidationError};
use crate::schema::field_def::{FieldDef, FieldType};
use serde_json::Value;

/// Lookup function signature: given an entity ID, returns:
/// - None if the entity does not exist
/// - Some((entity_type, deleted_at)) where deleted_at is None if not soft-deleted
///
/// The lifetime parameter `'a` allows callers to pass closures that borrow
/// non-`'static` data (e.g. a database connection reference).
pub type EntityLookup<'a> = dyn Fn(&str) -> Option<(String, Option<String>)> + 'a;

/// Validate that both endpoints of a relation exist and are not soft-deleted.
///
/// Also validates the relation_type: if it starts with "custom:", checks that it
/// exists in the approved custom relation types list.
pub fn validate_relation_refs(
    from_id: &str,
    to_id: &str,
    relation_type: &str,
    approved_custom_types: &[String],
    lookup: &EntityLookup<'_>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Validate from_id
    validate_entity_exists(from_id, "from_id", lookup, &mut errors);

    // Validate to_id
    validate_entity_exists(to_id, "to_id", lookup, &mut errors);

    // Validate relation_type
    let canonical = &GargoyleConfig::global().canonical_relation_types;

    if relation_type.starts_with("custom:") {
        if !approved_custom_types.iter().any(|t| t == relation_type) {
            errors.push(ValidationError {
                code: ErrorCode::RelationTypeNotApproved,
                field_path: "relation_type".to_string(),
                message: format!(
                    "Custom relation type '{}' is not in the approved list. Approved custom types: [{}]",
                    relation_type,
                    approved_custom_types.join(", ")
                ),
                expected: Some(format!("one of [{}]", approved_custom_types.join(", "))),
                actual: Some(relation_type.to_string()),
            });
        }
    } else if !canonical.iter().any(|t| t == relation_type) {
        let canonical_str = canonical.join(", ");
        errors.push(ValidationError {
            code: ErrorCode::UnknownRelationType,
            field_path: "relation_type".to_string(),
            message: format!(
                "Unknown relation type '{}'. Must be one of the canonical types or use 'custom:' prefix.",
                relation_type
            ),
            expected: Some(format!(
                "one of [{}] or 'custom:' prefix",
                canonical_str
            )),
            actual: Some(relation_type.to_string()),
        });
    }

    errors
}

/// Validate entity_ref fields within canonical_fields.
///
/// For each field defined as EntityRef(expected_type), checks that:
/// 1. The referenced entity exists
/// 2. The referenced entity is not soft-deleted
/// 3. The referenced entity's type matches the expected type
pub fn validate_entity_refs(
    fields: &Value,
    field_defs: &[FieldDef],
    lookup: &EntityLookup<'_>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let obj = match fields.as_object() {
        Some(obj) => obj,
        None => return errors, // Not an object; schema_validator already caught this
    };

    for field_def in field_defs {
        if let FieldType::EntityRef(expected_type) = &field_def.field_type {
            if let Some(value) = obj.get(&field_def.name) {
                // Skip null values (optional entity refs)
                if value.is_null() {
                    continue;
                }

                // Must be a string ID (schema_validator already checks type,
                // but we need the string value here)
                let entity_id = match value.as_str() {
                    Some(id) => id,
                    None => continue, // Type error already caught by schema_validator
                };

                let field_path = format!("canonical_fields.{}", field_def.name);

                match lookup(entity_id) {
                    None => {
                        errors.push(ValidationError {
                            code: ErrorCode::EntityNotFound,
                            field_path,
                            message: format!(
                                "Entity reference '{}' in field '{}' does not exist",
                                entity_id, field_def.name
                            ),
                            expected: Some(format!("existing entity of type '{}'", expected_type)),
                            actual: None,
                        });
                    }
                    Some((actual_type, deleted_at)) => {
                        // Check soft-delete
                        if deleted_at.is_some() {
                            errors.push(ValidationError {
                                code: ErrorCode::EntityDeleted,
                                field_path: field_path.clone(),
                                message: format!(
                                    "Entity '{}' referenced in field '{}' has been deleted",
                                    entity_id, field_def.name
                                ),
                                expected: Some("non-deleted entity".to_string()),
                                actual: Some(format!("deleted at {}", deleted_at.unwrap())),
                            });
                        }

                        // Check type match
                        if actual_type != *expected_type {
                            errors.push(ValidationError {
                                code: ErrorCode::EntityRefTypeMismatch,
                                field_path,
                                message: format!(
                                    "Entity '{}' in field '{}' is of type '{}', expected '{}'",
                                    entity_id, field_def.name, actual_type, expected_type
                                ),
                                expected: Some(expected_type.clone()),
                                actual: Some(actual_type),
                            });
                        }
                    }
                }
            }
        }
    }

    errors
}

/// Validate that an entity ID exists and is not soft-deleted.
/// Used for relation endpoint validation and claim evidence validation.
pub fn validate_entity_exists(
    entity_id: &str,
    field_path: &str,
    lookup: &EntityLookup<'_>,
    errors: &mut Vec<ValidationError>,
) {
    match lookup(entity_id) {
        None => {
            errors.push(ValidationError {
                code: ErrorCode::EntityNotFound,
                field_path: field_path.to_string(),
                message: format!("Entity '{}' does not exist", entity_id),
                expected: Some("existing entity".to_string()),
                actual: None,
            });
        }
        Some((_, Some(deleted_at))) => {
            errors.push(ValidationError {
                code: ErrorCode::EntityDeleted,
                field_path: field_path.to_string(),
                message: format!("Entity '{}' has been deleted", entity_id),
                expected: Some("non-deleted entity".to_string()),
                actual: Some(format!("deleted at {}", deleted_at)),
            });
        }
        Some((_, None)) => {
            // Entity exists and is not deleted -- OK
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mock_lookup(id: &str) -> Option<(String, Option<String>)> {
        match id {
            "entity-1" => Some(("metric".to_string(), None)),
            "entity-2" => Some(("experiment".to_string(), None)),
            "entity-3" => Some(("result".to_string(), None)),
            "deleted-entity" => Some(("metric".to_string(), Some("2025-01-01T00:00:00Z".to_string()))),
            "wrong-type" => Some(("experiment".to_string(), None)),
            _ => None,
        }
    }

    // --- validate_relation_refs tests ---

    #[test]
    fn test_valid_relation() {
        let errors = validate_relation_refs("entity-1", "entity-2", "related_to", &[], &mock_lookup);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_relation_from_not_found() {
        let errors = validate_relation_refs("nonexistent", "entity-2", "related_to", &[], &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityNotFound));
        assert_eq!(errors[0].field_path, "from_id");
    }

    #[test]
    fn test_relation_to_not_found() {
        let errors = validate_relation_refs("entity-1", "nonexistent", "related_to", &[], &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityNotFound));
        assert_eq!(errors[0].field_path, "to_id");
    }

    #[test]
    fn test_relation_both_not_found() {
        let errors = validate_relation_refs("nope1", "nope2", "related_to", &[], &mock_lookup);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_relation_from_deleted() {
        let errors = validate_relation_refs("deleted-entity", "entity-2", "related_to", &[], &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityDeleted));
    }

    #[test]
    fn test_relation_to_deleted() {
        let errors = validate_relation_refs("entity-1", "deleted-entity", "related_to", &[], &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityDeleted));
    }

    #[test]
    fn test_custom_relation_type_approved() {
        let approved = vec!["custom:depends_on".to_string(), "custom:blocks".to_string()];
        let errors = validate_relation_refs("entity-1", "entity-2", "custom:depends_on", &approved, &mock_lookup);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_custom_relation_type_not_approved() {
        let approved = vec!["custom:depends_on".to_string()];
        let errors = validate_relation_refs("entity-1", "entity-2", "custom:unknown", &approved, &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::RelationTypeNotApproved));
    }

    #[test]
    fn test_canonical_relation_type_accepted() {
        // "supports" is one of the 22 canonical types and should be accepted
        let errors = validate_relation_refs("entity-1", "entity-2", "supports", &[], &mock_lookup);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_unknown_relation_type_rejected() {
        // "foo_bar" is not a canonical type and doesn't start with "custom:"
        let errors = validate_relation_refs("entity-1", "entity-2", "foo_bar", &[], &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::UnknownRelationType));
        assert!(errors[0].message.contains("foo_bar"));
        assert!(errors[0].message.contains("canonical types"));
    }

    #[test]
    fn test_relates_to_typo_rejected() {
        // "relates_to" is a common typo for the canonical "related_to"
        let errors = validate_relation_refs("entity-1", "entity-2", "relates_to", &[], &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::UnknownRelationType));
    }

    #[test]
    fn test_custom_relation_type_goes_through_approval_check() {
        // "custom:sponsors" should go through custom type approval, not canonical check
        let approved = vec!["custom:sponsors".to_string()];
        let errors = validate_relation_refs("entity-1", "entity-2", "custom:sponsors", &approved, &mock_lookup);
        assert!(errors.is_empty());

        // Unapproved custom type should fail with RelationTypeNotApproved, not UnknownRelationType
        let errors = validate_relation_refs("entity-1", "entity-2", "custom:sponsors", &[], &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::RelationTypeNotApproved));
    }

    #[test]
    fn test_builtin_relation_type_not_checked_against_custom_list() {
        let approved = vec![];
        let errors = validate_relation_refs("entity-1", "entity-2", "related_to", &approved, &mock_lookup);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_all_canonical_types_accepted() {
        let canonical = &GargoyleConfig::global().canonical_relation_types;
        for relation_type in canonical {
            let errors = validate_relation_refs("entity-1", "entity-2", relation_type, &[], &mock_lookup);
            let type_errors: Vec<_> = errors.iter().filter(|e| e.field_path == "relation_type").collect();
            assert!(type_errors.is_empty(), "Canonical type '{}' should be accepted", relation_type);
        }
    }

    // --- validate_entity_refs tests ---

    fn entity_ref_field_defs() -> Vec<FieldDef> {
        vec![
            FieldDef {
                name: "parent_metric".to_string(),
                field_type: FieldType::EntityRef("metric".to_string()),
                required: false,
                description: None,
                added_in_version: None,
            },
            FieldDef {
                name: "linked_experiment".to_string(),
                field_type: FieldType::EntityRef("experiment".to_string()),
                required: false,
                description: None,
                added_in_version: None,
            },
            FieldDef {
                name: "name".to_string(),
                field_type: FieldType::String,
                required: true,
                description: None,
                added_in_version: None,
            },
        ]
    }

    #[test]
    fn test_valid_entity_refs() {
        let fields = json!({
            "parent_metric": "entity-1",
            "linked_experiment": "entity-2",
            "name": "test"
        });
        let errors = validate_entity_refs(&fields, &entity_ref_field_defs(), &mock_lookup);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_entity_ref_not_found() {
        let fields = json!({
            "parent_metric": "nonexistent",
            "name": "test"
        });
        let errors = validate_entity_refs(&fields, &entity_ref_field_defs(), &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityNotFound));
    }

    #[test]
    fn test_entity_ref_deleted() {
        let fields = json!({
            "parent_metric": "deleted-entity",
            "name": "test"
        });
        let errors = validate_entity_refs(&fields, &entity_ref_field_defs(), &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityDeleted));
    }

    #[test]
    fn test_entity_ref_type_mismatch() {
        let fields = json!({
            "parent_metric": "wrong-type",
            "name": "test"
        });
        let errors = validate_entity_refs(&fields, &entity_ref_field_defs(), &mock_lookup);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityRefTypeMismatch));
    }

    #[test]
    fn test_null_entity_ref_is_ok() {
        let fields = json!({
            "parent_metric": null,
            "name": "test"
        });
        let errors = validate_entity_refs(&fields, &entity_ref_field_defs(), &mock_lookup);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_missing_entity_ref_field_is_ok() {
        let fields = json!({
            "name": "test"
        });
        let errors = validate_entity_refs(&fields, &entity_ref_field_defs(), &mock_lookup);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_non_object_fields_returns_empty() {
        let fields = json!("not an object");
        let errors = validate_entity_refs(&fields, &entity_ref_field_defs(), &mock_lookup);
        assert!(errors.is_empty());
    }

    // --- validate_entity_exists tests ---

    #[test]
    fn test_entity_exists_and_active() {
        let mut errors = Vec::new();
        validate_entity_exists("entity-1", "test_field", &mock_lookup, &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_entity_does_not_exist() {
        let mut errors = Vec::new();
        validate_entity_exists("nonexistent", "test_field", &mock_lookup, &mut errors);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityNotFound));
    }

    #[test]
    fn test_entity_is_deleted() {
        let mut errors = Vec::new();
        validate_entity_exists("deleted-entity", "test_field", &mock_lookup, &mut errors);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::EntityDeleted));
    }
}
