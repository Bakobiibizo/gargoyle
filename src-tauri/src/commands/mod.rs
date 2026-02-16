pub mod entity_commands;
pub mod relation_commands;
pub mod search_commands;
pub mod context_commands;
pub mod template_commands;
pub mod graph_commands;
pub mod dedup_commands;

pub use entity_commands::{
    create_entity, update_entity, get_entity, list_entities, delete_entity, apply_patch_set,
    migrate_entity, migrate_all_entities, find_stale_entities,
};
pub use relation_commands::{create_relation, get_relations};
pub use search_commands::{search_fts, search_similar, generate_embedding, reindex_entity};
pub use context_commands::{get_context, set_context, delete_context, list_contexts};
pub use template_commands::{run_template, check_prerequisites, list_templates};
pub use graph_commands::{get_entity_graph, audit_related_to, rebuild_projection, approve_custom_relation_type, reclassify_relations};
pub use dedup_commands::{check_duplicates, list_dedup_suggestions, resolve_dedup_suggestion};
