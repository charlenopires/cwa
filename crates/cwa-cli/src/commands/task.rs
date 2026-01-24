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

    /// List all tasks
    List,

    /// Generate tasks from a spec's acceptance criteria
    Generate(GenerateTaskArgs),

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
pub struct GenerateTaskArgs {
    /// Spec ID or title
    pub spec: String,

    /// Initial task status (default: backlog)
    #[arg(long, default_value = "backlog")]
    pub status: String,

    /// Preview tasks without creating them
    #[arg(long)]
    pub dry_run: bool,

    /// Prefix for task titles
    #[arg(long)]
    pub prefix: Option<String>,
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

        TaskCommands::Generate(args) => {
            let spec = cwa_core::spec::get_spec(&pool, &project.id, &args.spec)?;

            if spec.acceptance_criteria.is_empty() {
                println!(
                    "{} Spec '{}' has no acceptance criteria. Add criteria first with 'cwa spec add-criteria'.",
                    "✗".red().bold(),
                    spec.title
                );
                return Ok(());
            }

            if args.dry_run {
                // Check existing tasks for skip count
                let existing_tasks = cwa_core::task::list_tasks_by_spec(&pool, &spec.id)?;
                let existing_titles: Vec<&str> = existing_tasks.iter().map(|t| t.title.as_str()).collect();

                let new_criteria: Vec<&String> = spec.acceptance_criteria.iter()
                    .filter(|c| {
                        let title = if let Some(prefix) = &args.prefix {
                            format!("{}: {}", prefix, c)
                        } else {
                            c.to_string()
                        };
                        !existing_titles.contains(&title.as_str())
                    })
                    .collect();

                let skipped = spec.acceptance_criteria.len() - new_criteria.len();

                println!(
                    "{} Would create {} task(s) for spec '{}'{}:\n",
                    "⊙".blue().bold(),
                    new_criteria.len(),
                    spec.title.cyan(),
                    if skipped > 0 { format!(" (skipping {} existing)", skipped) } else { String::new() }
                );
                for (i, criterion) in new_criteria.iter().enumerate() {
                    let title = if let Some(prefix) = &args.prefix {
                        format!("{}: {}", prefix, criterion)
                    } else {
                        criterion.to_string()
                    };
                    println!("  {}. {} {}", i + 1, title.cyan(), format!("[{}]", spec.priority.as_str()).dimmed());
                }
                return Ok(());
            }

            // If prefix is specified, temporarily update criteria with prefixed versions
            let criteria_to_use: Vec<String> = if let Some(prefix) = &args.prefix {
                spec.acceptance_criteria.iter()
                    .map(|c| format!("{}: {}", prefix, c))
                    .collect()
            } else {
                spec.acceptance_criteria.clone()
            };

            // If using prefix, we need to update the spec's criteria temporarily
            // Instead, we'll just create tasks directly with the prefixed titles
            if args.prefix.is_some() {
                let existing_tasks = cwa_core::task::list_tasks_by_spec(&pool, &spec.id)?;
                let existing_titles: Vec<String> = existing_tasks.iter().map(|t| t.title.clone()).collect();

                let mut created = 0;
                let mut skipped = 0;

                for title in &criteria_to_use {
                    if existing_titles.contains(title) {
                        skipped += 1;
                        continue;
                    }

                    let task = cwa_core::task::create_task(
                        &pool,
                        &project.id,
                        title,
                        None,
                        Some(&spec.id),
                        spec.priority.as_str(),
                    )?;

                    if args.status != "backlog" {
                        cwa_core::task::move_task(&pool, &project.id, &task.id, &args.status)?;
                    }

                    created += 1;
                    println!(
                        "  {}. {} ({})",
                        created,
                        title.cyan(),
                        task.id.dimmed()
                    );
                }

                println!(
                    "\n{} Generated {} task(s){}",
                    "✓".green().bold(),
                    created,
                    if skipped > 0 { format!(", skipped {} existing", skipped) } else { String::new() }
                );
            } else {
                let result = cwa_core::task::generate_tasks_from_spec(
                    &pool,
                    &project.id,
                    &args.spec,
                    &args.status,
                )?;

                if result.created.is_empty() && result.skipped > 0 {
                    println!(
                        "{} All {} criteria already have tasks. Nothing to generate.",
                        "⊙".blue().bold(),
                        result.skipped
                    );
                } else {
                    println!(
                        "{} Generated {} task(s) for spec '{}'{}:\n",
                        "✓".green().bold(),
                        result.created.len(),
                        spec.title.cyan(),
                        if result.skipped > 0 { format!(", skipped {} existing", result.skipped) } else { String::new() }
                    );
                    for (i, task) in result.created.iter().enumerate() {
                        println!(
                            "  {}. {} ({})",
                            i + 1,
                            task.title.cyan(),
                            task.id.dimmed()
                        );
                    }
                }
            }
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

        TaskCommands::List => {
            let tasks = cwa_core::task::list_tasks(&pool, &project.id)?;
            output::print_tasks_table(&tasks);
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
