//! Tech stack configuration commands.
//!
//! Manages `.cwa/stack.json` which declares the project's technology stack.
//! This file is read by `cwa codegen all` to select appropriate expert agents.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use std::path::Path;

#[derive(Subcommand)]
pub enum StackCommands {
    /// Set the project tech stack (writes .cwa/stack.json)
    Set {
        /// Technologies (e.g. rust axum redis neo4j qdrant)
        #[arg(required = true, num_args = 1..)]
        technologies: Vec<String>,
    },

    /// Show current tech stack and available agent templates
    Show,
}

pub async fn execute(cmd: StackCommands, project_dir: &Path) -> Result<()> {
    match cmd {
        StackCommands::Set { technologies } => cmd_set(project_dir, technologies),
        StackCommands::Show => cmd_show(project_dir),
    }
}

fn cmd_set(project_dir: &Path, technologies: Vec<String>) -> Result<()> {
    let cwa_dir = project_dir.join(".cwa");
    std::fs::create_dir_all(&cwa_dir)
        .context("Failed to create .cwa directory")?;

    let path = cwa_dir.join("stack.json");
    let content = serde_json::to_string_pretty(&serde_json::json!({
        "tech_stack": technologies
    }))?;

    std::fs::write(&path, &content)
        .context("Failed to write .cwa/stack.json")?;

    println!("{} Tech stack saved to {}", "✓".green().bold(), path.display());
    println!("  Stack: {}", technologies.join(", ").cyan());
    println!();
    println!("{}", "Run 'cwa codegen all' to regenerate agents for this stack.".dimmed());

    Ok(())
}

fn cmd_show(project_dir: &Path) -> Result<()> {
    let path = project_dir.join(".cwa/stack.json");

    if !path.exists() {
        println!("{}", "No .cwa/stack.json found.".dimmed());
        println!("Run 'cwa stack set <tech> [<tech2>...]' to configure your tech stack.");
        return Ok(());
    }

    let content = std::fs::read_to_string(&path)
        .context("Failed to read .cwa/stack.json")?;
    let val: serde_json::Value = serde_json::from_str(&content)
        .context("Failed to parse .cwa/stack.json")?;

    let tech_stack: Vec<String> = val["tech_stack"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if tech_stack.is_empty() {
        println!("{}", "Tech stack is empty.".dimmed());
        return Ok(());
    }

    println!("{}", "Tech Stack".bold());
    println!("{}", "─".repeat(40));
    for tech in &tech_stack {
        println!("  • {}", tech.cyan());
    }
    println!();

    // Show which agents would be generated
    let agents = cwa_codegen::select_agents_for_stack(&tech_stack);
    if agents.is_empty() {
        println!("{}", "No tech-stack agent templates match this stack.".dimmed());
    } else {
        println!("{} {} agent templates would be generated:", "→".dimmed(), agents.len());
        for agent in &agents {
            println!("  .claude/agents/{}", agent.filename.dimmed());
        }
    }

    Ok(())
}
