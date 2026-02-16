use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for brief entities.
pub const BRIEF_STATUSES: &[&str] = &["draft", "review", "approved", "archived"];

/// Returns the v1 field definitions for the `brief` entity type.
pub fn brief_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "brief_type".to_string(),
            field_type: FieldType::Enum(vec![
                "creative".to_string(),
                "campaign".to_string(),
                "product".to_string(),
                "event".to_string(),
            ]),
            required: false,
            description: Some("The type of brief".to_string()),
        },
        FieldDef {
            name: "deadline".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Deadline for brief deliverables (ISO 8601)".to_string()),
        },
        FieldDef {
            name: "stakeholders".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Key stakeholders for this brief".to_string()),
        },
        FieldDef {
            name: "deliverables".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Expected deliverables from this brief".to_string()),
        },
    ]
}

pub fn brief_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(brief_v1_fields()),
        _ => None,
    }
}

pub fn brief_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brief_v1_field_count() {
        assert_eq!(brief_v1_fields().len(), 4);
    }

    #[test]
    fn test_brief_type_enum() {
        let fields = brief_v1_fields();
        let bt = fields.iter().find(|f| f.name == "brief_type").unwrap();
        match &bt.field_type {
            FieldType::Enum(values) => assert_eq!(values.len(), 4),
            _ => panic!("brief_type should be Enum"),
        }
    }

    #[test]
    fn test_brief_statuses() {
        assert_eq!(BRIEF_STATUSES.len(), 4);
    }

    #[test]
    fn test_brief_current_version() {
        assert_eq!(brief_current_version(), 1);
    }
}
