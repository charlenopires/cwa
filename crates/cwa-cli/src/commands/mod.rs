//! CLI command definitions and handlers.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Find a CWA project by searching up the directory tree.
///
/// Starts from the current directory and walks up until it finds a directory
/// containing a `.cwa/` directory. This is used by MCP commands which may be
/// executed by Claude Desktop from "/" or other system directories.
fn find_cwa_project() -> Result<PathBuf> {
    let mut dir = std::env::current_dir().context("Failed to get current directory")?;

    loop {
        if dir.join(".cwa").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            anyhow::bail!(
                "No CWA project found. Run 'cwa init' in your project directory first, \
                or use '--project <path>' to specify the project location."
            );
        }
    }
}

pub mod analyze;
pub mod clean;
pub mod codegen;
pub mod context;
pub mod design;
pub mod domain;
pub mod git;
pub mod graph;
pub mod infra;
pub mod init;
pub mod memory;
pub mod mcp;
pub mod serve;
pub mod spec;
pub mod task;
pub mod tokens;
pub mod update;

/// Claude Workflow Architect - Development Workflow Orchestration
#[derive(Parser)]
#[command(name = "cwa")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Path to project directory (defaults to current directory)
    #[arg(short, long, global = true)]
    pub project: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new CWA project
    Init(init::InitArgs),

    /// Manage specifications (SDD)
    #[command(subcommand)]
    Spec(spec::SpecCommands),

    /// Domain modeling commands (DDD)
    #[command(subcommand)]
    Domain(domain::DomainCommands),

    /// Task management (Kanban)
    #[command(subcommand)]
    Task(task::TaskCommands),

    /// Memory management
    #[command(subcommand)]
    Memory(memory::MemoryCommands),

    /// Context status
    #[command(subcommand)]
    Context(context::ContextCommands),

    /// Analysis commands
    #[command(subcommand)]
    Analyze(analyze::AnalyzeCommands),

    /// Start the web server and MCP server
    Serve(serve::ServeArgs),

    /// MCP server commands
    #[command(subcommand)]
    Mcp(mcp::McpCommands),

    /// Knowledge Graph commands
    #[command(subcommand)]
    Graph(graph::GraphCommands),

    /// Design system commands
    #[command(subcommand)]
    Design(design::DesignCommands),

    /// Code generation commands
    #[command(subcommand)]
    Codegen(codegen::CodegenCommands),

    /// Token analysis commands
    #[command(subcommand)]
    Tokens(tokens::TokenCommands),

    /// Docker infrastructure management
    #[command(subcommand)]
    Infra(infra::InfraCommands),

    /// Git commands with Ollama-powered commit messages
    #[command(subcommand)]
    Git(git::GitCommands),

    /// Clean project (remove .cwa, .claude, CLAUDE.md, .mcp.json)
    Clean(clean::CleanArgs),

    /// Update project information and regenerate context files
    Update(update::UpdateArgs),
}

impl Cli {
    pub async fn execute(self) -> Result<()> {
        // For MCP commands, we need to find an existing CWA project
        // since Claude Desktop may run from "/" or another system directory
        let project_dir = if let Some(p) = self.project {
            p
        } else if matches!(self.command, Commands::Mcp(_)) {
            // For MCP commands, search up the directory tree for a CWA project
            find_cwa_project()?
        } else {
            std::env::current_dir()?
        };

        match self.command {
            Commands::Init(args) => init::execute(args).await,
            Commands::Spec(cmd) => spec::execute(cmd, &project_dir).await,
            Commands::Domain(cmd) => domain::execute(cmd, &project_dir).await,
            Commands::Task(cmd) => task::execute(cmd, &project_dir).await,
            Commands::Memory(cmd) => memory::execute(cmd, &project_dir).await,
            Commands::Context(cmd) => context::execute(cmd, &project_dir).await,
            Commands::Analyze(cmd) => analyze::execute(cmd, &project_dir).await,
            Commands::Serve(args) => serve::execute(args, &project_dir).await,
            Commands::Mcp(cmd) => mcp::execute(cmd, &project_dir).await,
            Commands::Graph(cmd) => graph::execute(cmd, &project_dir).await,
            Commands::Design(cmd) => design::execute(cmd, &project_dir).await,
            Commands::Codegen(cmd) => codegen::execute(cmd, &project_dir).await,
            Commands::Tokens(cmd) => tokens::execute(cmd, &project_dir).await,
            Commands::Infra(cmd) => infra::execute(cmd, &project_dir).await,
            Commands::Git(cmd) => git::execute(cmd).await,
            Commands::Clean(args) => clean::execute(args, &project_dir).await,
            Commands::Update(args) => update::execute(args, &project_dir).await,
        }
    }
}
