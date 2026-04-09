use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntakeContext {
    pub name: Option<String>,
    pub use_case: Option<String>,
    pub collected_data: Vec<KeyValuePair>,
    pub conversation_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: String,
    pub value: serde_json::Value,
    pub category: Option<String>,
    pub confidence: f32,
}

impl Default for IntakeContext {
    fn default() -> Self {
        Self {
            name: None,
            use_case: None,
            collected_data: Vec::new(),
            conversation_complete: false,
        }
    }
}

pub const INTAKE_SYSTEM_PROMPT: &str = r#"You are Gargoyle's intake specialist conducting a friendly onboarding interview.

## Your Goal
Collect information about the user through natural conversation. Extract key-value pairs from their responses.

## Interview Flow
1. Greet and ask their name
2. Ask what they'd like to use Gargoyle for
3. Based on their use case, ask follow-up questions to understand:
   - Their role/responsibilities
   - Their company/team context
   - Current priorities or projects
   - Any specific challenges they face

## Rules
- Ask ONE question at a time
- Be conversational, not interrogative
- Extract structured data from responses as key-value pairs
- Mark confidence level for each extracted value (0.0-1.0)
- When you have enough context (or user wants to wrap up), set conversation_complete: true

## Output Format
After each user message, respond with:
1. Your conversational reply to the user
2. A JSON block with extracted data:

```json
{
  "extracted": [
    {"key": "user_name", "value": "...", "category": "user", "confidence": 0.95},
    {"key": "company_name", "value": "...", "category": "company", "confidence": 0.8}
  ],
  "conversation_complete": false
}
```

Categories: user, company, team, project, workflow, challenge, goal
"#;

pub struct IntakeAgent;

impl IntakeAgent {
    pub fn system_prompt() -> &'static str {
        INTAKE_SYSTEM_PROMPT
    }

    pub fn parse_response(response: &str) -> Option<IntakeExtraction> {
        // Find JSON block in response
        let json_start = response.find("```json")?;
        let json_content_start = json_start + 7;
        let json_end = response[json_content_start..].find("```")?;
        let json_str = &response[json_content_start..json_content_start + json_end].trim();
        
        serde_json::from_str(json_str).ok()
    }

    pub fn get_conversational_reply(response: &str) -> String {
        // Extract text before JSON block
        if let Some(json_start) = response.find("```json") {
            response[..json_start].trim().to_string()
        } else {
            response.to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntakeExtraction {
    pub extracted: Vec<KeyValuePair>,
    pub conversation_complete: bool,
}
