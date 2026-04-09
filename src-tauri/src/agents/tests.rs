#[cfg(test)]
mod tests {
    use crate::agents::graph_build_agent::{ContextGraph, GraphBuildAgent};
    use crate::agents::intake_agent::{IntakeAgent, IntakeExtraction, KeyValuePair};
    use crate::agents::pipeline::{IntakePipeline, PipelineState};
    use crate::db::connection::create_connection;
    use crate::db::migrations::run_migrations;

    fn test_db() -> rusqlite::Connection {
        let conn = create_connection(":memory:").unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_intake_agent_parse_response() {
        let response = r#"Great to meet you, John! What brings you to Gargoyle today?

```json
{
  "extracted": [
    {"key": "user_name", "value": "John", "category": "user", "confidence": 0.95}
  ],
  "conversation_complete": false
}
```"#;

        let extraction = IntakeAgent::parse_response(response);
        assert!(extraction.is_some());

        let extraction = extraction.unwrap();
        assert_eq!(extraction.extracted.len(), 1);
        assert_eq!(extraction.extracted[0].key, "user_name");
        assert_eq!(extraction.extracted[0].value, "John");
        assert!(!extraction.conversation_complete);
    }

    #[test]
    fn test_intake_agent_get_conversational_reply() {
        let response = r#"Great to meet you, John! What brings you to Gargoyle today?

```json
{
  "extracted": [
    {"key": "user_name", "value": "John", "category": "user", "confidence": 0.95}
  ],
  "conversation_complete": false
}
```"#;

        let reply = IntakeAgent::get_conversational_reply(response);
        assert_eq!(
            reply,
            "Great to meet you, John! What brings you to Gargoyle today?"
        );
    }

    #[test]
    fn test_graph_build_agent_parse_response() {
        let response = r#"Based on the collected data, here's the knowledge graph:

```json
{
  "nodes": [
    {
      "id": "temp_1",
      "entity_type": "doc",
      "title": "User Profile: John Smith",
      "body": "CEO of Acme Corp, focused on AI products",
      "canonical": {"doc_type": "user_profile", "tags": ["user"]},
      "source_keys": ["user_name", "user_role", "company_name"]
    }
  ],
  "edges": [],
  "keywords": ["CEO", "AI", "Acme Corp"],
  "primitive_types_used": ["doc"]
}
```"#;

        let graph = GraphBuildAgent::parse_response(response);
        assert!(graph.is_some());

        let graph = graph.unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].entity_type, "doc");
        assert_eq!(graph.keywords.len(), 3);
        assert_eq!(graph.primitive_types_used, vec!["doc"]);
    }

    #[test]
    fn test_graph_build_agent_build_prompt() {
        let data = vec![
            KeyValuePair {
                key: "user_name".to_string(),
                value: serde_json::json!("John"),
                category: Some("user".to_string()),
                confidence: 0.95,
            },
            KeyValuePair {
                key: "company_name".to_string(),
                value: serde_json::json!("Acme Corp"),
                category: Some("company".to_string()),
                confidence: 0.9,
            },
        ];

        let prompt = GraphBuildAgent::build_prompt(&data);
        assert!(prompt.contains("user_name"));
        assert!(prompt.contains("Acme Corp"));
    }

    #[test]
    fn test_pipeline_start_session() {
        let conn = test_db();
        let status = IntakePipeline::start_session(&conn).unwrap();

        assert!(matches!(status.state, PipelineState::Intake));
        assert!(!status.session_id.is_empty());
        assert!(status.intake_context.collected_data.is_empty());
        assert!(!status.intake_context.conversation_complete);
    }

    #[test]
    fn test_pipeline_process_assistant_response() {
        let conn = test_db();
        let mut status = IntakePipeline::start_session(&conn).unwrap();

        let response = r#"Hi there! I'm excited to help you get started. What's your name?

```json
{
  "extracted": [],
  "conversation_complete": false
}
```"#;

        let (reply, complete) =
            IntakePipeline::process_assistant_response(&mut status, response).unwrap();

        assert_eq!(
            reply,
            "Hi there! I'm excited to help you get started. What's your name?"
        );
        assert!(!complete);
        assert!(matches!(status.state, PipelineState::Intake));
    }

    #[test]
    fn test_pipeline_transition_to_graph_build() {
        let conn = test_db();
        let mut status = IntakePipeline::start_session(&conn).unwrap();

        let response = r#"Thanks for all that info! I've got a good picture now.

```json
{
  "extracted": [
    {"key": "user_name", "value": "John", "category": "user", "confidence": 0.95},
    {"key": "company_name", "value": "Acme", "category": "company", "confidence": 0.9}
  ],
  "conversation_complete": true
}
```"#;

        let (_, complete) =
            IntakePipeline::process_assistant_response(&mut status, response).unwrap();

        assert!(complete);
        assert!(matches!(status.state, PipelineState::GraphBuild));
        assert_eq!(status.intake_context.collected_data.len(), 2);
    }

    #[test]
    fn test_pipeline_full_flow() {
        let conn = test_db();
        let mut status = IntakePipeline::start_session(&conn).unwrap();

        // Simulate intake completion
        let intake_response = r#"Got it!

```json
{
  "extracted": [
    {"key": "user_name", "value": "John", "category": "user", "confidence": 0.95},
    {"key": "role", "value": "CEO", "category": "user", "confidence": 0.9},
    {"key": "company", "value": "Acme Corp", "category": "company", "confidence": 0.85}
  ],
  "conversation_complete": true
}
```"#;

        IntakePipeline::process_assistant_response(&mut status, intake_response).unwrap();
        assert!(matches!(status.state, PipelineState::GraphBuild));

        // Build graph prompt
        let prompt = IntakePipeline::build_graph(&mut status).unwrap();
        assert!(prompt.contains("user_name"));

        // Simulate graph build response
        let graph_response = r#"Here's the graph:

```json
{
  "nodes": [
    {
      "id": "temp_1",
      "entity_type": "doc",
      "title": "User Profile: John",
      "body": "CEO of Acme Corp",
      "canonical": {"doc_type": "user_profile", "tags": ["user", "ceo"]},
      "source_keys": ["user_name", "role"]
    },
    {
      "id": "temp_2", 
      "entity_type": "doc",
      "title": "Company: Acme Corp",
      "body": null,
      "canonical": {"doc_type": "company_profile", "tags": ["company"]},
      "source_keys": ["company"]
    }
  ],
  "edges": [
    {
      "from_id": "temp_1",
      "to_id": "temp_2",
      "relation_type": "related_to",
      "properties": null
    }
  ],
  "keywords": ["John", "CEO", "Acme Corp"],
  "primitive_types_used": ["doc"]
}
```"#;

        IntakePipeline::process_graph_response(&mut status, graph_response).unwrap();
        assert!(matches!(status.state, PipelineState::DBSync));
        assert!(status.graph.is_some());

        let graph = status.graph.as_ref().unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);

        // Sync to DB
        let result = IntakePipeline::sync_to_db(&conn, &mut status).unwrap();
        assert_eq!(result.entities_created, 2);
        assert_eq!(result.relations_created, 1);
        assert!(matches!(status.state, PipelineState::Complete));

        // Generate summary
        let summary = IntakePipeline::generate_summary(&status);
        assert_eq!(summary.entities_created, 2);
        assert_eq!(summary.relations_created, 1);
        assert!(summary.keywords.contains(&"CEO".to_string()));
    }
}
