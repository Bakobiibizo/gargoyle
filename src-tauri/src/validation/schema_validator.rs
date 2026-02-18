// Step 1: canonical_fields against registry

use crate::error::{ErrorCode, ValidationError};
use crate::schema::field_def::{FieldDef, FieldType};
use serde_json::Value;

/// Validate canonical_fields against the provided field definitions.
///
/// Checks:
/// - Required fields are present and non-null
/// - Field types match their definitions (String, Number, Boolean, Enum)
/// - No unknown fields are present (fields not in the schema)
/// - EntityRef fields are skipped here (validated in step 4 by referential_validator)
pub fn validate_canonical_fields(
    canonical_fields: &Value,
    field_defs: &[FieldDef],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let obj = match canonical_fields.as_object() {
        Some(obj) => obj,
        None => {
            // canonical_fields must be a JSON object
            errors.push(ValidationError {
                code: ErrorCode::InvalidFieldType,
                field_path: "canonical_fields".to_string(),
                message: "canonical_fields must be a JSON object".to_string(),
                expected: Some("object".to_string()),
                actual: Some(json_type_name(canonical_fields).to_string()),
            });
            return errors;
        }
    };

    // Check required fields are present and non-null
    for field_def in field_defs {
        if field_def.required {
            match obj.get(&field_def.name) {
                None => {
                    errors.push(ValidationError {
                        code: ErrorCode::MissingRequiredField,
                        field_path: format!("canonical_fields.{}", field_def.name),
                        message: format!("Required field '{}' is missing", field_def.name),
                        expected: Some(field_type_display(&field_def.field_type)),
                        actual: None,
                    });
                }
                Some(Value::Null) => {
                    errors.push(ValidationError {
                        code: ErrorCode::MissingRequiredField,
                        field_path: format!("canonical_fields.{}", field_def.name),
                        message: format!("Required field '{}' cannot be null", field_def.name),
                        expected: Some(field_type_display(&field_def.field_type)),
                        actual: Some("null".to_string()),
                    });
                }
                _ => {}
            }
        }
    }

    // Check each provided field against its definition
    for (key, value) in obj {
        let field_def = field_defs.iter().find(|fd| fd.name == *key);
        match field_def {
            None => {
                errors.push(ValidationError {
                    code: ErrorCode::UnknownField,
                    field_path: format!("canonical_fields.{}", key),
                    message: format!("Unknown field '{}' is not defined in the schema", key),
                    expected: None,
                    actual: Some(key.clone()),
                });
            }
            Some(fd) => {
                // Skip null values for optional fields (they're allowed)
                if value.is_null() {
                    continue;
                }

                // Validate the type (skip EntityRef; that's step 4)
                validate_field_type(key, value, &fd.field_type, &mut errors);
            }
        }
    }

    errors
}

