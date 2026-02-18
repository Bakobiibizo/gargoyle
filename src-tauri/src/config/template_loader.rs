use std::collections::HashMap;
use std::path::Path;

use crate::services::template_runner::{
    MaturityTier, Prerequisite, TemplateDefinition,
};

/// Configuration for generic template generators, parsed from front matter.
#[derive(Debug, Clone)]
pub struct GenericConfig {
    pub entity_type: String,
    pub default_status: String,
    pub title_prefix: String,
}

/// Extended template definition loaded from markdown front matter.
#[derive(Debug, Clone)]
pub struct LoadedTemplate {
    pub definition: TemplateDefinition,
    pub generic_config: Option<GenericConfig>,
    pub response_format: Option<String>,
}

/// Parse template front matter from a markdown file's content.
///
/// Front matter is everything before the first `---` separator line.
/// Each line is `Key: Value` format.
fn parse_front_matter(content: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            if !key.is_empty() && !value.is_empty() {
                headers.insert(key, value);
            }
        }
    }

    headers
}

fn parse_maturity(s: &str) -> MaturityTier {
    match s.to_lowercase().as_str() {
        "foundational" => MaturityTier::Foundational,
        "workflow" => MaturityTier::Workflow,
        "advanced" => MaturityTier::Advanced,
        "diagnostic" => MaturityTier::Diagnostic,
        _ => MaturityTier::Workflow,
    }
}

fn parse_prerequisites(headers: &HashMap<String, String>) -> Vec<Prerequisite> {
    let mut prereqs = Vec::new();

    if let Some(prereq_str) = headers.get("Prerequisite") {
        // Format: "entity_type >= count | suggested: template_key | reason text"
        for part_group in prereq_str.split("||") {
            let parts: Vec<&str> = part_group.split('|').map(|s| s.trim()).collect();
            if parts.is_empty() {
                continue;
            }

            // Parse "entity_type >= count"
            let first = parts[0];
            let (entity_type, min_count) = if let Some((et, count_str)) = first.split_once(">=") {
                let et = et.trim().to_string();
                let count = count_str.trim().parse::<usize>().unwrap_or(1);
                (et, count)
            } else {
                continue;
            };

            let mut suggested_template = None;
            let mut reason = String::new();

            for &part in &parts[1..] {
                if let Some(tmpl) = part.strip_prefix("suggested:") {
                    suggested_template = Some(tmpl.trim().to_string());
                } else if !part.is_empty() {
                    reason = part.to_string();
                }
            }

            prereqs.push(Prerequisite {
                entity_type,
                min_count,
                suggested_template,
                reason,
            });
        }
    }

    prereqs
}

fn parse_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Load a single template from a markdown file.
pub fn load_template(path: &Path) -> Option<LoadedTemplate> {
    let content = std::fs::read_to_string(path).ok()?;
    let headers = parse_front_matter(&content);

    // Template key: from header or filename stem
    let key = headers
        .get("Template Key")
        .cloned()
        .or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })?;

    let category = headers.get("Category").cloned().unwrap_or_else(|| "general".to_string());
    let version = headers.get("Version").cloned().unwrap_or_else(|| "1.0".to_string());
    let maturity = headers
        .get("Maturity")
        .map(|s| parse_maturity(s))
        .unwrap_or(MaturityTier::Workflow);

    let produced_entity_types = headers
        .get("Produces Entities")
        .map(|s| parse_list(s))
        .unwrap_or_default();
    let produced_relation_types = headers
        .get("Produces Relations")
        .map(|s| parse_list(s))
        .unwrap_or_default();

    let prerequisites = parse_prerequisites(&headers);

    let definition = TemplateDefinition {
        key: key.clone(),
        version,
        category,
        maturity_tier: maturity,
        prerequisites,
        produced_entity_types,
        produced_relation_types,
    };

    // Generic config
    let generic_config = if headers.get("Generator").map(|s| s.as_str()) == Some("generic") {
        Some(GenericConfig {
            entity_type: headers
                .get("Generic Entity Type")
                .cloned()
                .unwrap_or_else(|| "note".to_string()),
            default_status: headers
                .get("Generic Default Status")
                .cloned()
                .unwrap_or_else(|| "draft".to_string()),
            title_prefix: headers
                .get("Generic Title Prefix")
                .cloned()
                .unwrap_or_else(|| key.clone()),
        })
    } else {
        None
    };

    let response_format = headers.get("Response Format").cloned();

    Some(LoadedTemplate {
        definition,
        generic_config,
        response_format,
    })
}

/// Load all templates from a directory of markdown files.
pub fn load_templates(dir: &Path) -> HashMap<String, LoadedTemplate> {
    let mut templates = HashMap::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return templates,
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        if let Some(loaded) = load_template(&path) {
            templates.insert(loaded.definition.key.clone(), loaded);
        }
    }

    templates
}
