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

    /// Manage domain objects
    #[command(subcommand)]
    Object(ObjectSubCommands),

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

#[derive(Subcommand)]
pub enum ObjectSubCommands {
    /// Create a new domain object
    New(NewObjectArgs),
}

#[derive(Args)]
pub struct NewObjectArgs {
    /// Object name
    pub name: String,

    /// Bounded context name
    #[arg(short, long)]
    pub context: String,

    /// Object type
    #[arg(short = 't', long = "type", value_parser = ["aggregate", "entity", "value_object", "service", "event"])]
    pub object_type: String,

    /// Description
    #[arg(short, long)]
    pub description: Option<String>,
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
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let pool = cwa_db::init_pool(&redis_url).await?;

    let project = cwa_core::project::get_default_project(&pool).await?
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
                ).await?;

                println!(
                    "{} Created context: {} ({})",
                    "✓".green().bold(),
                    context.name.cyan(),
                    context.id.dimmed()
                );
            }

            ContextSubCommands::List => {
                let contexts = cwa_core::domain::list_contexts(&pool, &project.id).await?;
                output::print_contexts(&contexts);
            }

            ContextSubCommands::Map => {
                let map = cwa_core::domain::get_context_map(&pool, &project.id).await?;
                output::print_context_map(&map);
            }
        },

        DomainCommands::Object(sub) => match sub {
            ObjectSubCommands::New(args) => {
                let context = cwa_core::domain::get_context_by_name(&pool, &project.id, &args.context).await?
                    .ok_or_else(|| anyhow::anyhow!(
                        "Bounded context '{}' not found. Use 'cwa domain context list' to see available contexts.",
                        args.context
                    ))?;

                let obj_id = cwa_core::domain::create_domain_object(
                    &pool,
                    &context.id,
                    &args.name,
                    &args.object_type,
                    args.description.as_deref(),
                ).await?;

                println!(
                    "{} Created domain object: {} ({}) in context {}",
                    "✓".green().bold(),
                    args.name.cyan(),
                    args.object_type.dimmed(),
                    context.name.cyan(),
                );

                // Try to embed (graceful failure if Qdrant/Ollama unavailable)
                match cwa_embedding::DomainObjectPipeline::default_pipeline() {
                    Ok(pipeline) => {
                        match pipeline.embed_domain_object(
                            &project.id,
                            &obj_id,
                            &args.name,
                            &args.object_type,
                            &context.name,
                            args.description.as_deref().unwrap_or(""),
                        ).await {
                            Ok(dim) => {
                                println!(
                                    "  {} Embedded ({} dims)",
                                    "→".dimmed(),
                                    dim
                                );
                            }
                            Err(e) => {
                                println!(
                                    "  {} Embedding skipped: {}",
                                    "!".yellow(),
                                    e
                                );
                            }
                        }
                    }
                    Err(_) => {
                        println!(
                            "  {} Embedding skipped (Qdrant/Ollama unavailable)",
                            "!".yellow()
                        );
                    }
                }
            }
        },

        DomainCommands::Glossary => {
            let terms = cwa_core::domain::list_glossary(&pool, &project.id).await?;
            output::print_glossary(&terms);
        }
    }

    Ok(())
}
