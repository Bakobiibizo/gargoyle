// Tauri commands for dedup suggestions

use tauri::State;

use crate::error::{GargoyleError, Result};
use crate::models::dedup::DedupSuggestion;
use crate::services::dedup::DedupPipeline;
use crate::AppState;

#[tauri::command]
pub fn check_duplicates(
    state: State<'_, AppState>,
    entity_id: String,
) -> Result<Vec<DedupSuggestion>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    DedupPipeline::check_for_duplicates(conn, &entity_id)
}

#[tauri::command]
pub fn list_dedup_suggestions(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<DedupSuggestion>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    DedupPipeline::get_suggestions(conn, status.as_deref())
}

#[tauri::command]
pub fn resolve_dedup_suggestion(
    state: State<'_, AppState>,
    suggestion_id: String,
    new_status: String,
) -> Result<()> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    DedupPipeline::resolve_suggestion(conn, &suggestion_id, &new_status)
}
