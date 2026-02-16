#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gargoyle_lib::AppState;
use gargoyle_lib::db;
use std::sync::Mutex;

fn main() {
    // Load .env file (fail silently if not found — env vars may be set externally)
    dotenvy::dotenv().ok();

    // Initialize the SQLite database
    let conn = db::connection::create_connection("./gargoyle.db")
        .expect("Failed to create database connection");
    db::migrations::run_migrations(&conn)
        .expect("Failed to run database migrations");

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
            gargoyle_lib::commands::llm_commands::llm_complete,
            gargoyle_lib::commands::llm_commands::llm_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
