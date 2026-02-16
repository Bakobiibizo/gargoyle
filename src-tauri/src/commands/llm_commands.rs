use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::services::llm::{LlmClient, LlmConfig, ChatMessage};

#[derive(Debug, Deserialize)]
pub struct LlmChatInput {
    pub messages: Vec<ChatMessageInput>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageInput {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct LlmChatOutput {
    pub reply: String,
    pub model: String,
    pub finish_reason: Option<String>,
    pub usage: Option<LlmUsageOutput>,
}

#[derive(Debug, Serialize)]
pub struct LlmUsageOutput {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct LlmStatusOutput {
    pub connected: bool,
    pub model: String,
    pub base_url: String,
    pub error: Option<String>,
}

/// Send a chat completion request to the configured LLM.
#[tauri::command]
pub fn llm_chat(input: LlmChatInput) -> Result<LlmChatOutput> {
    let config = LlmConfig::from_env()?;
    let client = LlmClient::new(config);

    let messages: Vec<ChatMessage> = input.messages
        .into_iter()
        .map(|m| ChatMessage { role: m.role, content: m.content })
        .collect();

    let response = client.chat(messages, input.temperature, input.max_tokens)?;

    let choice = response.choices.first()
        .ok_or_else(|| crate::error::GargoyleError::Schema("LLM returned no choices".into()))?;

    Ok(LlmChatOutput {
        reply: choice.message.content.clone(),
        model: client.model().to_string(),
        finish_reason: choice.finish_reason.clone(),
        usage: response.usage.map(|u| LlmUsageOutput {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
    })
}

/// Quick single-prompt completion.
#[tauri::command]
pub fn llm_complete(prompt: String) -> Result<String> {
    let config = LlmConfig::from_env()?;
    let client = LlmClient::new(config);
    client.complete(&prompt)
}

/// Check LLM connection status without sending a real prompt.
#[tauri::command]
pub fn llm_status() -> Result<LlmStatusOutput> {
    let config = match LlmConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            return Ok(LlmStatusOutput {
                connected: false,
                model: String::new(),
                base_url: String::new(),
                error: Some(e.to_string()),
            });
        }
    };

    let client = LlmClient::new(config);

    // Send a minimal ping to verify connectivity
    match client.complete("ping") {
        Ok(_) => Ok(LlmStatusOutput {
            connected: true,
            model: client.model().to_string(),
            base_url: client.base_url().to_string(),
            error: None,
        }),
        Err(e) => Ok(LlmStatusOutput {
            connected: false,
            model: client.model().to_string(),
            base_url: client.base_url().to_string(),
            error: Some(e.to_string()),
        }),
    }
}
