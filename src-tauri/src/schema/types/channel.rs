use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for channel entities.
pub const CHANNEL_STATUSES: &[&str] = &["evaluating", "active", "scaling", "paused", "deprecated"];

/// Returns the v1 field definitions for the `channel` entity type.
pub fn channel_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "channel_type".to_string(),
            field_type: FieldType::Enum(vec![
                "email".to_string(),
                "social".to_string(),
                "search".to_string(),
                "display".to_string(),
                "events".to_string(),
                "partnerships".to_string(),
                "content".to_string(),
                "referral".to_string(),
            ]),
            required: false,
            description: Some("The type of marketing/distribution channel".to_string()),
        },
        FieldDef {
            name: "cost_model".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Cost model (e.g., CPC, CPM, flat fee)".to_string()),
        },
        FieldDef {
            name: "primary_metric_id".to_string(),
            field_type: FieldType::EntityRef("metric".to_string()),
            required: false,
            description: Some("The primary metric used to measure this channel".to_string()),
        },
        FieldDef {
            name: "budget_allocation".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("Budget allocated to this channel".to_string()),
        },
    ]
}

pub fn channel_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(channel_v1_fields()),
        _ => None,
    }
}

pub fn channel_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_v1_field_count() {
        assert_eq!(channel_v1_fields().len(), 4);
    }

    #[test]
    fn test_channel_type_enum() {
        let fields = channel_v1_fields();
        let ct = fields.iter().find(|f| f.name == "channel_type").unwrap();
        match &ct.field_type {
            FieldType::Enum(values) => assert_eq!(values.len(), 8),
            _ => panic!("channel_type should be Enum"),
        }
    }

    #[test]
    fn test_channel_metric_ref() {
        let fields = channel_v1_fields();
        let field = fields.iter().find(|f| f.name == "primary_metric_id").unwrap();
        match &field.field_type {
            FieldType::EntityRef(t) => assert_eq!(t, "metric"),
            _ => panic!("primary_metric_id should be EntityRef"),
        }
    }

    #[test]
    fn test_channel_statuses() {
        assert_eq!(CHANNEL_STATUSES.len(), 5);
    }

    #[test]
    fn test_channel_current_version() {
        assert_eq!(channel_current_version(), 1);
    }
}
