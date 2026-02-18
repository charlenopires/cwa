//! Tech-stack-aware agent template selection and generation.
//!
//! Provides pre-built agent templates for common technology stacks.
//! Templates are selected based on the project's `tech_stack` field and
//! written to `.claude/agents/` alongside the domain-specific agents.
//!
//! ## Supported stacks
//! - **Rust**: axum, tokio, sqlx, general rust, AI/ML
//! - **Elixir**: phoenix, liveview, ecto, general elixir, AI/ML
//! - **TypeScript**: react, nextjs, bun, vite, typescript
//! - **Python**: fastapi, general python, AI/ML
//! - **Common**: ddd, tdd, security, docker, htmx, tailwindcss, shadcn-ui (always generated)

use anyhow::Result;
use std::path::Path;

/// A pre-built agent template tied to a set of technology keywords.
pub struct TechAgentTemplate {
    /// Output filename (e.g. "axum-expert.md").
    pub filename: &'static str,
    /// Technology keywords that trigger this template (matched case-insensitively).
    pub technologies: &'static [&'static str],
    /// Full markdown content of the agent file.
    pub content: &'static str,
}

/// All available tech-stack agent templates.
pub static ALL_TECH_AGENTS: &[TechAgentTemplate] = &[
    // ── Rust ────────────────────────────────────────────────────────────────
    TechAgentTemplate {
        filename: "rust-expert.md",
        technologies: &["rust"],
        content: include_str!("templates/agents/rust/rust-expert.md"),
    },
    TechAgentTemplate {
        filename: "axum-expert.md",
        technologies: &["rust", "axum"],
        content: include_str!("templates/agents/rust/axum-expert.md"),
    },
    TechAgentTemplate {
        filename: "tokio-expert.md",
        technologies: &["rust", "tokio"],
        content: include_str!("templates/agents/rust/tokio-expert.md"),
    },
    TechAgentTemplate {
        filename: "sqlx-expert.md",
        technologies: &["rust", "sqlx"],
        content: include_str!("templates/agents/rust/sqlx-expert.md"),
    },
    TechAgentTemplate {
        filename: "rust-ai-expert.md",
        technologies: &["rust", "candle", "onnx", "ml"],
        content: include_str!("templates/agents/rust/rust-ai-expert.md"),
    },
    // ── Elixir ──────────────────────────────────────────────────────────────
    TechAgentTemplate {
        filename: "elixir-expert.md",
        technologies: &["elixir"],
        content: include_str!("templates/agents/elixir/elixir-expert.md"),
    },
    TechAgentTemplate {
        filename: "phoenix-expert.md",
        technologies: &["elixir", "phoenix"],
        content: include_str!("templates/agents/elixir/phoenix-expert.md"),
    },
    TechAgentTemplate {
        filename: "liveview-expert.md",
        technologies: &["elixir", "liveview", "live_view"],
        content: include_str!("templates/agents/elixir/liveview-expert.md"),
    },
    TechAgentTemplate {
        filename: "ecto-expert.md",
        technologies: &["elixir", "ecto"],
        content: include_str!("templates/agents/elixir/ecto-expert.md"),
    },
    TechAgentTemplate {
        filename: "elixir-ai-expert.md",
        technologies: &["elixir", "nx", "bumblebee", "axon", "ml"],
        content: include_str!("templates/agents/elixir/elixir-ai-expert.md"),
    },
    // ── TypeScript / JavaScript ─────────────────────────────────────────────
    TechAgentTemplate {
        filename: "typescript-expert.md",
        technologies: &["typescript", "javascript"],
        content: include_str!("templates/agents/typescript/typescript-expert.md"),
    },
    TechAgentTemplate {
        filename: "react-expert.md",
        technologies: &["typescript", "javascript", "react"],
        content: include_str!("templates/agents/typescript/react-expert.md"),
    },
    TechAgentTemplate {
        filename: "nextjs-expert.md",
        technologies: &["typescript", "nextjs", "next.js"],
        content: include_str!("templates/agents/typescript/nextjs-expert.md"),
    },
    TechAgentTemplate {
        filename: "bun-expert.md",
        technologies: &["bun", "typescript", "javascript"],
        content: include_str!("templates/agents/typescript/bun-expert.md"),
    },
    TechAgentTemplate {
        filename: "vite-expert.md",
        technologies: &["vite", "typescript", "javascript"],
        content: include_str!("templates/agents/typescript/vite-expert.md"),
    },
    // ── Python ──────────────────────────────────────────────────────────────
    TechAgentTemplate {
        filename: "python-expert.md",
        technologies: &["python"],
        content: include_str!("templates/agents/python/python-expert.md"),
    },
    TechAgentTemplate {
        filename: "fastapi-expert.md",
        technologies: &["python", "fastapi"],
        content: include_str!("templates/agents/python/fastapi-expert.md"),
    },
    TechAgentTemplate {
        filename: "python-ai-expert.md",
        technologies: &["python", "langchain", "llamaindex", "huggingface", "ml", "ai"],
        content: include_str!("templates/agents/python/python-ai-expert.md"),
    },
    // ── Common (always included) ────────────────────────────────────────────
    TechAgentTemplate {
        filename: "ddd-expert.md",
        technologies: &[], // empty = always selected
        content: include_str!("templates/agents/common/ddd-expert.md"),
    },
    TechAgentTemplate {
        filename: "tdd-expert.md",
        technologies: &[],
        content: include_str!("templates/agents/common/tdd-expert.md"),
    },
    TechAgentTemplate {
        filename: "security-expert.md",
        technologies: &[],
        content: include_str!("templates/agents/common/security-expert.md"),
    },
    TechAgentTemplate {
        filename: "docker-expert.md",
        technologies: &[], // docker is always useful
        content: include_str!("templates/agents/common/docker-expert.md"),
    },
    // ── SDD/DDD workflow agents (always included) ───────────────────────────
    TechAgentTemplate {
        filename: "spec-reviewer.md",
        technologies: &[],
        content: include_str!("templates/agents/common/spec-reviewer.md"),
    },
    TechAgentTemplate {
        filename: "kanban-manager.md",
        technologies: &[],
        content: include_str!("templates/agents/common/kanban-manager.md"),
    },
    TechAgentTemplate {
        filename: "memory-observer.md",
        technologies: &[],
        content: include_str!("templates/agents/common/memory-observer.md"),
    },
    // ── Optional common (selected by keyword) ──────────────────────────────
    TechAgentTemplate {
        filename: "htmx-expert.md",
        technologies: &["htmx"],
        content: include_str!("templates/agents/common/htmx-expert.md"),
    },
    TechAgentTemplate {
        filename: "tailwindcss-expert.md",
        technologies: &["tailwind", "tailwindcss", "css"],
        content: include_str!("templates/agents/common/tailwindcss-expert.md"),
    },
    TechAgentTemplate {
        filename: "shadcn-ui-expert.md",
        technologies: &["shadcn", "shadcn-ui", "react", "typescript"],
        content: include_str!("templates/agents/common/shadcn-ui-expert.md"),
    },
];

