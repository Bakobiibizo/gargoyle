// Tauri commands for dedup suggestions

use tauri::State;
use tracing::{debug, error, info, instrument};

use crate::error::{GargoyleError, Result};
use crate::models::dedup::DedupSuggestion;
use crate::services::dedup::DedupPipeline;
use crate::AppState;

#[tauri::command]
#[instrument(skip(state), fields(entity_id = %entity_id))]
pub fn check_duplicates(
    state: State<'_, AppState>,
    entity_id: String,
) -> Result<Vec<DedupSuggestion>> {
    debug!("Checking for duplicates");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = DedupPipeline::check_for_duplicates(conn, &entity_id);
    match &result {
        Ok(suggestions) => debug!(count = suggestions.len(), "Duplicates checked"),
        Err(e) => error!(error = %e, "Failed to check duplicates"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(status = ?status))]
pub fn list_dedup_suggestions(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<DedupSuggestion>> {
    debug!("Listing dedup suggestions");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = DedupPipeline::get_suggestions(conn, status.as_deref());
    match &result {
        Ok(suggestions) => debug!(count = suggestions.len(), "Dedup suggestions listed"),
        Err(e) => error!(error = %e, "Failed to list dedup suggestions"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(suggestion_id = %suggestion_id, new_status = %new_status))]
pub fn resolve_dedup_suggestion(
    state: State<'_, AppState>,
    suggestion_id: String,
    new_status: String,
) -> Result<()> {
    info!("Resolving dedup suggestion");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = DedupPipeline::resolve_suggestion(conn, &suggestion_id, &new_status);
    match &result {
        Ok(_) => info!("Dedup suggestion resolved"),
        Err(e) => error!(error = %e, "Failed to resolve dedup suggestion"),
    }
    result
}
