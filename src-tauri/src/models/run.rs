use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub run_id: String,
    pub template_key: String,
    pub template_version: String,
    pub template_category: String,
    pub inputs_snapshot: serde_json::Value,
    pub outputs_snapshot: serde_json::Value,
    pub patch_set: serde_json::Value,
    pub status: RunStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Pending,
    Applied,
    Rejected,
    Partial,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunStatus::Pending => write!(f, "pending"),
            RunStatus::Applied => write!(f, "applied"),
            RunStatus::Rejected => write!(f, "rejected"),
            RunStatus::Partial => write!(f, "partial"),
        }
    }
}
