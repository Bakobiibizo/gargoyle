use serde::{Deserialize, Serialize};

use super::intake_agent::KeyValuePair;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub entity_type: String,
    pub title: String,
    pub body: Option<String>,
    pub canonical: serde_json::Value,
    pub source_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from_id: String,
    pub to_id: String,
    pub relation_type: String,
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub keywords: Vec<String>,
    pub primitive_types_used: Vec<String>,
}

impl Default for ContextGraph {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            keywords: Vec::new(),
            primitive_types_used: Vec::new(),
        }
    }
}

pub const GRAPH_BUILD_SYSTEM_PROMPT: &str = r#"You are Gargoyle's GraphBuildAgent. Your job is to transform collected interview data into a structured knowledge graph.

## Input
You receive a list of key-value pairs extracted from a user intake interview.

## Your Task
1. Identify entities that should be created (using ONLY these types: doc, note, idea, task)
2. Extract keywords and tags from the content
3. Identify relationships between entities
4. Structure canonical fields for each entity

## Entity Type Guidelines
- **doc**: Long-form content, documents, company info, user profiles
- **note**: Quick observations, preferences, contextual details
- **idea**: Concepts to explore, potential projects, opportunities
- **task**: Actionable items mentioned by user

## Output Format
Return a JSON object:

```json
{
  "nodes": [
    {
      "id": "temp_1",
      "entity_type": "doc",
      "title": "User Profile: John Smith",
      "body": "CEO of Acme Corp...",
      "canonical": {
        "doc_type": "user_profile",
        "tags": ["user", "profile"]
      },
      "source_keys": ["user_name", "user_role"]
    }
  ],
  "edges": [
    {
      "from_id": "temp_1",
      "to_id": "temp_2",
      "relation_type": "works_at",
      "properties": null
    }
  ],
  "keywords": ["CEO", "startup", "AI"],
  "primitive_types_used": ["doc", "note"]
}
```

## Rules
- Generate temporary IDs (temp_1, temp_2, etc.) - real UUIDs assigned later
- Only use entity types: doc, note, idea, task
- Only use relation types: related_to, part_of, depends_on, mentions, supports, derived_from
- Include source_keys to trace which interview data produced each node
- Extract meaningful keywords for search/indexing
"#;

pub struct GraphBuildAgent;

impl GraphBuildAgent {
    pub fn system_prompt() -> &'static str {
        GRAPH_BUILD_SYSTEM_PROMPT
    }

    pub fn build_prompt(collected_data: &[KeyValuePair]) -> String {
        let data_json =
            serde_json::to_string_pretty(collected_data).unwrap_or_else(|_| "[]".to_string());

        format!(
            "Build a knowledge graph from this collected interview data:\n\n```json\n{}\n```\n\nAnalyze the data and create appropriate entities and relationships.",
            data_json
        )
    }

    pub fn parse_response(response: &str) -> Option<ContextGraph> {
        // Find JSON block in response
        let json_start = response.find("```json")?;
        let json_content_start = json_start + 7;
        let json_end = response[json_content_start..].find("```")?;
        let json_str = &response[json_content_start..json_content_start + json_end].trim();

        serde_json::from_str(json_str).ok()
    }
}
