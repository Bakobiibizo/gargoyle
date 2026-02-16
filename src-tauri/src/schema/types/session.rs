use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for session entities.
pub const SESSION_STATUSES: &[&str] = &["scheduled", "in_progress", "completed", "cancelled"];

/// Returns the v1 field definitions for the `session` entity type.
pub fn session_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "session_type".to_string(),
            field_type: FieldType::Enum(vec![
                "planning".to_string(),
                "review".to_string(),
                "standup".to_string(),
                "workshop".to_string(),
                "retro".to_string(),
            ]),
            required: false,
            description: Some("The type of session".to_string()),
        },
        FieldDef {
            name: "participants".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Comma-separated list of participant names or IDs".to_string()),
        },
        FieldDef {
            name: "agenda".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Agenda or topics to cover in this session".to_string()),
        },
        FieldDef {
            name: "outcomes".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Outcomes or conclusions from this session".to_string()),
        },
    ]
}

/// Returns field definitions for a given session schema version.
pub fn session_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(session_v1_fields()),
        _ => None,
    }
}

/// Returns the latest schema version for the session entity type.
pub fn session_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_v1_has_four_fields() {
        let fields = session_v1_fields();
        assert_eq!(fields.len(), 4);
    }

    #[test]
    fn test_session_v1_field_names() {
        let fields = session_v1_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"session_type"));
        assert!(names.contains(&"participants"));
        assert!(names.contains(&"agenda"));
        assert!(names.contains(&"outcomes"));
    }

    #[test]
    fn test_session_v1_all_optional() {
        let fields = session_v1_fields();
        for field in &fields {
            assert!(!field.required, "field {} should be optional", field.name);
        }
    }

    #[test]
    fn test_session_type_is_enum() {
        let fields = session_v1_fields();
        let session_type = fields.iter().find(|f| f.name == "session_type").unwrap();
        match &session_type.field_type {
            FieldType::Enum(values) => {
                assert_eq!(values.len(), 5);
                assert!(values.contains(&"planning".to_string()));
                assert!(values.contains(&"review".to_string()));
                assert!(values.contains(&"standup".to_string()));
                assert!(values.contains(&"workshop".to_string()));
                assert!(values.contains(&"retro".to_string()));
            }
            _ => panic!("session_type should be an Enum type"),
        }
    }

    #[test]
    fn test_session_unknown_version_returns_none() {
        assert!(session_fields(99).is_none());
    }

    #[test]
    fn test_session_current_version() {
        assert_eq!(session_current_version(), 1);
    }

    #[test]
    fn test_session_statuses() {
        assert_eq!(SESSION_STATUSES.len(), 4);
        assert!(SESSION_STATUSES.contains(&"scheduled"));
        assert!(SESSION_STATUSES.contains(&"in_progress"));
        assert!(SESSION_STATUSES.contains(&"completed"));
        assert!(SESSION_STATUSES.contains(&"cancelled"));
    }
}
