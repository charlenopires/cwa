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

    /// Run MCP planner server for Claude Desktop
    Planner,

    /// Show MCP server status
    Status,
}

pub async fn execute(cmd: McpCommands, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");

    match cmd {
        McpCommands::Stdio => {
            eprintln!(
                "  {} {} {}",
                "●".green().bold(),
                "CWA MCP".cyan().bold(),
                "server running (stdio)".bold()
            );
            eprintln!("  {} 24 tools | 5 resources", "▸".dimmed());
            eprintln!("  {} Ctrl+C to stop", "▸".dimmed());
            eprintln!();

            let pool = Arc::new(cwa_db::init_pool(&db_path)?);
            cwa_mcp::run_stdio_server(pool).await?;
        }

        McpCommands::Planner => {
            eprintln!(
                "  {} {} {}",
                "●".cyan().bold(),
                "CWA Planner".cyan().bold(),
                "server running (stdio)".bold()
            );
            eprintln!("  {} 1 tool: cwa_plan_software", "▸".dimmed());
            eprintln!("  {} Ctrl+C to stop", "▸".dimmed());
            eprintln!();

            cwa_mcp::run_planner_stdio().await?;
        }

        McpCommands::Status => {
            println!("{} MCP Server Configuration", "ℹ".blue().bold());
            println!();
            println!("  {} Claude Code (.mcp.json):", "▸".dimmed());
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
            println!();
            println!("  {} Claude Desktop (claude_desktop_config.json):", "▸".dimmed());
            println!();
            println!(
                r#"  {{
    "mcpServers": {{
      "cwa-planner": {{
        "command": "cwa",
        "args": ["mcp", "planner"]
      }}
    }}
  }}"#
            );
        }
    }

    Ok(())
}
