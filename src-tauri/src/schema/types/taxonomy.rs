use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for taxonomy entities.
pub const TAXONOMY_STATUSES: &[&str] = &["draft", "active", "archived"];

/// Returns the v1 field definitions for the `taxonomy` entity type.
pub fn taxonomy_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "taxonomy_type".to_string(),
            field_type: FieldType::Enum(vec![
                "category".to_string(),
                "tag".to_string(),
                "hierarchy".to_string(),
            ]),
            required: false,
            description: Some("The classification system this taxonomy uses".to_string()),
        },
        FieldDef {
            name: "parent_id".to_string(),
            field_type: FieldType::EntityRef("taxonomy".to_string()),
            required: false,
            description: Some("Reference to the parent taxonomy node".to_string()),
        },
        FieldDef {
            name: "level".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("Depth level in the taxonomy hierarchy".to_string()),
        },
    ]
}

pub fn taxonomy_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(taxonomy_v1_fields()),
        _ => None,
    }
}

pub fn taxonomy_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taxonomy_v1_field_count() {
        assert_eq!(taxonomy_v1_fields().len(), 3);
    }

    #[test]
    fn test_taxonomy_type_enum() {
        let fields = taxonomy_v1_fields();
        let tt = fields.iter().find(|f| f.name == "taxonomy_type").unwrap();
        match &tt.field_type {
            FieldType::Enum(values) => assert_eq!(values.len(), 3),
            _ => panic!("taxonomy_type should be Enum"),
        }
    }

    #[test]
    fn test_taxonomy_parent_id_ref() {
        let fields = taxonomy_v1_fields();
        let field = fields.iter().find(|f| f.name == "parent_id").unwrap();
        match &field.field_type {
            FieldType::EntityRef(t) => assert_eq!(t, "taxonomy"),
            _ => panic!("parent_id should be EntityRef"),
        }
    }

    #[test]
    fn test_taxonomy_statuses() {
        assert_eq!(TAXONOMY_STATUSES.len(), 3);
    }

    #[test]
    fn test_taxonomy_current_version() {
        assert_eq!(taxonomy_current_version(), 1);
    }
}
