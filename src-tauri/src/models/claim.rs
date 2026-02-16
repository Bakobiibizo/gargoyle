use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub claim_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub evidence_entity_id: String,
    pub provenance_run_id: Option<String>,
    pub promoted_to_entity_id: Option<String>,
    pub created_at: String,
}
