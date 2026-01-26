//! # CWA Codegen
//!
//! Generates Claude Code artifacts from the domain model.
//!
//! Produces subagents, skills, hooks, commands, and CLAUDE.md files
//! based on bounded contexts, specs, and domain objects.

pub mod agents;
pub mod claude_md;
pub mod commands;
pub mod design_system;
pub mod hooks;
pub mod skills;

pub use agents::{GeneratedAgent, generate_agent, generate_all_agents, write_agents};
pub use claude_md::{GeneratedClaudeMd, generate_claude_md, write_claude_md};
pub use commands::{GeneratedCommand, generate_all_commands, write_commands};
pub use design_system::{GeneratedDesignSystem, generate_design_system_md, write_design_system_md};
pub use hooks::{GeneratedHooks, generate_hooks, write_hooks};
pub use skills::{GeneratedSkill, generate_skill, generate_all_skills, write_skills};