/// A generated tech-stack agent file ready to write to disk.
#[derive(Debug, Clone)]
pub struct TechAgent {
    pub filename: String,
    pub content: &'static str,
}

/// Select agent templates compatible with the given tech stack.
///
/// Templates with an empty `technologies` slice are always selected.
/// Otherwise, at least one technology keyword must match a term in `tech_stack`.
pub fn select_agents_for_stack(tech_stack: &[String]) -> Vec<TechAgent> {
    let stack_lower: Vec<String> = tech_stack.iter().map(|s| s.to_lowercase()).collect();

    ALL_TECH_AGENTS
        .iter()
        .filter(|t| {
            // Always include common agents (empty technologies)
            t.technologies.is_empty()
                // Or at least one tech keyword matches a stack entry
                || t.technologies.iter().any(|tech| {
                    stack_lower.iter().any(|s| s.contains(tech))
                })
        })
        .map(|t| TechAgent {
            filename: t.filename.to_string(),
            content: t.content,
        })
        .collect()
}

/// Write selected tech agent files to the `.claude/agents/` directory.
///
/// Returns the list of written file paths.
pub fn write_tech_agents(agents: &[TechAgent], output_dir: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(output_dir)?;
    let mut written = Vec::new();
    for agent in agents {
        let path = output_dir.join(&agent.filename);
        std::fs::write(&path, agent.content)?;
        written.push(path.display().to_string());
    }
    Ok(written)
}
