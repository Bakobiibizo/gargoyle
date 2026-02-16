use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for spec entities.
pub const SPEC_STATUSES: &[&str] = &["draft", "review", "approved", "deprecated"];

/// Returns the v1 field definitions for the `spec` entity type.
pub fn spec_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "spec_type".to_string(),
            field_type: FieldType::Enum(vec![
                "technical".to_string(),
                "product".to_string(),
                "design".to_string(),
                "process".to_string(),
            ]),
            required: false,
            description: Some("The type of specification".to_string()),
        },
        FieldDef {
            name: "version".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Version string for this spec".to_string()),
        },
        FieldDef {
            name: "approval_status".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Current approval status".to_string()),
        },
        FieldDef {
            name: "author".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Author of this specification".to_string()),
        },
    ]
}

pub fn spec_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(spec_v1_fields()),
        _ => None,
    }
}

pub fn spec_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_v1_field_count() {
        assert_eq!(spec_v1_fields().len(), 4);
    }

    #[test]
    fn test_spec_type_enum() {
        let fields = spec_v1_fields();
        let st = fields.iter().find(|f| f.name == "spec_type").unwrap();
        match &st.field_type {
            FieldType::Enum(values) => assert_eq!(values.len(), 4),
            _ => panic!("spec_type should be Enum"),
        }
    }

    #[test]
    fn test_spec_statuses() {
        assert_eq!(SPEC_STATUSES.len(), 4);
    }

    #[test]
    fn test_spec_current_version() {
        assert_eq!(spec_current_version(), 1);
    }
}
