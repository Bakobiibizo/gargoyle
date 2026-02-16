use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for project entities.
pub const PROJECT_STATUSES: &[&str] = &["planning", "active", "paused", "completed", "archived"];

/// Returns the v1 field definitions for the `project` entity type.
pub fn project_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "owner_id".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("The owner or lead of this project".to_string()),
        },
        FieldDef {
            name: "objective".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("The primary objective of this project".to_string()),
        },
        FieldDef {
            name: "success_criteria".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Criteria that define project success".to_string()),
        },
        FieldDef {
            name: "timeline".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Expected timeline or deadline for the project".to_string()),
        },
    ]
}

/// Returns field definitions for a given project schema version.
pub fn project_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(project_v1_fields()),
        _ => None,
    }
}

/// Returns the latest schema version for the project entity type.
pub fn project_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_v1_has_four_fields() {
        let fields = project_v1_fields();
        assert_eq!(fields.len(), 4);
    }

    #[test]
    fn test_project_v1_field_names() {
        let fields = project_v1_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"owner_id"));
        assert!(names.contains(&"objective"));
        assert!(names.contains(&"success_criteria"));
        assert!(names.contains(&"timeline"));
    }

    #[test]
    fn test_project_v1_all_optional() {
        let fields = project_v1_fields();
        for field in &fields {
            assert!(!field.required, "field {} should be optional", field.name);
        }
    }

    #[test]
    fn test_project_unknown_version_returns_none() {
        assert!(project_fields(99).is_none());
    }

    #[test]
    fn test_project_current_version() {
        assert_eq!(project_current_version(), 1);
    }

    #[test]
    fn test_project_statuses() {
        assert_eq!(PROJECT_STATUSES.len(), 5);
        assert!(PROJECT_STATUSES.contains(&"planning"));
        assert!(PROJECT_STATUSES.contains(&"active"));
        assert!(PROJECT_STATUSES.contains(&"completed"));
    }
}
