use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalContext {
    pub context_id: String,
    pub context_key: String,
    pub context_value: serde_json::Value,
    pub updated_at: String,
    pub updated_by_run_id: Option<String>,
}
