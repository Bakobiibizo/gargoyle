use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for task entities.
pub const TASK_STATUSES: &[&str] = &["backlog", "todo", "in_progress", "blocked", "done", "archived"];

/// Returns the v1 field definitions for the `task` entity type.
pub fn task_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "assignee".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("The person assigned to this task".to_string()),
        },
        FieldDef {
            name: "effort_estimate".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Estimated effort for the task (e.g., 'S', 'M', 'L', '3 days')".to_string()),
        },
        FieldDef {
            name: "project_id".to_string(),
            field_type: FieldType::EntityRef("project".to_string()),
            required: false,
            description: Some("Reference to the project this task belongs to".to_string()),
        },
        FieldDef {
            name: "acceptance_criteria".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Criteria that must be met for this task to be considered done".to_string()),
        },
    ]
}

/// Returns field definitions for a given task schema version.
pub fn task_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(task_v1_fields()),
        _ => None,
    }
}

/// Returns the latest schema version for the task entity type.
pub fn task_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_v1_has_four_fields() {
        let fields = task_v1_fields();
        assert_eq!(fields.len(), 4);
    }

    #[test]
    fn test_task_v1_field_names() {
        let fields = task_v1_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"assignee"));
        assert!(names.contains(&"effort_estimate"));
        assert!(names.contains(&"project_id"));
        assert!(names.contains(&"acceptance_criteria"));
    }

    #[test]
    fn test_task_v1_all_optional() {
        let fields = task_v1_fields();
        for field in &fields {
            assert!(!field.required, "field {} should be optional", field.name);
        }
    }

    #[test]
    fn test_task_project_id_is_entity_ref() {
        let fields = task_v1_fields();
        let project_id = fields.iter().find(|f| f.name == "project_id").unwrap();
        match &project_id.field_type {
            FieldType::EntityRef(target) => {
                assert_eq!(target, "project");
            }
            _ => panic!("project_id should be an EntityRef type"),
        }
    }

    #[test]
    fn test_task_unknown_version_returns_none() {
        assert!(task_fields(99).is_none());
    }

    #[test]
    fn test_task_current_version() {
        assert_eq!(task_current_version(), 1);
    }

    #[test]
    fn test_task_statuses() {
        assert_eq!(TASK_STATUSES.len(), 6);
        assert!(TASK_STATUSES.contains(&"backlog"));
        assert!(TASK_STATUSES.contains(&"todo"));
        assert!(TASK_STATUSES.contains(&"in_progress"));
        assert!(TASK_STATUSES.contains(&"blocked"));
        assert!(TASK_STATUSES.contains(&"done"));
        assert!(TASK_STATUSES.contains(&"archived"));
    }
}
