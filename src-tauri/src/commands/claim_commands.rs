// Tauri commands for claims

use tauri::State;
use tracing::{debug, error, info, instrument};

use crate::error::{GargoyleError, Result};
use crate::models::claim::Claim;
use crate::services::claim_service::ClaimService;
use crate::AppState;

#[tauri::command]
#[instrument(skip(state), fields(evidence_entity_id = ?evidence_entity_id))]
pub fn list_claims(
    state: State<'_, AppState>,
    evidence_entity_id: Option<String>,
) -> Result<Vec<Claim>> {
    debug!("Listing claims");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ClaimService::list_claims(conn, evidence_entity_id.as_deref());
    match &result {
        Ok(claims) => debug!(count = claims.len(), "Claims listed"),
        Err(e) => error!(error = %e, "Failed to list claims"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(claim_id = %claim_id))]
pub fn get_claim(state: State<'_, AppState>, claim_id: String) -> Result<Claim> {
    debug!("Getting claim");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ClaimService::get_claim(conn, &claim_id);
    match &result {
        Ok(_) => debug!("Claim fetched"),
        Err(e) => error!(error = %e, "Failed to get claim"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(claim_id = %claim_id, entity_type = %entity_type))]
pub fn promote_claim(
    state: State<'_, AppState>,
    claim_id: String,
    entity_type: String,
    source: String,
) -> Result<String> {
    info!("Promoting claim to entity");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = ClaimService::promote_claim(conn, &claim_id, &entity_type, &source);
    match &result {
        Ok(entity_id) => info!(entity_id = %entity_id, "Claim promoted to entity"),
        Err(e) => error!(error = %e, "Failed to promote claim"),
    }
    result
}
