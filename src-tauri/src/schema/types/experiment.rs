use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for experiment entities.
pub const EXPERIMENT_STATUSES: &[&str] = &["draft", "running", "concluded", "archived"];

/// Returns the v1 field definitions for the `experiment` entity type.
pub fn experiment_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "hypothesis".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("The hypothesis being tested by this experiment".to_string()),
        },
        FieldDef {
            name: "funnel_position".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some(
                "Where in the funnel this experiment operates".to_string(),
            ),
        },
        FieldDef {
            name: "source_experiment_id".to_string(),
            field_type: FieldType::EntityRef("experiment".to_string()),
            required: false,
            description: Some(
                "Reference to the experiment this one was derived from".to_string(),
            ),
        },
    ]
}

/// Returns field definitions for a given experiment schema version.
/// Currently only v1 is defined.
pub fn experiment_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(experiment_v1_fields()),
        _ => None,
    }
}

/// Returns the latest schema version for the experiment entity type.
pub fn experiment_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experiment_v1_has_three_fields() {
        let fields = experiment_v1_fields();
        assert_eq!(fields.len(), 3);
    }

    #[test]
    fn test_experiment_v1_field_names() {
        let fields = experiment_v1_fields();
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"hypothesis"));
        assert!(names.contains(&"funnel_position"));
        assert!(names.contains(&"source_experiment_id"));
    }

    #[test]
    fn test_experiment_v1_all_optional() {
        let fields = experiment_v1_fields();
        for field in &fields {
            assert!(!field.required, "field {} should be optional", field.name);
        }
    }

    #[test]
    fn test_experiment_source_is_entity_ref() {
        let fields = experiment_v1_fields();
        let source = fields
            .iter()
            .find(|f| f.name == "source_experiment_id")
            .unwrap();
        match &source.field_type {
            FieldType::EntityRef(target) => {
                assert_eq!(target, "experiment");
            }
            _ => panic!("source_experiment_id should be an EntityRef type"),
        }
    }

    #[test]
    fn test_experiment_unknown_version_returns_none() {
        assert!(experiment_fields(99).is_none());
    }

    #[test]
    fn test_experiment_current_version() {
        assert_eq!(experiment_current_version(), 1);
    }
}
