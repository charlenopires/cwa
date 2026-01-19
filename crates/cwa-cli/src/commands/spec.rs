//! Specification management commands.

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;
use std::path::Path;

use crate::output;

#[derive(Subcommand)]
pub enum SpecCommands {
    /// Create a new specification
    New(NewSpecArgs),

    /// List all specifications
    List,

    /// Show specification status
    Status(StatusArgs),

    /// Validate a specification
    Validate(ValidateArgs),

    /// Archive a specification
    Archive(ArchiveArgs),
}

#[derive(Args)]
pub struct NewSpecArgs {
    /// Specification name/title
    pub name: String,

    /// Description
    #[arg(short, long)]
    pub description: Option<String>,

    /// Priority (low, medium, high, critical)
    #[arg(long, default_value = "medium")]
    pub priority: String,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Spec ID or name (optional, shows all if omitted)
    pub spec: Option<String>,
}

#[derive(Args)]
pub struct ValidateArgs {
    /// Spec ID or name
    pub spec: String,
}

#[derive(Args)]
pub struct ArchiveArgs {
    /// Spec ID
    pub spec_id: String,

    /// Reason for archiving
    #[arg(long)]
    pub reason: Option<String>,
}

pub async fn execute(cmd: SpecCommands, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");
    let pool = cwa_db::init_pool(&db_path)?;

    let project = cwa_core::project::get_default_project(&pool)?
        .ok_or_else(|| anyhow::anyhow!("No project found. Run 'cwa init' first."))?;

    match cmd {
        SpecCommands::New(args) => {
            let spec = cwa_core::spec::create_spec(
                &pool,
                &project.id,
                &args.name,
                args.description.as_deref(),
                &args.priority,
            )?;

            println!(
                "{} Created spec: {} ({})",
                "✓".green().bold(),
                spec.title.cyan(),
                spec.id.dimmed()
            );
        }

        SpecCommands::List => {
            let specs = cwa_core::spec::list_specs(&pool, &project.id)?;
            output::print_specs_table(&specs);
        }

        SpecCommands::Status(args) => {
            if let Some(spec_name) = args.spec {
                let spec = cwa_core::spec::get_spec(&pool, &project.id, &spec_name)?;
                output::print_spec(&spec);
            } else {
                let specs = cwa_core::spec::list_specs(&pool, &project.id)?;
                output::print_specs_table(&specs);
            }
        }

        SpecCommands::Validate(args) => {
            let spec = cwa_core::spec::get_spec(&pool, &project.id, &args.spec)?;
            let result = cwa_core::spec::validate_spec(&pool, &spec.id)?;

            if result.is_valid {
                println!("{} Spec is valid", "✓".green().bold());
            } else {
                println!("{} Validation issues:", "✗".red().bold());
                for issue in result.issues {
                    println!("  - {}", issue);
                }
            }
        }

        SpecCommands::Archive(args) => {
            cwa_core::spec::archive_spec(&pool, &args.spec_id)?;
            println!(
                "{} Archived spec: {}",
                "✓".green().bold(),
                args.spec_id.dimmed()
            );
        }
    }

    Ok(())
}
