use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub relation_type: String,
    pub weight: Option<f64>,
    pub confidence: Option<f64>,
    pub provenance_run_id: Option<String>,
    pub created_at: String,
}
