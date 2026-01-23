//! Token counting and analysis.
//!
//! Uses tiktoken-rs with cl100k_base encoding (Claude-compatible)
//! to count tokens in files and text content.

use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

/// Token count for a single file or content source.
#[derive(Debug, Clone, Serialize)]
pub struct TokenCount {
    pub source: String,
    pub tokens: usize,
    pub characters: usize,
    pub lines: usize,
}

/// Analyze a file and count its tokens.
pub fn analyze_file(path: &Path) -> Result<TokenCount> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read: {}", path.display()))?;

    let tokens = count_tokens(&content)?;
    let characters = content.len();
    let lines = content.lines().count();

    Ok(TokenCount {
        source: path.display().to_string(),
        tokens,
        characters,
        lines,
    })
}

/// Analyze a string and count its tokens.
pub fn analyze_text(source: &str, content: &str) -> Result<TokenCount> {
    let tokens = count_tokens(content)?;
    let characters = content.len();
    let lines = content.lines().count();

    Ok(TokenCount {
        source: source.to_string(),
        tokens,
        characters,
        lines,
    })
}

/// Count tokens using cl100k_base encoding.
pub fn count_tokens(text: &str) -> Result<usize> {
    let bpe = tiktoken_rs::cl100k_base()
        .context("Failed to load cl100k_base tokenizer")?;

    let tokens = bpe.encode_with_special_tokens(text);
    Ok(tokens.len())
}

/// Analyze all context files in a project directory.
pub fn analyze_project(project_dir: &Path) -> Result<Vec<TokenCount>> {
    let mut results = Vec::new();

    // CLAUDE.md (always loaded)
    let claude_md = project_dir.join("CLAUDE.md");
    if claude_md.exists() {
        results.push(analyze_file(&claude_md)?);
    }

    // .claude/agents/*.md
    let agents_dir = project_dir.join(".claude/agents");
    if agents_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "md") {
                    results.push(analyze_file(&path)?);
                }
            }
        }
    }

    // .claude/skills/*/SKILL.md
    let skills_dir = project_dir.join(".claude/skills");
    if skills_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skill_md = path.join("SKILL.md");
                    if skill_md.exists() {
                        results.push(analyze_file(&skill_md)?);
                    }
                }
            }
        }
    }

    // .claude/commands/*.md
    let commands_dir = project_dir.join(".claude/commands");
    if commands_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&commands_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "md") {
                    results.push(analyze_file(&path)?);
                }
            }
        }
    }

    // .claude/rules/*.md
    let rules_dir = project_dir.join(".claude/rules");
    if rules_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&rules_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "md") {
                    results.push(analyze_file(&path)?);
                }
            }
        }
    }

    Ok(results)
}
