//! Docker infrastructure management CLI commands.

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Subcommand)]
pub enum InfraCommands {
    /// Start Docker infrastructure (Neo4j, Qdrant, Ollama)
    Up,

    /// Stop Docker infrastructure
    Down,

    /// Show status of all services
    Status,

    /// Show logs from a service
    Logs {
        /// Service name (neo4j, qdrant, ollama)
        service: Option<String>,
        /// Follow log output
        #[arg(long, short)]
        follow: bool,
    },

    /// Reset infrastructure (destroys all data)
    Reset {
        /// Confirm destructive operation
        #[arg(long)]
        confirm: bool,
    },
}

/// Service definitions for health checking.
struct ServiceInfo {
    name: &'static str,
    health_url: &'static str,
}

const SERVICES: &[ServiceInfo] = &[
    ServiceInfo { name: "neo4j", health_url: "http://localhost:7474" },
    ServiceInfo { name: "qdrant", health_url: "http://localhost:6333/healthz" },
    ServiceInfo { name: "ollama", health_url: "http://localhost:11434/api/tags" },
];

pub async fn execute(cmd: InfraCommands, project_dir: &Path) -> Result<()> {
    match cmd {
        InfraCommands::Up => cmd_up(project_dir).await,
        InfraCommands::Down => cmd_down(project_dir),
        InfraCommands::Status => cmd_status().await,
        InfraCommands::Logs { service, follow } => cmd_logs(project_dir, service, follow),
        InfraCommands::Reset { confirm } => cmd_reset(project_dir, confirm),
    }
}

/// Find the docker-compose.yml file.
fn find_compose_file(project_dir: &Path) -> Result<PathBuf> {
    // Check project-local docker directory
    let local = project_dir.join("docker/docker-compose.yml");
    if local.exists() {
        return Ok(local);
    }

    // Check .cwa directory (for initialized projects)
    let cwa_dir = project_dir.join(".cwa/docker/docker-compose.yml");
    if cwa_dir.exists() {
        return Ok(cwa_dir);
    }

    bail!(
        "docker-compose.yml not found. Searched:\n  - {}\n  - {}\n\nRun 'cwa init --with-graph' or ensure docker/ directory exists.",
        local.display(),
        cwa_dir.display()
    )
}

/// Start all infrastructure services.
async fn cmd_up(project_dir: &Path) -> Result<()> {
    let compose_file = find_compose_file(project_dir)?;
    let compose_dir = compose_file.parent().unwrap();

    println!("{}", "Starting CWA infrastructure...".bold());

    // Copy .env.example to .env if .env doesn't exist
    let env_file = compose_dir.join(".env");
    let env_example = compose_dir.join(".env.example");
    if !env_file.exists() && env_example.exists() {
        std::fs::copy(&env_example, &env_file)
            .context("Failed to create .env from .env.example")?;
        println!("  Created .env from .env.example");
    }

    // Run docker compose up
    let status = Command::new("docker")
        .args(["compose", "-f", compose_file.to_str().unwrap(), "up", "-d"])
        .status()
        .context("Failed to run 'docker compose'. Is Docker installed?")?;

    if !status.success() {
        bail!("docker compose up failed with exit code: {}", status);
    }

    println!("\n{}", "Waiting for services to be healthy...".dimmed());

    // Wait for services to be healthy
    for service in SERVICES {
        print!("  {} ... ", service.name);
        if wait_for_health(service.health_url, 60).await {
            println!("{}", "healthy".green());
        } else {
            println!("{}", "timeout".red());
        }
    }

    // Pull the embedding model
    println!("\n{}", "Pulling embedding model (nomic-embed-text)...".dimmed());
    let pull_status = Command::new("docker")
        .args(["exec", "cwa-ollama", "ollama", "pull", "nomic-embed-text"])
        .status();

    match pull_status {
        Ok(s) if s.success() => println!("  {}", "Model ready".green()),
        _ => println!("  {} (you can pull it manually: docker exec cwa-ollama ollama pull nomic-embed-text)", "Model pull failed".yellow()),
    }

    // Run Qdrant init script
    let init_script = compose_dir.join("scripts/init-qdrant.sh");
    if init_script.exists() {
        println!("\n{}", "Initializing Qdrant collections...".dimmed());
        let _ = Command::new("bash")
            .arg(init_script.to_str().unwrap())
            .status();
    }

    println!("\n{}", "Infrastructure ready.".green().bold());
    println!("  Neo4j Browser: http://localhost:7474");
    println!("  Qdrant API:    http://localhost:6333");
    println!("  Ollama API:    http://localhost:11434");

    Ok(())
}

