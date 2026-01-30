//! Git commands with Ollama-powered commit message generation.
//!
//! Uses local Ollama LLM to generate commit messages from git diff,
//! avoiding the need to spend Claude tokens for routine commits.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use colored::Colorize;
use std::process::Command;

/// Default Ollama API URL.
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Default generation model.
const DEFAULT_GEN_MODEL: &str = "qwen2.5-coder:3b";

#[derive(Subcommand)]
pub enum GitCommands {
    /// Generate commit message using local Ollama (doesn't commit)
    Msg {
        /// Use specific model
        #[arg(long)]
        model: Option<String>,
    },

    /// Commit with auto-generated message
    Commit {
        /// Use specific model
        #[arg(long)]
        model: Option<String>,

        /// Stage all changes before commit (-a)
        #[arg(short, long)]
        all: bool,

        /// Edit message before committing
        #[arg(short, long)]
        edit: bool,
    },

    /// Commit and push with auto-generated message
    Commitpush {
        /// Use specific model
        #[arg(long)]
        model: Option<String>,

        /// Stage all changes before commit
        #[arg(short, long)]
        all: bool,
    },
}

/// Ollama generation client.
struct OllamaGenerator {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaGenerator {
    /// Create a new Ollama generator with specified URL and model.
    fn new(model: Option<String>) -> Self {
        let base_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string());

        let model = model.unwrap_or_else(|| {
            std::env::var("OLLAMA_GEN_MODEL")
                .unwrap_or_else(|_| DEFAULT_GEN_MODEL.to_string())
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model,
            client,
        }
    }

    /// Generate text from a prompt.
    async fn generate(&self, prompt: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false
        });

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request_body)
            .send()
            .await
            .context("Failed to connect to Ollama. Is it running? Try: cwa infra up")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            if body.contains("model") && body.contains("not found") {
                bail!(
                    "Model '{}' not found. Pull it with: cwa infra models pull {}",
                    self.model,
                    self.model
                );
            }
            bail!("Ollama API error ({}): {}", status, body);
        }

        let result: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let response_text = result["response"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        Ok(response_text)
    }

    /// Generate a commit message from a git diff.
    async fn generate_commit_message(&self, diff: &str) -> Result<String> {
        let prompt = format!(
            r#"Generate a concise git commit message for the following changes.

Rules:
1. Use conventional commits format: type(scope): description
2. Types: feat, fix, docs, style, refactor, test, chore, perf, ci, build
3. Keep the first line under 72 characters
4. Be specific but concise
5. Return ONLY the commit message, nothing else

Changes:
{}

Commit message:"#,
            diff
        );

        let message = self.generate(&prompt).await?;

        // Clean up the response - remove any markdown formatting or extra text
        let message = message
            .trim()
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .lines()
            .next()
            .unwrap_or("")
            .to_string();

        Ok(message)
    }
}

/// Get git diff (staged or all changes).
fn get_git_diff(staged_only: bool) -> Result<String> {
    let args = if staged_only {
        vec!["diff", "--staged"]
    } else {
        vec!["diff"]
    };

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to run git diff")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed: {}", stderr);
    }

    let diff = String::from_utf8_lossy(&output.stdout).to_string();

    // If staged diff is empty, try unstaged
    if diff.trim().is_empty() && staged_only {
        return get_git_diff(false);
    }

    Ok(diff)
}

/// Truncate diff to reasonable size for LLM context.
fn truncate_diff(diff: &str, max_chars: usize) -> String {
    if diff.len() <= max_chars {
        return diff.to_string();
    }

    // Truncate but try to end at a line boundary
    let truncated = &diff[..max_chars];
    if let Some(last_newline) = truncated.rfind('\n') {
        format!(
            "{}\n\n... (truncated, {} more characters)",
            &truncated[..last_newline],
            diff.len() - last_newline
        )
    } else {
        format!("{}... (truncated)", truncated)
    }
}

