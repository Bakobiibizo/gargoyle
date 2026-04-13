#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gargoyle_lib::db;
use gargoyle_lib::logging;
use gargoyle_lib::AppState;
use std::sync::Mutex;
use tracing::{error, info};

fn main() {
    // Load .env file (fail silently if not found — env vars may be set externally)
    dotenvy::dotenv().ok();

    // Initialize structured logging
    logging::init_logging();

    info!("Starting Gargoyle application");

    // Initialize the SQLite database
    let conn = match db::connection::create_connection("./gargoyle.db") {
        Ok(c) => {
            info!("Database connection established");
            c
        }
        Err(e) => {
            error!(error = %e, "Failed to create database connection");
            panic!("Failed to create database connection: {}", e);
        }
    };

    if let Err(e) = db::migrations::run_migrations(&conn) {
        error!(error = %e, "Failed to run database migrations");
        panic!("Failed to run database migrations: {}", e);
    }
    info!("Database migrations complete");

    if let Err(e) = db::seeders::seed_templates(&conn) {
        error!(error = %e, "Failed to seed templates");
        panic!("Failed to seed templates: {}", e);
    }
    info!("Templates seeded");

    tauri::Builder::default()
        .manage(AppState {
            db: Mutex::new(Some(conn)),
        })
        .invoke_handler(tauri::generate_handler![
            gargoyle_lib::commands::entity_commands::create_entity,
            gargoyle_lib::commands::entity_commands::update_entity,
            gargoyle_lib::commands::entity_commands::get_entity,
            gargoyle_lib::commands::entity_commands::list_entities,
            gargoyle_lib::commands::entity_commands::delete_entity,
            gargoyle_lib::commands::entity_commands::apply_patch_set,
            gargoyle_lib::commands::entity_commands::migrate_entity,
            gargoyle_lib::commands::entity_commands::migrate_all_entities,
            gargoyle_lib::commands::entity_commands::find_stale_entities,
            gargoyle_lib::commands::relation_commands::create_relation,
            gargoyle_lib::commands::relation_commands::get_relations,
            gargoyle_lib::commands::search_commands::search_fts,
            gargoyle_lib::commands::search_commands::search_similar,
            gargoyle_lib::commands::search_commands::generate_embedding,
            gargoyle_lib::commands::search_commands::reindex_entity,
            gargoyle_lib::commands::context_commands::get_context,
            gargoyle_lib::commands::context_commands::set_context,
            gargoyle_lib::commands::context_commands::delete_context,
            gargoyle_lib::commands::context_commands::list_contexts,
            gargoyle_lib::commands::template_commands::run_template,
            gargoyle_lib::commands::template_commands::check_prerequisites,
            gargoyle_lib::commands::template_commands::list_templates,
            gargoyle_lib::commands::template_commands::list_runs,
            gargoyle_lib::commands::template_commands::create_template,
            gargoyle_lib::commands::template_commands::get_template,
            gargoyle_lib::commands::template_commands::update_template,
            gargoyle_lib::commands::template_commands::delete_template,
            gargoyle_lib::commands::template_commands::list_templates_db,
            gargoyle_lib::commands::template_commands::search_templates,
            gargoyle_lib::commands::graph_commands::get_entity_graph,
            gargoyle_lib::commands::graph_commands::audit_related_to,
            gargoyle_lib::commands::graph_commands::rebuild_projection,
            gargoyle_lib::commands::graph_commands::approve_custom_relation_type,
            gargoyle_lib::commands::graph_commands::reclassify_relations,
            gargoyle_lib::commands::dedup_commands::check_duplicates,
            gargoyle_lib::commands::dedup_commands::list_dedup_suggestions,
            gargoyle_lib::commands::dedup_commands::resolve_dedup_suggestion,
            gargoyle_lib::commands::claim_commands::list_claims,
            gargoyle_lib::commands::claim_commands::get_claim,
            gargoyle_lib::commands::claim_commands::promote_claim,
            gargoyle_lib::commands::llm_commands::llm_chat,
            gargoyle_lib::commands::llm_commands::llm_chat_with_tools,
            gargoyle_lib::commands::llm_commands::llm_complete,
            gargoyle_lib::commands::llm_commands::llm_status,
            gargoyle_lib::commands::chat_commands::create_chat_session,
            gargoyle_lib::commands::chat_commands::list_chat_sessions,
            gargoyle_lib::commands::chat_commands::get_chat_messages,
            gargoyle_lib::commands::chat_commands::add_chat_message,
            gargoyle_lib::commands::chat_commands::update_chat_session_title,
            gargoyle_lib::commands::chat_commands::delete_chat_session,
            gargoyle_lib::commands::intake_commands::start_intake,
            gargoyle_lib::commands::intake_commands::get_intake_system_prompt,
            gargoyle_lib::commands::intake_commands::process_intake_message,
            gargoyle_lib::commands::intake_commands::process_intake_response,
            gargoyle_lib::commands::intake_commands::get_graph_build_prompt,
            gargoyle_lib::commands::intake_commands::process_graph_response,
            gargoyle_lib::commands::intake_commands::sync_intake_to_db,
            gargoyle_lib::commands::intake_commands::get_intake_summary,
            gargoyle_lib::commands::agent_commands::agent_dispatch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
