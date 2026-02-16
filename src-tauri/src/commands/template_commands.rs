// Tauri commands for template execution

use tauri::State;

use crate::error::{GargoyleError, Result};
use crate::models::run::Run;
use crate::services::store::StoreService;
use crate::services::template_runner::{
    self, PrerequisiteResult, TemplateDefinition, TemplateInput, TemplateOutput,
};
use crate::AppState;

#[tauri::command]
pub fn run_template(
    state: State<'_, AppState>,
    input: TemplateInput,
) -> Result<TemplateOutput> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    template_runner::run_template_full(conn, &input)
}

#[tauri::command]
pub fn check_prerequisites(
    state: State<'_, AppState>,
    template_key: String,
) -> Result<Vec<PrerequisiteResult>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    template_runner::check_prerequisites(conn, &template_key)
}

#[tauri::command]
pub fn list_templates(
    _state: State<'_, AppState>,
) -> Result<Vec<TemplateDefinition>> {
    Ok(template_runner::list_template_definitions())
}

#[tauri::command]
pub fn list_runs(
    state: State<'_, AppState>,
    template_key: Option<String>,
) -> Result<Vec<Run>> {
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    StoreService::list_runs(conn, template_key.as_deref())
}
