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

Get and start working on the next available task.

## Usage

```
/next-task
```

## Steps

1. Call MCP tool `cwa_get_next_steps` to identify next available work
2. If a task is available:
   a. Move it to "in_progress" using `cwa_update_task_status`
   b. Display task details and context
   c. Begin implementation planning
3. If no tasks available, suggest creating new tasks or specs
"#;

    GeneratedCommand {
        filename: "next-task.md".to_string(),
        content: content.to_string(),
        name: "next-task".to_string(),
    }
}

/// Generate all built-in commands.
pub fn generate_all_commands() -> Vec<GeneratedCommand> {
    vec![
        generate_tasks_command(),
        run_backlog_command(),
        project_status_command(),
        next_task_command(),
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
