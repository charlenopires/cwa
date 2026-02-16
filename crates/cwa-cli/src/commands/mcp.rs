//! MCP server commands.

use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use colored::Colorize;
use dialoguer::{Confirm, MultiSelect};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Number of tools and resources available in the MCP server.
const MCP_TOOLS_COUNT: usize = 34;
const MCP_RESOURCES_COUNT: usize = 11;

/// Supported software targets for MCP installation
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpTarget {
    /// Claude Desktop (~/Library/Application Support/Claude/claude_desktop_config.json)
    ClaudeDesktop,
    /// Claude Code CLI (~/.claude.json or via `claude mcp add`)
    ClaudeCode,
    /// Gemini CLI (~/.gemini/settings.json)
    GeminiCli,
    /// Google AntiGravity (~/.gemini/antigravity/mcp_config.json)
    Antigravity,
    /// VSCode (~/Library/Application Support/Code/User/mcp.json)
    Vscode,
    /// VSCode Insiders (~/Library/Application Support/Code - Insiders/User/mcp.json)
    VscodeInsiders,
    /// All supported targets
    All,
}

impl std::fmt::Display for McpTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpTarget::ClaudeDesktop => write!(f, "Claude Desktop"),
            McpTarget::ClaudeCode => write!(f, "Claude Code"),
            McpTarget::GeminiCli => write!(f, "Gemini CLI"),
            McpTarget::Antigravity => write!(f, "AntiGravity"),
            McpTarget::Vscode => write!(f, "VSCode"),
            McpTarget::VscodeInsiders => write!(f, "VSCode Insiders"),
            McpTarget::All => write!(f, "All"),
        }
    }
}

/// MCP server variant to install
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum McpServerVariant {
    /// Standard MCP server (34 tools, 11 resources)
    #[default]
    Stdio,
    /// Planner server with DDD/SDD methodology (1 tool: cwa_plan_software)
    Planner,
}

impl std::fmt::Display for McpServerVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpServerVariant::Stdio => write!(f, "stdio"),
            McpServerVariant::Planner => write!(f, "planner"),
        }
    }
}

#[derive(Args)]
pub struct InstallArgs {
    /// Target software(s) to install to (interactive selection if not specified)
    #[arg(value_enum)]
    pub target: Option<McpTarget>,

    /// Server variant to install
    #[arg(short = 't', long, value_enum, default_value = "stdio")]
    pub variant: McpServerVariant,

    /// Custom server name (default: "cwa" for stdio, "cwa-planner" for planner)
    #[arg(short, long)]
    pub name: Option<String>,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Show configuration without installing (dry run)
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum McpCommands {
    /// Run MCP server over stdio
    Stdio,

    /// Run MCP planner server for Claude Desktop
    Planner,

    /// Show MCP server status and available tools/resources
    Status,

    /// Install CWA MCP server to supported software
    Install(InstallArgs),

    /// Uninstall CWA MCP server from supported software
    Uninstall {
        /// Target software to uninstall from
        #[arg(value_enum)]
        target: Option<McpTarget>,

        /// Server name to remove (default: "cwa")
        #[arg(short, long)]
        name: Option<String>,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

pub async fn execute(cmd: McpCommands, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");

    match cmd {
        McpCommands::Stdio => {
            let pool = Arc::new(cwa_db::init_pool(&db_path)?);
            // Running standalone - no broadcast channel (uses HTTP fallback)
            cwa_mcp::run_stdio_server(pool, None).await?;
        }

        McpCommands::Planner => {
            cwa_mcp::run_planner_stdio().await?;
        }

        McpCommands::Status => {
            print_mcp_status();
        }

        McpCommands::Install(args) => {
            install_mcp_server(args).await?;
        }

        McpCommands::Uninstall { target, name, yes } => {
            uninstall_mcp_server(target, name, yes).await?;
        }
    }

    Ok(())
}

/// Get the config file path for a target
fn get_config_path(target: McpTarget) -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    match target {
        McpTarget::ClaudeDesktop => {
            Some(home.join("Library/Application Support/Claude/claude_desktop_config.json"))
        }
        McpTarget::ClaudeCode => Some(home.join(".claude.json")),
        McpTarget::GeminiCli => Some(home.join(".gemini/settings.json")),
        McpTarget::Antigravity => Some(home.join(".gemini/antigravity/mcp_config.json")),
        McpTarget::Vscode => {
            Some(home.join("Library/Application Support/Code/User/mcp.json"))
        }
        McpTarget::VscodeInsiders => {
            Some(home.join("Library/Application Support/Code - Insiders/User/mcp.json"))
        }
        McpTarget::All => None,
    }
}

/// Get server configuration for the specified variant
fn get_server_config(variant: McpServerVariant, target: McpTarget) -> Value {
    let args = match variant {
        McpServerVariant::Stdio => vec!["mcp", "stdio"],
        McpServerVariant::Planner => vec!["mcp", "planner"],
    };

    let command = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "cwa".to_string());

    // VSCode uses a different format with "servers" key instead of "mcpServers"
    match target {
        McpTarget::Vscode | McpTarget::VscodeInsiders => {
            json!({
                "command": command,
                "args": args
            })
        }
        _ => {
            json!({
                "command": command,
                "args": args
            })
        }
    }
}

/// Get the default server name based on variant
fn get_default_server_name(variant: McpServerVariant) -> &'static str {
    match variant {
        McpServerVariant::Stdio => "cwa",
        McpServerVariant::Planner => "cwa-planner",
    }
}

