//! MCP server commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;

/// Number of tools and resources available in the MCP server.
const MCP_TOOLS_COUNT: usize = 34;
const MCP_RESOURCES_COUNT: usize = 11;

#[derive(Subcommand)]
pub enum McpCommands {
    /// Run MCP server over stdio
    Stdio,

    /// Run MCP planner server for Claude Desktop
    Planner,

    /// Show MCP server status and available tools/resources
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
            eprintln!("  {} {} tools | {} resources", "▸".dimmed(), MCP_TOOLS_COUNT, MCP_RESOURCES_COUNT);
            eprintln!("  {} Use 'cwa serve' for live WebSocket updates", "▸".dimmed());
            eprintln!("  {} Ctrl+C to stop", "▸".dimmed());
            eprintln!();

            let pool = Arc::new(cwa_db::init_pool(&db_path)?);
            // Running standalone - no broadcast channel (uses HTTP fallback)
            cwa_mcp::run_stdio_server(pool, None).await?;
        }

        McpCommands::Planner => {
            eprintln!(
                "  {} {} {}",
                "●".cyan().bold(),
                "CWA Planner".cyan().bold(),
                "server running (stdio)".bold()
            );
            eprintln!("  {} 1 tool: cwa_plan_software", "▸".dimmed());
            eprintln!("  {} Generates 8-phase bootstrap plans for new projects", "▸".dimmed());
            eprintln!("  {} Ctrl+C to stop", "▸".dimmed());
            eprintln!();
            eprintln!("  {} Use 'cwa mcp status' to see all {} tools from the main server", "💡".yellow(), MCP_TOOLS_COUNT);
            eprintln!();

            cwa_mcp::run_planner_stdio().await?;
        }

        McpCommands::Status => {
            print_mcp_status();
        }
    }

    Ok(())
}

fn print_mcp_status() {
    println!();
    println!("{} CWA MCP Server Status", "●".green().bold());
    println!();
    println!("  {} {} tools | {} resources", "▸".dimmed(), MCP_TOOLS_COUNT.to_string().cyan().bold(), MCP_RESOURCES_COUNT.to_string().cyan().bold());
    println!();

    // Tools by category
    println!("{}", "  Tools".bold().underline());
    println!();

    println!("  {} {}", "Project & Context".yellow(), "(4)".dimmed());
    println!("    {} {}", "cwa_get_project_info".cyan(), "Get project metadata".dimmed());
    println!("    {} {}", "cwa_get_context_summary".cyan(), "Compact context summary".dimmed());
    println!("    {} {}", "cwa_get_domain_model".cyan(), "DDD bounded contexts".dimmed());
    println!("    {} {}", "cwa_get_context_map".cyan(), "Context relationships".dimmed());
    println!();

    println!("  {} {}", "Specifications".yellow(), "(6)".dimmed());
    println!("    {} {}", "cwa_get_spec".cyan(), "Get by ID/title".dimmed());
    println!("    {} {}", "cwa_list_specs".cyan(), "List all (filterable)".dimmed());
    println!("    {} {}", "cwa_create_spec".cyan(), "Create new".dimmed());
    println!("    {} {}", "cwa_update_spec_status".cyan(), "Update status".dimmed());
    println!("    {} {}", "cwa_add_acceptance_criteria".cyan(), "Add criteria".dimmed());
    println!("    {} {}", "cwa_validate_spec".cyan(), "Validate completeness".dimmed());
    println!();

    println!("  {} {}", "Tasks & Kanban".yellow(), "(7)".dimmed());
    println!("    {} {}", "cwa_get_current_task".cyan(), "Current in-progress".dimmed());
    println!("    {} {}", "cwa_list_tasks".cyan(), "List all (filterable)".dimmed());
    println!("    {} {}", "cwa_create_task".cyan(), "Create new".dimmed());
    println!("    {} {}", "cwa_update_task_status".cyan(), "Move between statuses".dimmed());
    println!("    {} {}", "cwa_generate_tasks".cyan(), "Generate from spec".dimmed());
    println!("    {} {}", "cwa_get_wip_status".cyan(), "WIP limits status".dimmed());
    println!("    {} {}", "cwa_set_wip_limit".cyan(), "Set column limit".dimmed());
    println!();

    println!("  {} {}", "Memory & Observations".yellow(), "(8)".dimmed());
    println!("    {} {}", "cwa_search_memory".cyan(), "Text search".dimmed());
    println!("    {} {}", "cwa_memory_semantic_search".cyan(), "Vector search (Qdrant)".dimmed());
    println!("    {} {}", "cwa_memory_search_all".cyan(), "Unified search".dimmed());
    println!("    {} {}", "cwa_memory_add".cyan(), "Store with embedding".dimmed());
    println!("    {} {}", "cwa_observe".cyan(), "Record observation".dimmed());
    println!("    {} {}", "cwa_memory_timeline".cyan(), "Recent timeline".dimmed());
    println!("    {} {}", "cwa_memory_get".cyan(), "Get by ID".dimmed());
    println!("    {} {}", "cwa_get_next_steps".cyan(), "Suggested next steps".dimmed());
    println!();

    println!("  {} {}", "Domain Modeling (DDD)".yellow(), "(4)".dimmed());
    println!("    {} {}", "cwa_create_context".cyan(), "Create bounded context".dimmed());
    println!("    {} {}", "cwa_create_domain_object".cyan(), "Create domain object".dimmed());
    println!("    {} {}", "cwa_get_glossary".cyan(), "Get glossary".dimmed());
    println!("    {} {}", "cwa_add_glossary_term".cyan(), "Add term".dimmed());
    println!();

    println!("  {} {}", "Decisions (ADRs)".yellow(), "(2)".dimmed());
    println!("    {} {}", "cwa_add_decision".cyan(), "Register ADR".dimmed());
    println!("    {} {}", "cwa_list_decisions".cyan(), "List all".dimmed());
    println!();

    println!("  {} {}", "Knowledge Graph (Neo4j)".yellow(), "(3)".dimmed());
    println!("    {} {}", "cwa_graph_query".cyan(), "Execute Cypher".dimmed());
    println!("    {} {}", "cwa_graph_impact".cyan(), "Impact analysis".dimmed());
    println!("    {} {}", "cwa_graph_sync".cyan(), "Sync to Neo4j".dimmed());
    println!();

    // Resources
    println!("{}", "  Resources".bold().underline());
    println!();
    println!("    {} Project metadata", "project://info".green());
    println!("    {} Core values and constraints", "project://constitution".green());
    println!("    {} Active spec being worked on", "project://current-spec".green());
    println!("    {} DDD bounded contexts", "project://domain-model".green());
    println!("    {} Task board state", "project://kanban-board".green());
    println!("    {} ADR log", "project://decisions".green());
    println!("    {} All specifications", "project://specs".green());
    println!("    {} All tasks", "project://tasks".green());
    println!("    {} Domain terms", "project://glossary".green());
    println!("    {} WIP limits", "project://wip-status".green());
    println!("    {} Context relationships", "project://context-map".green());
    println!();

    // Configuration
    println!("{}", "  Configuration".bold().underline());
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
    println!();
}