/// Validate that a single field value matches its expected FieldType.
/// EntityRef fields are intentionally skipped (validated in step 4).
fn validate_field_type(
    field_name: &str,
    value: &Value,
    field_type: &FieldType,
    errors: &mut Vec<ValidationError>,
) {
    let field_path = format!("canonical_fields.{}", field_name);

    match field_type {
        FieldType::String => {
            if !value.is_string() {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path,
                    message: format!("Field '{}' must be a string", field_name),
                    expected: Some("string".to_string()),
                    actual: Some(json_type_name(value).to_string()),
                });
            }
        }
        FieldType::Number => {
            if !value.is_number() {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path,
                    message: format!("Field '{}' must be a number", field_name),
                    expected: Some("number".to_string()),
                    actual: Some(json_type_name(value).to_string()),
                });
            }
        }
        FieldType::Boolean => {
            if !value.is_boolean() {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path,
                    message: format!("Field '{}' must be a boolean", field_name),
                    expected: Some("boolean".to_string()),
                    actual: Some(json_type_name(value).to_string()),
                });
            }
        }
        FieldType::Enum(valid_values) => {
            match value.as_str() {
                Some(s) => {
                    if !valid_values.contains(&s.to_string()) {
                        errors.push(ValidationError {
                            code: ErrorCode::InvalidEnumValue,
                            field_path,
                            message: format!(
                                "Field '{}' has invalid value '{}'. Valid values: [{}]",
                                field_name,
                                s,
                                valid_values.join(", ")
                            ),
                            expected: Some(format!("one of [{}]", valid_values.join(", "))),
                            actual: Some(s.to_string()),
                        });
                    }
                }
                None => {
                    errors.push(ValidationError {
                        code: ErrorCode::InvalidFieldType,
                        field_path,
                        message: format!(
                            "Field '{}' must be a string (enum value), got {}",
                            field_name,
                            json_type_name(value)
                        ),
                        expected: Some(format!("one of [{}]", valid_values.join(", "))),
                        actual: Some(json_type_name(value).to_string()),
                    });
                }
            }
        }
        FieldType::Date => {
            // Date values must be strings. Lenient validation for now.
            if !value.is_string() {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path,
                    message: format!("Field '{}' must be a date string (ISO 8601)", field_name),
                    expected: Some("string (ISO 8601 date)".to_string()),
                    actual: Some(json_type_name(value).to_string()),
                });
            }
        }
        FieldType::DateTime => {
            // DateTime values must be strings. Lenient validation for now.
            if !value.is_string() {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path,
                    message: format!("Field '{}' must be a datetime string (ISO 8601)", field_name),
                    expected: Some("string (ISO 8601 datetime)".to_string()),
                    actual: Some(json_type_name(value).to_string()),
                });
            }
        }
        FieldType::Json => {
            // Json type accepts any valid JSON value -- no type validation needed.
        }
        FieldType::Array(items_type) => {
            match value.as_array() {
                Some(arr) => {
                    for (i, item) in arr.iter().enumerate() {
                        let item_field_name = format!("{}[{}]", field_name, i);
                        let item_field_type = match items_type.as_str() {
                            "string" => FieldType::String,
                            "number" => FieldType::Number,
                            "boolean" => FieldType::Boolean,
                            "date" => FieldType::Date,
                            "datetime" => FieldType::DateTime,
                            "json" => FieldType::Json,
                            _ => continue,
                        };
                        validate_field_type(&item_field_name, item, &item_field_type, errors);
                    }
                }
                None => {
                    errors.push(ValidationError {
                        code: ErrorCode::InvalidFieldType,
                        field_path,
                        message: format!("Field '{}' must be an array", field_name),
                        expected: Some("array".to_string()),
                        actual: Some(json_type_name(value).to_string()),
                    });
                }
            }
        }
        FieldType::EntityRef(_) => {
            // EntityRef validation is handled by referential_validator (step 4).
            // Here we only check it's a string (the ID format).
            if !value.is_string() {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path,
                    message: format!("Field '{}' (entity reference) must be a string ID", field_name),
                    expected: Some("string (entity ID)".to_string()),
                    actual: Some(json_type_name(value).to_string()),
                });
            }
        }
        FieldType::EntityRefArray(_ref_type) => {
            // EntityRefArray values must be JSON arrays of strings.
            // Referential integrity is deferred to step 4.
            match value.as_array() {
                Some(arr) => {
                    for (i, item) in arr.iter().enumerate() {
                        if !item.is_string() {
                            errors.push(ValidationError {
                                code: ErrorCode::InvalidFieldType,
                                field_path: format!("canonical_fields.{}[{}]", field_name, i),
                                message: format!(
                                    "Field '{}[{}]' must be a string (entity ID), got {}",
                                    field_name, i, json_type_name(item)
                                ),
                                expected: Some("string (entity ID)".to_string()),
                                actual: Some(json_type_name(item).to_string()),
                            });
                        }
                    }
                }
                None => {
                    errors.push(ValidationError {
                        code: ErrorCode::InvalidFieldType,
                        field_path,
                        message: format!("Field '{}' must be an array of entity references", field_name),
                        expected: Some("array".to_string()),
                        actual: Some(json_type_name(value).to_string()),
                    });
                }
            }
        }
    }
}

