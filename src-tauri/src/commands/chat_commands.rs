use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tauri::State;
use tracing::{debug, error, info, instrument, warn};

use crate::error::{GargoyleError, Result};
use crate::models::chat::{ChatMessageRow, ChatSession};
use crate::models::memory::MessageRole;
use crate::services::chat_service::ChatService;
use crate::services::memory_service::MemoryService;
use crate::AppState;

#[tauri::command]
#[instrument(skip(state), fields(title = %title, has_system_prompt = system_prompt.is_some()))]
pub fn create_chat_session(
    state: State<'_, AppState>,
    title: String,
    system_prompt: Option<String>,
) -> Result<ChatSession> {
    info!("Creating chat session");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ChatService::create_session(conn, &title, system_prompt.as_deref());
    match &result {
        Ok(session) => info!(session_id = %session.id, "Chat session created"),
        Err(e) => error!(error = %e, "Failed to create chat session"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state))]
pub fn list_chat_sessions(state: State<'_, AppState>) -> Result<Vec<ChatSession>> {
    debug!("Listing chat sessions");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ChatService::list_sessions(conn);
    match &result {
        Ok(sessions) => debug!(count = sessions.len(), "Chat sessions listed"),
        Err(e) => error!(error = %e, "Failed to list chat sessions"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(session_id = %session_id))]
pub fn get_chat_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<ChatMessageRow>> {
    debug!("Fetching chat messages");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ChatService::list_messages(conn, &session_id);
    match &result {
        Ok(messages) => debug!(count = messages.len(), "Chat messages fetched"),
        Err(e) => error!(error = %e, "Failed to fetch chat messages"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state, content), fields(session_id = %session_id, role = %role, content_len = content.len()))]
pub fn add_chat_message(
    state: State<'_, AppState>,
    session_id: String,
    role: String,
    content: String,
    model: Option<String>,
    tokens: Option<i64>,
) -> Result<ChatMessageRow> {
    debug!("Adding chat message");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result =
        ChatService::add_message(conn, &session_id, &role, &content, model.as_deref(), tokens)?;
    info!(message_id = %result.id, "Chat message added");

    // Record to memory system (silently - errors don't affect chat)
    let db_path = conn.path().map(|p| p.to_string()).unwrap_or_default();
    drop(guard);

    if !db_path.is_empty() {
        if let Ok(mem_conn) = Connection::open(&db_path) {
            let arc_conn = Arc::new(Mutex::new(mem_conn));
            let memory_service = MemoryService::new(arc_conn);
            if memory_service.is_enabled() {
                // Ensure conversation exists
                if memory_service
                    .get_conversation(&session_id)
                    .ok()
                    .flatten()
                    .is_none()
                {
                    let _ = memory_service.create_conversation_with_id(Some(session_id.clone()));
                }
                // Record segment
                let mem_role = match role.as_str() {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    "system" => MessageRole::System,
                    _ => MessageRole::User,
                };
                let _ = memory_service.add_segment(&session_id, mem_role, content.clone(), None);
            }
        }
    }

    Ok(result)
}

#[tauri::command]
#[instrument(skip(state), fields(session_id = %session_id, title = %title))]
pub fn update_chat_session_title(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<()> {
    info!("Updating chat session title");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ChatService::update_session_title(conn, &session_id, &title);
    match &result {
        Ok(_) => info!("Chat session title updated"),
        Err(e) => error!(error = %e, "Failed to update chat session title"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(session_id = %session_id))]
pub fn delete_chat_session(state: State<'_, AppState>, session_id: String) -> Result<()> {
    info!("Deleting chat session");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ChatService::delete_session(conn, &session_id);
    match &result {
        Ok(_) => info!("Chat session deleted"),
        Err(e) => error!(error = %e, "Failed to delete chat session"),
    }
    result
}
