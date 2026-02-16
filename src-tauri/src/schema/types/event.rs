use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for event entities.
pub const EVENT_STATUSES: &[&str] = &["proposed", "confirmed", "in_progress", "completed", "cancelled"];

/// Returns the v1 field definitions for the `event` entity type.
pub fn event_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "event_type".to_string(),
            field_type: FieldType::Enum(vec![
                "conference".to_string(),
                "webinar".to_string(),
                "meetup".to_string(),
                "workshop".to_string(),
                "launch".to_string(),
            ]),
            required: false,
            description: Some("The type of event".to_string()),
        },
        FieldDef {
            name: "venue".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Venue or location for the event".to_string()),
        },
        FieldDef {
            name: "start_date".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Event start date (ISO 8601)".to_string()),
        },
        FieldDef {
            name: "end_date".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Event end date (ISO 8601)".to_string()),
        },
        FieldDef {
            name: "expected_attendees".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("Expected number of attendees".to_string()),
        },
    ]
}

pub fn event_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(event_v1_fields()),
        _ => None,
    }
}

pub fn event_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_v1_field_count() {
        assert_eq!(event_v1_fields().len(), 5);
    }

    #[test]
    fn test_event_type_enum() {
        let fields = event_v1_fields();
        let et = fields.iter().find(|f| f.name == "event_type").unwrap();
        match &et.field_type {
            FieldType::Enum(values) => assert_eq!(values.len(), 5),
            _ => panic!("event_type should be Enum"),
        }
    }

    #[test]
    fn test_event_statuses() {
        assert_eq!(EVENT_STATUSES.len(), 5);
    }

    #[test]
    fn test_event_current_version() {
        assert_eq!(event_current_version(), 1);
    }
}
