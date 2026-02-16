use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for person entities.
pub const PERSON_STATUSES: &[&str] = &["active", "inactive", "archived"];

/// Returns the v1 field definitions for the `person` entity type.
pub fn person_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "email".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Email address of the person".to_string()),
        },
        FieldDef {
            name: "role".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Role or job title of the person".to_string()),
        },
        FieldDef {
            name: "team".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Team or department the person belongs to".to_string()),
        },
        FieldDef {
            name: "external".to_string(),
            field_type: FieldType::Boolean,
            required: false,
            description: Some("Whether this person is external to the organization".to_string()),
        },
    ]
}

/// Returns field definitions for a given person schema version.
pub fn person_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(person_v1_fields()),
        _ => None,
    }
}

/// Returns the latest schema version for the person entity type.
pub fn person_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_v1_has_four_fields() {
        let fields = person_v1_fields();
        assert_eq!(fields.len(), 4);
    }

    #[test]
    fn test_person_v1_field_names() {
        let fields = person_v1_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"email"));
        assert!(names.contains(&"role"));
        assert!(names.contains(&"team"));
        assert!(names.contains(&"external"));
    }

    #[test]
    fn test_person_v1_all_optional() {
        let fields = person_v1_fields();
        for field in &fields {
            assert!(!field.required, "field {} should be optional", field.name);
        }
    }

    #[test]
    fn test_person_external_is_boolean() {
        let fields = person_v1_fields();
        let external = fields.iter().find(|f| f.name == "external").unwrap();
        assert_eq!(external.field_type, FieldType::Boolean);
    }

    #[test]
    fn test_person_unknown_version_returns_none() {
        assert!(person_fields(99).is_none());
    }

    #[test]
    fn test_person_current_version() {
        assert_eq!(person_current_version(), 1);
    }

    #[test]
    fn test_person_statuses() {
        assert_eq!(PERSON_STATUSES.len(), 3);
        assert!(PERSON_STATUSES.contains(&"active"));
        assert!(PERSON_STATUSES.contains(&"inactive"));
        assert!(PERSON_STATUSES.contains(&"archived"));
    }
}
