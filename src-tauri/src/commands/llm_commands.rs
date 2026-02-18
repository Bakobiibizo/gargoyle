use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{GargoyleError, Result};
use crate::services::llm::{LlmClient, LlmConfig, ChatMessage};
use crate::services::tool_executor;
use crate::AppState;

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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct ToolCallLog {
    pub tool_name: String,
    pub arguments: String,
    pub result: String,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct LlmToolChatOutput {
    pub reply: String,
    pub model: String,
    pub finish_reason: Option<String>,
    pub usage: Option<LlmUsageOutput>,
    pub tool_calls_made: Vec<ToolCallLog>,
}

/// Send a chat completion request to the configured LLM.
#[tauri::command]
pub fn llm_chat(input: LlmChatInput) -> Result<LlmChatOutput> {
    let config = LlmConfig::from_env()?;
    let client = LlmClient::new(config);

    let messages: Vec<ChatMessage> = input.messages
        .into_iter()
        .map(|m| ChatMessage {
            role: m.role,
            content: Some(m.content),
            tool_calls: None,
            tool_call_id: None,
        })
        .collect();

    let response = client.chat(messages, input.temperature, input.max_tokens)?;

    let choice = response.choices.first()
        .ok_or_else(|| GargoyleError::Schema("LLM returned no choices".into()))?;

    Ok(LlmChatOutput {
        reply: choice.message.content.clone().unwrap_or_default(),
        model: client.model().to_string(),
        finish_reason: choice.finish_reason.clone(),
        usage: response.usage.map(|u| LlmUsageOutput {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
    })
}

/// Chat with tool-calling support. Runs the full tool-calling loop (max 10 iterations).
#[tauri::command]
pub fn llm_chat_with_tools(
    state: State<'_, AppState>,
    input: LlmChatInput,
) -> Result<LlmToolChatOutput> {
    let config = LlmConfig::from_env()?;
    let client = LlmClient::new(config);
    let tools = tool_executor::get_tool_definitions();

    let mut messages: Vec<ChatMessage> = input.messages
        .into_iter()
        .map(|m| ChatMessage {
            role: m.role,
            content: Some(m.content),
            tool_calls: None,
            tool_call_id: None,
        })
        .collect();

    let mut all_tool_logs: Vec<ToolCallLog> = Vec::new();
    let mut last_usage: Option<LlmUsageOutput> = None;
    let mut last_finish_reason: Option<String> = None;

    let max_iterations = crate::config::GargoyleConfig::global().llm_tuning.max_tool_iterations;

    for iteration in 0..max_iterations {
        let response = match client.chat_with_tools(
            messages.clone(),
            input.temperature,
            input.max_tokens,
            Some(tools.clone()),
        ) {
            Ok(r) => r,
            Err(e) => {
                // If this is the first iteration, propagate the error.
                // Otherwise, break and return what we have so far.
                if iteration == 0 && all_tool_logs.is_empty() {
                    return Err(e);
                }
                return Ok(LlmToolChatOutput {
                    reply: format!("Tool loop interrupted: {}", e),
                    model: client.model().to_string(),
                    finish_reason: last_finish_reason,
                    usage: last_usage,
                    tool_calls_made: all_tool_logs,
                });
            }
        };

        let choice = match response.choices.first() {
            Some(c) => c,
            None => {
                return Ok(LlmToolChatOutput {
                    reply: "LLM returned an empty response.".to_string(),
                    model: client.model().to_string(),
                    finish_reason: None,
                    usage: last_usage,
                    tool_calls_made: all_tool_logs,
                });
            }
        };

        last_finish_reason = choice.finish_reason.clone();
        last_usage = response.usage.map(|u| LlmUsageOutput {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        // Check if the LLM wants to call tools
        if let Some(ref tool_calls) = choice.message.tool_calls {
            if !tool_calls.is_empty() {
                // Append the assistant message (with tool_calls) to the conversation
                messages.push(choice.message.clone());

                // Execute each tool call and append results
                let guard = state.db.lock().unwrap();
                let conn = guard.as_ref().ok_or_else(|| {
                    GargoyleError::Schema("Database not initialized".to_string())
                })?;

                for tc in tool_calls {
                    let tool_result = match tool_executor::execute_tool(
                        conn,
                        &tc.function.name,
                        &tc.function.arguments,
                    ) {
                        Ok(result) => {
                            all_tool_logs.push(ToolCallLog {
                                tool_name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                                result: result.clone(),
                                success: true,
                            });
                            result
                        }
                        Err(e) => {
                            let err_msg = format!("Error: {}", e);
                            all_tool_logs.push(ToolCallLog {
                                tool_name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                                result: err_msg.clone(),
                                success: false,
                            });
                            err_msg
                        }
                    };

                    // Append tool result message
                    messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: Some(tool_result),
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                    });
                }

                // Drop the guard before the next iteration's HTTP call
                drop(guard);
                continue;
            }
        }

        // No tool calls — we have a final text response
        return Ok(LlmToolChatOutput {
            reply: choice.message.content.clone().unwrap_or_default(),
            model: client.model().to_string(),
            finish_reason: last_finish_reason,
            usage: last_usage,
            tool_calls_made: all_tool_logs,
        });
    }

    // Exhausted iterations — return whatever we have
    let last_content = messages.last()
        .and_then(|m| m.content.clone())
        .unwrap_or_else(|| "The assistant used all available tool-calling iterations.".to_string());

    Ok(LlmToolChatOutput {
        reply: last_content,
        model: client.model().to_string(),
        finish_reason: last_finish_reason,
        usage: last_usage,
        tool_calls_made: all_tool_logs,
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
