// Tauri commands for claims

use tauri::State;

use crate::error::{GargoyleError, Result};
use crate::models::claim::Claim;
use crate::services::claim_service::ClaimService;
use crate::AppState;

#[tauri::command]
pub fn list_claims(
    state: State<'_, AppState>,
    evidence_entity_id: Option<String>,
) -> Result<Vec<Claim>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    ClaimService::list_claims(conn, evidence_entity_id.as_deref())
}

#[tauri::command]
pub fn get_claim(
    state: State<'_, AppState>,
    claim_id: String,
) -> Result<Claim> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    ClaimService::get_claim(conn, &claim_id)
}

#[tauri::command]
pub fn promote_claim(
    state: State<'_, AppState>,
    claim_id: String,
    entity_type: String,
    source: String,
) -> Result<String> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    ClaimService::promote_claim(conn, &claim_id, &entity_type, &source)
}
