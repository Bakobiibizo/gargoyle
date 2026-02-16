use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for audience entities.
pub const AUDIENCE_STATUSES: &[&str] = &["draft", "validated", "active", "archived"];

/// Returns the v1 field definitions for the `audience` entity type.
pub fn audience_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "segment_criteria".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Criteria that define this audience segment".to_string()),
        },
        FieldDef {
            name: "estimated_size".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("Estimated audience size".to_string()),
        },
        FieldDef {
            name: "icp_id".to_string(),
            field_type: FieldType::EntityRef("person".to_string()),
            required: false,
            description: Some("Reference to the ICP person this audience maps to".to_string()),
        },
        FieldDef {
            name: "channels".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Preferred channels for reaching this audience".to_string()),
        },
    ]
}

pub fn audience_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(audience_v1_fields()),
        _ => None,
    }
}

pub fn audience_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audience_v1_field_count() {
        assert_eq!(audience_v1_fields().len(), 4);
    }

    #[test]
    fn test_audience_icp_ref() {
        let fields = audience_v1_fields();
        let field = fields.iter().find(|f| f.name == "icp_id").unwrap();
        match &field.field_type {
            FieldType::EntityRef(t) => assert_eq!(t, "person"),
            _ => panic!("icp_id should be EntityRef"),
        }
    }

    #[test]
    fn test_audience_statuses() {
        assert_eq!(AUDIENCE_STATUSES.len(), 4);
    }

    #[test]
    fn test_audience_current_version() {
        assert_eq!(audience_current_version(), 1);
    }
}
