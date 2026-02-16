use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for decision entities.
pub const DECISION_STATUSES: &[&str] = &["proposed", "accepted", "deprecated", "superseded"];

/// Returns the v1 field definitions for the `decision` entity type.
pub fn decision_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "owner_id".to_string(),
            field_type: FieldType::String,
            required: true,
            description: Some("The person responsible for this decision".to_string()),
        },
        FieldDef {
            name: "decided_at".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("When the decision was made (ISO 8601)".to_string()),
        },
        FieldDef {
            name: "rationale".to_string(),
            field_type: FieldType::String,
            required: true,
            description: Some("The reasoning behind this decision".to_string()),
        },
        FieldDef {
            name: "revisit_triggers".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Conditions that would trigger revisiting this decision".to_string()),
        },
        FieldDef {
            name: "options_considered".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Other options that were considered before this decision".to_string()),
        },
    ]
}

/// Returns field definitions for a given decision schema version.
pub fn decision_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(decision_v1_fields()),
        _ => None,
    }
}

/// Returns the latest schema version for the decision entity type.
pub fn decision_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_v1_has_five_fields() {
        let fields = decision_v1_fields();
        assert_eq!(fields.len(), 5);
    }

    #[test]
    fn test_decision_v1_field_names() {
        let fields = decision_v1_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"owner_id"));
        assert!(names.contains(&"decided_at"));
        assert!(names.contains(&"rationale"));
        assert!(names.contains(&"revisit_triggers"));
        assert!(names.contains(&"options_considered"));
    }

    #[test]
    fn test_decision_required_fields() {
        let fields = decision_v1_fields();
        let owner_id = fields.iter().find(|f| f.name == "owner_id").unwrap();
        assert!(owner_id.required, "owner_id should be required");

        let rationale = fields.iter().find(|f| f.name == "rationale").unwrap();
        assert!(rationale.required, "rationale should be required");
    }

    #[test]
    fn test_decision_optional_fields() {
        let fields = decision_v1_fields();
        let optional_names = ["decided_at", "revisit_triggers", "options_considered"];
        for name in &optional_names {
            let field = fields.iter().find(|f| f.name == *name).unwrap();
            assert!(!field.required, "field {} should be optional", name);
        }
    }

    #[test]
    fn test_decision_unknown_version_returns_none() {
        assert!(decision_fields(99).is_none());
    }

    #[test]
    fn test_decision_current_version() {
        assert_eq!(decision_current_version(), 1);
    }

    #[test]
    fn test_decision_statuses() {
        assert_eq!(DECISION_STATUSES.len(), 4);
        assert!(DECISION_STATUSES.contains(&"proposed"));
        assert!(DECISION_STATUSES.contains(&"accepted"));
        assert!(DECISION_STATUSES.contains(&"deprecated"));
        assert!(DECISION_STATUSES.contains(&"superseded"));
    }
}
