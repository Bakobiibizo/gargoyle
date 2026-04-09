// Tauri commands for operational context

use tauri::State;
use tracing::{debug, error, info, instrument};

use crate::error::{GargoyleError, Result};
use crate::models::operational_context::OperationalContext;
use crate::services::context_manager::ContextManager;
use crate::AppState;

#[tauri::command]
#[instrument(skip(state), fields(key = %key))]
pub fn get_context(state: State<'_, AppState>, key: String) -> Result<Option<OperationalContext>> {
    debug!("Getting context");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ContextManager::get(conn, &key);
    match &result {
        Ok(Some(_)) => debug!("Context found"),
        Ok(None) => debug!("Context not found"),
        Err(e) => error!(error = %e, "Failed to get context"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state, value), fields(key = %key, has_run_id = run_id.is_some()))]
pub fn set_context(
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
    run_id: Option<String>,
) -> Result<()> {
    info!("Setting context");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ContextManager::set(conn, &key, &value, run_id.as_deref());
    match &result {
        Ok(_) => info!("Context set"),
        Err(e) => error!(error = %e, "Failed to set context"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(key = %key))]
pub fn delete_context(state: State<'_, AppState>, key: String) -> Result<()> {
    info!("Deleting context");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ContextManager::delete(conn, &key);
    match &result {
        Ok(_) => info!("Context deleted"),
        Err(e) => error!(error = %e, "Failed to delete context"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state))]
pub fn list_contexts(state: State<'_, AppState>) -> Result<Vec<OperationalContext>> {
    debug!("Listing contexts");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ContextManager::list(conn);
    match &result {
        Ok(contexts) => debug!(count = contexts.len(), "Contexts listed"),
        Err(e) => error!(error = %e, "Failed to list contexts"),
    }
    result
}
