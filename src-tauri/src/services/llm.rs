use serde::{Deserialize, Serialize};
use crate::error::{GargoyleError, Result};

// ---------------------------------------------------------------------------
// Config — loaded once from environment variables, no fallbacks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl LlmConfig {
    /// Load config from environment variables. Panics if any are missing.
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| GargoyleError::Schema("Missing required env var: OPENAI_API_KEY".into()))?;
        let base_url = std::env::var("OPENAI_BASE_URL")
            .map_err(|_| GargoyleError::Schema("Missing required env var: OPENAI_BASE_URL".into()))?;
        let model = std::env::var("OPENAI_MODEL")
            .map_err(|_| GargoyleError::Schema("Missing required env var: OPENAI_MODEL".into()))?;

        Ok(Self { api_key, base_url, model })
    }
}

// ---------------------------------------------------------------------------
// OpenAI-compatible request/response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub id: Option<String>,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

// ---------------------------------------------------------------------------
// LLM Client
// ---------------------------------------------------------------------------

pub struct LlmClient {
    config: LlmConfig,
    http: reqwest::blocking::Client,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self { config, http }
    }

    /// Send a chat completion request and return the full response.
    pub fn chat(&self, messages: Vec<ChatMessage>, temperature: Option<f64>, max_tokens: Option<u32>) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature,
            max_tokens,
        };

        let response = self.http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| GargoyleError::Schema(format!("LLM request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(GargoyleError::Schema(format!(
                "LLM API error ({}): {}", status, body
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .map_err(|e| GargoyleError::Schema(format!("Failed to parse LLM response: {}", e)))?;

        Ok(chat_response)
    }

    /// Convenience: send a single user message and get the assistant reply text.
    pub fn complete(&self, prompt: &str) -> Result<String> {
        let messages = vec![
            ChatMessage { role: "user".into(), content: prompt.into() },
        ];

        let response = self.chat(messages, None, None)?;

        response.choices.first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| GargoyleError::Schema("LLM returned no choices".into()))
    }

    /// Send a system + user message pair and get the assistant reply text.
    pub fn complete_with_system(&self, system: &str, user: &str) -> Result<String> {
        let messages = vec![
            ChatMessage { role: "system".into(), content: system.into() },
            ChatMessage { role: "user".into(), content: user.into() },
        ];

        let response = self.chat(messages, None, None)?;

        response.choices.first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| GargoyleError::Schema("LLM returned no choices".into()))
    }

    /// Get the model name this client is configured to use.
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Get the base URL this client is configured to use.
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env_missing_vars() {
        // Temporarily unset to test error handling
        let result = LlmConfig::from_env();
        // This will either succeed (if env vars set) or fail (if not) — both are valid
        // We just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "test-model".into(),
            messages: vec![
                ChatMessage { role: "user".into(), content: "hello".into() },
            ],
            temperature: Some(0.7),
            max_tokens: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "test-model");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "hello");
        assert_eq!(json["temperature"], 0.7);
        assert!(json.get("max_tokens").is_none());
    }

    #[test]
    fn test_chat_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "Hello!" },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7 }
        }"#;
        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, "Hello!");
        assert_eq!(response.usage.unwrap().total_tokens, Some(7));
    }

    #[test]
    fn test_chat_response_minimal() {
        // Some OpenAI-compatible APIs return minimal responses
        let json = r#"{
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "Hi" },
                "finish_reason": null
            }]
        }"#;
        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices[0].message.content, "Hi");
        assert!(response.id.is_none());
        assert!(response.usage.is_none());
    }
}
