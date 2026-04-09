use rusqlite::Connection;
use tracing::{debug, instrument};

use crate::error::Result;

use super::graph_build_agent::GraphBuildAgent;
use super::intake_agent::IntakeAgent;
use super::pipeline::IntakePipeline;
use super::types::{IntakeRequest, IntakeResponse};

pub struct IntakeHandler;

impl IntakeHandler {
    #[instrument(skip(conn), fields(action = ?std::mem::discriminant(&request)))]
    pub fn handle(conn: &Connection, request: IntakeRequest) -> Result<IntakeResponse> {
        debug!("Handling intake request");
        match request {
            IntakeRequest::StartSession => {
                let status = IntakePipeline::start_session(conn)?;
                Ok(IntakeResponse::SessionStarted { status })
            }

            IntakeRequest::GetSystemPrompt => Ok(IntakeResponse::SystemPrompt {
                prompt: IntakeAgent::system_prompt().to_string(),
            }),

            IntakeRequest::ProcessUserMessage {
                mut status,
                message,
            } => {
                IntakePipeline::process_user_message(conn, &mut status, &message)?;
                Ok(IntakeResponse::MessageProcessed { status })
            }

            IntakeRequest::ProcessAssistantResponse {
                mut status,
                response,
            } => {
                let (reply, complete) =
                    IntakePipeline::process_assistant_response(&mut status, &response)?;
                Ok(IntakeResponse::ConversationReply {
                    status,
                    reply,
                    complete,
                })
            }

            IntakeRequest::BuildGraph { status } => {
                let user_prompt = IntakePipeline::build_graph(&mut status.clone())?;
                Ok(IntakeResponse::GraphPrompt {
                    system_prompt: GraphBuildAgent::system_prompt().to_string(),
                    user_prompt,
                })
            }

            IntakeRequest::ProcessGraphResponse {
                mut status,
                response,
            } => {
                IntakePipeline::process_graph_response(&mut status, &response)?;
                Ok(IntakeResponse::GraphProcessed { status })
            }

            IntakeRequest::SyncToDb { mut status } => {
                let result = IntakePipeline::sync_to_db(conn, &mut status)?;
                Ok(IntakeResponse::Synced {
                    status,
                    entities: result.entities_created,
                    relations: result.relations_created,
                })
            }

            IntakeRequest::GetSummary { status } => {
                let summary = IntakePipeline::generate_summary(&status);
                Ok(IntakeResponse::Summary { summary })
            }
        }
    }
}
