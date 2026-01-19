//! Domain modeling commands.

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;
use std::path::Path;

use crate::output;

#[derive(Subcommand)]
pub enum DomainCommands {
    /// Discover domain concepts (placeholder)
    Discover,

    /// Manage bounded contexts
    #[command(subcommand)]
    Context(ContextSubCommands),

    /// Display domain glossary
    Glossary,
}

#[derive(Subcommand)]
pub enum ContextSubCommands {
    /// Create a new bounded context
    New(NewContextArgs),

    /// List all bounded contexts
    List,

    /// Show context map
    Map,
}

#[derive(Args)]
pub struct NewContextArgs {
    /// Context name
    pub name: String,

    /// Description
    #[arg(short, long)]
    pub description: Option<String>,
}

pub async fn execute(cmd: DomainCommands, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");
    let pool = cwa_db::init_pool(&db_path)?;

    let project = cwa_core::project::get_default_project(&pool)?
        .ok_or_else(|| anyhow::anyhow!("No project found. Run 'cwa init' first."))?;

    match cmd {
        DomainCommands::Discover => {
            println!(
                "{} Domain discovery requires interactive analysis with Claude.",
                "ℹ".blue().bold()
            );
            println!("  Use the architect agent or /project:discover-domain command.");
        }

        DomainCommands::Context(sub) => match sub {
            ContextSubCommands::New(args) => {
                let context = cwa_core::domain::create_context(
                    &pool,
                    &project.id,
                    &args.name,
                    args.description.as_deref(),
                )?;

                println!(
                    "{} Created context: {} ({})",
                    "✓".green().bold(),
                    context.name.cyan(),
                    context.id.dimmed()
                );
            }

            ContextSubCommands::List => {
                let contexts = cwa_core::domain::list_contexts(&pool, &project.id)?;
                output::print_contexts(&contexts);
            }

            ContextSubCommands::Map => {
                let map = cwa_core::domain::get_context_map(&pool, &project.id)?;
                output::print_context_map(&map);
            }
        },

        DomainCommands::Glossary => {
            let terms = cwa_core::domain::list_glossary(&pool, &project.id)?;
            output::print_glossary(&terms);
        }
    }

    Ok(())
}
