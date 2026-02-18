use tauri::State;

use crate::AppState;
use crate::error::{GargoyleError, Result};
use crate::models::chat::{ChatMessageRow, ChatSession};
use crate::services::chat_service::ChatService;

#[tauri::command]
pub fn create_chat_session(
    state: State<'_, AppState>,
    title: String,
    system_prompt: Option<String>,
) -> Result<ChatSession> {
    let guard = state.db.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| GargoyleError::Schema("Database not initialized".to_string()))?;
    ChatService::create_session(conn, &title, system_prompt.as_deref())
}

#[tauri::command]
pub fn list_chat_sessions(state: State<'_, AppState>) -> Result<Vec<ChatSession>> {
    let guard = state.db.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| GargoyleError::Schema("Database not initialized".to_string()))?;
    ChatService::list_sessions(conn)
}

#[tauri::command]
pub fn get_chat_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<ChatMessageRow>> {
    let guard = state.db.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| GargoyleError::Schema("Database not initialized".to_string()))?;
    ChatService::list_messages(conn, &session_id)
}

#[tauri::command]
pub fn add_chat_message(
    state: State<'_, AppState>,
    session_id: String,
    role: String,
    content: String,
    model: Option<String>,
    tokens: Option<i64>,
) -> Result<ChatMessageRow> {
    let guard = state.db.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| GargoyleError::Schema("Database not initialized".to_string()))?;
    ChatService::add_message(conn, &session_id, &role, &content, model.as_deref(), tokens)
}

#[tauri::command]
pub fn update_chat_session_title(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<()> {
    let guard = state.db.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| GargoyleError::Schema("Database not initialized".to_string()))?;
    ChatService::update_session_title(conn, &session_id, &title)
}

#[tauri::command]
pub fn delete_chat_session(state: State<'_, AppState>, session_id: String) -> Result<()> {
    let guard = state.db.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| GargoyleError::Schema("Database not initialized".to_string()))?;
    ChatService::delete_session(conn, &session_id)
}
