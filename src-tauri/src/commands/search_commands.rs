// Tauri commands for FTS + semantic search

use tauri::State;

use crate::error::{GargoyleError, Result};
use crate::services::indexer::{IndexerService, SearchResult};
use crate::AppState;

#[tauri::command]
pub fn search_fts(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    IndexerService::search_fts(conn, &query, limit)
}

#[tauri::command]
pub fn search_similar(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
    threshold: Option<f64>,
) -> Result<Vec<SearchResult>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    IndexerService::search_similar(conn, &query, limit, threshold)
}

#[tauri::command]
pub fn generate_embedding(
    state: State<'_, AppState>,
    entity_id: String,
) -> Result<()> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    IndexerService::generate_embedding(conn, &entity_id)
}

#[tauri::command]
pub fn reindex_entity(
    state: State<'_, AppState>,
    entity_id: String,
) -> Result<()> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    IndexerService::reindex_entity(conn, &entity_id)
}
