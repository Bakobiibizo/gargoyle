use rusqlite::Connection;

use crate::error::Result;
use crate::models::patch::{CreateEntityPayload, CreateRelationPayload, PatchOp, PatchSet};
use crate::patch::apply::apply_patch_set;

use super::graph_build_agent::ContextGraph;

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub entities_created: usize,
    pub relations_created: usize,
    pub entity_ids: Vec<String>,
    pub errors: Vec<String>,
}

pub struct DBSyncAgent;

impl DBSyncAgent {
    pub fn sync_graph(conn: &Connection, graph: &ContextGraph, run_id: &str) -> Result<SyncResult> {
        let mut entity_ops: Vec<PatchOp> = Vec::new();
        let mut temp_id_to_index: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        // Phase 1: Create entities and track temp_id -> op_index mapping
        for (index, node) in graph.nodes.iter().enumerate() {
            temp_id_to_index.insert(node.id.clone(), index);

            let canonical = node.canonical.clone();

            entity_ops.push(PatchOp::CreateEntity(CreateEntityPayload {
                entity_type: node.entity_type.clone(),
                title: node.title.clone(),
                source: "agent".to_string(),
                canonical_fields: canonical,
                body_md: node.body.clone(),
                status: None,
                category: None,
                priority: None,
                reason: Some("Created from user intake interview".to_string()),
            }));
        }

        // Apply entity operations first
        let entity_patch_set = PatchSet {
            run_id: run_id.to_string(),
            ops: entity_ops,
        };
        let entity_result = apply_patch_set(conn, &entity_patch_set)?;

        // Build mapping from temp_id to real entity_id using result
        let mut id_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut entity_ids: Vec<String> = Vec::new();

        for (temp_id, &op_index) in &temp_id_to_index {
            if let Some(applied) = entity_result.applied.get(op_index) {
                if let Some(ref real_id) = applied.entity_id {
                    id_map.insert(temp_id.clone(), real_id.clone());
                    entity_ids.push(real_id.clone());
                }
            }
        }

        // Phase 2: Create relations (entities now exist in DB)
        if !graph.edges.is_empty() {
            let mut relation_ops: Vec<PatchOp> = Vec::new();

            for edge in &graph.edges {
                let from_id = id_map
                    .get(&edge.from_id)
                    .cloned()
                    .unwrap_or_else(|| edge.from_id.clone());
                let to_id = id_map
                    .get(&edge.to_id)
                    .cloned()
                    .unwrap_or_else(|| edge.to_id.clone());

                relation_ops.push(PatchOp::CreateRelation(CreateRelationPayload {
                    from_id,
                    to_id,
                    relation_type: edge.relation_type.clone(),
                    weight: None,
                    confidence: None,
                    provenance_run_id: Some(run_id.to_string()),
                    reason: Some("Relationship from intake interview".to_string()),
                }));
            }

            let relation_patch_set = PatchSet {
                run_id: run_id.to_string(),
                ops: relation_ops,
            };
            apply_patch_set(conn, &relation_patch_set)?;
        }

        Ok(SyncResult {
            entities_created: graph.nodes.len(),
            relations_created: graph.edges.len(),
            entity_ids,
            errors: Vec::new(),
        })
    }
}
