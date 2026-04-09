use crate::models::memory::{
    ConversationSegment, LongTermMemoryType, MemorySearchResult, MessageRole, ShortTermMemory,
    ShortTermMemoryType,
};
use crate::services::memory_service::{MemoryError, MemoryService};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAgentRequest {
    pub action: MemoryAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MemoryAction {
    StartConversation,
    EndConversation {
        conversation_id: String,
        summary: Option<String>,
    },
    RecordMessage {
        conversation_id: String,
        role: String,
        content: String,
    },
    CreateObservation {
        content: String,
        conversation_id: Option<String>,
    },
    CreateInsight {
        content: String,
        conversation_id: Option<String>,
    },
    SearchMemories {
        query: String,
        limit: Option<usize>,
    },
    GetRecentMemories {
        limit: Option<usize>,
        memory_type: Option<String>,
    },
    PromoteToLongTerm {
        stm_id: String,
        memory_type: String,
        category: Option<String>,
    },
    GetContext {
        conversation_id: Option<String>,
        query: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAgentResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memories: Option<Vec<MemorySearchResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl MemoryAgentResponse {
    fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            conversation_id: None,
            memory_id: None,
            memories: None,
            context: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            conversation_id: None,
            memory_id: None,
            memories: None,
            context: None,
        }
    }

    fn with_conversation_id(mut self, id: String) -> Self {
        self.conversation_id = Some(id);
        self
    }

    fn with_memory_id(mut self, id: String) -> Self {
        self.memory_id = Some(id);
        self
    }

    fn with_memories(mut self, memories: Vec<MemorySearchResult>) -> Self {
        self.memories = Some(memories);
        self
    }

    fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }
}

pub struct MemoryAgent {
    service: MemoryService,
}

