use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub description: Option<String>,
    /// The schema version in which this field was first introduced.
    /// None means the field has been present since v1.
    #[serde(default)]
    pub added_in_version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    String,
    Number,
    Boolean,
    /// ISO 8601 date string (YYYY-MM-DD). Validated leniently as a string for now.
    Date,
    /// ISO 8601 datetime string. Validated leniently as a string for now.
    DateTime,
    /// Arbitrary valid JSON value (object, array, string, number, bool, null).
    Json,
    /// A JSON array where each element should match the specified items type.
    /// The String parameter is the items_type (e.g., "string", "number").
    Array(String),
    Enum(Vec<String>),
    EntityRef(String),
    /// A JSON array of entity reference strings (UUIDs) referencing the specified entity type.
    EntityRefArray(String),
}
