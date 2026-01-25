//! Clean project command - reset project to initial state.

use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

#[derive(Args)]
pub struct CleanArgs {
    /// Confirm destructive operation
    #[arg(long)]
    pub confirm: bool,

    /// Also stop and remove Docker infrastructure (containers, volumes, images)
    #[arg(long)]
    pub infra: bool,
}

pub async fn execute(args: CleanArgs, project_dir: &Path) -> Result<()> {
    if !args.confirm {
        println!("{}", "This will permanently delete:".red().bold());
        println!("  {} .cwa/ (database, constitution, docker config)", "•".red());
        println!("  {} .claude/ (agents, skills, commands, rules, hooks)", "•".red());
        println!("  {} CLAUDE.md", "•".red());
        println!("  {} .mcp.json", "•".red());
        if args.infra {
            println!("  {} Docker containers, volumes, and images", "•".red());
        }
        println!();
        println!("Run with {} to confirm.", "--confirm".bold());
        if !args.infra {
            println!("{}", "  Add --infra to also remove Docker infrastructure".dimmed());
        }
        return Ok(());
    }

    println!("{}", "Cleaning project...".red().bold());

    // Stop Docker infrastructure first if requested
    if args.infra {
        let compose_file = project_dir.join(".cwa/docker/docker-compose.yml");
        if compose_file.exists() {
            println!("  {} Stopping Docker infrastructure...", "→".dimmed());
            let _ = Command::new("docker")
                .args(["compose", "-f", compose_file.to_str().unwrap(), "down", "-v", "--rmi", "all"])
                .status();

            // Remove any orphan containers
            let _ = Command::new("docker")
                .args(["rm", "-f", "cwa-neo4j", "cwa-qdrant", "cwa-ollama"])
                .status();

            println!("  {} Docker infrastructure removed", "✓".green());
        }
    }

    // Remove .cwa directory
    let cwa_dir = project_dir.join(".cwa");
    if cwa_dir.exists() {
        std::fs::remove_dir_all(&cwa_dir)?;
        println!("  {} Removed .cwa/", "✓".green());
    }

    // Remove .claude directory
    let claude_dir = project_dir.join(".claude");
    if claude_dir.exists() {
        std::fs::remove_dir_all(&claude_dir)?;
        println!("  {} Removed .claude/", "✓".green());
    }

    // Remove CLAUDE.md
    let claude_md = project_dir.join("CLAUDE.md");
    if claude_md.exists() {
        std::fs::remove_file(&claude_md)?;
        println!("  {} Removed CLAUDE.md", "✓".green());
    }

    // Remove .mcp.json
    let mcp_json = project_dir.join(".mcp.json");
    if mcp_json.exists() {
        std::fs::remove_file(&mcp_json)?;
        println!("  {} Removed .mcp.json", "✓".green());
    }

    println!();
    println!("{}", "Project cleaned. Run 'cwa init <name>' to start fresh.".green().bold());

    Ok(())
}
