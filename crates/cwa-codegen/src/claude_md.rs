//! Regenerate CLAUDE.md from current project state.
//!
//! Produces a comprehensive CLAUDE.md file containing project context,
//! domain model, active specs, and key decisions.

use anyhow::Result;
use std::path::Path;

use cwa_db::DbPool;

/// Generated CLAUDE.md content.
#[derive(Debug, Clone)]
pub struct GeneratedClaudeMd {
    pub content: String,
}

/// Project info structure for deserialization.
#[derive(Debug, Clone, serde::Deserialize)]
struct ProjectInfo {
    name: String,
    description: String,
    tech_stack: Vec<String>,
    main_features: Vec<String>,
    constraints: Vec<String>,
    #[allow(dead_code)]
    updated_at: String,
}

/// Generate CLAUDE.md content from the current project state.
pub fn generate_claude_md(db: &DbPool, project_id: &str) -> Result<GeneratedClaudeMd> {
    let project = cwa_db::queries::projects::get_project(db, project_id)
        .map_err(|e| anyhow::anyhow!("Project not found: {}", e))?;

    let mut content = String::new();

    // Try to get project info (extended metadata)
    let project_info = cwa_db::queries::projects::get_project_info(db, project_id)
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<ProjectInfo>(&json).ok());

    // Header with project info
    if let Some(ref info) = project_info {
        content.push_str(&format!("# {}\n\n", info.name));
        if !info.description.is_empty() {
            content.push_str(&format!("{}\n\n", info.description));
        }

        // Tech Stack
        if !info.tech_stack.is_empty() {
            content.push_str(&format!("**Tech Stack:** {}\n\n", info.tech_stack.join(", ")));
        }

        // Key Features
        if !info.main_features.is_empty() {
            content.push_str("## Key Features\n\n");
            for feature in &info.main_features {
                content.push_str(&format!("- {}\n", feature));
            }
            content.push('\n');
        }

        // Constraints
        if !info.constraints.is_empty() {
            content.push_str("## Constraints\n\n");
            for constraint in &info.constraints {
                content.push_str(&format!("- {}\n", constraint));
            }
            content.push('\n');
        }
    } else {
        // Fallback to basic project info
        content.push_str(&format!("# {}\n\n", project.name));
        if let Some(ref desc) = project.description {
            content.push_str(&format!("{}\n\n", desc));
        }
    }

    // Workflow Guidelines
    content.push_str("## Workflow Guidelines\n\n");
    content.push_str("**IMPORTANT:** Always update task status on the Kanban board as you work:\n\n");
    content.push_str("1. **Before starting work:** Move task to `in_progress`\n");
    content.push_str("   ```\n   cwa task move <task-id> in_progress\n   ```\n");
    content.push_str("   Or via MCP: `cwa_update_task_status(task_id, \"in_progress\")`\n\n");
    content.push_str("2. **When ready for review:** Move task to `review`\n");
    content.push_str("   ```\n   cwa task move <task-id> review\n   ```\n\n");
    content.push_str("3. **When complete:** Move task to `done`\n");
    content.push_str("   ```\n   cwa task move <task-id> done\n   ```\n\n");
    content.push_str("**Live Board:** Run `cwa serve` and open http://127.0.0.1:3030 to see real-time updates.\n\n");

    // Domain Model
    let contexts = cwa_db::queries::domains::list_contexts(db, project_id)
        .map_err(|e| anyhow::anyhow!("Failed to list contexts: {}", e))?;

    if !contexts.is_empty() {
        content.push_str("## Domain Model\n\n");
        for ctx in &contexts {
            content.push_str(&format!("### {}\n\n", ctx.name));
            if let Some(ref desc) = ctx.description {
                content.push_str(&format!("{}\n\n", desc));
            }

            let objects = cwa_db::queries::domains::list_domain_objects(db, &ctx.id)
                .unwrap_or_default();

            if !objects.is_empty() {
                content.push_str("**Entities:**\n");
                for obj in &objects {
                    let desc_suffix = obj.description.as_deref()
                        .map(|d| format!(" - {}", d))
                        .unwrap_or_default();
                    content.push_str(&format!("- `{}` ({}){}\n", obj.name, obj.object_type, desc_suffix));
                }
                content.push('\n');
            }
        }
    }

    // Active Specs
    let specs = cwa_db::queries::specs::list_specs(db, project_id)
        .unwrap_or_default();

    let active_specs: Vec<_> = specs.iter()
        .filter(|s| s.status == "active" || s.status == "approved")
        .collect();

    if !active_specs.is_empty() {
        content.push_str("## Active Specifications\n\n");
        for spec in &active_specs {
            content.push_str(&format!("### {} [{}]\n\n", spec.title, spec.priority));
            if let Some(ref desc) = spec.description {
                content.push_str(&format!("{}\n\n", desc));
            }
            if let Some(ref criteria_json) = spec.acceptance_criteria {
                if let Ok(criteria) = serde_json::from_str::<Vec<String>>(criteria_json) {
                    content.push_str("**Acceptance Criteria:**\n");
                    for c in &criteria {
                        content.push_str(&format!("- [ ] {}\n", c));
                    }
                    content.push('\n');
                }
            }
        }
    }

    // Key Decisions
    let decisions = cwa_db::queries::decisions::list_decisions(db, project_id)
        .unwrap_or_default();

    let accepted: Vec<_> = decisions.iter()
        .filter(|d| d.status == "accepted")
        .take(10)
        .collect();

    if !accepted.is_empty() {
        content.push_str("## Key Decisions\n\n");
        for d in &accepted {
            content.push_str(&format!("- **{}**: {}\n", d.title, d.decision));
        }
        content.push('\n');
    }

    // Glossary
    let terms = cwa_db::queries::domains::list_glossary(db, project_id)
        .unwrap_or_default();

    if !terms.is_empty() {
        content.push_str("## Glossary\n\n");
        content.push_str("| Term | Definition |\n");
        content.push_str("|------|------------|\n");
        for term in &terms {
            content.push_str(&format!("| {} | {} |\n", term.term, term.definition));
        }
        content.push('\n');
    }

    // Current Tasks
    let tasks = cwa_db::queries::tasks::list_tasks(db, project_id)
        .unwrap_or_default();

    let in_progress: Vec<_> = tasks.iter()
        .filter(|t| t.status == "in_progress")
        .collect();

    if !in_progress.is_empty() {
        content.push_str("## Current Work\n\n");
        for task in &in_progress {
            content.push_str(&format!("- {} [{}]\n", task.title, task.priority));
        }
        content.push('\n');
    }

    // Design System
    if let Ok(Some(_)) = cwa_db::queries::design_systems::get_latest_design_system(db, project_id) {
        content.push_str("## Design System\n\n");
        content.push_str("Design tokens reference: `.claude/design-system.md`\n\n");
        content.push_str("All UI implementation must follow the design system tokens defined above.\n\n");
    }

    // Recent Observations (high-confidence, top 10)
    let high_conf_observations = cwa_db::queries::observations::list_high_confidence(db, project_id, 0.7, 10)
        .unwrap_or_default();

    if !high_conf_observations.is_empty() {
        content.push_str("## Recent Observations\n\n");
        for obs in &high_conf_observations {
            let narrative_suffix = obs.narrative.as_deref()
                .map(|n| format!(" -- {}", n))
                .unwrap_or_default();
            content.push_str(&format!(
                "- **[{}]** {}{}\n",
                obs.obs_type.to_uppercase(),
                obs.title,
                narrative_suffix
            ));
        }
        content.push('\n');
    }

    // Last Session Summary
    let summaries = cwa_db::queries::observations::get_recent_summaries(db, project_id, 1)
        .unwrap_or_default();

    if let Some(summary) = summaries.first() {
        content.push_str("## Last Session Summary\n\n");
        content.push_str(&summary.content);
        content.push_str("\n\n");
    }

    Ok(GeneratedClaudeMd { content })
}

/// Write the generated CLAUDE.md to disk.
pub fn write_claude_md(generated: &GeneratedClaudeMd, project_dir: &Path) -> Result<String> {
    let path = project_dir.join("CLAUDE.md");
    std::fs::write(&path, &generated.content)?;
    Ok(path.display().to_string())
}
