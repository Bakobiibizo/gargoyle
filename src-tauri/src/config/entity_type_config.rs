use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::schema::field_def::{FieldDef, FieldType};

/// TOML-deserializable entity type definition.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityTypeDef {
    pub version: i32,
    #[serde(default)]
    pub statuses: Vec<String>,
    #[serde(default)]
    pub fields: Vec<FieldDefConfig>,
}

/// TOML-deserializable field definition.
#[derive(Debug, Clone, Deserialize)]
pub struct FieldDefConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: FieldTypeConfig,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
}

/// TOML-deserializable field type (mirrors `FieldType` but serde-friendly for TOML).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FieldTypeConfig {
    /// Tagged variant: { EntityRef = "person" }, { Array = "string" }, { Enum = ["a","b"] }, etc.
    Tagged(TaggedFieldType),
    /// Simple string variant: "String", "Number", "Boolean", "Date", "DateTime", "Json"
    Simple(String),
}

#[derive(Debug, Clone, Deserialize)]
pub enum TaggedFieldType {
    EntityRef(String),
    EntityRefArray(String),
    Array(String),
    Enum(Vec<String>),
}

impl FieldDefConfig {
    /// Convert to the internal `FieldDef` representation.
    pub fn to_field_def(&self) -> FieldDef {
        FieldDef {
            name: self.name.clone(),
            field_type: self.field_type.to_field_type(),
            required: self.required,
            description: self.description.clone(),
            added_in_version: None,
        }
    }
}

impl FieldTypeConfig {
    fn to_field_type(&self) -> FieldType {
        match self {
            FieldTypeConfig::Simple(s) => match s.as_str() {
                "String" => FieldType::String,
                "Number" => FieldType::Number,
                "Boolean" => FieldType::Boolean,
                "Date" => FieldType::Date,
                "DateTime" => FieldType::DateTime,
                "Json" => FieldType::Json,
                other => {
                    // Fallback: treat as String with a warning
                    eprintln!("Warning: unknown simple field type '{}', treating as String", other);
                    FieldType::String
                }
            },
            FieldTypeConfig::Tagged(tagged) => match tagged {
                TaggedFieldType::EntityRef(target) => FieldType::EntityRef(target.clone()),
                TaggedFieldType::EntityRefArray(target) => FieldType::EntityRefArray(target.clone()),
                TaggedFieldType::Array(items) => FieldType::Array(items.clone()),
                TaggedFieldType::Enum(values) => FieldType::Enum(values.clone()),
            },
        }
    }
}

impl EntityTypeDef {
    /// Convert all field configs to internal FieldDef representations.
    pub fn field_defs(&self) -> Vec<FieldDef> {
        self.fields.iter().map(|f| f.to_field_def()).collect()
    }
}

/// Load all entity type definitions from TOML files in the given directory.
///
/// Each `.toml` file in the directory defines one entity type.
/// The filename (without extension) becomes the entity type name.
pub fn load_entity_types(dir: &Path) -> Result<HashMap<String, EntityTypeDef>, String> {
    let mut types = HashMap::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read entity types directory {:?}: {}", dir, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        let type_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("Invalid filename: {:?}", path))?
            .to_string();

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;

        let def: EntityTypeDef = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;

        types.insert(type_name, def);
    }

    Ok(types)
}
