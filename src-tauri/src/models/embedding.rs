use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub embedding_id: String,
    pub entity_id: String,
    pub model: String,
    pub vector: Vec<u8>,
    pub dimensions: i32,
    pub created_at: String,
}
