use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tracing::{debug, instrument};

use crate::error::Result;

use super::entity_manager::EntityManagerAgent;
use super::graph_query::GraphQueryAgent;
use super::intake_handler::IntakeHandler;
use super::memory_agent::MemoryAgent;
use super::template_curator::TemplateCuratorAgent;
use super::types::*;

pub struct AgentRouter;

impl AgentRouter {
    #[instrument(skip(conn, request), fields(agent = ?std::mem::discriminant(&request)))]
    pub fn dispatch(conn: &Connection, request: AgentRequest) -> Result<AgentResponse> {
        debug!("Routing agent request");
        match request {
            AgentRequest::TemplateCurator(req) => {
                let response = TemplateCuratorAgent::handle(conn, req)?;
                Ok(AgentResponse::TemplateCurator(response))
            }
            AgentRequest::Intake(req) => {
                let response = IntakeHandler::handle(conn, req)?;
                Ok(AgentResponse::Intake(response))
            }
            AgentRequest::GraphQuery(req) => {
                let response = GraphQueryAgent::handle(conn, req)?;
                Ok(AgentResponse::GraphQuery(response))
            }
            AgentRequest::EntityManager(req) => {
                let response = EntityManagerAgent::handle(conn, req)?;
                Ok(AgentResponse::EntityManager(response))
            }
            AgentRequest::Memory(req) => {
                // Memory agent needs Arc<Mutex<Connection>> for internal service
                // For now, we create a wrapper - in production this should be passed through
                let conn_path = conn
                    .path()
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| ":memory:".to_string());
                let new_conn = Connection::open(&conn_path)?;
                let arc_conn = Arc::new(Mutex::new(new_conn));
                let agent = MemoryAgent::new(arc_conn);
                let response = agent.handle(req);
                Ok(AgentResponse::Memory(response))
            }
        }
    }
}
