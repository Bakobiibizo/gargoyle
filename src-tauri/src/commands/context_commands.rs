// Tauri commands for operational context

use tauri::State;

use crate::error::{GargoyleError, Result};
use crate::models::operational_context::OperationalContext;
use crate::services::context_manager::ContextManager;
use crate::AppState;

#[tauri::command]
pub fn get_context(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<OperationalContext>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    ContextManager::get(conn, &key)
}

#[tauri::command]
pub fn set_context(
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
    run_id: Option<String>,
) -> Result<()> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    ContextManager::set(conn, &key, &value, run_id.as_deref())
}

#[tauri::command]
pub fn delete_context(
    state: State<'_, AppState>,
    key: String,
) -> Result<()> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    ContextManager::delete(conn, &key)
}

#[tauri::command]
pub fn list_contexts(
    state: State<'_, AppState>,
) -> Result<Vec<OperationalContext>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    ContextManager::list(conn)
}
