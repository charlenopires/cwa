//! Generate Claude subagent files from bounded contexts.
//!
//! Each BoundedContext produces an agent markdown file that defines
//! the agent's role, domain knowledge, and available entities.

use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use cwa_db::DbPool;

/// A generated agent definition.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedAgent {
    pub filename: String,
    pub content: String,
    pub context_name: String,
}

/// Generate an agent from a bounded context.
pub async fn generate_agent(db: &DbPool, context_id: &str) -> Result<GeneratedAgent> {
    let ctx = cwa_db::queries::domains::get_context(db, context_id).await
        .map_err(|e| anyhow::anyhow!("Context not found: {}", e))?;

    let objects = cwa_db::queries::domains::list_domain_objects(db, context_id).await
        .map_err(|e| anyhow::anyhow!("Failed to list domain objects: {}", e))?;

    let terms = cwa_db::queries::domains::list_glossary(db, &ctx.project_id).await
        .map_err(|e| anyhow::anyhow!("Failed to list glossary: {}", e))?;

    // Filter terms for this context
    let context_terms: Vec<_> = terms.iter()
        .filter(|t| t.context_id.as_deref() == Some(context_id))
        .collect();

    let slug = slugify(&ctx.name);
    let filename = format!("{}-expert.md", slug);

    let mut content = String::new();

    content.push_str(&format!("# {} Expert Agent\n\n", ctx.name));
    content.push_str(&format!("## Role\n\n"));
    content.push_str(&format!(
        "You are an expert in the **{}** bounded context.\n",
        ctx.name
    ));

    if let Some(ref desc) = ctx.description {
        content.push_str(&format!("{}\n", desc));
    }
    content.push('\n');

    // Responsibilities
    if let Some(ref responsibilities) = ctx.responsibilities {
        content.push_str("## Responsibilities\n\n");
        // Parse as JSON array if possible, otherwise treat as plain text
        if let Ok(items) = serde_json::from_str::<Vec<String>>(responsibilities) {
            for item in &items {
                content.push_str(&format!("- {}\n", item));
            }
        } else {
            content.push_str(&format!("{}\n", responsibilities));
        }
        content.push('\n');
    }

    // Domain entities
    if !objects.is_empty() {
        content.push_str("## Domain Entities\n\n");
        for obj in &objects {
            content.push_str(&format!("### {} ({})\n\n", obj.name, obj.object_type));
            if let Some(ref desc) = obj.description {
                content.push_str(&format!("{}\n\n", desc));
            }
            if let Some(ref props) = obj.properties {
                if let Ok(items) = serde_json::from_str::<Vec<String>>(props) {
                    content.push_str("**Properties:**\n");
                    for item in &items {
                        content.push_str(&format!("- {}\n", item));
                    }
                    content.push('\n');
                }
            }
            if let Some(ref invariants) = obj.invariants {
                if let Ok(items) = serde_json::from_str::<Vec<String>>(invariants) {
                    content.push_str("**Invariants:**\n");
                    for item in &items {
                        content.push_str(&format!("- {}\n", item));
                    }
                    content.push('\n');
                }
            }
        }
    }

    // Ubiquitous language
    if !context_terms.is_empty() {
        content.push_str("## Ubiquitous Language\n\n");
        content.push_str("| Term | Definition |\n");
        content.push_str("|------|------------|\n");
        for term in &context_terms {
            content.push_str(&format!("| {} | {} |\n", term.term, term.definition));
        }
        content.push('\n');
    }

    // Context boundaries
    if ctx.upstream_contexts.is_some() || ctx.downstream_contexts.is_some() {
        content.push_str("## Context Boundaries\n\n");
        if let Some(ref upstream) = ctx.upstream_contexts {
            if let Ok(items) = serde_json::from_str::<Vec<String>>(upstream) {
                content.push_str("**Depends on:** ");
                content.push_str(&items.join(", "));
                content.push_str("\n\n");
            }
        }
        if let Some(ref downstream) = ctx.downstream_contexts {
            if let Ok(items) = serde_json::from_str::<Vec<String>>(downstream) {
                content.push_str("**Consumed by:** ");
                content.push_str(&items.join(", "));
                content.push_str("\n\n");
            }
        }
    }

    Ok(GeneratedAgent {
        filename,
        content,
        context_name: ctx.name,
    })
}

/// Generate agents for all bounded contexts in a project.
pub async fn generate_all_agents(db: &DbPool, project_id: &str) -> Result<Vec<GeneratedAgent>> {
    let contexts = cwa_db::queries::domains::list_contexts(db, project_id).await
        .map_err(|e| anyhow::anyhow!("Failed to list contexts: {}", e))?;

    let mut agents = Vec::new();
    for ctx in &contexts {
        agents.push(generate_agent(db, &ctx.id).await?);
    }

    Ok(agents)
}

/// Write generated agents to disk.
pub fn write_agents(agents: &[GeneratedAgent], output_dir: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(output_dir)?;
    let mut written = Vec::new();

    for agent in agents {
        let path = output_dir.join(&agent.filename);
        std::fs::write(&path, &agent.content)?;
        written.push(path.display().to_string());
    }

    Ok(written)
}

/// Convert a name to a URL-safe slug.
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
