//! Context status commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use std::path::Path;

#[derive(Subcommand)]
pub enum ContextCommands {
    /// Show current context status
    Status,

    /// Show context summary
    Summary,
}

pub async fn execute(cmd: ContextCommands, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");
    let pool = cwa_db::init_pool(&db_path)?;

    let project = cwa_core::project::get_default_project(&pool)?
        .ok_or_else(|| anyhow::anyhow!("No project found. Run 'cwa init' first."))?;

    match cmd {
        ContextCommands::Status | ContextCommands::Summary => {
            let summary = cwa_core::memory::get_context_summary(&pool, &project.id)?;

            println!("{}", summary.project_name.cyan().bold());
            println!();

            println!("{}", "Current Focus".bold());
            if let Some(task) = &summary.current_task {
                println!("  Task: {}", task.cyan());
            } else {
                println!("  Task: {}", "None".dimmed());
            }

            if let Some(spec) = &summary.active_spec {
                println!("  Spec: {}", spec.cyan());
            } else {
                println!("  Spec: {}", "None".dimmed());
            }

            println!();
            println!("{}", "Board Status".bold());
            println!(
                "  {} backlog | {} todo | {} in progress | {} review | {} done",
                summary.task_counts.backlog,
                summary.task_counts.todo,
                summary.task_counts.in_progress.to_string().yellow(),
                summary.task_counts.review,
                summary.task_counts.done.to_string().green()
            );

            if !summary.recent_decisions.is_empty() {
                println!();
                println!("{}", "Recent Decisions".bold());
                for decision in &summary.recent_decisions {
                    println!("  - {}", decision);
                }
            }

            if !summary.recent_insights.is_empty() {
                println!();
                println!("{}", "Recent Insights".bold());
                for insight in &summary.recent_insights {
                    println!("  - {}", insight);
                }
            }
        }
    }

    Ok(())
}
