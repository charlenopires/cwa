//! Generate Claude Code hooks configuration.
//!
//! Produces a `.claude/hooks.json` file in the correct Claude Code object format
//! covering all hook events: PreToolUse, PostToolUse, UserPromptSubmit, Stop.
//! Tech-stack-specific hooks (cargo fmt, prettier, black) are added conditionally.

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

/// Generate hooks configuration.
///
/// Always generates the full set of standard hooks plus optional tech-stack-specific
/// hooks. Domain invariants are included as additional PreToolUse checks.
pub async fn generate_hooks(db: &DbPool, project_id: &str, tech_stack: &[String]) -> Result<GeneratedHooks> {
    let stack_lower: Vec<String> = tech_stack.iter().map(|s| s.to_lowercase()).collect();

    // ─── PreToolUse ──────────────────────────────────────────────────────────
    let mut pre_tool_use: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "matcher": "Bash",
            "hooks": [{
                "type": "command",
                "command": "echo \"$CLAUDE_TOOL_INPUT\" | grep -qE '(rm -rf /|DROP TABLE|git push.*--force.*main)' && exit 2 || exit 0"
            }]
        }),
    ];

    // Add domain-invariant hooks from the database
    let contexts = cwa_db::queries::domains::list_contexts(db, project_id).await
        .unwrap_or_default();

    for ctx in &contexts {
        let objects = cwa_db::queries::domains::list_domain_objects_by_context(db, project_id, &ctx.id).await
            .unwrap_or_default();

        for obj in &objects {
            if let Some(ref invariants_json) = obj.invariants {
                if let Ok(invariants) = serde_json::from_str::<Vec<String>>(invariants_json) {
                    for invariant in &invariants {
                        pre_tool_use.push(serde_json::json!({
                            "matcher": "Bash",
                            "hooks": [{
                                "type": "command",
                                "command": format!(
                                    "echo 'Domain invariant check [{} - {}]: {}'",
                                    ctx.name, obj.name, invariant
                                )
                            }]
                        }));
                    }
                }
            }
        }
    }

    // ─── PostToolUse ─────────────────────────────────────────────────────────
    let mut post_tool_use: Vec<serde_json::Value> = vec![
        // Capture file creation
        serde_json::json!({
            "matcher": "Write",
            "hooks": [{
                "type": "command",
                "command": "cwa memory observe \"File created: $CLAUDE_TOOL_INPUT_FILE_PATH\" --obs-type change --files-modified \"$CLAUDE_TOOL_INPUT_FILE_PATH\" 2>/dev/null || true"
            }]
        }),
        // Capture file edits
        serde_json::json!({
            "matcher": "Edit|MultiEdit",
            "hooks": [{
                "type": "command",
                "command": "cwa memory observe \"File modified: $CLAUDE_TOOL_INPUT_FILE_PATH\" --obs-type change --files-modified \"$CLAUDE_TOOL_INPUT_FILE_PATH\" 2>/dev/null || true"
            }]
        }),
    ];

    // Tech-stack-specific PostToolUse hooks
    if stack_lower.contains(&"rust".to_string()) {
        post_tool_use.push(serde_json::json!({
            "matcher": "Edit",
            "hooks": [{
                "type": "command",
                "command": "case \"$CLAUDE_TOOL_INPUT_FILE_PATH\" in *.rs) cargo fmt -- \"$CLAUDE_TOOL_INPUT_FILE_PATH\" 2>/dev/null || true ;; esac"
            }]
        }));
    }

    if stack_lower.iter().any(|t| t == "typescript" || t == "react" || t == "nextjs" || t == "next.js") {
        post_tool_use.push(serde_json::json!({
            "matcher": "Edit",
            "hooks": [{
                "type": "command",
                "command": "case \"$CLAUDE_TOOL_INPUT_FILE_PATH\" in *.ts|*.tsx|*.js|*.jsx) prettier --write \"$CLAUDE_TOOL_INPUT_FILE_PATH\" 2>/dev/null || true ;; esac"
            }]
        }));
    }

    if stack_lower.contains(&"python".to_string()) {
        post_tool_use.push(serde_json::json!({
            "matcher": "Edit",
            "hooks": [{
                "type": "command",
                "command": "case \"$CLAUDE_TOOL_INPUT_FILE_PATH\" in *.py) black \"$CLAUDE_TOOL_INPUT_FILE_PATH\" 2>/dev/null || true ;; esac"
            }]
        }));
    }

    // ─── UserPromptSubmit ────────────────────────────────────────────────────
    let user_prompt_submit: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "matcher": "",
            "hooks": [{
                "type": "command",
                "command": "cwa context status 2>/dev/null || true"
            }]
        }),
    ];

    // ─── Stop ────────────────────────────────────────────────────────────────
    let stop: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "matcher": "",
            "hooks": [{
                "type": "command",
                "command": "cwa task list --status in_progress 2>/dev/null || true"
            }]
        }),
    ];

    // Count total hooks
    let hook_count = pre_tool_use.len() + post_tool_use.len() + user_prompt_submit.len() + stop.len();

    let config = serde_json::json!({
        "hooks": {
            "PreToolUse": pre_tool_use,
            "PostToolUse": post_tool_use,
            "UserPromptSubmit": user_prompt_submit,
            "Stop": stop
        }
    });

    let content = serde_json::to_string_pretty(&config)?;

    Ok(GeneratedHooks {
        content,
        hook_count,
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
