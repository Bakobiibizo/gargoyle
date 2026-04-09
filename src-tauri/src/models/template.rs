use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub key: String,
    pub version: String,
    pub category: String,
    pub description: Option<String>,
    pub content: String,
    pub response_format: Option<String>,
    pub produces_entities: Vec<String>,
    pub produces_relations: Vec<String>,
    pub generator_type: Option<String>,
    pub generator_config: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateIndex {
    pub key: String,
    pub category: String,
    pub description: Option<String>,
    pub produces_entities: Vec<String>,
    pub usage_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplatePayload {
    pub key: String,
    pub category: String,
    pub description: Option<String>,
    pub content: String,
    pub response_format: Option<String>,
    pub produces_entities: Option<Vec<String>>,
    pub produces_relations: Option<Vec<String>>,
    pub generator_type: Option<String>,
    pub generator_config: Option<serde_json::Value>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTemplatePayload {
    pub key: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub response_format: Option<String>,
    pub produces_entities: Option<Vec<String>>,
    pub produces_relations: Option<Vec<String>>,
    pub generator_type: Option<String>,
    pub generator_config: Option<serde_json::Value>,
}