/// Get all installable targets (excluding All)
fn get_all_targets() -> Vec<McpTarget> {
    vec![
        McpTarget::ClaudeDesktop,
        McpTarget::ClaudeCode,
        McpTarget::GeminiCli,
        McpTarget::Antigravity,
        McpTarget::Vscode,
        McpTarget::VscodeInsiders,
    ]
}

/// Check if config file exists and is accessible
fn check_config_exists(target: McpTarget) -> (bool, Option<PathBuf>) {
    if let Some(path) = get_config_path(target) {
        (path.exists(), Some(path))
    } else {
        (false, None)
    }
}

/// Read or create JSON config file
fn read_or_create_config(path: &Path) -> Result<Value> {
    if path.exists() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        if content.trim().is_empty() {
            return Ok(json!({}));
        }

        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON in: {}", path.display()))
    } else {
        Ok(json!({}))
    }
}

/// Write JSON config to file with pretty formatting
fn write_config(path: &Path, config: &Value) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(config)
        .context("Failed to serialize JSON")?;

    fs::write(path, content)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(())
}

/// Add MCP server to a config file
fn add_server_to_config(
    config: &mut Value,
    server_name: &str,
    server_config: Value,
    target: McpTarget,
) -> Result<bool> {
    // VSCode uses "servers" inside an "mcp" object
    let servers_key = match target {
        McpTarget::Vscode | McpTarget::VscodeInsiders => {
            // Ensure mcp.servers exists
            if config.get("servers").is_none() {
                config["servers"] = json!({});
            }
            "servers"
        }
        _ => {
            // Standard MCP config uses "mcpServers"
            if config.get("mcpServers").is_none() {
                config["mcpServers"] = json!({});
            }
            "mcpServers"
        }
    };

    let servers = config.get_mut(servers_key)
        .ok_or_else(|| anyhow::anyhow!("Failed to get servers object"))?;

    // Check if server already exists
    if servers.get(server_name).is_some() {
        return Ok(false); // Already exists
    }

    servers[server_name] = server_config;
    Ok(true) // Added new
}

