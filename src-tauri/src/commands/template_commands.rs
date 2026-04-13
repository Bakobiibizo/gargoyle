// Tauri commands for template management and execution

use tauri::State;
use tracing::{debug, error, info, instrument};

use crate::error::{GargoyleError, Result};
use crate::models::run::Run;
use crate::models::template::{
    CreateTemplatePayload, Template, TemplateIndex, UpdateTemplatePayload,
};
use crate::services::store::StoreService;
use crate::services::template_runner::{
    self, PrerequisiteResult, TemplateDefinition, TemplateInput, TemplateOutput,
};
use crate::services::template_service::TemplateService;
use crate::AppState;

// =============================================================================
// Template CRUD (database-backed)
// =============================================================================

#[tauri::command]
#[instrument(skip(state), fields(key = %payload.key, category = %payload.category))]
pub fn create_template(
    state: State<'_, AppState>,
    payload: CreateTemplatePayload,
) -> Result<Template> {
    info!("Creating template");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = TemplateService::create(conn, payload);
    match &result {
        Ok(t) => info!(template_id = %t.id, "Template created"),
        Err(e) => error!(error = %e, "Failed to create template"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(key = %key))]
pub fn get_template(state: State<'_, AppState>, key: String) -> Result<Template> {
    debug!("Getting template");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = TemplateService::get_by_key(conn, &key);
    match &result {
        Ok(_) => debug!("Template fetched"),
        Err(e) => error!(error = %e, "Failed to get template"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state, payload), fields(key = %payload.key))]
pub fn update_template(
    state: State<'_, AppState>,
    payload: UpdateTemplatePayload,
) -> Result<Template> {
    info!("Updating template");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = TemplateService::update(conn, payload);
    match &result {
        Ok(_) => info!("Template updated"),
        Err(e) => error!(error = %e, "Failed to update template"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(key = %key))]
pub fn delete_template(state: State<'_, AppState>, key: String) -> Result<()> {
    info!("Deleting template");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = TemplateService::delete(conn, &key);
    match &result {
        Ok(_) => info!("Template deleted"),
        Err(e) => error!(error = %e, "Failed to delete template"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(category = ?category))]
pub fn list_templates_db(
    state: State<'_, AppState>,
    category: Option<String>,
) -> Result<Vec<TemplateIndex>> {
    debug!("Listing templates from database");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = TemplateService::list(conn, category.as_deref());
    match &result {
        Ok(templates) => debug!(count = templates.len(), "Templates listed"),
        Err(e) => error!(error = %e, "Failed to list templates"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(query_len = query.len(), limit = ?limit))]
pub fn search_templates(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<TemplateIndex>> {
    debug!("Searching templates");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = TemplateService::search(conn, &query, limit.unwrap_or(10));
    match &result {
        Ok(templates) => debug!(count = templates.len(), "Templates found"),
        Err(e) => error!(error = %e, "Failed to search templates"),
    }
    result
}

// =============================================================================
// Template execution (legacy file-based, will be deprecated)
// =============================================================================

#[tauri::command]
#[instrument(skip(state, input), fields(template_key = %input.template_key))]
pub fn run_template(state: State<'_, AppState>, input: TemplateInput) -> Result<TemplateOutput> {
    info!("Running template");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = template_runner::run_template_full(conn, &input);
    match &result {
        Ok(output) => info!(run_id = %output.run_id, "Template run completed"),
        Err(e) => error!(error = %e, "Failed to run template"),
    }
    result
}

#[tauri::command]
#[instrument(skip(state), fields(template_key = %template_key))]
pub fn check_prerequisites(
    state: State<'_, AppState>,
    template_key: String,
) -> Result<Vec<PrerequisiteResult>> {
    debug!("Checking template prerequisites");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = template_runner::check_prerequisites(conn, &template_key);
    match &result {
        Ok(prereqs) => debug!(count = prereqs.len(), "Prerequisites checked"),
        Err(e) => error!(error = %e, "Failed to check prerequisites"),
    }
    result
}

#[tauri::command]
#[instrument(skip(_state))]
pub fn list_templates(_state: State<'_, AppState>) -> Result<Vec<TemplateDefinition>> {
    debug!("Listing template definitions");
    let templates = template_runner::list_template_definitions();
    debug!(count = templates.len(), "Template definitions listed");
    Ok(templates)
}

#[tauri::command]
#[instrument(skip(state), fields(template_key = ?template_key))]
pub fn list_runs(state: State<'_, AppState>, template_key: Option<String>) -> Result<Vec<Run>> {
    debug!("Listing runs");
    let guard = state.db.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        GargoyleError::Schema("Database not initialized".to_string())
    })?;
    let result = StoreService::list_runs(conn, template_key.as_deref());
    match &result {
        Ok(runs) => debug!(count = runs.len(), "Runs listed"),
        Err(e) => error!(error = %e, "Failed to list runs"),
    }
    result
}
