use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for competitor entities.
pub const COMPETITOR_STATUSES: &[&str] = &["tracking", "dormant", "archived"];

/// Returns the v1 field definitions for the `competitor` entity type.
pub fn competitor_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "website".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Competitor website URL".to_string()),
        },
        FieldDef {
            name: "positioning".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("How this competitor positions themselves in the market".to_string()),
        },
        FieldDef {
            name: "strengths".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Key strengths of this competitor".to_string()),
        },
        FieldDef {
            name: "weaknesses".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Key weaknesses of this competitor".to_string()),
        },
        FieldDef {
            name: "market_share".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Estimated market share".to_string()),
        },
    ]
}

pub fn competitor_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(competitor_v1_fields()),
        _ => None,
    }
}

pub fn competitor_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_competitor_v1_field_count() {
        assert_eq!(competitor_v1_fields().len(), 5);
    }

    #[test]
    fn test_competitor_statuses() {
        assert_eq!(COMPETITOR_STATUSES.len(), 3);
    }

    #[test]
    fn test_competitor_current_version() {
        assert_eq!(competitor_current_version(), 1);
    }
}