/// Stop all infrastructure services.
fn cmd_down(project_dir: &Path) -> Result<()> {
    let compose_file = find_compose_file(project_dir)?;

    println!("{}", "Stopping CWA infrastructure...".bold());

    let status = Command::new("docker")
        .args(["compose", "-f", compose_file.to_str().unwrap(), "down"])
        .status()
        .context("Failed to run 'docker compose'")?;

    if !status.success() {
        bail!("docker compose down failed");
    }

    println!("{}", "Infrastructure stopped.".green());
    Ok(())
}

/// Check health status of all services.
async fn cmd_status() -> Result<()> {
    println!("{}", "CWA Infrastructure Status".bold());
    println!("{}", "─".repeat(40));

    for service in SERVICES {
        let status = check_health(service.health_url).await;
        let indicator = if status { "●".green() } else { "●".red() };
        let label = if status { "running".green() } else { "stopped".red() };
        println!("  {} {:<10} {}", indicator, service.name, label);
    }

    // Check if ollama has the model
    if check_health("http://localhost:11434/api/tags").await {
        let has_model = check_ollama_model().await;
        let model_status = if has_model { "ready".green() } else { "not pulled".yellow() };
        println!("  {} {:<10} {}", "  ".dimmed(), "model", model_status);
    }

    println!("{}", "─".repeat(40));
    Ok(())
}

/// Show logs from Docker services.
fn cmd_logs(project_dir: &Path, service: Option<String>, follow: bool) -> Result<()> {
    let compose_file = find_compose_file(project_dir)?;

    let mut args = vec!["compose", "-f", compose_file.to_str().unwrap(), "logs"];

    if follow {
        args.push("-f");
    }

    args.push("--tail");
    args.push("100");

    let service_str;
    if let Some(ref svc) = service {
        service_str = svc.clone();
        args.push(&service_str);
    }

    let status = Command::new("docker")
        .args(&args)
        .status()
        .context("Failed to run 'docker compose logs'")?;

    if !status.success() {
        bail!("docker compose logs failed");
    }

    Ok(())
}

/// Reset infrastructure (destroy all data).
fn cmd_reset(project_dir: &Path, confirm: bool) -> Result<()> {
    if !confirm {
        println!("{}", "This will destroy all Neo4j, Qdrant, and Ollama data!".red().bold());
        println!("Run with {} to confirm.", "--confirm".bold());
        return Ok(());
    }

    let compose_file = find_compose_file(project_dir)?;

    println!("{}", "Resetting CWA infrastructure...".red().bold());

    let status = Command::new("docker")
        .args(["compose", "-f", compose_file.to_str().unwrap(), "down", "-v"])
        .status()
        .context("Failed to run 'docker compose'")?;

    if !status.success() {
        bail!("docker compose down -v failed");
    }

    println!("{}", "Infrastructure reset complete. All data destroyed.".yellow());
    Ok(())
}

/// Wait for a health endpoint to respond, with timeout in seconds.
async fn wait_for_health(url: &str, timeout_secs: u64) -> bool {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            return false;
        }

        if check_health(url).await {
            return true;
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

/// Check if a health endpoint responds with 2xx.
async fn check_health(url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(_) => return false,
    };

    match client.get(url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Check if Ollama has the nomic-embed-text model.
async fn check_ollama_model() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                text.contains("nomic-embed-text")
            } else {
                false
            }
        }
        Err(_) => false,
    }
}
