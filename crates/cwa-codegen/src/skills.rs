//! Generate Claude skill files from specs.
//!
//! Each approved Spec produces a skill definition with steps
//! and acceptance criteria.

use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use cwa_db::DbPool;

/// A generated skill definition.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedSkill {
    pub dirname: String,
    pub filename: String,
    pub content: String,
    pub spec_title: String,
}

/// Generate a skill from a spec.
pub fn generate_skill(db: &DbPool, spec_id: &str) -> Result<GeneratedSkill> {
    let spec = cwa_db::queries::specs::get_spec(db, spec_id)
        .map_err(|e| anyhow::anyhow!("Spec not found: {}", e))?;

    let slug = slugify(&spec.title);
    let dirname = slug.clone();
    let filename = "SKILL.md".to_string();

    let mut content = String::new();

    content.push_str(&format!("# {}\n\n", spec.title));

    if let Some(ref desc) = spec.description {
        content.push_str(&format!("{}\n\n", desc));
    }

    content.push_str(&format!("**Priority:** {}\n", spec.priority));
    content.push_str(&format!("**Status:** {}\n\n", spec.status));

    // Acceptance criteria
    if let Some(ref criteria_json) = spec.acceptance_criteria {
        if let Ok(criteria) = serde_json::from_str::<Vec<String>>(criteria_json) {
            content.push_str("## Acceptance Criteria\n\n");
            for (i, criterion) in criteria.iter().enumerate() {
                content.push_str(&format!("{}. {}\n", i + 1, criterion));
            }
            content.push('\n');
        }
    }

    // Dependencies
    if let Some(ref deps_json) = spec.dependencies {
        if let Ok(deps) = serde_json::from_str::<Vec<String>>(deps_json) {
            if !deps.is_empty() {
                content.push_str("## Dependencies\n\n");
                for dep in &deps {
                    content.push_str(&format!("- {}\n", dep));
                }
                content.push('\n');
            }
        }
    }

    // Implementation steps (generated from title + description)
    content.push_str("## Steps\n\n");
    content.push_str("1. Understand the requirements above\n");
    content.push_str("2. Review related code and dependencies\n");
    content.push_str("3. Implement the changes\n");
    content.push_str("4. Verify acceptance criteria are met\n");
    content.push_str("5. Update task status when complete\n");

    Ok(GeneratedSkill {
        dirname,
        filename,
        content,
        spec_title: spec.title,
    })
}

/// Generate skills for all approved/active specs in a project.
pub fn generate_all_skills(db: &DbPool, project_id: &str) -> Result<Vec<GeneratedSkill>> {
    let specs = cwa_db::queries::specs::list_specs(db, project_id)
        .map_err(|e| anyhow::anyhow!("Failed to list specs: {}", e))?;

    let mut skills = Vec::new();
    for spec in &specs {
        // Only generate skills for active/approved specs
        if spec.status == "active" || spec.status == "approved" {
            skills.push(generate_skill(db, &spec.id)?);
        }
    }

    Ok(skills)
}

/// Write generated skills to disk.
pub fn write_skills(skills: &[GeneratedSkill], output_dir: &Path) -> Result<Vec<String>> {
    let mut written = Vec::new();

    for skill in skills {
        let skill_dir = output_dir.join(&skill.dirname);
        std::fs::create_dir_all(&skill_dir)?;

        let path = skill_dir.join(&skill.filename);
        std::fs::write(&path, &skill.content)?;
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