/// Stage all changes.
fn stage_all() -> Result<()> {
    let status = Command::new("git")
        .args(["add", "-A"])
        .status()
        .context("Failed to run git add")?;

    if !status.success() {
        bail!("git add -A failed");
    }

    Ok(())
}

/// Create a git commit with the given message.
fn create_commit(message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["commit", "-m", message])
        .status()
        .context("Failed to run git commit")?;

    if !status.success() {
        bail!("git commit failed");
    }

    Ok(())
}

/// Push to remote.
fn git_push() -> Result<()> {
    let status = Command::new("git")
        .args(["push"])
        .status()
        .context("Failed to run git push")?;

    if !status.success() {
        bail!("git push failed");
    }

    Ok(())
}

/// Check if there are changes to commit.
fn has_changes() -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to run git status")?;

    Ok(!output.stdout.is_empty())
}

pub async fn execute(cmd: GitCommands) -> Result<()> {
    match cmd {
        GitCommands::Msg { model } => cmd_msg(model).await,
        GitCommands::Commit { model, all, edit } => cmd_commit(model, all, edit).await,
        GitCommands::Commitpush { model, all } => cmd_commitpush(model, all).await,
    }
}

/// Generate and display commit message.
async fn cmd_msg(model: Option<String>) -> Result<()> {
    let generator = OllamaGenerator::new(model.clone());

    println!(
        "{}",
        format!("Generating commit message using {}...", generator.model).dimmed()
    );

    let diff = get_git_diff(true)?;
    if diff.trim().is_empty() {
        bail!("No changes to commit. Stage changes with 'git add' first.");
    }

    let truncated_diff = truncate_diff(&diff, 4000);
    let message = generator.generate_commit_message(&truncated_diff).await?;

    println!("\n{}", "Generated commit message:".bold());
    println!("  {}", message.green());

    Ok(())
}

/// Commit with auto-generated message.
async fn cmd_commit(model: Option<String>, all: bool, edit: bool) -> Result<()> {
    if all {
        println!("{}", "Staging all changes...".dimmed());
        stage_all()?;
    }

    if !has_changes()? {
        bail!("No changes to commit.");
    }

    let generator = OllamaGenerator::new(model.clone());

    println!(
        "{}",
        format!("Generating commit message using {}...", generator.model).dimmed()
    );

    let diff = get_git_diff(true)?;
    if diff.trim().is_empty() {
        bail!("No staged changes. Use -a to stage all or 'git add' files first.");
    }

    let truncated_diff = truncate_diff(&diff, 4000);
    let message = generator.generate_commit_message(&truncated_diff).await?;

    println!("{} {}", "Message:".bold(), message.green());

    if edit {
        // Open editor for message
        println!("{}", "Opening editor for message editing...".dimmed());
        let status = Command::new("git")
            .args(["commit", "-e", "-m", &message])
            .status()
            .context("Failed to open editor")?;

        if !status.success() {
            bail!("Commit cancelled or failed");
        }
    } else {
        create_commit(&message)?;
    }

    println!("{}", "Committed successfully!".green().bold());

    Ok(())
}

/// Commit and push with auto-generated message.
async fn cmd_commitpush(model: Option<String>, all: bool) -> Result<()> {
    if all {
        println!("{}", "Staging all changes...".dimmed());
        stage_all()?;
    }

    if !has_changes()? {
        bail!("No changes to commit.");
    }

    let generator = OllamaGenerator::new(model.clone());

    println!(
        "{}",
        format!("Generating commit message using {}...", generator.model).dimmed()
    );

    let diff = get_git_diff(true)?;
    if diff.trim().is_empty() {
        bail!("No staged changes. Use -a to stage all or 'git add' files first.");
    }

    let truncated_diff = truncate_diff(&diff, 4000);
    let message = generator.generate_commit_message(&truncated_diff).await?;

    println!("{} {}", "Message:".bold(), message.green());

    create_commit(&message)?;
    println!("{}", "Committed successfully!".green());

    println!("{}", "Pushing to remote...".dimmed());
    git_push()?;
    println!("{}", "Pushed successfully!".green().bold());

    Ok(())
}
