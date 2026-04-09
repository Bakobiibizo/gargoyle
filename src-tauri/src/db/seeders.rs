use rusqlite::Connection;

use crate::error::Result;
use crate::models::template::CreateTemplatePayload;
use crate::services::template_service::TemplateService;

pub fn seed_templates(conn: &Connection) -> Result<()> {
    // Check if initialize template already exists
    if TemplateService::get_by_key(conn, "initialize").is_ok() {
        return Ok(());
    }

    // Seed the initialize template
    let initialize_content = include_str!("../../../templates/initialize.md");

    // Extract content after the front matter separator
    let content = if let Some(idx) = initialize_content.find("\n---\n") {
        &initialize_content[idx + 5..]
    } else {
        initialize_content
    };

    TemplateService::create(conn, CreateTemplatePayload {
        key: "initialize".to_string(),
        category: "bootstrap".to_string(),
        description: Some("User intake interview - collects context about the user, company, and operational environment".to_string()),
        content: content.to_string(),
        response_format: Some("structured".to_string()),
        produces_entities: None,
        produces_relations: None,
        generator_type: None,
        generator_config: None,
        created_by: Some("system".to_string()),
    })?;

    Ok(())
}
