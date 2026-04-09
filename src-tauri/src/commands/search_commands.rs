// Tauri commands for FTS + semantic search

use tauri::State;
use tracing::{debug, error, info, instrument};

use crate::error::{GargoyleError, Result};
use crate::services::indexer::{IndexerService, SearchResult};
use crate::AppState;

#[tauri::command]
#[instrument(skip(state), fields(query_len = query.len(), limit = limit))]
pub fn search_fts(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    debug!("Full-text search");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = IndexerService::search_fts(conn, &query, limit);
    match &result {
        Ok(results) => debug!(result_count = results.len(), "FTS search complete"),
        Err(e) => error!(error = %e, "FTS search failed"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(query_len = query.len(), limit = limit, threshold = ?threshold))]
pub fn search_similar(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
    threshold: Option<f64>,
) -> Result<Vec<SearchResult>> {
    debug!("Semantic similarity search");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = IndexerService::search_similar(conn, &query, limit, threshold);
    match &result {
        Ok(results) => debug!(result_count = results.len(), "Semantic search complete"),
        Err(e) => error!(error = %e, "Semantic search failed"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(entity_id = %entity_id))]
pub fn generate_embedding(state: State<'_, AppState>, entity_id: String) -> Result<()> {
    info!("Generating embedding");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = IndexerService::generate_embedding(conn, &entity_id);
    match &result {
        Ok(_) => info!("Embedding generated"),
        Err(e) => error!(error = %e, "Failed to generate embedding"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(entity_id = %entity_id))]
pub fn reindex_entity(state: State<'_, AppState>, entity_id: String) -> Result<()> {
    info!("Reindexing entity");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = IndexerService::reindex_entity(conn, &entity_id);
    match &result {
        Ok(_) => info!("Entity reindexed"),
        Err(e) => error!(error = %e, "Failed to reindex entity"),
    }
    result
}
