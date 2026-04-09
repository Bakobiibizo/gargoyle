use tauri::State;
use tracing::{error, info, instrument};

use crate::agents::{AgentRequest, AgentResponse, AgentRouter};
use crate::AppState;

/// Single unified command for all agent operations.
/// Replaces the need for individual commands per agent action.
///
/// Example frontend usage:
/// ```typescript
/// // Template search
/// await invoke('agent_dispatch', {
///   request: {
///     agent: 'TemplateCurator',
///     request: { action: 'Search', query: 'project planning', limit: 5 }
///   }
/// });
///
/// // Start intake
/// await invoke('agent_dispatch', {
///   request: {
///     agent: 'Intake',
///     request: { action: 'StartSession' }
///   }
/// });
///
/// // Query graph
/// await invoke('agent_dispatch', {
///   request: {
///     agent: 'GraphQuery',
///     request: { action: 'GetStatistics', entity_type: null }
///   }
/// });
/// ```
#[tauri::command]
#[instrument(skip(state, request), fields(agent = ?std::mem::discriminant(&request)))]
pub fn agent_dispatch(
    state: State<AppState>,
    request: AgentRequest,
) -> Result<AgentResponse, String> {
    info!("Agent dispatch");
    let guard = state.db.lock().map_err(|e: std::sync::PoisonError<_>| {
        error!(error = %e, "Failed to acquire database lock");
        e.to_string()
    })?;
    let conn = guard.as_ref().ok_or_else(|| {
        error!("Database not initialized");
        "Database not initialized".to_string()
    })?;
    let result = AgentRouter::dispatch(conn, request);
    match &result {
        Ok(_) => info!("Agent dispatch completed"),
        Err(e) => error!(error = %e, "Agent dispatch failed"),
    }
    result.map_err(|e| e.to_string())
}
