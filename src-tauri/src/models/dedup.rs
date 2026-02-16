use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupSuggestion {
    pub suggestion_id: String,
    pub new_entity_id: String,
    pub existing_entity_id: String,
    pub detection_method: DetectionMethod,
    pub confidence: f64,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DetectionMethod {
    ExactTitle,
    FuzzyTitle,
    EmbeddingProximity,
}

impl std::fmt::Display for DetectionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectionMethod::ExactTitle => write!(f, "exact_title"),
            DetectionMethod::FuzzyTitle => write!(f, "fuzzy_title"),
            DetectionMethod::EmbeddingProximity => write!(f, "embedding_proximity"),
        }
    }
}
