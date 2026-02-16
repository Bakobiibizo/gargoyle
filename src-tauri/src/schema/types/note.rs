use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for note entities.
pub const NOTE_STATUSES: &[&str] = &["draft", "final", "archived"];

/// Returns the v1 field definitions for the `note` entity type.
pub fn note_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "context".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Context or topic area for this note".to_string()),
        },
        FieldDef {
            name: "tags".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Comma-separated tags for categorizing this note".to_string()),
        },
        FieldDef {
            name: "linked_entity_id".to_string(),
            field_type: FieldType::EntityRef("*".to_string()),
            required: false,
            description: Some("Reference to any entity this note is about".to_string()),
        },
    ]
}

/// Returns field definitions for a given note schema version.
pub fn note_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(note_v1_fields()),
        _ => None,
    }
}

/// Returns the latest schema version for the note entity type.
pub fn note_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_v1_has_three_fields() {
        let fields = note_v1_fields();
        assert_eq!(fields.len(), 3);
    }

    #[test]
    fn test_note_v1_field_names() {
        let fields = note_v1_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"context"));
        assert!(names.contains(&"tags"));
        assert!(names.contains(&"linked_entity_id"));
    }

    #[test]
    fn test_note_v1_all_optional() {
        let fields = note_v1_fields();
        for field in &fields {
            assert!(!field.required, "field {} should be optional", field.name);
        }
    }

    #[test]
    fn test_note_linked_entity_is_wildcard_ref() {
        let fields = note_v1_fields();
        let linked = fields.iter().find(|f| f.name == "linked_entity_id").unwrap();
        match &linked.field_type {
            FieldType::EntityRef(target) => {
                assert_eq!(target, "*", "linked_entity_id should accept any entity type");
            }
            _ => panic!("linked_entity_id should be an EntityRef type"),
        }
    }

    #[test]
    fn test_note_unknown_version_returns_none() {
        assert!(note_fields(99).is_none());
    }

    #[test]
    fn test_note_current_version() {
        assert_eq!(note_current_version(), 1);
    }

    #[test]
    fn test_note_statuses() {
        assert_eq!(NOTE_STATUSES.len(), 3);
        assert!(NOTE_STATUSES.contains(&"draft"));
        assert!(NOTE_STATUSES.contains(&"final"));
        assert!(NOTE_STATUSES.contains(&"archived"));
    }
}
