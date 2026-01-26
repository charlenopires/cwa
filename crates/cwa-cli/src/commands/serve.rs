//! Web server command.

use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Args)]
pub struct ServeArgs {
    /// Port to listen on
    #[arg(long, default_value = "3030")]
    pub port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Enable logging to file (.cwa/serve.log)
    #[arg(long)]
    pub log: bool,

    /// Custom log file path
    #[arg(long)]
    pub log_file: Option<PathBuf>,
}

pub async fn execute(args: ServeArgs, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");
    let pool = Arc::new(cwa_db::init_pool(&db_path)?);

    println!();
    println!(
        "  {} {}",
        "CWA".cyan().bold(),
        "Web Server".bold()
    );
    println!();
    println!(
        "  {}  http://{}:{}",
        "Dashboard".green(),
        args.host,
        args.port
    );
    println!(
        "  {}       http://{}:{}/api",
        "API".green(),
        args.host,
        args.port
    );
    println!(
        "  {}  ws://{}:{}/ws",
        "WebSocket".green(),
        args.host,
        args.port
    );

    if args.log {
        let log_path = args
            .log_file
            .clone()
            .unwrap_or_else(|| project_dir.join(".cwa/serve.log"));
        println!();
        println!(
            "  {}    {}",
            "Logging".yellow(),
            log_path.display()
        );
    }

    println!();
    println!("  {}", "Ctrl+C to stop".dimmed());
    println!();

    cwa_web::run_server(pool, args.port).await?;

    Ok(())
}
