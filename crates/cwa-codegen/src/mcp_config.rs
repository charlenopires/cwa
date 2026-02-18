//! Generates `.mcp.json` configuration for Claude Code / Claude Desktop.
//!
//! The generated file points Claude Code at this project's CWA MCP server
//! running in stdio mode. Placing `.mcp.json` in the project root allows
//! Claude Code to auto-discover the server when opened in that directory.

use anyhow::Result;
use std::path::Path;

/// Generate the content of `.mcp.json` for the given project directory.
///
/// The file instructs Claude Code to launch `cwa mcp stdio` with the
/// project path, connecting it to this project's domain model, specs,
/// tasks, and memory graph.
pub fn generate_mcp_config(project_dir: &Path) -> Result<String> {
    let project_path = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf())
        .display()
        .to_string();

    // Build the JSON manually for deterministic output and no extra deps
    let json = format!(
        r#"{{
  "mcpServers": {{
    "cwa": {{
      "command": "cwa",
      "args": ["--project", "{project_path}", "mcp", "stdio"],
      "description": "CWA — Claude Workflow Architect (specs, tasks, domain model, memory)"
    }}
  }}
}}"#,
        project_path = project_path
    );

    Ok(json)
}

/// Write `.mcp.json` to the project directory root.
///
/// Returns the path of the written file.
pub fn write_mcp_config(project_dir: &Path) -> Result<String> {
    let content = generate_mcp_config(project_dir)?;
    let path = project_dir.join(".mcp.json");
    std::fs::write(&path, &content)?;
    Ok(path.display().to_string())
}
