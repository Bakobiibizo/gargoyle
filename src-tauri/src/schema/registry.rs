use std::collections::HashMap;
use std::sync::OnceLock;

use crate::config::GargoyleConfig;
use crate::config::entity_type_config::EntityTypeDef;
use crate::error::{ErrorCode, ValidationError};
use crate::schema::field_def::{FieldDef, FieldType};
use crate::schema::version::SchemaVersion;

/// Global singleton schema registry, initialized on first access.
static REGISTRY: OnceLock<SchemaRegistry> = OnceLock::new();

/// The SchemaRegistry is the central authority for entity type schemas.
/// It provides field definitions, version tracking, and canonical_fields validation
/// for all known entity types.
///
/// Entity types are loaded from configuration (TOML files or hardcoded defaults).
pub struct SchemaRegistry {
    schema_version: SchemaVersion,
    /// Entity type definitions keyed by type name.
    entity_types: HashMap<String, EntityTypeDef>,
}

impl SchemaRegistry {
    /// Creates a new SchemaRegistry from the global config.
    pub fn new() -> Self {
        let config = GargoyleConfig::global();
        Self::from_entity_types(config.entity_types.clone())
    }

    /// Creates a SchemaRegistry from a given set of entity type definitions.
    pub fn from_entity_types(entity_types: HashMap<String, EntityTypeDef>) -> Self {
        let schema_version = SchemaVersion::from_entity_types(&entity_types);
        Self {
            schema_version,
            entity_types,
        }
    }

