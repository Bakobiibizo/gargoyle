use tauri::State;
use tracing::{debug, error, info, instrument};

use crate::agents::graph_build_agent::GraphBuildAgent;
use crate::agents::intake_agent::IntakeAgent;
use crate::agents::pipeline::{IntakePipeline, IntakeSummary, PipelineStatus};
use crate::AppState;

#[tauri::command]
#[instrument(skip(state))]
pub fn start_intake(state: State<AppState>) -> Result<PipelineStatus, String> {
    info!("Starting intake session");
    let guard = state.db.lock().map_err(|e| {
        error!(error = %e, "Failed to acquire database lock");
        e.to_string()
    })?;
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        "Database not initialized".to_string()
    })?;

    let result = IntakePipeline::start_session(conn);
    match &result {
        Ok(status) => info!(state = ?status.state, "Intake session started"),
        Err(e) => error!(error = %e, "Failed to start intake session"),
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument]
pub fn get_intake_system_prompt() -> String {
    debug!("Getting intake system prompt");
    IntakeAgent::system_prompt().to_string()
}

#[tauri::command]
#[instrument(skip(state, status, user_message), fields(state = ?status.state, msg_len = user_message.len()))]
pub fn process_intake_message(
    state: State<AppState>,
    status: PipelineStatus,
    user_message: String,
) -> Result<PipelineStatus, String> {
    debug!("Processing intake message");
    let guard = state.db.lock().map_err(|e| {
        error!(error = %e, "Failed to acquire database lock");
        e.to_string()
    })?;
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        "Database not initialized".to_string()
    })?;

    let mut status = status;
    let result = IntakePipeline::process_user_message(conn, &mut status, &user_message);
    match &result {
        Ok(_) => debug!(new_state = ?status.state, "Intake message processed"),
        Err(e) => error!(error = %e, "Failed to process intake message"),
    }
    result.map_err(|e| e.to_string())?;
    Ok(status)
}

#[tauri::command]
#[instrument(skip(status, assistant_response), fields(state = ?status.state, response_len = assistant_response.len()))]
pub fn process_intake_response(
    status: PipelineStatus,
    assistant_response: String,
) -> Result<(PipelineStatus, String, bool), String> {
    debug!("Processing intake response");
    let mut status = status;
    let result = IntakePipeline::process_assistant_response(&mut status, &assistant_response);
    match &result {
        Ok((_, complete)) => debug!(complete = complete, "Intake response processed"),
        Err(e) => error!(error = %e, "Failed to process intake response"),
    }
    let (reply, complete) = result.map_err(|e| e.to_string())?;
    Ok((status, reply, complete))
}

#[tauri::command]
#[instrument(skip(status), fields(state = ?status.state))]
pub fn get_graph_build_prompt(status: PipelineStatus) -> Result<(String, String), String> {
    debug!("Getting graph build prompt");
    let system_prompt = GraphBuildAgent::system_prompt().to_string();
    let result = IntakePipeline::build_graph(&mut status.clone());
    match &result {
        Ok(_) => debug!("Graph build prompt generated"),
        Err(e) => error!(error = %e, "Failed to generate graph build prompt"),
    }
    let user_prompt = result.map_err(|e| e.to_string())?;
    Ok((system_prompt, user_prompt))
}

#[tauri::command]
#[instrument(skip(status, graph_response), fields(state = ?status.state, response_len = graph_response.len()))]
pub fn process_graph_response(
    status: PipelineStatus,
    graph_response: String,
) -> Result<PipelineStatus, String> {
    info!("Processing graph response");
    let mut status = status;
    let result = IntakePipeline::process_graph_response(&mut status, &graph_response);
    match &result {
        Ok(_) => info!("Graph response processed"),
        Err(e) => error!(error = %e, "Failed to process graph response"),
    }
    result.map_err(|e| e.to_string())?;
    Ok(status)
}

#[tauri::command]
#[instrument(skip(state, status), fields(state = ?status.state))]
pub fn sync_intake_to_db(
    state: State<AppState>,
    status: PipelineStatus,
) -> Result<PipelineStatus, String> {
    info!("Syncing intake to database");
    let guard = state.db.lock().map_err(|e| {
        error!(error = %e, "Failed to acquire database lock");
        e.to_string()
    })?;
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        "Database not initialized".to_string()
    })?;

    let mut status = status;
    let result = IntakePipeline::sync_to_db(conn, &mut status);
    match &result {
        Ok(_) => info!("Intake synced to database"),
        Err(e) => error!(error = %e, "Failed to sync intake to database"),
    }
    result.map_err(|e| e.to_string())?;
    Ok(status)
}

#[tauri::command]
#[instrument(skip(status), fields(state = ?status.state))]
pub fn get_intake_summary(status: PipelineStatus) -> IntakeSummary {
    debug!("Generating intake summary");
    IntakePipeline::generate_summary(&status)
}
