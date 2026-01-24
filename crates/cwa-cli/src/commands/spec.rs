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

    /// Parse a long prompt and create multiple specifications
    FromPrompt(FromPromptArgs),

    /// Add acceptance criteria to an existing specification
    AddCriteria(AddCriteriaArgs),

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

    /// Acceptance criteria (can be specified multiple times)
    #[arg(short = 'c', long = "criteria")]
    pub criteria: Vec<String>,
}

#[derive(Args)]
pub struct FromPromptArgs {
    /// Long prompt text (use quotes). If omitted, reads from stdin.
    pub text: Option<String>,

    /// Read prompt from a file
    #[arg(short, long)]
    pub file: Option<String>,

    /// Priority for all created specs (low, medium, high, critical)
    #[arg(long, default_value = "medium")]
    pub priority: String,

    /// Preview parsed specs without creating them
    #[arg(long)]
    pub dry_run: bool,
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
pub struct AddCriteriaArgs {
    /// Spec ID or title
    pub spec: String,

    /// Acceptance criteria to add (one or more)
    pub criteria: Vec<String>,
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
            let criteria = if args.criteria.is_empty() {
                None
            } else {
                Some(args.criteria.as_slice())
            };

            let spec = cwa_core::spec::create_spec_with_criteria(
                &pool,
                &project.id,
                &args.name,
                args.description.as_deref(),
                &args.priority,
                criteria,
            )?;

            println!(
                "{} Created spec: {} ({})",
                "✓".green().bold(),
                spec.title.cyan(),
                spec.id.dimmed()
            );
            if !spec.acceptance_criteria.is_empty() {
                println!("  {} acceptance criteria added", spec.acceptance_criteria.len());
            }
        }

        SpecCommands::FromPrompt(args) => {
            let input = if let Some(file_path) = &args.file {
                std::fs::read_to_string(file_path)
                    .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", file_path, e))?
            } else if let Some(text) = &args.text {
                text.clone()
            } else {
                // Read from stdin
                use std::io::Read;
                let mut buffer = String::new();
                eprintln!("{}", "Reading from stdin (Ctrl+D to finish):".dimmed());
                std::io::stdin().read_to_string(&mut buffer)?;
                buffer
            };

            let parsed = cwa_core::spec::parser::parse_prompt(&input);

            if parsed.is_empty() {
                println!("{} No specs could be parsed from the input.", "✗".red().bold());
                return Ok(());
            }

            if args.dry_run {
                println!(
                    "{} Would create {} spec(s):\n",
                    "⊙".blue().bold(),
                    parsed.len()
                );
                for (i, entry) in parsed.iter().enumerate() {
                    println!("  {}. {} {}", i + 1, entry.title.cyan(), format!("[{}]", args.priority).dimmed());
                    if let Some(desc) = &entry.description {
                        for line in desc.lines().take(3) {
                            println!("     {}", line.dimmed());
                        }
                    }
                }
                return Ok(());
            }

            let specs = cwa_core::spec::create_specs_from_prompt(
                &pool,
                &project.id,
                &input,
                &args.priority,
            )?;

            println!(
                "{} Created {} spec(s):\n",
                "✓".green().bold(),
                specs.len()
            );
            for (i, spec) in specs.iter().enumerate() {
                println!(
                    "  {}. {} ({})",
                    i + 1,
                    spec.title.cyan(),
                    spec.id.dimmed()
                );
            }
        }

        SpecCommands::AddCriteria(args) => {
            if args.criteria.is_empty() {
                println!("{} No criteria provided.", "✗".red().bold());
                return Ok(());
            }

            let spec = cwa_core::spec::add_acceptance_criteria(
                &pool,
                &project.id,
                &args.spec,
                &args.criteria,
            )?;

            println!(
                "{} Added {} criteria to spec '{}' (total: {})",
                "✓".green().bold(),
                args.criteria.len(),
                spec.title.cyan(),
                spec.acceptance_criteria.len()
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
