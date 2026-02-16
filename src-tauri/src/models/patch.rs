use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op_type")]
pub enum PatchOp {
    #[serde(rename = "create_entity")]
    CreateEntity(CreateEntityPayload),
    #[serde(rename = "update_entity")]
    UpdateEntity(UpdateEntityPayload),
    #[serde(rename = "create_relation")]
    CreateRelation(CreateRelationPayload),
    #[serde(rename = "create_claim")]
    CreateClaim(CreateClaimPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntityPayload {
    pub entity_type: String,
    pub title: String,
    pub source: String,
    pub canonical_fields: serde_json::Value,
    pub body_md: Option<String>,
    pub status: Option<String>,
    pub category: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEntityPayload {
    pub entity_id: String,
    pub expected_updated_at: String,
    pub title: Option<String>,
    pub body_md: Option<String>,
    pub status: Option<String>,
    pub canonical_fields: Option<serde_json::Value>,
    pub category: Option<String>,
    pub priority: Option<i32>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRelationPayload {
    pub from_id: String,
    pub to_id: String,
    pub relation_type: String,
    pub weight: Option<f64>,
    pub confidence: Option<f64>,
    pub provenance_run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClaimPayload {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub evidence_entity_id: String,
    pub provenance_run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchSet {
    pub ops: Vec<PatchOp>,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResult {
    pub applied: Vec<AppliedOp>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedOp {
    pub op_index: usize,
    pub entity_id: Option<String>,
    pub relation_id: Option<String>,
    pub claim_id: Option<String>,
}