/// Return a human-readable type name for a JSON value.
fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Return a display string for a FieldType.
fn field_type_display(ft: &FieldType) -> String {
    match ft {
        FieldType::String => "string".to_string(),
        FieldType::Number => "number".to_string(),
        FieldType::Boolean => "boolean".to_string(),
        FieldType::Date => "date (ISO 8601)".to_string(),
        FieldType::DateTime => "datetime (ISO 8601)".to_string(),
        FieldType::Json => "json".to_string(),
        FieldType::Array(items_type) => format!("array({})", items_type),
        FieldType::Enum(values) => format!("one of [{}]", values.join(", ")),
        FieldType::EntityRef(target) => format!("entity_ref({})", target),
        FieldType::EntityRefArray(target) => format!("entity_ref_array({})", target),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_field_defs() -> Vec<FieldDef> {
        vec![
            FieldDef {
                name: "name".to_string(),
                field_type: FieldType::String,
                required: true,
                description: None,
                added_in_version: None,
            },
            FieldDef {
                name: "count".to_string(),
                field_type: FieldType::Number,
                required: false,
                description: None,
                added_in_version: None,
            },
            FieldDef {
                name: "active".to_string(),
                field_type: FieldType::Boolean,
                required: false,
                description: None,
                added_in_version: None,
            },
            FieldDef {
                name: "priority".to_string(),
                field_type: FieldType::Enum(vec!["low".to_string(), "medium".to_string(), "high".to_string()]),
                required: false,
                description: None,
                added_in_version: None,
            },
            FieldDef {
                name: "parent_id".to_string(),
                field_type: FieldType::EntityRef("metric".to_string()),
                required: false,
                description: None,
                added_in_version: None,
            },
        ]
    }

    #[test]
    fn test_valid_fields_no_errors() {
        let fields = json!({
            "name": "test metric",
            "count": 42,
            "active": true,
            "priority": "high"
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_missing_required_field() {
        let fields = json!({
            "count": 10
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::MissingRequiredField));
        assert_eq!(errors[0].field_path, "canonical_fields.name");
    }

    #[test]
    fn test_null_required_field() {
        let fields = json!({
            "name": null
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::MissingRequiredField));
    }

    #[test]
    fn test_wrong_type_string() {
        let fields = json!({
            "name": 123
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
    }

    #[test]
    fn test_wrong_type_number() {
        let fields = json!({
            "name": "ok",
            "count": "not a number"
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
    }

    #[test]
    fn test_wrong_type_boolean() {
        let fields = json!({
            "name": "ok",
            "active": "yes"
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
    }

    #[test]
    fn test_invalid_enum_value() {
        let fields = json!({
            "name": "ok",
            "priority": "urgent"
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidEnumValue));
    }

    #[test]
    fn test_enum_wrong_type() {
        let fields = json!({
            "name": "ok",
            "priority": 99
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
    }

    #[test]
    fn test_unknown_field() {
        let fields = json!({
            "name": "ok",
            "unknown_field": "value"
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::UnknownField));
    }

    #[test]
    fn test_not_an_object() {
        let fields = json!("just a string");
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
    }

    #[test]
    fn test_entity_ref_must_be_string() {
        let fields = json!({
            "name": "ok",
            "parent_id": 12345
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
    }

    #[test]
    fn test_entity_ref_string_passes_schema_validation() {
        let fields = json!({
            "name": "ok",
            "parent_id": "some-entity-id"
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert!(errors.is_empty(), "EntityRef with string ID should pass schema validation");
    }

    #[test]
    fn test_null_optional_field_is_ok() {
        let fields = json!({
            "name": "ok",
            "count": null,
            "active": null
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_multiple_errors() {
        let fields = json!({
            "count": "wrong",
            "active": 42,
            "unknown": true
        });
        let errors = validate_canonical_fields(&fields, &sample_field_defs());
        // missing required "name" + wrong type "count" + wrong type "active" + unknown field
        assert_eq!(errors.len(), 4);
    }
}