    /// Returns a reference to the global singleton SchemaRegistry.
    pub fn global() -> &'static SchemaRegistry {
        REGISTRY.get_or_init(SchemaRegistry::new)
    }

    /// Returns the field definitions for a given entity type and version.
    /// Returns None if the entity type or version is not recognized.
    pub fn get_schema(&self, entity_type: &str, version: i32) -> Option<Vec<FieldDef>> {
        let def = self.entity_types.get(entity_type)?;
        if version == def.version {
            Some(def.field_defs())
        } else {
            None
        }
    }

    /// Returns the current (latest) schema version for the given entity type.
    /// Returns None if the entity type is not recognized.
    pub fn current_version(&self, entity_type: &str) -> Option<i32> {
        self.entity_types.get(entity_type).map(|def| def.version)
    }

    /// Returns the valid status values for the given entity type.
    /// Returns None if the entity type is not recognized.
    pub fn valid_statuses(&self, entity_type: &str) -> Option<Vec<String>> {
        self.entity_types
            .get(entity_type)
            .map(|def| def.statuses.clone())
    }

    /// Returns a reference to the internal SchemaVersion tracker.
    pub fn schema_version(&self) -> &SchemaVersion {
        &self.schema_version
    }

    /// Validates canonical_fields JSON against the schema for the given entity type and version.
    ///
    /// Validation rules:
    /// - Missing required fields produce MissingRequiredField errors
    /// - Wrong field types produce InvalidFieldType errors
    /// - Invalid enum values produce InvalidEnumValue errors with the list of valid values
    /// - Unknown fields (not in schema) are allowed for forward compatibility -- no error
    /// - EntityRef type checking is deferred to the referential_validator -- only checks
    ///   that the value is a string here
    ///
    /// Returns an empty Vec if all validations pass.
    pub fn validate_canonical_fields(
        &self,
        entity_type: &str,
        version: i32,
        fields: &serde_json::Value,
    ) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Look up the schema for this entity type + version
        let schema = match self.get_schema(entity_type, version) {
            Some(s) => s,
            None => {
                errors.push(ValidationError {
                    code: ErrorCode::SchemaVersionMismatch,
                    field_path: String::new(),
                    message: format!(
                        "No schema found for entity type '{}' version {}",
                        entity_type, version
                    ),
                    expected: None,
                    actual: None,
                });
                return errors;
            }
        };

        // The fields value must be a JSON object (or null, which we treat as empty)
        let fields_map = match fields {
            serde_json::Value::Object(map) => map,
            serde_json::Value::Null => {
                // Null canonical_fields: check for required fields only
                for field_def in &schema {
                    if field_def.required {
                        errors.push(ValidationError {
                            code: ErrorCode::MissingRequiredField,
                            field_path: format!("canonical_fields.{}", field_def.name),
                            message: format!(
                                "Required field '{}' is missing",
                                field_def.name
                            ),
                            expected: Some(format!("{:?}", field_def.field_type)),
                            actual: None,
                        });
                    }
                }
                return errors;
            }
            _ => {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path: "canonical_fields".to_string(),
                    message: "canonical_fields must be a JSON object or null".to_string(),
                    expected: Some("object".to_string()),
                    actual: Some(json_type_name(fields).to_string()),
                });
                return errors;
            }
        };

        // Validate each field defined in the schema
        for field_def in &schema {
            match fields_map.get(&field_def.name) {
                None => {
                    // Field is absent
                    if field_def.required {
                        errors.push(ValidationError {
                            code: ErrorCode::MissingRequiredField,
                            field_path: format!("canonical_fields.{}", field_def.name),
                            message: format!(
                                "Required field '{}' is missing",
                                field_def.name
                            ),
                            expected: Some(format!("{:?}", field_def.field_type)),
                            actual: None,
                        });
                    }
                }
                Some(serde_json::Value::Null) => {
                    // Null value: treat as absent. Only error if required.
                    if field_def.required {
                        errors.push(ValidationError {
                            code: ErrorCode::MissingRequiredField,
                            field_path: format!("canonical_fields.{}", field_def.name),
                            message: format!(
                                "Required field '{}' is null",
                                field_def.name
                            ),
                            expected: Some(format!("{:?}", field_def.field_type)),
                            actual: Some("null".to_string()),
                        });
                    }
                }
                Some(value) => {
                    // Field is present and non-null -- validate its type
                    validate_field_type(
                        &field_def.name,
                        &field_def.field_type,
                        value,
                        &mut errors,
                    );
                }
            }
        }

        // Unknown fields are allowed (forward compatibility) -- no errors generated.

        errors
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates that a JSON value matches the expected FieldType.
/// Pushes validation errors into the provided errors vec.
fn validate_field_type(
    field_name: &str,
    field_type: &FieldType,
    value: &serde_json::Value,
    errors: &mut Vec<ValidationError>,
) {
    let field_path = format!("canonical_fields.{}", field_name);

    match field_type {
        FieldType::String => {
            if !value.is_string() {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path,
                    message: format!(
                        "Field '{}' expected string, got {}",
                        field_name,
                        json_type_name(value)
                    ),
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
                    message: format!(
                        "Field '{}' expected number, got {}",
                        field_name,
                        json_type_name(value)
                    ),
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
                    message: format!(
                        "Field '{}' expected boolean, got {}",
                        field_name,
                        json_type_name(value)
                    ),
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
                                "Field '{}' has invalid enum value '{}'. Valid values: {:?}",
                                field_name, s, valid_values
                            ),
                            expected: Some(format!("one of {:?}", valid_values)),
                            actual: Some(s.to_string()),
                        });
                    }
                }
                None => {
                    // Not a string at all -- type error
                    errors.push(ValidationError {
                        code: ErrorCode::InvalidFieldType,
                        field_path,
                        message: format!(
                            "Field '{}' expected enum string, got {}",
                            field_name,
                            json_type_name(value)
                        ),
                        expected: Some(format!("enum string, one of {:?}", valid_values)),
                        actual: Some(json_type_name(value).to_string()),
                    });
                }
            }
        }
        FieldType::Date => {
            if !value.is_string() {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path,
                    message: format!(
                        "Field '{}' expected date string (ISO 8601), got {}",
                        field_name,
                        json_type_name(value)
                    ),
                    expected: Some("string (ISO 8601 date)".to_string()),
                    actual: Some(json_type_name(value).to_string()),
                });
            }
        }
        FieldType::DateTime => {
            if !value.is_string() {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path,
                    message: format!(
                        "Field '{}' expected datetime string (ISO 8601), got {}",
                        field_name,
                        json_type_name(value)
                    ),
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
                            _ => {
                                continue;
                            }
                        };
                        validate_field_type(
                            &item_field_name,
                            &item_field_type,
                            item,
                            errors,
                        );
                    }
                }
                None => {
                    errors.push(ValidationError {
                        code: ErrorCode::InvalidFieldType,
                        field_path,
                        message: format!(
                            "Field '{}' expected array, got {}",
                            field_name,
                            json_type_name(value)
                        ),
                        expected: Some("array".to_string()),
                        actual: Some(json_type_name(value).to_string()),
                    });
                }
            }
        }
        FieldType::EntityRef(_target_type) => {
            if !value.is_string() {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidFieldType,
                    field_path,
                    message: format!(
                        "Field '{}' expected entity reference (string ID), got {}",
                        field_name,
                        json_type_name(value)
                    ),
                    expected: Some("string (entity ID)".to_string()),
                    actual: Some(json_type_name(value).to_string()),
                });
            }
        }
        FieldType::EntityRefArray(_ref_type) => {
            match value.as_array() {
                Some(arr) => {
                    for (i, item) in arr.iter().enumerate() {
                        if !item.is_string() {
                            errors.push(ValidationError {
                                code: ErrorCode::InvalidFieldType,
                                field_path: format!("canonical_fields.{}[{}]", field_name, i),
                                message: format!(
                                    "Field '{}[{}]' expected string (entity ID), got {}",
                                    field_name,
                                    i,
                                    json_type_name(item)
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
                        message: format!(
                            "Field '{}' expected array of entity references, got {}",
                            field_name,
                            json_type_name(value)
                        ),
                        expected: Some("array".to_string()),
                        actual: Some(json_type_name(value).to_string()),
                    });
                }
            }
        }
    }
}

/// Returns a human-readable name for a JSON value's type.
fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn registry() -> SchemaRegistry {
        SchemaRegistry::from_entity_types(
            crate::config::GargoyleConfig::defaults().entity_types,
        )
    }

    // -- get_schema tests --

    #[test]
    fn test_get_schema_metric_v1() {
        let reg = registry();
        let schema = reg.get_schema("metric", 1).unwrap();
        assert_eq!(schema.len(), 4);
    }

    #[test]
    fn test_get_schema_experiment_v1() {
        let reg = registry();
        let schema = reg.get_schema("experiment", 1).unwrap();
        assert_eq!(schema.len(), 3);
    }

    #[test]
    fn test_get_schema_result_v1() {
        let reg = registry();
        let schema = reg.get_schema("result", 1).unwrap();
        assert_eq!(schema.len(), 3);
    }

    #[test]
    fn test_get_schema_unknown_type() {
        let reg = registry();
        assert!(reg.get_schema("widget", 1).is_none());
    }

    #[test]
    fn test_get_schema_unknown_version() {
        let reg = registry();
        assert!(reg.get_schema("metric", 99).is_none());
    }

    // -- current_version tests --

    #[test]
    fn test_current_version_metric() {
        let reg = registry();
        assert_eq!(reg.current_version("metric"), Some(1));
    }

    #[test]
    fn test_current_version_experiment() {
        let reg = registry();
        assert_eq!(reg.current_version("experiment"), Some(1));
    }

    #[test]
    fn test_current_version_result() {
        let reg = registry();
        assert_eq!(reg.current_version("result"), Some(1));
    }

    #[test]
    fn test_current_version_unknown() {
        let reg = registry();
        assert_eq!(reg.current_version("widget"), None);
    }

    // -- valid_statuses tests --

    #[test]
    fn test_valid_statuses_metric() {
        let reg = registry();
        let statuses = reg.valid_statuses("metric").unwrap();
        assert_eq!(statuses, &["active", "paused", "deprecated", "archived"]);
    }

    #[test]
    fn test_valid_statuses_experiment() {
        let reg = registry();
        let statuses = reg.valid_statuses("experiment").unwrap();
        assert_eq!(statuses, &["draft", "running", "concluded", "archived"]);
    }

    #[test]
    fn test_valid_statuses_result() {
        let reg = registry();
        let statuses = reg.valid_statuses("result").unwrap();
        assert_eq!(statuses, &["preliminary", "final", "invalidated"]);
    }

    #[test]
    fn test_valid_statuses_unknown() {
        let reg = registry();
        assert!(reg.valid_statuses("widget").is_none());
    }

    // -- validate_canonical_fields: valid cases --

    #[test]
    fn test_validate_empty_object_all_optional() {
        let reg = registry();
        let errors = reg.validate_canonical_fields("metric", 1, &json!({}));
        assert!(errors.is_empty(), "All metric fields are optional, so empty object should pass");
    }

    #[test]
    fn test_validate_null_fields_all_optional() {
        let reg = registry();
        let errors = reg.validate_canonical_fields("metric", 1, &serde_json::Value::Null);
        assert!(errors.is_empty(), "Null canonical_fields should pass when all fields are optional");
    }

    #[test]
    fn test_validate_valid_metric_fields() {
        let reg = registry();
        let fields = json!({
            "current_value": 42.5,
            "target_value": 100,
            "trend": "up",
            "data_source": "analytics"
        });
        let errors = reg.validate_canonical_fields("metric", 1, &fields);
        assert!(errors.is_empty(), "Valid metric fields should produce no errors: {:?}", errors);
    }

    #[test]
    fn test_validate_valid_experiment_fields() {
        let reg = registry();
        let fields = json!({
            "hypothesis": "Changing the button color increases conversions",
            "funnel_position": "checkout",
            "primary_metric": "conversion_rate"
        });
        let errors = reg.validate_canonical_fields("experiment", 1, &fields);
        assert!(errors.is_empty(), "Valid experiment fields should produce no errors: {:?}", errors);
    }

    #[test]
    fn test_validate_valid_result_fields() {
        let reg = registry();
        let fields = json!({
            "source_experiment_id": "exp-uuid-123",
            "outcome": "Button color had no significant effect",
            "confidence_level": "high"
        });
        let errors = reg.validate_canonical_fields("result", 1, &fields);
        assert!(errors.is_empty(), "Valid result fields should produce no errors: {:?}", errors);
    }

    #[test]
    fn test_validate_partial_fields_ok() {
        let reg = registry();
        let fields = json!({
            "current_value": 42.5
        });
        let errors = reg.validate_canonical_fields("metric", 1, &fields);
        assert!(errors.is_empty(), "Partial optional fields should pass");
    }

    // -- validate_canonical_fields: unknown fields allowed --

    #[test]
    fn test_validate_unknown_fields_allowed() {
        let reg = registry();
        let fields = json!({
            "current_value": 42.5,
            "some_future_field": "hello",
            "another_unknown": 123
        });
        let errors = reg.validate_canonical_fields("metric", 1, &fields);
        assert!(errors.is_empty(), "Unknown fields should be allowed for forward compatibility");
    }

    // -- validate_canonical_fields: type errors --

    #[test]
    fn test_validate_wrong_type_string_for_number() {
        let reg = registry();
        let fields = json!({
            "current_value": "not a number"
        });
        let errors = reg.validate_canonical_fields("metric", 1, &fields);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
        assert_eq!(errors[0].field_path, "canonical_fields.current_value");
    }

    #[test]
    fn test_validate_wrong_type_number_for_string() {
        let reg = registry();
        let fields = json!({
            "outcome": 12345
        });
        let errors = reg.validate_canonical_fields("result", 1, &fields);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
        assert_eq!(errors[0].field_path, "canonical_fields.outcome");
    }

    #[test]
    fn test_validate_wrong_type_bool_for_enum() {
        let reg = registry();
        let fields = json!({
            "confidence_level": true,
            "outcome": "Test outcome"
        });
        let errors = reg.validate_canonical_fields("result", 1, &fields);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
    }

    // -- validate_canonical_fields: enum errors --

    #[test]
    fn test_validate_invalid_enum_value() {
        let reg = registry();
        let fields = json!({
            "trend": "sideways"
        });
        let errors = reg.validate_canonical_fields("metric", 1, &fields);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidEnumValue));
        assert!(errors[0].message.contains("sideways"));
        assert!(errors[0].expected.as_ref().unwrap().contains("up"));
    }

    #[test]
    fn test_validate_enum_wrong_type() {
        let reg = registry();
        let fields = json!({
            "trend": 42
        });
        let errors = reg.validate_canonical_fields("metric", 1, &fields);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
    }

    // -- validate_canonical_fields: entity ref --

    #[test]
    fn test_validate_entity_ref_string_ok() {
        let reg = registry();
        let fields = json!({
            "project_id": "some-uuid",
            "effort_estimate": "M"
        });
        let errors = reg.validate_canonical_fields("task", 1, &fields);
        assert!(errors.is_empty(), "Entity ref as string should be valid");
    }

    #[test]
    fn test_validate_entity_ref_wrong_type() {
        let reg = registry();
        let fields = json!({
            "project_id": 42,
            "effort_estimate": "M"
        });
        let errors = reg.validate_canonical_fields("task", 1, &fields);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
    }

    // -- validate_canonical_fields: schema not found --

    #[test]
    fn test_validate_unknown_entity_type() {
        let reg = registry();
        let errors = reg.validate_canonical_fields("widget", 1, &json!({}));
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::SchemaVersionMismatch));
    }

    #[test]
    fn test_validate_unknown_version() {
        let reg = registry();
        let errors = reg.validate_canonical_fields("metric", 99, &json!({}));
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::SchemaVersionMismatch));
    }

    // -- validate_canonical_fields: non-object fields --

    #[test]
    fn test_validate_fields_not_object() {
        let reg = registry();
        let errors = reg.validate_canonical_fields("metric", 1, &json!("not an object"));
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
        assert_eq!(errors[0].field_path, "canonical_fields");
    }

    #[test]
    fn test_validate_fields_array_not_object() {
        let reg = registry();
        let errors = reg.validate_canonical_fields("metric", 1, &json!([1, 2, 3]));
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
    }

    // -- validate_canonical_fields: multiple errors at once --

    #[test]
    fn test_validate_multiple_errors() {
        let reg = registry();
        let fields = json!({
            "current_value": "bad",
            "target_value": true,
            "trend": "sideways"
        });
        let errors = reg.validate_canonical_fields("metric", 1, &fields);
        assert_eq!(errors.len(), 3, "Should have 3 errors: {:?}", errors);
    }

    // -- validate_canonical_fields: null field values --

    #[test]
    fn test_validate_null_field_value_optional() {
        let reg = registry();
        let fields = json!({
            "current_value": null
        });
        let errors = reg.validate_canonical_fields("metric", 1, &fields);
        assert!(errors.is_empty(), "Null optional field should not produce error");
    }

    // -- global singleton --

    #[test]
    fn test_global_singleton() {
        let reg = SchemaRegistry::global();
        assert_eq!(reg.current_version("metric"), Some(1));
    }

    // -- valid enum values --

    #[test]
    fn test_validate_all_valid_trend_values() {
        let reg = registry();
        for trend in &["up", "down", "flat"] {
            let fields = json!({ "trend": trend });
            let errors = reg.validate_canonical_fields("metric", 1, &fields);
            assert!(errors.is_empty(), "Trend '{}' should be valid", trend);
        }
    }

    // -- integer numbers are valid numbers --

    #[test]
    fn test_validate_integer_as_number() {
        let reg = registry();
        let fields = json!({
            "current_value": 42,
            "target_value": 100
        });
        let errors = reg.validate_canonical_fields("metric", 1, &fields);
        assert!(errors.is_empty(), "Integer values should be valid numbers");
    }

    // -- validate_field_type: Date --

    #[test]
    fn test_validate_date_valid_string() {
        let mut errors = Vec::new();
        validate_field_type("start_date", &FieldType::Date, &json!("2025-01-15"), &mut errors);
        assert!(errors.is_empty(), "ISO date string should be valid for Date type");
    }

    #[test]
    fn test_validate_date_any_string_accepted() {
        let mut errors = Vec::new();
        validate_field_type("start_date", &FieldType::Date, &json!("not-a-date"), &mut errors);
        assert!(errors.is_empty(), "Lenient Date validation should accept any string");
    }

    #[test]
    fn test_validate_date_wrong_type() {
        let mut errors = Vec::new();
        validate_field_type("start_date", &FieldType::Date, &json!(12345), &mut errors);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
        assert!(errors[0].message.contains("date string"));
    }

    // -- validate_field_type: DateTime --

    #[test]
    fn test_validate_datetime_valid_string() {
        let mut errors = Vec::new();
        validate_field_type(
            "created_at",
            &FieldType::DateTime,
            &json!("2025-01-15T10:30:00Z"),
            &mut errors,
        );
        assert!(errors.is_empty(), "ISO datetime string should be valid for DateTime type");
    }

    #[test]
    fn test_validate_datetime_any_string_accepted() {
        let mut errors = Vec::new();
        validate_field_type("created_at", &FieldType::DateTime, &json!("any-string"), &mut errors);
        assert!(errors.is_empty(), "Lenient DateTime validation should accept any string");
    }

    #[test]
    fn test_validate_datetime_wrong_type() {
        let mut errors = Vec::new();
        validate_field_type("created_at", &FieldType::DateTime, &json!(true), &mut errors);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
        assert!(errors[0].message.contains("datetime string"));
    }

    // -- validate_field_type: Json --

    #[test]
    fn test_validate_json_object() {
        let mut errors = Vec::new();
        validate_field_type("metadata", &FieldType::Json, &json!({"key": "value"}), &mut errors);
        assert!(errors.is_empty(), "JSON object should be valid for Json type");
    }

    #[test]
    fn test_validate_json_array() {
        let mut errors = Vec::new();
        validate_field_type("metadata", &FieldType::Json, &json!([1, 2, 3]), &mut errors);
        assert!(errors.is_empty(), "JSON array should be valid for Json type");
    }

    #[test]
    fn test_validate_json_string() {
        let mut errors = Vec::new();
        validate_field_type("metadata", &FieldType::Json, &json!("hello"), &mut errors);
        assert!(errors.is_empty(), "JSON string should be valid for Json type");
    }

    #[test]
    fn test_validate_json_number() {
        let mut errors = Vec::new();
        validate_field_type("metadata", &FieldType::Json, &json!(42), &mut errors);
        assert!(errors.is_empty(), "JSON number should be valid for Json type");
    }

    #[test]
    fn test_validate_json_boolean() {
        let mut errors = Vec::new();
        validate_field_type("metadata", &FieldType::Json, &json!(true), &mut errors);
        assert!(errors.is_empty(), "JSON boolean should be valid for Json type");
    }

    // -- validate_field_type: Array --

    #[test]
    fn test_validate_array_of_strings_valid() {
        let mut errors = Vec::new();
        validate_field_type(
            "tags",
            &FieldType::Array("string".to_string()),
            &json!(["tag1", "tag2", "tag3"]),
            &mut errors,
        );
        assert!(errors.is_empty(), "Array of strings should be valid for Array(string)");
    }

    #[test]
    fn test_validate_array_of_numbers_valid() {
        let mut errors = Vec::new();
        validate_field_type(
            "scores",
            &FieldType::Array("number".to_string()),
            &json!([1, 2.5, 3]),
            &mut errors,
        );
        assert!(errors.is_empty(), "Array of numbers should be valid for Array(number)");
    }

    #[test]
    fn test_validate_array_empty_valid() {
        let mut errors = Vec::new();
        validate_field_type(
            "tags",
            &FieldType::Array("string".to_string()),
            &json!([]),
            &mut errors,
        );
        assert!(errors.is_empty(), "Empty array should be valid for Array type");
    }

    #[test]
    fn test_validate_array_wrong_element_type() {
        let mut errors = Vec::new();
        validate_field_type(
            "tags",
            &FieldType::Array("string".to_string()),
            &json!(["ok", 42, "also_ok"]),
            &mut errors,
        );
        assert_eq!(errors.len(), 1, "One invalid element should produce one error");
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
        assert!(errors[0].field_path.contains("tags[1]"));
    }

    #[test]
    fn test_validate_array_not_array() {
        let mut errors = Vec::new();
        validate_field_type(
            "tags",
            &FieldType::Array("string".to_string()),
            &json!("not-an-array"),
            &mut errors,
        );
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
        assert!(errors[0].message.contains("expected array"));
    }

    // -- validate_field_type: EntityRefArray --

    #[test]
    fn test_validate_entity_ref_array_valid() {
        let mut errors = Vec::new();
        validate_field_type(
            "related_tasks",
            &FieldType::EntityRefArray("task".to_string()),
            &json!(["uuid-1", "uuid-2", "uuid-3"]),
            &mut errors,
        );
        assert!(errors.is_empty(), "Array of string UUIDs should be valid for EntityRefArray");
    }

    #[test]
    fn test_validate_entity_ref_array_empty() {
        let mut errors = Vec::new();
        validate_field_type(
            "related_tasks",
            &FieldType::EntityRefArray("task".to_string()),
            &json!([]),
            &mut errors,
        );
        assert!(errors.is_empty(), "Empty array should be valid for EntityRefArray");
    }

    #[test]
    fn test_validate_entity_ref_array_wrong_element_type() {
        let mut errors = Vec::new();
        validate_field_type(
            "related_tasks",
            &FieldType::EntityRefArray("task".to_string()),
            &json!(["uuid-1", 42, "uuid-3"]),
            &mut errors,
        );
        assert_eq!(errors.len(), 1, "Non-string element should produce error");
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
        assert!(errors[0].field_path.contains("related_tasks[1]"));
    }

    #[test]
    fn test_validate_entity_ref_array_not_array() {
        let mut errors = Vec::new();
        validate_field_type(
            "related_tasks",
            &FieldType::EntityRefArray("task".to_string()),
            &json!("not-an-array"),
            &mut errors,
        );
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
        assert!(errors[0].message.contains("expected array of entity references"));
    }
}
