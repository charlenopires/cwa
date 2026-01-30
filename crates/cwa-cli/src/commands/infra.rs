//! Docker infrastructure management CLI commands.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde_json;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Subcommand)]
pub enum InfraCommands {
    /// Start Docker infrastructure (Neo4j, Qdrant, Ollama)
    Up,

    /// Stop Docker infrastructure
    Down {
        /// Also remove volumes (data) and images
        #[arg(long)]
        clean: bool,
    },

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

    /// Manage Ollama models
    Models {
        #[command(subcommand)]
        cmd: Option<ModelsCommands>,
    },
}

#[derive(Subcommand)]
pub enum ModelsCommands {
    /// Pull a model from Ollama registry
    Pull {
        /// Model name (e.g., qwen2.5-coder:3b)
        model: String,
    },
    /// Set default generation model
    Set {
        /// Model name to use as default
        model: String,
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
        InfraCommands::Down { clean } => cmd_down(project_dir, clean),
        InfraCommands::Status => cmd_status().await,
        InfraCommands::Logs { service, follow } => cmd_logs(project_dir, service, follow),
        InfraCommands::Reset { confirm } => cmd_reset(project_dir, confirm),
        InfraCommands::Models { cmd } => cmd_models(cmd).await,
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

    // Pull the generation model (for commit messages, etc)
    let gen_model = std::env::var("OLLAMA_GEN_MODEL").unwrap_or_else(|_| "qwen2.5-coder:3b".to_string());
    println!("\n{}", format!("Pulling generation model ({})...", gen_model).dimmed());
    let gen_pull_status = Command::new("docker")
        .args(["exec", "cwa-ollama", "ollama", "pull", &gen_model])
        .status();

    match gen_pull_status {
        Ok(s) if s.success() => println!("  {}", "Generation model ready".green()),
        _ => println!("  {} (you can pull it manually: cwa infra models pull {})", "Model pull failed".yellow(), gen_model),
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
fn cmd_down(project_dir: &Path, clean: bool) -> Result<()> {
    let compose_file = find_compose_file(project_dir)?;

    if clean {
        println!("{}", "Stopping and removing CWA infrastructure...".bold());

        // Stop containers and remove volumes
        let status = Command::new("docker")
            .args(["compose", "-f", compose_file.to_str().unwrap(), "down", "-v", "--rmi", "all"])
            .status()
            .context("Failed to run 'docker compose'")?;

        if !status.success() {
            bail!("docker compose down failed");
        }

        // Also remove any orphan containers with cwa- prefix
        let _ = Command::new("docker")
            .args(["rm", "-f", "cwa-neo4j", "cwa-qdrant", "cwa-ollama"])
            .status();

        println!("{}", "Infrastructure stopped, volumes and images removed.".green());
    } else {
        println!("{}", "Stopping CWA infrastructure...".bold());

        let status = Command::new("docker")
            .args(["compose", "-f", compose_file.to_str().unwrap(), "down"])
            .status()
            .context("Failed to run 'docker compose'")?;

        if !status.success() {
            bail!("docker compose down failed");
        }

        println!("{}", "Infrastructure stopped.".green());
        println!("{}", "  Use --clean to also remove volumes and images".dimmed());
    }

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

    // Check if ollama has the models
    if check_health("http://localhost:11434/api/tags").await {
        // Embedding model
        let has_embed = check_ollama_has_model("nomic-embed-text").await;
        let embed_status = if has_embed { "ready".green() } else { "not pulled".yellow() };
        println!("  {} {:<10} {} (embeddings)", "  ".dimmed(), "model", embed_status);

        // Generation model
        let gen_model = std::env::var("OLLAMA_GEN_MODEL")
            .unwrap_or_else(|_| "qwen2.5-coder:3b".to_string());
        let has_gen = check_ollama_has_model(&gen_model).await;
        let gen_status = if has_gen { "ready".green() } else { "not pulled".yellow() };
        println!("  {} {:<10} {} ({})", "  ".dimmed(), "gen", gen_status, gen_model);
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

/// Check if Ollama has a specific model.
async fn check_ollama_has_model(model: &str) -> bool {
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
                text.contains(model)
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// Get list of installed Ollama models.
async fn get_ollama_models() -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .context("Failed to create HTTP client")?;

    let resp = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .context("Failed to connect to Ollama. Is it running? Try: cwa infra up")?;

    let json: serde_json::Value = resp
        .json()
        .await
        .context("Failed to parse Ollama response")?;

    let models = json["models"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(models)
}

/// Manage Ollama models.
async fn cmd_models(cmd: Option<ModelsCommands>) -> Result<()> {
    match cmd {
        None => {
            // List models
            println!("{}", "Ollama Models".bold());
            println!("{}", "─".repeat(40));

            let models = get_ollama_models().await?;
            if models.is_empty() {
                println!("  {}", "No models installed".dimmed());
                println!("\n  Suggested models for CWA:");
                println!("    cwa infra models pull nomic-embed-text    # embeddings");
                println!("    cwa infra models pull qwen2.5-coder:3b    # commit messages");
            } else {
                let gen_model = std::env::var("OLLAMA_GEN_MODEL")
                    .unwrap_or_else(|_| "qwen2.5-coder:3b".to_string());

                for model in &models {
                    let suffix = if model == "nomic-embed-text" {
                        " (embeddings)".dimmed().to_string()
                    } else if model.contains(&gen_model) || model == &gen_model {
                        " (generation - active)".green().to_string()
                    } else if model.contains("qwen") || model.contains("coder") {
                        " (generation)".dimmed().to_string()
                    } else {
                        String::new()
                    };
                    println!("  {} {}{}", "●".green(), model, suffix);
                }
            }

            println!("{}", "─".repeat(40));
            Ok(())
        }
        Some(ModelsCommands::Pull { model }) => {
            println!("{}", format!("Pulling model: {}", model).bold());

            let status = Command::new("docker")
                .args(["exec", "cwa-ollama", "ollama", "pull", &model])
                .status()
                .context("Failed to run ollama pull. Is Docker running?")?;

            if status.success() {
                println!("{}", format!("Model {} ready", model).green());
            } else {
                bail!("Failed to pull model {}", model);
            }

            Ok(())
        }
        Some(ModelsCommands::Set { model }) => {
            // Check if model exists
            if !check_ollama_has_model(&model).await {
                println!("{}", format!("Model {} not found. Pulling...", model).yellow());
                let status = Command::new("docker")
                    .args(["exec", "cwa-ollama", "ollama", "pull", &model])
                    .status();

                if status.is_err() || !status.unwrap().success() {
                    bail!("Failed to pull model {}", model);
                }
            }

            println!("{}", format!("Set {} as default generation model", model).green());
            println!("\nTo persist this setting, add to your shell profile:");
            println!("  export OLLAMA_GEN_MODEL={}", model);

            Ok(())
        }
    }
}
