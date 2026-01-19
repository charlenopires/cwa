//! MCP server commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;

#[derive(Subcommand)]
pub enum McpCommands {
    /// Run MCP server over stdio
    Stdio,

    /// Show MCP server status
    Status,
}

pub async fn execute(cmd: McpCommands, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");

    match cmd {
        McpCommands::Stdio => {
            let pool = Arc::new(cwa_db::init_pool(&db_path)?);
            cwa_mcp::run_stdio_server(pool).await?;
        }

        McpCommands::Status => {
            println!("{} MCP Server Configuration", "ℹ".blue().bold());
            println!();
            println!("  To use with Claude Code, add to .mcp.json:");
            println!();
            println!(
                r#"  {{
    "mcpServers": {{
      "cwa": {{
        "command": "cwa",
        "args": ["mcp", "stdio"]
      }}
    }}
  }}"#
            );
        }
    }

    Ok(())
}
