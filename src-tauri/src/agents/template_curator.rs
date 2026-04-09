use rusqlite::Connection;

use crate::error::Result;
use crate::services::template_service::TemplateService;

use super::types::{TemplateCuratorRequest, TemplateCuratorResponse};

pub const TEMPLATE_COMPOSER_SYSTEM_PROMPT: &str = r##"You are Gargoyle's Template Composer. Your job is to create new prompt templates based on user requirements.

## Template Structure
Templates have:
- **key**: Unique identifier (kebab-case, e.g., "weekly-review")
- **category**: One of: bootstrap, workflow, analysis, content, org, dev, mkt
- **description**: Short summary for search/discovery
- **content**: The actual prompt/instructions (markdown)
- **produces_entities**: Entity types this template creates (doc, note, idea, task)
- **response_format**: "structured", "freeform", or "conversational"

## Output Format
Return a JSON object:
```json
{
  "key": "template-key",
  "category": "workflow",
  "description": "Brief description for discovery",
  "content": "# Template Title (use literal newlines, not escape sequences)",
  "response_format": "structured",
  "produces_entities": ["doc", "task"]
}
```

## Guidelines
- Keep templates focused on one goal
- Include clear instructions for the LLM
- Specify output format expectations
- Reference entity types correctly (only: doc, note, idea, task)
- Make templates reusable across contexts
"##;

pub struct TemplateCuratorAgent;

impl TemplateCuratorAgent {
    pub fn handle(
        conn: &Connection,
        request: TemplateCuratorRequest,
    ) -> Result<TemplateCuratorResponse> {
        match request {
            TemplateCuratorRequest::Search { query, limit } => {
                let results = TemplateService::search(conn, &query, limit.unwrap_or(10))?;
                Ok(TemplateCuratorResponse::TemplateList { templates: results })
            }

            TemplateCuratorRequest::ListByCategory { category } => {
                let templates = TemplateService::list(conn, Some(&category))?;
                Ok(TemplateCuratorResponse::TemplateList { templates })
            }

            TemplateCuratorRequest::GetSummaries { keys } => {
                let mut summaries = Vec::new();
                for key in keys {
                    if let Ok(template) = TemplateService::get_by_key(conn, &key) {
                        summaries.push(crate::models::template::TemplateIndex {
                            key: template.key,
                            category: template.category,
                            description: template.description,
                            produces_entities: template.produces_entities,
                            usage_count: template.usage_count,
                        });
                    }
                }
                Ok(TemplateCuratorResponse::TemplateList {
                    templates: summaries,
                })
            }

            TemplateCuratorRequest::Create { payload } => {
                let template = TemplateService::create(conn, payload)?;
                Ok(TemplateCuratorResponse::Created { key: template.key })
            }

            TemplateCuratorRequest::Update { key: _, payload } => {
                TemplateService::update(conn, payload)?;
                Ok(TemplateCuratorResponse::Updated)
            }

            TemplateCuratorRequest::Delete { key } => {
                TemplateService::delete(conn, &key)?;
                Ok(TemplateCuratorResponse::Deleted)
            }

            TemplateCuratorRequest::Get { key } => {
                let template = TemplateService::get_by_key(conn, &key)?;
                Ok(TemplateCuratorResponse::Template { template })
            }

            TemplateCuratorRequest::List { limit: _ } => {
                let templates = TemplateService::list(conn, None)?;
                Ok(TemplateCuratorResponse::TemplateList { templates })
            }

            TemplateCuratorRequest::ComposeTemplate {
                description,
                produces_entities,
                similar_to,
            } => {
                let mut user_prompt = format!(
                    "Create a template with the following requirements:\n\n**Description**: {}\n**Should produce**: {:?}",
                    description,
                    produces_entities
                );

                if let Some(ref similar_key) = similar_to {
                    if let Ok(similar) = TemplateService::get_by_key(conn, similar_key) {
                        user_prompt.push_str(&format!(
                            "\n\n**Similar to existing template**:\n```\n{}\n```",
                            similar.content
                        ));
                    }
                }

                Ok(TemplateCuratorResponse::ComposePrompt {
                    system_prompt: TEMPLATE_COMPOSER_SYSTEM_PROMPT.to_string(),
                    user_prompt,
                })
            }

            TemplateCuratorRequest::GetRelevantContext {
                user_query,
                max_tokens,
            } => {
                let results = TemplateService::search(conn, &user_query, 5)?;

                if results.is_empty() {
                    return Ok(TemplateCuratorResponse::Context {
                        context: "No relevant templates found.".to_string(),
                    });
                }

                let mut context = format!("## Relevant Templates ({} found)\n\n", results.len());
                let mut tokens_used = context.len() / 4; // rough estimate

                for (i, template) in results.iter().enumerate() {
                    let entry = format!(
                        "{}. **{}** ({}) - {}\n   Produces: {:?}\n\n",
                        i + 1,
                        template.key,
                        template.category,
                        template.description.as_deref().unwrap_or("No description"),
                        template.produces_entities
                    );

                    if tokens_used + entry.len() / 4 > max_tokens {
                        break;
                    }

                    context.push_str(&entry);
                    tokens_used += entry.len() / 4;
                }

                context.push_str("Use `TemplateCurator.Get { key }` for full content.");

                Ok(TemplateCuratorResponse::Context { context })
            }
        }
    }
}
