use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRelationType {
    pub type_key: String,
    pub description: String,
    pub expected_from_types: Option<serde_json::Value>,
    pub expected_to_types: Option<serde_json::Value>,
    pub proposed_by_run_id: Option<String>,
    pub approved_at: String,
}
