//! Web server command.

use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;

#[derive(Args)]
pub struct ServeArgs {
    /// Port to listen on
    #[arg(long, default_value = "3030")]
    pub port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
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
    println!();
    println!("  {}", "Ctrl+C to stop".dimmed());
    println!();

    cwa_web::run_server(pool, args.port).await?;

    Ok(())
}
