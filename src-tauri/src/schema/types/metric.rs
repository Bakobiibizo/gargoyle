use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for metric entities.
pub const METRIC_STATUSES: &[&str] = &["active", "paused", "deprecated", "archived"];

/// Returns the v1 field definitions for the `metric` entity type.
pub fn metric_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "current_value".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("The current measured value of the metric".to_string()),
        },
        FieldDef {
            name: "target_value".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("The target value the metric aims to reach".to_string()),
        },
        FieldDef {
            name: "trend".to_string(),
            field_type: FieldType::Enum(vec![
                "up".to_string(),
                "down".to_string(),
                "flat".to_string(),
            ]),
            required: false,
            description: Some("The directional trend of the metric".to_string()),
        },
        FieldDef {
            name: "data_source".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Where the metric data comes from".to_string()),
        },
    ]
}

/// Returns field definitions for a given metric schema version.
/// Currently only v1 is defined.
pub fn metric_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(metric_v1_fields()),
        _ => None,
    }
}

/// Returns the latest schema version for the metric entity type.
pub fn metric_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_v1_has_four_fields() {
        let fields = metric_v1_fields();
        assert_eq!(fields.len(), 4);
    }

    #[test]
    fn test_metric_v1_field_names() {
        let fields = metric_v1_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"current_value"));
        assert!(names.contains(&"target_value"));
        assert!(names.contains(&"trend"));
        assert!(names.contains(&"data_source"));
    }

    #[test]
    fn test_metric_v1_all_optional() {
        let fields = metric_v1_fields();
        for field in &fields {
            assert!(!field.required, "field {} should be optional", field.name);
        }
    }

    #[test]
    fn test_metric_trend_is_enum() {
        let fields = metric_v1_fields();
        let trend = fields.iter().find(|f| f.name == "trend").unwrap();
        match &trend.field_type {
            FieldType::Enum(values) => {
                assert_eq!(values, &vec!["up", "down", "flat"]);
            }
            _ => panic!("trend should be an Enum type"),
        }
    }

    #[test]
    fn test_metric_unknown_version_returns_none() {
        assert!(metric_fields(99).is_none());
    }

    #[test]
    fn test_metric_current_version() {
        assert_eq!(metric_current_version(), 1);
    }
}
