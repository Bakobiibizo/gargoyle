use crate::schema::field_def::{FieldDef, FieldType};

/// Valid status values for vendor entities.
pub const VENDOR_STATUSES: &[&str] = &["evaluating", "active", "on_hold", "terminated"];

/// Returns the v1 field definitions for the `vendor` entity type.
pub fn vendor_v1_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            name: "vendor_type".to_string(),
            field_type: FieldType::Enum(vec![
                "agency".to_string(),
                "saas".to_string(),
                "contractor".to_string(),
                "platform".to_string(),
            ]),
            required: false,
            description: Some("Type of vendor".to_string()),
        },
        FieldDef {
            name: "contract_value".to_string(),
            field_type: FieldType::Number,
            required: false,
            description: Some("Total contract value".to_string()),
        },
        FieldDef {
            name: "contract_end".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Contract end date (ISO 8601)".to_string()),
        },
        FieldDef {
            name: "primary_contact".to_string(),
            field_type: FieldType::String,
            required: false,
            description: Some("Primary contact person at this vendor".to_string()),
        },
    ]
}

pub fn vendor_fields(version: i32) -> Option<Vec<FieldDef>> {
    match version {
        1 => Some(vendor_v1_fields()),
        _ => None,
    }
}

pub fn vendor_current_version() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vendor_v1_field_count() {
        assert_eq!(vendor_v1_fields().len(), 4);
    }

    #[test]
    fn test_vendor_type_enum() {
        let fields = vendor_v1_fields();
        let vt = fields.iter().find(|f| f.name == "vendor_type").unwrap();
        match &vt.field_type {
            FieldType::Enum(values) => assert_eq!(values.len(), 4),
            _ => panic!("vendor_type should be Enum"),
        }
    }

    #[test]
    fn test_vendor_statuses() {
        assert_eq!(VENDOR_STATUSES.len(), 4);
    }

    #[test]
    fn test_vendor_current_version() {
        assert_eq!(vendor_current_version(), 1);
    }
}
