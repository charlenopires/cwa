//! Generate Claude hooks from domain rules.
//!
//! Produces a settings.json fragment with hook configurations
//! based on domain invariants and validation rules.

use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use cwa_db::DbPool;

/// A generated hook configuration.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedHooks {
    pub content: String,
    pub hook_count: usize,
}

/// Generate hooks configuration from domain invariants.
pub async fn generate_hooks(db: &DbPool, project_id: &str) -> Result<GeneratedHooks> {
    let contexts = cwa_db::queries::domains::list_contexts(db, project_id).await
        .map_err(|e| anyhow::anyhow!("Failed to list contexts: {}", e))?;

    let mut hooks = Vec::new();

    for ctx in &contexts {
        let objects = cwa_db::queries::domains::list_domain_objects(db, &ctx.id).await
            .map_err(|e| anyhow::anyhow!("Failed to list domain objects: {}", e))?;

        for obj in &objects {
            if let Some(ref invariants_json) = obj.invariants {
                if let Ok(invariants) = serde_json::from_str::<Vec<String>>(invariants_json) {
                    for invariant in &invariants {
                        hooks.push(serde_json::json!({
                            "event": "pre-commit",
                            "command": format!("echo 'Verify: {} - {}'", obj.name, invariant),
                            "description": format!("[{}] {}: {}", ctx.name, obj.name, invariant)
                        }));
                    }
                }
            }
        }
    }

    // Auto-capture hooks for observations
    hooks.push(serde_json::json!({
        "event": "PostToolUse",
        "matcher": { "tool_name": "write_to_file" },
        "command": "cwa memory observe \"File created: $TOOL_INPUT_PATH\" --obs-type change --files-modified \"$TOOL_INPUT_PATH\"",
        "timeout": 10000
    }));

    hooks.push(serde_json::json!({
        "event": "PostToolUse",
        "matcher": { "tool_name": "edit_file" },
        "command": "cwa memory observe \"File modified: $TOOL_INPUT_PATH\" --obs-type change --files-modified \"$TOOL_INPUT_PATH\"",
        "timeout": 10000
    }));

    let config = serde_json::json!({
        "hooks": hooks
    });

    let content = serde_json::to_string_pretty(&config)?;

    Ok(GeneratedHooks {
        content,
        hook_count: hooks.len(),
    })
}

/// Write hooks configuration to disk.
pub fn write_hooks(hooks: &GeneratedHooks, project_dir: &Path) -> Result<String> {
    let settings_dir = project_dir.join(".claude");
    std::fs::create_dir_all(&settings_dir)?;

    let path = settings_dir.join("hooks.json");
    std::fs::write(&path, &hooks.content)?;

    Ok(path.display().to_string())
}
