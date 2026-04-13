pub mod agent_commands;
pub mod chat_commands;
pub mod claim_commands;
pub mod context_commands;
pub mod dedup_commands;
pub mod entity_commands;
pub mod graph_commands;
pub mod intake_commands;
pub mod llm_commands;
pub mod relation_commands;
pub mod search_commands;
pub mod template_commands;

pub use chat_commands::{
    add_chat_message, create_chat_session, delete_chat_session, get_chat_messages,
    list_chat_sessions, update_chat_session_title,
};
pub use claim_commands::{get_claim, list_claims, promote_claim};
pub use context_commands::{delete_context, get_context, list_contexts, set_context};
pub use dedup_commands::{check_duplicates, list_dedup_suggestions, resolve_dedup_suggestion};
pub use entity_commands::{
    apply_patch_set, create_entity, delete_entity, find_stale_entities, get_entity, list_entities,
    migrate_all_entities, migrate_entity, update_entity,
};
pub use graph_commands::{
    approve_custom_relation_type, audit_related_to, get_entity_graph, rebuild_projection,
    reclassify_relations,
};
pub use llm_commands::{llm_chat, llm_chat_with_tools, llm_complete, llm_status};
pub use relation_commands::{create_relation, get_relations};
pub use search_commands::{generate_embedding, reindex_entity, search_fts, search_similar};
pub use template_commands::{check_prerequisites, list_runs, list_templates, run_template};
