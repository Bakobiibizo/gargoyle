use std::sync::OnceLock;

use crate::error::{ErrorCode, ValidationError};
use crate::schema::field_def::{FieldDef, FieldType};
use crate::schema::types::{
    experiment::{experiment_current_version, experiment_fields, EXPERIMENT_STATUSES},
    metric::{metric_current_version, metric_fields, METRIC_STATUSES},
    result::{result_current_version, result_fields, RESULT_STATUSES},
};
use crate::schema::version::SchemaVersion;

/// Global singleton schema registry, initialized on first access.
static REGISTRY: OnceLock<SchemaRegistry> = OnceLock::new();

/// The SchemaRegistry is the central authority for entity type schemas.
/// It provides field definitions, version tracking, and canonical_fields validation
/// for all known entity types (metric, experiment, result).
///
/// The registry uses a static/lazy approach -- no database table is needed.
/// Schemas are defined in code and versioned explicitly.
pub struct SchemaRegistry {
    schema_version: SchemaVersion,
}

impl SchemaRegistry {
    /// Creates a new SchemaRegistry with current versions for all entity types.
    pub fn new() -> Self {
        Self {
            schema_version: SchemaVersion::new(),
        }
    }

    /// Returns a reference to the global singleton SchemaRegistry.
    pub fn global() -> &'static SchemaRegistry {
        REGISTRY.get_or_init(SchemaRegistry::new)
    }

    /// Returns the field definitions for a given entity type and version.
    /// Returns None if the entity type or version is not recognized.
    pub fn get_schema(&self, entity_type: &str, version: i32) -> Option<Vec<FieldDef>> {
        match entity_type {
            "metric" => metric_fields(version),
            "experiment" => experiment_fields(version),
            "result" => result_fields(version),
            _ => None,
        }
    }

    /// Returns the current (latest) schema version for the given entity type.
    /// Returns None if the entity type is not recognized.
    pub fn current_version(&self, entity_type: &str) -> Option<i32> {
        match entity_type {
            "metric" => Some(metric_current_version()),
            "experiment" => Some(experiment_current_version()),
            "result" => Some(result_current_version()),
            _ => None,
        }
    }

    /// Returns the valid status values for the given entity type.
    /// Returns None if the entity type is not recognized.
    pub fn valid_statuses(&self, entity_type: &str) -> Option<&'static [&'static str]> {
        match entity_type {
            "metric" => Some(METRIC_STATUSES),
            "experiment" => Some(EXPERIMENT_STATUSES),
            "result" => Some(RESULT_STATUSES),
            _ => None,
        }
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
        FieldType::EntityRef(_target_type) => {
            // EntityRef values must be strings (the referenced entity ID).
            // Actual referential integrity checking is deferred to the referential_validator.
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
        SchemaRegistry::new()
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
        assert_eq!(statuses, &["draft", "final", "archived"]);
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
            "source_experiment_id": "exp-uuid-123"
        });
        let errors = reg.validate_canonical_fields("experiment", 1, &fields);
        assert!(errors.is_empty(), "Valid experiment fields should produce no errors: {:?}", errors);
    }

    #[test]
    fn test_validate_valid_result_fields() {
        let reg = registry();
        let fields = json!({
            "findings": "Button color had no significant effect",
            "methodology": "A/B test with 10k users",
            "confidence_level": 0.95
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
            "findings": 12345
        });
        let errors = reg.validate_canonical_fields("result", 1, &fields);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidFieldType));
        assert_eq!(errors[0].field_path, "canonical_fields.findings");
    }

    #[test]
    fn test_validate_wrong_type_bool_for_number() {
        let reg = registry();
        let fields = json!({
            "confidence_level": true
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
            "source_experiment_id": "some-uuid"
        });
        let errors = reg.validate_canonical_fields("experiment", 1, &fields);
        assert!(errors.is_empty(), "Entity ref as string should be valid");
    }

    #[test]
    fn test_validate_entity_ref_wrong_type() {
        let reg = registry();
        let fields = json!({
            "source_experiment_id": 42
        });
        let errors = reg.validate_canonical_fields("experiment", 1, &fields);
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
}
