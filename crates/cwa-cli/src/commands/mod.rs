//! CLI command definitions and handlers.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod analyze;
pub mod codegen;
pub mod context;
pub mod design;
pub mod domain;
pub mod graph;
pub mod infra;
pub mod init;
pub mod memory;
pub mod mcp;
pub mod serve;
pub mod spec;
pub mod task;
pub mod tokens;

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
}

impl Cli {
    pub async fn execute(self) -> Result<()> {
        let project_dir = self
            .project
            .unwrap_or_else(|| std::env::current_dir().unwrap());

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
        }
    }
}
