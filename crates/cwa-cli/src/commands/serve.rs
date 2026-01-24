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

    println!(
        "{} Starting CWA server on {}:{}",
        "→".blue().bold(),
        args.host,
        args.port
    );
    println!();
    println!("  Dashboard: http://{}:{}", args.host, args.port);
    println!("  API:       http://{}:{}/api", args.host, args.port);
    println!();
    println!("{}", "Press Ctrl+C to stop".dimmed());

    cwa_web::run_server(pool, args.port).await?;

    Ok(())
}