/// Remove MCP server from a config file
fn remove_server_from_config(
    config: &mut Value,
    server_name: &str,
    target: McpTarget,
) -> Result<bool> {
    let servers_key = match target {
        McpTarget::Vscode | McpTarget::VscodeInsiders => "servers",
        _ => "mcpServers",
    };

    if let Some(servers) = config.get_mut(servers_key) {
        if let Some(obj) = servers.as_object_mut() {
            if obj.remove(server_name).is_some() {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Install to a single target
fn install_to_target(
    target: McpTarget,
    variant: McpServerVariant,
    server_name: &str,
    dry_run: bool,
) -> Result<InstallResult> {
    let path = get_config_path(target)
        .ok_or_else(|| anyhow::anyhow!("No config path for target"))?;

    let mut config = read_or_create_config(&path)?;
    let server_config = get_server_config(variant, target);

    if dry_run {
        println!("\n{} {} ({}):", "Would install to".yellow(), target, path.display());
        let preview_config = json!({
            server_name: server_config
        });
        println!("{}", serde_json::to_string_pretty(&preview_config)?);
        return Ok(InstallResult::DryRun);
    }

    let added = add_server_to_config(&mut config, server_name, server_config, target)?;

    if added {
        write_config(&path, &config)?;
        Ok(InstallResult::Installed)
    } else {
        Ok(InstallResult::AlreadyExists)
    }
}

/// Result of installation attempt
#[derive(Debug, PartialEq)]
enum InstallResult {
    Installed,
    AlreadyExists,
    DryRun,
}

/// Interactive target selection
fn select_targets_interactively() -> Result<Vec<McpTarget>> {
    let targets = get_all_targets();
    let items: Vec<String> = targets
        .iter()
        .map(|t| {
            let (exists, path) = check_config_exists(*t);
            let status = if exists {
                format!("{} (config exists)", "●".green())
            } else if path.is_some() {
                format!("{} (will create)", "○".yellow())
            } else {
                format!("{} (unavailable)", "✗".red())
            };
            format!("{} {}", t, status)
        })
        .collect();

    let selections = MultiSelect::new()
        .with_prompt("Select target software (Space to select, Enter to confirm)")
        .items(&items)
        .interact()
        .context("Failed to get user selection")?;

    if selections.is_empty() {
        anyhow::bail!("No targets selected");
    }

    Ok(selections.into_iter().map(|i| targets[i]).collect())
}

/// Install MCP server to selected targets
async fn install_mcp_server(args: InstallArgs) -> Result<()> {
    println!();
    println!("{} CWA MCP Server Installation", "●".cyan().bold());
    println!();

    let server_name = args.name.as_deref()
        .unwrap_or_else(|| get_default_server_name(args.variant));

    let targets: Vec<McpTarget> = if let Some(target) = args.target {
        if target == McpTarget::All {
            get_all_targets()
        } else {
            vec![target]
        }
    } else {
        // Interactive selection
        select_targets_interactively()?
    };

    println!("  {} Server: {} ({})", "▸".dimmed(), server_name.cyan(), args.variant);
    println!("  {} Targets: {}", "▸".dimmed(), targets.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", "));
    println!();

    if !args.yes && !args.dry_run {
        let confirm = Confirm::new()
            .with_prompt("Proceed with installation?")
            .default(true)
            .interact()
            .context("Failed to get confirmation")?;

        if !confirm {
            println!("{} Installation cancelled", "✗".red().bold());
            return Ok(());
        }
        println!();
    }

    let mut installed = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for target in &targets {
        let result = install_to_target(*target, args.variant, server_name, args.dry_run);

        match result {
            Ok(InstallResult::Installed) => {
                let path = get_config_path(*target).unwrap();
                println!(
                    "{} {} {} ({})",
                    "✓".green().bold(),
                    "Installed to".green(),
                    target.to_string().cyan(),
                    path.display().to_string().dimmed()
                );
                installed += 1;
            }
            Ok(InstallResult::AlreadyExists) => {
                println!(
                    "{} {} {} (already configured)",
                    "○".yellow().bold(),
                    "Skipped".yellow(),
                    target.to_string().cyan()
                );
                skipped += 1;
            }
            Ok(InstallResult::DryRun) => {
                // Already printed in install_to_target
            }
            Err(e) => {
                println!(
                    "{} {} {} ({})",
                    "✗".red().bold(),
                    "Failed".red(),
                    target.to_string().cyan(),
                    e.to_string().dimmed()
                );
                failed += 1;
            }
        }
    }

    println!();
    if args.dry_run {
        println!("{} Dry run complete. No changes were made.", "ℹ".blue().bold());
    } else {
        println!(
            "{} Installation complete: {} installed, {} skipped, {} failed",
            "●".green().bold(),
            installed.to_string().green(),
            skipped.to_string().yellow(),
            failed.to_string().red()
        );

        if installed > 0 {
            println!();
            println!("{}", "  Next Steps:".bold().underline());
            println!("    {} Restart the target application(s) for changes to take effect", "1.".dimmed());
            println!("    {} Run 'cwa mcp status' to see available tools and resources", "2.".dimmed());
            println!("    {} For live updates, run 'cwa serve' in your project directory", "3.".dimmed());
        }
    }
    println!();

    Ok(())
}

/// Uninstall MCP server from selected targets
async fn uninstall_mcp_server(target: Option<McpTarget>, name: Option<String>, yes: bool) -> Result<()> {
    println!();
    println!("{} CWA MCP Server Uninstallation", "●".red().bold());
    println!();

    let server_name = name.as_deref().unwrap_or("cwa");

    let targets: Vec<McpTarget> = if let Some(target) = target {
        if target == McpTarget::All {
            get_all_targets()
        } else {
            vec![target]
        }
    } else {
        select_targets_interactively()?
    };

    if !yes {
        let confirm = Confirm::new()
            .with_prompt(format!("Remove '{}' from {} target(s)?", server_name, targets.len()))
            .default(false)
            .interact()
            .context("Failed to get confirmation")?;

        if !confirm {
            println!("{} Uninstallation cancelled", "✗".red().bold());
            return Ok(());
        }
    }

    println!();

    let mut removed = 0;
    let mut not_found = 0;

    for target in &targets {
        let path = match get_config_path(*target) {
            Some(p) => p,
            None => continue,
        };

        if !path.exists() {
            println!(
                "{} {} {} (config not found)",
                "○".dimmed(),
                "Skipped".dimmed(),
                target.to_string().cyan()
            );
            not_found += 1;
            continue;
        }

        let mut config = read_or_create_config(&path)?;
        let was_removed = remove_server_from_config(&mut config, server_name, *target)?;

        if was_removed {
            write_config(&path, &config)?;
            println!(
                "{} {} {}",
                "✓".green().bold(),
                "Removed from".green(),
                target.to_string().cyan()
            );
            removed += 1;
        } else {
            println!(
                "{} {} {} (not configured)",
                "○".yellow().bold(),
                "Skipped".yellow(),
                target.to_string().cyan()
            );
            not_found += 1;
        }
    }

    println!();
    println!(
        "{} Uninstallation complete: {} removed, {} not found",
        "●".green().bold(),
        removed.to_string().green(),
        not_found.to_string().yellow()
    );
    println!();

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
