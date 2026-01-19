//! Memory management commands.

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;
use std::path::Path;

#[derive(Subcommand)]
pub enum MemoryCommands {
    /// Sync memory with CLAUDE.md
    Sync,

    /// Compact memory (remove expired entries)
    Compact,

    /// Export memory for new session
    Export(ExportArgs),

    /// Search memory
    Search(SearchArgs),
}

#[derive(Args)]
pub struct ExportArgs {
    /// Output file
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,
}

pub async fn execute(cmd: MemoryCommands, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");
    let pool = cwa_db::init_pool(&db_path)?;

    let project = cwa_core::project::get_default_project(&pool)?
        .ok_or_else(|| anyhow::anyhow!("No project found. Run 'cwa init' first."))?;

    match cmd {
        MemoryCommands::Sync => {
            // Update CLAUDE.md with current context
            let summary = cwa_core::memory::get_context_summary(&pool, &project.id)?;
            let content = summary.to_compact_string();

            let claude_md_path = project_dir.join("CLAUDE.md");
            std::fs::write(&claude_md_path, content)?;

            println!(
                "{} Synced CLAUDE.md",
                "✓".green().bold()
            );
        }

        MemoryCommands::Compact => {
            let count = cwa_core::memory::cleanup_memory(&pool)?;
            println!(
                "{} Removed {} expired memory entries",
                "✓".green().bold(),
                count
            );
        }

        MemoryCommands::Export(args) => {
            let entries = cwa_core::memory::list_memory(&pool, &project.id, Some(100))?;
            let json = serde_json::to_string_pretty(&entries)?;

            if let Some(output) = args.output {
                std::fs::write(&output, &json)?;
                println!("{} Exported to {}", "✓".green().bold(), output);
            } else {
                println!("{}", json);
            }
        }

        MemoryCommands::Search(args) => {
            let results = cwa_core::memory::search_memory(&pool, &project.id, &args.query)?;

            if results.is_empty() {
                println!("{} No results found", "ℹ".blue().bold());
            } else {
                println!("{} Found {} entries:", "✓".green().bold(), results.len());
                for entry in results {
                    println!("  [{}] {}", entry.entry_type.dimmed(), entry.content);
                }
            }
        }
    }

    Ok(())
}
