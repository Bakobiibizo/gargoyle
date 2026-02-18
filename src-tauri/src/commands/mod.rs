pub mod entity_commands;
pub mod relation_commands;
pub mod search_commands;
pub mod context_commands;
pub mod template_commands;
pub mod graph_commands;
pub mod dedup_commands;
pub mod claim_commands;
pub mod llm_commands;
pub mod chat_commands;

pub use entity_commands::{
    create_entity, update_entity, get_entity, list_entities, delete_entity, apply_patch_set,
    migrate_entity, migrate_all_entities, find_stale_entities,
};
pub use relation_commands::{create_relation, get_relations};
pub use search_commands::{search_fts, search_similar, generate_embedding, reindex_entity};
pub use context_commands::{get_context, set_context, delete_context, list_contexts};
pub use template_commands::{run_template, check_prerequisites, list_templates, list_runs};
pub use graph_commands::{get_entity_graph, audit_related_to, rebuild_projection, approve_custom_relation_type, reclassify_relations};
pub use dedup_commands::{check_duplicates, list_dedup_suggestions, resolve_dedup_suggestion};
pub use claim_commands::{list_claims, get_claim, promote_claim};
pub use llm_commands::{llm_chat, llm_chat_with_tools, llm_complete, llm_status};
pub use chat_commands::{create_chat_session, list_chat_sessions, get_chat_messages, add_chat_message, update_chat_session_title, delete_chat_session};
