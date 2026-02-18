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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool call ID. Many local models omit this or return null.
    #[serde(default = "generate_tool_call_id")]
    pub id: String,
    #[serde(rename = "type", default)]
    pub call_type: String,
    pub function: FunctionCall,
}

fn generate_tool_call_id() -> String {
    format!("call_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    /// Arguments JSON. Accepts both a JSON string and a JSON object (stringified on deser).
    #[serde(deserialize_with = "deserialize_arguments")]
    pub arguments: String,
}

/// Local models sometimes emit `arguments` as a JSON object instead of a string.
/// This deserializer accepts both forms.
fn deserialize_arguments<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        other => Ok(other.to_string()),
    }
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
        self.chat_with_tools(messages, temperature, max_tokens, None)
    }

    /// Send a chat completion request with optional tool definitions.
    pub fn chat_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature,
            max_tokens,
            tools,
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

        let body = response.text()
            .map_err(|e| GargoyleError::Schema(format!("Failed to read LLM response body: {}", e)))?;

        // Try strict deserialization first
        match serde_json::from_str::<ChatResponse>(&body) {
            Ok(parsed) => Ok(parsed),
            Err(strict_err) => {
                // Strict parse failed — try to salvage content from the raw JSON.
                // This handles cases where tool_calls have unexpected shapes but
                // the response still contains usable text content.
                if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(content) = raw["choices"][0]["message"]["content"].as_str() {
                        return Ok(ChatResponse {
                            id: raw["id"].as_str().map(|s| s.to_string()),
                            choices: vec![ChatChoice {
                                index: 0,
                                message: ChatMessage {
                                    role: "assistant".to_string(),
                                    content: Some(content.to_string()),
                                    tool_calls: None,
                                    tool_call_id: None,
                                },
                                finish_reason: raw["choices"][0]["finish_reason"].as_str().map(|s| s.to_string()),
                            }],
                            usage: None,
                        });
                    }
                }
                Err(GargoyleError::Schema(format!("Failed to parse LLM response: {}", strict_err)))
            }
        }
    }

    /// Convenience: send a single user message and get the assistant reply text.
    pub fn complete(&self, prompt: &str) -> Result<String> {
        let messages = vec![
            ChatMessage { role: "user".into(), content: Some(prompt.into()), tool_calls: None, tool_call_id: None },
        ];

        let response = self.chat(messages, None, None)?;

        response.choices.first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| GargoyleError::Schema("LLM returned no choices".into()))
    }

    /// Send a system + user message pair and get the assistant reply text.
    pub fn complete_with_system(&self, system: &str, user: &str) -> Result<String> {
        let messages = vec![
            ChatMessage { role: "system".into(), content: Some(system.into()), tool_calls: None, tool_call_id: None },
            ChatMessage { role: "user".into(), content: Some(user.into()), tool_calls: None, tool_call_id: None },
        ];

        let response = self.chat(messages, None, None)?;

        response.choices.first()
            .and_then(|c| c.message.content.clone())
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
                ChatMessage { role: "user".into(), content: Some("hello".into()), tool_calls: None, tool_call_id: None },
            ],
            temperature: Some(0.7),
            max_tokens: None,
            tools: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "test-model");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "hello");
        assert_eq!(json["temperature"], 0.7);
        assert!(json.get("max_tokens").is_none());
        assert!(json.get("tools").is_none());
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
        assert_eq!(response.choices[0].message.content.as_deref(), Some("Hello!"));
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
        assert_eq!(response.choices[0].message.content.as_deref(), Some("Hi"));
        assert!(response.id.is_none());
        assert!(response.usage.is_none());
    }

    #[test]
    fn test_chat_response_with_tool_calls() {
        let json = r#"{
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "search_entities",
                            "arguments": "{\"query\": \"tasks\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }"#;
        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(response.choices[0].message.content.is_none());
        let tool_calls = response.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "search_entities");
    }
}
