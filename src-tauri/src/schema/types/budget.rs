use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for budget entities.
pub const BUDGET_STATUSES: &[&str] = &["draft", "approved", "active", "closed"];

/// Returns the v1 field definitions for the `budget` entity type.
pub fn budget_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "total_amount".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("Total budget amount".to_string()),
        },
        FieldDef {
            name: "currency".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Currency code (e.g., USD, EUR)".to_string()),
        },
        FieldDef {
            name: "period".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Budget period (e.g., Q1 2024, FY2024)".to_string()),
        },
        FieldDef {
            name: "allocated".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("Amount currently allocated".to_string()),
        },
        FieldDef {
            name: "spent".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("Amount currently spent".to_string()),
        },
    ]
}

pub fn budget_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(budget_v1_fields()),
        _ => None,
    }
}

pub fn budget_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_v1_field_count() {
        assert_eq!(budget_v1_fields().len(), 5);
    }

    #[test]
    fn test_budget_statuses() {
        assert_eq!(BUDGET_STATUSES.len(), 4);
    }

    #[test]
    fn test_budget_current_version() {
        assert_eq!(budget_current_version(), 1);
    }
}
