use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for backlog entities.
pub const BACKLOG_STATUSES: &[&str] = &["open", "triaged", "scheduled", "closed"];

/// Returns the v1 field definitions for the `backlog` entity type.
pub fn backlog_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "priority_score".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("Numeric priority score for ranking".to_string()),
        },
        FieldDef {
            name: "effort".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Estimated effort for this backlog item".to_string()),
        },
        FieldDef {
            name: "requester".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Person or team who requested this item".to_string()),
        },
        FieldDef {
            name: "target_sprint".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Target sprint or iteration for scheduling".to_string()),
        },
    ]
}

pub fn backlog_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(backlog_v1_fields()),
        _ => None,
    }
}

pub fn backlog_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backlog_v1_field_count() {
        assert_eq!(backlog_v1_fields().len(), 4);
    }

    #[test]
    fn test_backlog_field_names() {
        let fields = backlog_v1_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"priority_score"));
        assert!(names.contains(&"effort"));
        assert!(names.contains(&"requester"));
        assert!(names.contains(&"target_sprint"));
    }

    #[test]
    fn test_backlog_statuses() {
        assert_eq!(BACKLOG_STATUSES.len(), 4);
    }

    #[test]
    fn test_backlog_current_version() {
        assert_eq!(backlog_current_version(), 1);
    }
}
