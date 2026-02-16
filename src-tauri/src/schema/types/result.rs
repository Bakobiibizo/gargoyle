use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for result entities.
pub const RESULT_STATUSES: &[&str] = &["draft", "final", "archived"];

/// Returns the v1 field definitions for the `result` entity type.
pub fn result_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "findings".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("The findings or conclusions of the result".to_string()),
        },
        FieldDef {
            name: "methodology".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("The methodology used to produce this result".to_string()),
        },
        FieldDef {
            name: "confidence_level".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some(
                "The confidence level of the result, typically 0.0 to 1.0".to_string(),
            ),
        },
    ]
}

/// Returns field definitions for a given result schema version.
/// Currently only v1 is defined.
pub fn result_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(result_v1_fields()),
        _ => None,
    }
}

/// Returns the latest schema version for the result entity type.
pub fn result_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_v1_has_three_fields() {
        let fields = result_v1_fields();
        assert_eq!(fields.len(), 3);
    }

    #[test]
    fn test_result_v1_field_names() {
        let fields = result_v1_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"findings"));
        assert!(names.contains(&"methodology"));
        assert!(names.contains(&"confidence_level"));
    }

    #[test]
    fn test_result_v1_all_optional() {
        let fields = result_v1_fields();
        for field in &fields {
            assert!(!field.required, "field {} should be optional", field.name);
        }
    }

    #[test]
    fn test_result_confidence_is_number() {
        let fields = result_v1_fields();
        let confidence = fields
            .iter()
            .find(|f| f.name == "confidence_level")
            .unwrap();
        assert_eq!(confidence.field_type, FieldType::Number);
    }

    #[test]
    fn test_result_unknown_version_returns_none() {
        assert!(result_fields(99).is_none());
    }

    #[test]
    fn test_result_current_version() {
        assert_eq!(result_current_version(), 1);
    }
}
