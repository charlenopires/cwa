//! Task management commands.

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;
use std::path::Path;

use crate::output;

#[derive(Subcommand)]
pub enum TaskCommands {
    /// Create a new task
    New(NewTaskArgs),

    /// Move a task to a different status
    Move(MoveTaskArgs),

    /// Display the Kanban board
    Board,

    /// Show work-in-progress status
    Wip,
}

#[derive(Args)]
pub struct NewTaskArgs {
    /// Task title
    pub title: String,

    /// Task description
    #[arg(short, long)]
    pub description: Option<String>,

    /// Link to spec ID
    #[arg(short, long)]
    pub spec: Option<String>,

    /// Priority (low, medium, high, critical)
    #[arg(long, default_value = "medium")]
    pub priority: String,
}

#[derive(Args)]
pub struct MoveTaskArgs {
    /// Task ID
    pub task_id: String,

    /// Target status (backlog, todo, in_progress, review, done)
    pub status: String,
}

pub async fn execute(cmd: TaskCommands, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");
    let pool = cwa_db::init_pool(&db_path)?;

    let project = cwa_core::project::get_default_project(&pool)?
        .ok_or_else(|| anyhow::anyhow!("No project found. Run 'cwa init' first."))?;

    match cmd {
        TaskCommands::New(args) => {
            let task = cwa_core::task::create_task(
                &pool,
                &project.id,
                &args.title,
                args.description.as_deref(),
                args.spec.as_deref(),
                &args.priority,
            )?;

            println!(
                "{} Created task: {} ({})",
                "✓".green().bold(),
                task.title.cyan(),
                task.id.dimmed()
            );
        }

        TaskCommands::Move(args) => {
            cwa_core::task::move_task(&pool, &project.id, &args.task_id, &args.status)?;
            println!(
                "{} Moved task {} to {}",
                "✓".green().bold(),
                args.task_id.dimmed(),
                args.status.cyan()
            );
        }

        TaskCommands::Board => {
            let board = cwa_core::task::get_board(&pool, &project.id)?;
            output::print_board(&board);
        }

        TaskCommands::Wip => {
            let wip = cwa_core::task::get_wip_status(&pool, &project.id)?;
            output::print_wip(&wip);
        }
    }

    Ok(())
}
