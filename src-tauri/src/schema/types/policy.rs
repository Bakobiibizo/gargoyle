use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for policy entities.
pub const POLICY_STATUSES: &[&str] = &["draft", "active", "under_review", "deprecated"];

/// Returns the v1 field definitions for the `policy` entity type.
pub fn policy_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "policy_type".to_string(),
            field_type: FieldType::Enum(vec![
                "security".to_string(),
                "hr".to_string(),
                "compliance".to_string(),
                "operational".to_string(),
            ]),
            required: false,
            description: Some("The category of policy".to_string()),
        },
        FieldDef {
            name: "effective_date".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Date when the policy becomes effective (ISO 8601)".to_string()),
        },
        FieldDef {
            name: "review_date".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Next scheduled review date (ISO 8601)".to_string()),
        },
        FieldDef {
            name: "owner".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Owner responsible for this policy".to_string()),
        },
    ]
}

pub fn policy_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(policy_v1_fields()),
        _ => None,
    }
}

pub fn policy_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_v1_field_count() {
        assert_eq!(policy_v1_fields().len(), 4);
    }

    #[test]
    fn test_policy_type_enum() {
        let fields = policy_v1_fields();
        let pt = fields.iter().find(|f| f.name == "policy_type").unwrap();
        match &pt.field_type {
            FieldType::Enum(values) => assert_eq!(values.len(), 4),
            _ => panic!("policy_type should be Enum"),
        }
    }

    #[test]
    fn test_policy_statuses() {
        assert_eq!(POLICY_STATUSES.len(), 4);
    }

    #[test]
    fn test_policy_current_version() {
        assert_eq!(policy_current_version(), 1);
    }
}
