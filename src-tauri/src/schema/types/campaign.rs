use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for campaign entities.
pub const CAMPAIGN_STATUSES: &[&str] = &["planning", "active", "paused", "completed", "archived"];

/// Returns the v1 field definitions for the `campaign` entity type.
pub fn campaign_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "objective".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("The primary objective of this campaign".to_string()),
        },
        FieldDef {
            name: "budget".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("Campaign budget amount".to_string()),
        },
        FieldDef {
            name: "channel".to_string(),
            field_type: FieldType::Enum(vec![
                "email".to_string(),
                "paid_social".to_string(),
                "paid_search".to_string(),
                "organic".to_string(),
                "events".to_string(),
                "partnerships".to_string(),
            ]),
            required: false,
            description: Some("Primary channel for this campaign".to_string()),
        },
        FieldDef {
            name: "start_date".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Campaign start date (ISO 8601)".to_string()),
        },
        FieldDef {
            name: "end_date".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Campaign end date (ISO 8601)".to_string()),
        },
        FieldDef {
            name: "target_audience_id".to_string(),
            field_type: FieldType::EntityRef("audience".to_string()),
            required: false,
            description: Some("Reference to the target audience for this campaign".to_string()),
        },
    ]
}

pub fn campaign_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(campaign_v1_fields()),
        _ => None,
    }
}

pub fn campaign_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_campaign_v1_field_count() {
        assert_eq!(campaign_v1_fields().len(), 6);
    }

    #[test]
    fn test_campaign_channel_enum() {
        let fields = campaign_v1_fields();
        let channel = fields.iter().find(|f| f.name == "channel").unwrap();
        match &channel.field_type {
            FieldType::Enum(values) => assert_eq!(values.len(), 6),
            _ => panic!("channel should be Enum"),
        }
    }

    #[test]
    fn test_campaign_target_audience_ref() {
        let fields = campaign_v1_fields();
        let field = fields.iter().find(|f| f.name == "target_audience_id").unwrap();
        match &field.field_type {
            FieldType::EntityRef(t) => assert_eq!(t, "audience"),
            _ => panic!("target_audience_id should be EntityRef"),
        }
    }

    #[test]
    fn test_campaign_statuses() {
        assert_eq!(CAMPAIGN_STATUSES.len(), 5);
    }

    #[test]
    fn test_campaign_current_version() {
        assert_eq!(campaign_current_version(), 1);
    }
}
