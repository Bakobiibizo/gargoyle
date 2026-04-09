use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;
use crate::services::chat_service::ChatService;

use super::db_sync_agent::{DBSyncAgent, SyncResult};
use super::graph_build_agent::{ContextGraph, GraphBuildAgent};
use super::intake_agent::{IntakeAgent, IntakeContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineState {
    Intake,
    GraphBuild,
    DBSync,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStatus {
    pub state: PipelineState,
    pub session_id: String,
    pub intake_context: IntakeContext,
    pub graph: Option<ContextGraph>,
    pub sync_result: Option<SyncResultSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResultSummary {
    pub entities_created: usize,
    pub relations_created: usize,
    pub entity_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntakeSummary {
    pub user_message: String,
    pub keywords: Vec<String>,
    pub primitive_types: Vec<String>,
    pub entities_created: usize,
    pub relations_created: usize,
    pub graph_structure: GraphStructureSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStructureSummary {
    pub nodes: Vec<NodeSummary>,
    pub edge_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub id: String,
    pub entity_type: String,
    pub title: String,
}

pub struct IntakePipeline;

impl IntakePipeline {
    pub fn start_session(conn: &Connection) -> Result<PipelineStatus> {
        let session = ChatService::create_session(
            conn,
            "User Intake Interview",
            Some(IntakeAgent::system_prompt()),
        )?;

        Ok(PipelineStatus {
            state: PipelineState::Intake,
            session_id: session.id,
            intake_context: IntakeContext::default(),
            graph: None,
            sync_result: None,
        })
    }

    pub fn process_user_message(
        conn: &Connection,
        status: &mut PipelineStatus,
        user_message: &str,
    ) -> Result<String> {
        // Store user message
        ChatService::add_message(conn, &status.session_id, "user", user_message, None, None)?;

        // Return placeholder - actual LLM call happens in frontend/command layer
        Ok("awaiting_llm_response".to_string())
    }

    pub fn process_assistant_response(
        status: &mut PipelineStatus,
        response: &str,
    ) -> Result<(String, bool)> {
        // Parse extraction from response
        if let Some(extraction) = IntakeAgent::parse_response(response) {
            // Add extracted data to context
            for kv in extraction.extracted {
                status.intake_context.collected_data.push(kv);
            }

            // Check if conversation complete
            if extraction.conversation_complete {
                status.intake_context.conversation_complete = true;
                status.state = PipelineState::GraphBuild;
            }
        }

        // Get conversational reply for user
        let reply = IntakeAgent::get_conversational_reply(response);
        let complete = status.intake_context.conversation_complete;

        Ok((reply, complete))
    }

    pub fn build_graph(status: &mut PipelineStatus) -> Result<String> {
        // Return the prompt for GraphBuildAgent
        let prompt = GraphBuildAgent::build_prompt(&status.intake_context.collected_data);
        Ok(prompt)
    }

    pub fn process_graph_response(status: &mut PipelineStatus, response: &str) -> Result<()> {
        if let Some(graph) = GraphBuildAgent::parse_response(response) {
            status.graph = Some(graph);
            status.state = PipelineState::DBSync;
        }
        Ok(())
    }

    pub fn sync_to_db(conn: &Connection, status: &mut PipelineStatus) -> Result<SyncResult> {
        let graph = status
            .graph
            .as_ref()
            .ok_or_else(|| crate::error::GargoyleError::Schema("No graph to sync".to_string()))?;

        let run_id = Uuid::new_v4().to_string();
        let result = DBSyncAgent::sync_graph(conn, graph, &run_id)?;

        status.sync_result = Some(SyncResultSummary {
            entities_created: result.entities_created,
            relations_created: result.relations_created,
            entity_ids: result.entity_ids.clone(),
        });
        status.state = PipelineState::Complete;

        Ok(result)
    }

    pub fn generate_summary(status: &PipelineStatus) -> IntakeSummary {
        let graph = status.graph.as_ref();
        let sync = status.sync_result.as_ref();

        IntakeSummary {
            user_message: format!(
                "Intake complete! I've learned about you and created {} entities with {} relationships.",
                sync.map(|s| s.entities_created).unwrap_or(0),
                sync.map(|s| s.relations_created).unwrap_or(0),
            ),
            keywords: graph.map(|g| g.keywords.clone()).unwrap_or_default(),
            primitive_types: graph.map(|g| g.primitive_types_used.clone()).unwrap_or_default(),
            entities_created: sync.map(|s| s.entities_created).unwrap_or(0),
            relations_created: sync.map(|s| s.relations_created).unwrap_or(0),
            graph_structure: GraphStructureSummary {
                nodes: graph
                    .map(|g| {
                        g.nodes
                            .iter()
                            .enumerate()
                            .map(|(i, n)| NodeSummary {
                                id: sync
                                    .and_then(|s| s.entity_ids.get(i).cloned())
                                    .unwrap_or_else(|| n.id.clone()),
                                entity_type: n.entity_type.clone(),
                                title: n.title.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                edge_count: graph.map(|g| g.edges.len()).unwrap_or(0),
            },
        }
    }
}
