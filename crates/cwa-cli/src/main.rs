//! CWA CLI - Claude Workflow Architect
//!
//! A development workflow orchestration tool integrated with Claude Code.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod output;

use commands::{Cli, Commands};

/// Initialize tracing with optional file logging.
///
/// When `mcp_mode` is true, all tracing output goes to stderr with ANSI disabled
/// to prevent corrupting the JSON-RPC protocol on stdout.
fn init_tracing(log_file: Option<&std::path::Path>, mcp_mode: bool) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "cwa=info,cwa_web=debug,cwa_mcp=debug".into());

    if let Some(path) = log_file {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Set up file appender
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("Failed to open log file");

        // Log to both stdout and file when --log is used
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer()) // stdout
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::sync::Mutex::new(file))
                    .with_ansi(false),
            )
            .init();
    } else if mcp_mode {
        // MCP mode: write to stderr only, no ANSI codes
        // Prevents tracing output from corrupting JSON-RPC on stdout
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
                    .with_ansi(false),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check if serve command with --log
    let log_file = match &cli.command {
        Commands::Serve(args) if args.log => {
            let project_dir = cli
                .project
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap());
            Some(
                args.log_file
                    .clone()
                    .unwrap_or_else(|| project_dir.join(".cwa/serve.log")),
            )
        }
        _ => None,
    };

    let mcp_mode = matches!(&cli.command, Commands::Mcp(_));
    init_tracing(log_file.as_deref(), mcp_mode);

    cli.execute().await
}
