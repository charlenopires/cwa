//! # CWA Codegen
//!
//! Generates Claude Code artifacts from the domain model.
//!
//! Produces subagents, skills, hooks, and CLAUDE.md files
//! based on bounded contexts, specs, and domain objects.

pub mod agents;
pub mod skills;
pub mod hooks;
pub mod claude_md;

pub use agents::{GeneratedAgent, generate_agent, generate_all_agents, write_agents};
pub use skills::{GeneratedSkill, generate_skill, generate_all_skills, write_skills};
pub use hooks::{GeneratedHooks, generate_hooks, write_hooks};
pub use claude_md::{GeneratedClaudeMd, generate_claude_md, write_claude_md};
