use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::config::GargoyleConfig;

#[derive(Debug, Clone)]
pub struct ErasmusEmbeddings {
    base_url: String,
    client: Client,
    model: String,
}

#[derive(Debug, Serialize)]
struct EmbedRequest {
    input: serde_json::Value,
    normalize: bool,
    model: String,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingResult {
    pub embedding: Vec<f32>,
    pub dimensions: usize,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct BatchEmbeddingResult {
    pub embeddings: Vec<Vec<f32>>,
    pub dimensions: usize,
    pub model: String,
    pub count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Embedder service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Empty input")]
    EmptyInput,
}

impl ErasmusEmbeddings {
    /// Create a new embeddings client using config defaults.
    pub fn from_config() -> Self {
        let config = &GargoyleConfig::global().indexer;
        Self {
            base_url: config.embedder_url.clone(),
            client: Client::new(),
            model: config.embedding_model.clone(),
        }
    }

    /// Create with explicit URL and model (overrides config).
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        let config = &GargoyleConfig::global().indexer;
        Self {
            base_url: base_url.unwrap_or_else(|| config.embedder_url.clone()),
            client: Client::new(),
            model: model.unwrap_or_else(|| config.embedding_model.clone()),
        }
    }

    /// Create with explicit URL, model, and timeout.
    pub fn with_timeout(
        base_url: Option<String>,
        model: Option<String>,
        timeout_secs: u64,
    ) -> Self {
        let config = &GargoyleConfig::global().indexer;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            base_url: base_url.unwrap_or_else(|| config.embedder_url.clone()),
            client,
            model: model.unwrap_or_else(|| config.embedding_model.clone()),
        }
    }

    #[instrument(skip(self, text), fields(text_len = text.len(), model = %self.model))]
    pub async fn embed(&self, text: &str) -> Result<EmbeddingResult, EmbeddingError> {
        if text.trim().is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }
        debug!("Generating embedding");

        let request = EmbedRequest {
            input: serde_json::Value::String(text.to_string()),
            normalize: true,
            model: self.model.clone(),
        };

        let response = self
            .client
            .post(&format!("{}/embed", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::ServiceUnavailable(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let result: EmbeddingResult = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;

        Ok(result)
    }

    pub async fn embed_batch(
        &self,
        texts: &[String],
    ) -> Result<BatchEmbeddingResult, EmbeddingError> {
        if texts.is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }

        let non_empty: Vec<&String> = texts.iter().filter(|t| !t.trim().is_empty()).collect();
        if non_empty.is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }

        let request = EmbedRequest {
            input: serde_json::json!(non_empty),
            normalize: true,
            model: self.model.clone(),
        };

        let response = self
            .client
            .post(&format!("{}/embed", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::ServiceUnavailable(format!(
                "Status {}: {}",
                status, body
            )));
        }

        let result: BatchEmbeddingResult = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;

        Ok(result)
    }

    pub fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
        embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    pub fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
        blob.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_blob_roundtrip() {
        let embedding = vec![0.1, 0.2, 0.3, -0.4, 0.5];
        let blob = ErasmusEmbeddings::embedding_to_blob(&embedding);
        let recovered = ErasmusEmbeddings::blob_to_embedding(&blob);

        assert_eq!(embedding.len(), recovered.len());
        for (a, b) in embedding.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((ErasmusEmbeddings::cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!(ErasmusEmbeddings::cosine_similarity(&a, &c).abs() < 1e-6);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((ErasmusEmbeddings::cosine_similarity(&a, &d) + 1.0).abs() < 1e-6);
    }
}
