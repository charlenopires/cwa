//! Generate Claude Code command files.
//!
//! Commands are slash commands that provide quick-access workflows.

use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// A generated command definition.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedCommand {
    pub filename: String,
    pub content: String,
    pub name: String,
}

/// Generate the /generate-tasks command.
fn generate_tasks_command() -> GeneratedCommand {
    let content = r#"# /generate-tasks

Generate tasks from a specification's acceptance criteria.

## Usage

```
/generate-tasks <spec-id>
```

## Steps

1. Get the spec details using MCP tool `cwa_get_spec` with the provided spec ID
2. Analyze each acceptance criterion in the spec
3. For each criterion, create a task using MCP tool `cwa_create_task`:
   - Title: Based on the criterion
   - Description: Include the criterion text and any context
   - Link to the spec ID
4. Report the created tasks to the user

## Example

```
/generate-tasks spec-123
```

This will create individual tasks from spec-123's acceptance criteria.
"#;

    GeneratedCommand {
        filename: "generate-tasks.md".to_string(),
        content: content.to_string(),
        name: "generate-tasks".to_string(),
    }
}

/// Generate the /run-backlog command.
fn run_backlog_command() -> GeneratedCommand {
    let content = r#"# /run-backlog

Plan and execute all tasks in the backlog.

## Usage

```
/run-backlog [--dry-run]
```

## Steps

1. Get the current board state using MCP tool `cwa_get_context_summary`
2. List all tasks with status "backlog" or "todo"
3. For each task in order:
   a. Move the task to "in_progress" using `cwa_update_task_status`
   b. Get task details with `cwa_get_current_task`
   c. Plan the implementation approach
   d. Execute the implementation
   e. Verify the task is complete
   f. Move to "review" then "done" as appropriate
4. Report progress after each task

## Options

- `--dry-run`: Only show which tasks would be executed without making changes

## Notes

- Respects WIP limits (only 1 task in_progress at a time)
- Will pause and ask for input if blocked or uncertain
- Uses the project's domain model and specs for context
"#;

    GeneratedCommand {
        filename: "run-backlog.md".to_string(),
        content: content.to_string(),
        name: "run-backlog".to_string(),
    }
}

/// Generate the /project-status command.
fn project_status_command() -> GeneratedCommand {
    let content = r#"# /project-status

Show current project status including specs, tasks, and domain model.

## Usage

```
/project-status
```

## Steps

1. Call MCP tool `cwa_get_context_summary` to get overall status
2. Display:
   - Active specs with acceptance criteria progress
   - Task board summary (counts per column)
   - Current in-progress work
   - Recent observations/decisions
"#;

    GeneratedCommand {
        filename: "project-status.md".to_string(),
        content: content.to_string(),
        name: "project-status".to_string(),
    }
}

