use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for playbook entities.
pub const PLAYBOOK_STATUSES: &[&str] = &["draft", "active", "deprecated", "archived"];

/// Returns the v1 field definitions for the `playbook` entity type.
pub fn playbook_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "playbook_type".to_string(),
            field_type: FieldType::Enum(vec![
                "sales".to_string(),
                "marketing".to_string(),
                "ops".to_string(),
                "cs".to_string(),
                "dev".to_string(),
            ]),
            required: false,
            description: Some("The functional area this playbook serves".to_string()),
        },
        FieldDef {
            name: "trigger_conditions".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Conditions that trigger execution of this playbook".to_string()),
        },
        FieldDef {
            name: "expected_outcome".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Expected outcome from executing this playbook".to_string()),
        },
        FieldDef {
            name: "owner".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Owner responsible for maintaining this playbook".to_string()),
        },
    ]
}

pub fn playbook_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(playbook_v1_fields()),
        _ => None,
    }
}

pub fn playbook_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playbook_v1_field_count() {
        assert_eq!(playbook_v1_fields().len(), 4);
    }

    #[test]
    fn test_playbook_type_enum() {
        let fields = playbook_v1_fields();
        let pt = fields.iter().find(|f| f.name == "playbook_type").unwrap();
        match &pt.field_type {
            FieldType::Enum(values) => assert_eq!(values.len(), 5),
            _ => panic!("playbook_type should be Enum"),
        }
    }

    #[test]
    fn test_playbook_statuses() {
        assert_eq!(PLAYBOOK_STATUSES.len(), 4);
    }

    #[test]
    fn test_playbook_current_version() {
        assert_eq!(playbook_current_version(), 1);
    }
}