impl MemoryAgent {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            service: MemoryService::new(conn),
        }
    }

    pub fn with_service(service: MemoryService) -> Self {
        Self { service }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.service.set_enabled(enabled);
    }

    pub fn is_enabled(&self) -> bool {
        self.service.is_enabled()
    }

    pub fn handle(&self, request: MemoryAgentRequest) -> MemoryAgentResponse {
        match request.action {
            MemoryAction::StartConversation => self.start_conversation(),
            MemoryAction::EndConversation {
                conversation_id,
                summary,
            } => self.end_conversation(&conversation_id, summary),
            MemoryAction::RecordMessage {
                conversation_id,
                role,
                content,
            } => self.record_message(&conversation_id, &role, content),
            MemoryAction::CreateObservation {
                content,
                conversation_id,
            } => self.create_observation(content, conversation_id),
            MemoryAction::CreateInsight {
                content,
                conversation_id,
            } => self.create_insight(content, conversation_id),
            MemoryAction::SearchMemories { query, limit } => {
                self.search_memories(&query, limit.unwrap_or(10))
            }
            MemoryAction::GetRecentMemories { limit, memory_type } => {
                self.get_recent_memories(limit.unwrap_or(10), memory_type)
            }
            MemoryAction::PromoteToLongTerm {
                stm_id,
                memory_type,
                category,
            } => self.promote_to_long_term(&stm_id, &memory_type, category),
            MemoryAction::GetContext {
                conversation_id,
                query,
            } => self.get_context(conversation_id, query),
        }
    }

    fn start_conversation(&self) -> MemoryAgentResponse {
        match self.service.create_conversation() {
            Ok(conv) => {
                MemoryAgentResponse::success("Conversation started").with_conversation_id(conv.id)
            }
            Err(e) => MemoryAgentResponse::error(format!("Failed to start conversation: {}", e)),
        }
    }

    fn end_conversation(&self, id: &str, summary: Option<String>) -> MemoryAgentResponse {
        match self.service.end_conversation(id, summary) {
            Ok(()) => MemoryAgentResponse::success("Conversation ended"),
            Err(e) => MemoryAgentResponse::error(format!("Failed to end conversation: {}", e)),
        }
    }

    fn record_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: String,
    ) -> MemoryAgentResponse {
        let role = match role.to_lowercase().as_str() {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => return MemoryAgentResponse::error(format!("Invalid role: {}", role)),
        };

        match self
            .service
            .add_segment(conversation_id, role, content, None)
        {
            Ok(segment) => {
                MemoryAgentResponse::success("Message recorded").with_memory_id(segment.id)
            }
            Err(e) => MemoryAgentResponse::error(format!("Failed to record message: {}", e)),
        }
    }

    fn create_observation(
        &self,
        content: String,
        conversation_id: Option<String>,
    ) -> MemoryAgentResponse {
        let mut memory = ShortTermMemory::new(content, ShortTermMemoryType::Observation);
        if let Some(conv_id) = conversation_id {
            memory = memory.with_source(conv_id, None);
        }

        match self.service.create_stm(memory) {
            Ok(stm) => MemoryAgentResponse::success("Observation recorded").with_memory_id(stm.id),
            Err(e) => MemoryAgentResponse::error(format!("Failed to create observation: {}", e)),
        }
    }

    fn create_insight(
        &self,
        content: String,
        conversation_id: Option<String>,
    ) -> MemoryAgentResponse {
        let mut memory = ShortTermMemory::new(content, ShortTermMemoryType::Insight);
        if let Some(conv_id) = conversation_id {
            memory = memory.with_source(conv_id, None);
        }

        match self.service.create_stm(memory) {
            Ok(stm) => MemoryAgentResponse::success("Insight recorded").with_memory_id(stm.id),
            Err(e) => MemoryAgentResponse::error(format!("Failed to create insight: {}", e)),
        }
    }

    fn search_memories(&self, query: &str, limit: usize) -> MemoryAgentResponse {
        match self.service.search_all(query, limit) {
            Ok(results) => {
                let count = results.len();
                MemoryAgentResponse::success(format!("Found {} memories", count))
                    .with_memories(results)
            }
            Err(e) => MemoryAgentResponse::error(format!("Search failed: {}", e)),
        }
    }

    fn get_recent_memories(
        &self,
        limit: usize,
        memory_type: Option<String>,
    ) -> MemoryAgentResponse {
        let stm_type = memory_type.and_then(|t| t.parse::<ShortTermMemoryType>().ok());

        match self.service.get_recent_stm(limit, stm_type) {
            Ok(memories) => {
                let results: Vec<MemorySearchResult> = memories
                    .into_iter()
                    .map(|m| MemorySearchResult {
                        memory_id: m.id.clone(),
                        content: m.content.clone(),
                        memory_type: m.memory_type.to_string(),
                        score: m.current_relevance(),
                        source: "short_term".to_string(),
                    })
                    .collect();

                let count = results.len();
                MemoryAgentResponse::success(format!("Retrieved {} recent memories", count))
                    .with_memories(results)
            }
            Err(e) => MemoryAgentResponse::error(format!("Failed to get recent memories: {}", e)),
        }
    }

    fn promote_to_long_term(
        &self,
        stm_id: &str,
        memory_type: &str,
        category: Option<String>,
    ) -> MemoryAgentResponse {
        let ltm_type = match memory_type.to_lowercase().as_str() {
            "fact" => LongTermMemoryType::Fact,
            "preference" => LongTermMemoryType::Preference,
            "pattern" => LongTermMemoryType::Pattern,
            "relationship" => LongTermMemoryType::Relationship,
            "skill" => LongTermMemoryType::Skill,
            "context" => LongTermMemoryType::Context,
            _ => {
                return MemoryAgentResponse::error(format!("Invalid memory type: {}", memory_type))
            }
        };

        match self.service.promote_to_ltm(stm_id, ltm_type, category) {
            Ok(ltm) => {
                MemoryAgentResponse::success("Memory promoted to long-term").with_memory_id(ltm.id)
            }
            Err(e) => MemoryAgentResponse::error(format!("Failed to promote memory: {}", e)),
        }
    }

    fn get_context(
        &self,
        conversation_id: Option<String>,
        query: Option<String>,
    ) -> MemoryAgentResponse {
        let mut context_parts = Vec::new();

        if let Some(conv_id) = &conversation_id {
            match self.service.get_conversation_segments(conv_id) {
                Ok(segments) => {
                    if !segments.is_empty() {
                        let recent: Vec<&ConversationSegment> =
                            segments.iter().rev().take(10).collect();
                        let history: String = recent
                            .iter()
                            .rev()
                            .map(|s| format!("{}: {}", s.role, s.content))
                            .collect::<Vec<_>>()
                            .join("\n");
                        context_parts.push(format!("## Recent Conversation\n{}", history));
                    }
                }
                Err(e) => {
                    context_parts.push(format!("(Error loading conversation: {})", e));
                }
            }
        }

        if let Some(q) = &query {
            match self.service.search_all(q, 5) {
                Ok(memories) if !memories.is_empty() => {
                    let mem_context: String = memories
                        .iter()
                        .map(|m| format!("- [{}] {}", m.memory_type, m.content))
                        .collect::<Vec<_>>()
                        .join("\n");
                    context_parts.push(format!("## Relevant Memories\n{}", mem_context));
                }
                _ => {}
            }
        }

        match self.service.get_recent_stm(5, None) {
            Ok(recent) if !recent.is_empty() => {
                let recent_context: String = recent
                    .iter()
                    .map(|m| format!("- [{}] {}", m.memory_type, m.content))
                    .collect::<Vec<_>>()
                    .join("\n");
                context_parts.push(format!("## Recent Observations\n{}", recent_context));
            }
            _ => {}
        }

        let context = if context_parts.is_empty() {
            "No relevant context found.".to_string()
        } else {
            context_parts.join("\n\n")
        };

        MemoryAgentResponse::success("Context retrieved").with_context(context)
    }

    pub fn extract_observations_from_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ShortTermMemory>, MemoryError> {
        let segments = self.service.get_conversation_segments(conversation_id)?;
        let mut observations = Vec::new();

        for segment in segments {
            if segment.role == MessageRole::User {
                if Self::contains_preference(&segment.content) {
                    let mut mem = ShortTermMemory::new(
                        segment.content.clone(),
                        ShortTermMemoryType::Preference,
                    );
                    mem = mem.with_source(conversation_id.to_string(), Some(segment.id.clone()));
                    observations.push(self.service.create_stm(mem)?);
                } else if Self::contains_fact(&segment.content) {
                    let mut mem =
                        ShortTermMemory::new(segment.content.clone(), ShortTermMemoryType::Fact);
                    mem = mem.with_source(conversation_id.to_string(), Some(segment.id.clone()));
                    observations.push(self.service.create_stm(mem)?);
                }
            }
        }

        Ok(observations)
    }

    fn contains_preference(content: &str) -> bool {
        let lower = content.to_lowercase();
        lower.contains("i prefer")
            || lower.contains("i like")
            || lower.contains("i don't like")
            || lower.contains("i want")
            || lower.contains("i need")
            || lower.contains("my favorite")
            || lower.contains("i always")
            || lower.contains("i never")
    }

    fn contains_fact(content: &str) -> bool {
        let lower = content.to_lowercase();
        lower.contains("my name is")
            || lower.contains("i work at")
            || lower.contains("i live in")
            || lower.contains("i am a")
            || lower.contains("i'm a")
            || lower.contains("my job is")
            || lower.contains("my email is")
            || lower.contains("my phone is")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_agent() -> MemoryAgent {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../migrations/008_memory_tables.sql"))
            .unwrap();
        MemoryAgent::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_conversation_flow() {
        let agent = setup_test_agent();

        let start = agent.handle(MemoryAgentRequest {
            action: MemoryAction::StartConversation,
        });
        assert!(start.success);
        let conv_id = start.conversation_id.unwrap();

        let record = agent.handle(MemoryAgentRequest {
            action: MemoryAction::RecordMessage {
                conversation_id: conv_id.clone(),
                role: "user".to_string(),
                content: "Hello!".to_string(),
            },
        });
        assert!(record.success);

        let end = agent.handle(MemoryAgentRequest {
            action: MemoryAction::EndConversation {
                conversation_id: conv_id,
                summary: Some("Test conversation".to_string()),
            },
        });
        assert!(end.success);
    }

    #[test]
    fn test_observation_creation() {
        let agent = setup_test_agent();

        let response = agent.handle(MemoryAgentRequest {
            action: MemoryAction::CreateObservation {
                content: "User prefers dark mode".to_string(),
                conversation_id: None,
            },
        });

        assert!(response.success);
        assert!(response.memory_id.is_some());
    }

    #[test]
    fn test_search_memories() {
        let agent = setup_test_agent();

        agent.handle(MemoryAgentRequest {
            action: MemoryAction::CreateObservation {
                content: "User works with Rust programming".to_string(),
                conversation_id: None,
            },
        });

        let search = agent.handle(MemoryAgentRequest {
            action: MemoryAction::SearchMemories {
                query: "Rust".to_string(),
                limit: Some(5),
            },
        });

        assert!(search.success);
        assert!(search.memories.is_some());
    }

    #[test]
    fn test_get_context() {
        let agent = setup_test_agent();

        agent.handle(MemoryAgentRequest {
            action: MemoryAction::CreateObservation {
                content: "User is building a knowledge graph app".to_string(),
                conversation_id: None,
            },
        });

        let context = agent.handle(MemoryAgentRequest {
            action: MemoryAction::GetContext {
                conversation_id: None,
                query: Some("knowledge graph".to_string()),
            },
        });

        assert!(context.success);
        assert!(context.context.is_some());
    }
}