/// Generate the /next-task command.
fn next_task_command() -> GeneratedCommand {
    let content = r#"# /next-task

Get and start working on the next available task using the CWA Kanban workflow.

## Usage

```
/next-task
```

## Steps

1. Run `cwa task wip` to verify WIP capacity before starting
2. Call `cwa_get_next_steps` to identify the next available work
3. If a task is available:
   a. `cwa task move <id> todo` (ensure it's in todo first)
   b. `cwa task move <id> in_progress`
   c. `cwa task board` to confirm the move
   d. Load task details via `cwa_get_current_task`
   e. Load spec via `cwa_get_spec` if the task is linked to a spec
   f. Begin TDD: write failing test first, then implement
4. When work is complete:
   a. `cwa task move <id> review`
   b. Run tests and get review approval
   c. `cwa task move <id> done`
   d. `cwa task board` to see updated board
5. If no tasks available, suggest `cwa task generate <spec-id>` or creating new specs
"#;

    GeneratedCommand {
        filename: "next-task.md".to_string(),
        content: content.to_string(),
        name: "next-task".to_string(),
    }
}

/// Generate the /kanban command.
fn kanban_command() -> GeneratedCommand {
    let content = r#"# /kanban

Display the Kanban board and manage task flow.

## Usage

```
/kanban
```

## Steps

1. Run `cwa task board` for a terminal view of the Kanban board
2. Call `cwa_get_wip_status` for WIP limits and current counts per column
3. Call `cwa_list_tasks` for full task details per column
4. Open http://127.0.0.1:3030 for the web view (if `cwa serve` is running)

## Tips

- Tasks in **in_progress** that exceed WIP limits slow the team — resolve them first
- Use `cwa task move <id> <column>` to move tasks between columns
- Run `cwa task wip` to check WIP limit status quickly
"#;

    GeneratedCommand {
        filename: "kanban.md".to_string(),
        content: content.to_string(),
        name: "kanban".to_string(),
    }
}

/// Generate the /wip-check command.
fn wip_check_command() -> GeneratedCommand {
    let content = r#"# /wip-check

Verify WIP limits and flag violations to maintain flow efficiency.

## Usage

```
/wip-check
```

## Steps

1. Call `cwa_get_wip_status` to get limits and current counts per column
2. Flag any column at or over its WIP limit
3. Suggest which tasks to move to restore flow:
   - Move blocked tasks back to `todo`
   - Prioritize completing `review` tasks before starting new ones
4. Run `cwa task wip` for a terminal summary

## WIP Limit Guidelines

- **in_progress**: max 2 (focus on completion, not starting)
- **review**: max 3 (unreviewed work is unfinished work)
- Columns at 100% capacity should trigger a stop-and-fix response
"#;

    GeneratedCommand {
        filename: "wip-check.md".to_string(),
        content: content.to_string(),
        name: "wip-check".to_string(),
    }
}

/// Generate the /sync command.
fn sync_command() -> GeneratedCommand {
    let content = r#"# /sync

Sync project data to the knowledge graph and vector store, then regenerate CLAUDE.md.

## Usage

```
/sync
```

## Steps

1. Call `cwa_graph_sync` to sync Redis → Neo4j knowledge graph
2. Run `cwa memory sync` to update CLAUDE.md with current observations
3. Run `cwa codegen claude-md` to regenerate CLAUDE.md from current state
4. Report sync summary: contexts synced, specs synced, memories indexed

## When to Use

- After a major batch of domain modeling (`cwa domain context new`, `cwa domain object new`)
- After recording many observations (`cwa memory observe`)
- Before a planning session to ensure CLAUDE.md reflects current state
- After `cwa codegen all` to keep the knowledge graph up to date
"#;

    GeneratedCommand {
        filename: "sync.md".to_string(),
        content: content.to_string(),
        name: "sync".to_string(),
    }
}

/// Generate the /spec-review command.
fn spec_review_command() -> GeneratedCommand {
    let content = r#"# /spec-review

Review a specification for SDD completeness and quality.

## Usage

```
/spec-review <spec-id>
```

## Steps

1. Call `cwa_get_spec` with the provided spec ID
2. Call `cwa_validate_spec` to run automated validation
3. Review each acceptance criterion against quality rules:
   - Is it testable (can you write an automated test)?
   - Is it specific (no vague terms like "fast" or "good")?
   - Does it use Given-When-Then or "Should" format?
4. Check that the spec is linked to a bounded context
5. Report:
   - **Status**: READY / NEEDS WORK
   - **Issues found**: List each gap with a concrete suggestion
   - **Suggested criteria**: Draft any missing acceptance criteria
6. If spec is ready, suggest moving it to `active` status

## Example

```
/spec-review abc-123
```

This will review spec abc-123 and provide a quality assessment.
"#;

    GeneratedCommand {
        filename: "spec-review.md".to_string(),
        content: content.to_string(),
        name: "spec-review".to_string(),
    }
}

/// Generate the /domain-model command.
fn domain_model_command() -> GeneratedCommand {
    let content = r#"# /domain-model

Display the complete domain model for the current project.

## Usage

```
/domain-model
```

## Steps

1. Call `cwa_get_domain_model` to get the full domain model
2. Display a structured overview:
   - **Bounded Contexts**: Name, description, responsibilities
   - **Domain Objects per context**: Entities, aggregates, value objects, services, events
   - **Context Map**: Relationships between contexts (partnership, ACL, conformist, etc.)
   - **Glossary**: Key ubiquitous language terms
3. Identify and highlight:
   - Core domain (highest business value)
   - Supporting subdomains
   - Generic subdomains (candidates for off-the-shelf solutions)
4. Suggest improvements if gaps are detected

## Tips

- Run after `cwa domain context new` to verify the model is correct
- Use to onboard new contributors to the domain model
- Reference when creating new specs to ensure correct context association
"#;

    GeneratedCommand {
        filename: "domain-model.md".to_string(),
        content: content.to_string(),
        name: "domain-model".to_string(),
    }
}

/// Generate the /observe command.
fn observe_command() -> GeneratedCommand {
    let content = r#"# /observe

Record a development observation, decision, or insight into CWA memory.

## Usage

```
/observe
```

## Steps

1. Ask the user what they want to record (or summarize the current session)
2. Classify the observation:
   - **discovery**: Something unexpected found during development
   - **decision**: An architectural or design choice made
   - **issue**: A problem identified that needs tracking
   - **improvement**: A pattern or approach that worked well
3. Call `cwa_observe` with:
   - `title`: One-line summary (imperative: "Discovered X causes Y")
   - `narrative`: 2-3 sentences with context and implications
   - `type`: One of the types above
   - `confidence`: 0.0 to 1.0 (how certain are you?)
4. Confirm the observation was recorded

## Examples

```
# Record a discovery
/observe
> Discovered that Redis SCAN is O(N) — use KEYS patterns sparingly on large datasets

# Record an architectural decision
/observe
> Decided to use Qdrant for vector search instead of pgvector due to better filtering
```

## When to Use

- After finding an unexpected bug or behavior
- After making a significant design decision
- Before ending a session (capture what you learned)
- After a code review reveals important patterns
"#;

    GeneratedCommand {
        filename: "observe.md".to_string(),
        content: content.to_string(),
        name: "observe".to_string(),
    }
}

/// Generate the /tech-stack command.
fn tech_stack_command() -> GeneratedCommand {
    let content = r#"# /tech-stack

View and understand the project's technology stack and which agents are available.

## Usage

```
/tech-stack
```

## Steps

1. Call `cwa_get_tech_stack` to retrieve the current tech stack
2. Display the tech stack with categorization:
   - **Languages**: Rust, Python, TypeScript, Elixir, etc.
   - **Frameworks**: Axum, Phoenix, FastAPI, React, etc.
   - **Databases**: PostgreSQL, Redis, Neo4j, Qdrant, etc.
   - **Infrastructure**: Docker, Kubernetes, etc.
3. List which tech-stack agents are available in `.claude/agents/` for this stack
4. Suggest running `cwa codegen all` if stack was recently updated to regenerate agents

## Updating the Tech Stack

To update the tech stack, use the `cwa update` CLI command:

```bash
cwa update
# Follow prompts to update tech_stack field
```

Then regenerate agents:
```bash
cwa codegen all
```
"#;

    GeneratedCommand {
        filename: "tech-stack.md".to_string(),
        content: content.to_string(),
        name: "tech-stack".to_string(),
    }
}

/// Generate all built-in commands.
pub fn generate_all_commands() -> Vec<GeneratedCommand> {
    vec![
        generate_tasks_command(),
        run_backlog_command(),
        project_status_command(),
        next_task_command(),
        spec_review_command(),
        domain_model_command(),
        observe_command(),
        tech_stack_command(),
        kanban_command(),
        wip_check_command(),
        sync_command(),
    ]
}

/// Write generated commands to disk.
pub fn write_commands(commands: &[GeneratedCommand], output_dir: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(output_dir)?;

    let mut written = Vec::new();

    for cmd in commands {
        let path = output_dir.join(&cmd.filename);
        std::fs::write(&path, &cmd.content)?;
        written.push(path.display().to_string());
    }

    Ok(written)
}
