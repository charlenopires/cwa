//! Memory and context management.

pub mod model;

use crate::error::CwaResult;
use crate::task;
use crate::spec;
use crate::decision;
use cwa_db::DbPool;
use cwa_db::queries::memory as queries;
use cwa_db::queries::projects as project_queries;
use model::{MemoryEntry, Session, ContextSummary};
use uuid::Uuid;

/// Create a memory entry.
pub fn add_memory(
    pool: &DbPool,
    project_id: &str,
    entry_type: &str,
    content: &str,
    importance: &str,
    session_id: Option<&str>,
) -> CwaResult<()> {
    let id = Uuid::new_v4().to_string();
    queries::create_memory_entry(pool, &id, project_id, entry_type, content, importance, session_id)?;
    Ok(())
}

/// List memory entries.
pub fn list_memory(pool: &DbPool, project_id: &str, limit: Option<i64>) -> CwaResult<Vec<MemoryEntry>> {
    let rows = queries::list_memory(pool, project_id, limit)?;
    Ok(rows.into_iter().map(MemoryEntry::from_row).collect())
}

/// Search memory by query.
pub fn search_memory(pool: &DbPool, project_id: &str, query: &str) -> CwaResult<Vec<MemoryEntry>> {
    let rows = queries::search_memory(pool, project_id, query)?;
    Ok(rows.into_iter().map(MemoryEntry::from_row).collect())
}

/// Start a new session.
pub fn start_session(pool: &DbPool, project_id: &str, goals: Option<&str>) -> CwaResult<Session> {
    let id = Uuid::new_v4().to_string();
    queries::create_session(pool, &id, project_id, goals)?;

    Ok(Session {
        id,
        project_id: project_id.to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        ended_at: None,
        summary: None,
        goals: goals.map(String::from).map(|g| vec![g]).unwrap_or_default(),
        accomplishments: Vec::new(),
    })
}

/// End a session.
pub fn end_session(pool: &DbPool, id: &str, summary: Option<&str>, accomplishments: Option<&str>) -> CwaResult<()> {
    queries::end_session(pool, id, summary, accomplishments)?;
    Ok(())
}

/// Get the active session.
pub fn get_active_session(pool: &DbPool, project_id: &str) -> CwaResult<Option<Session>> {
    let row = queries::get_active_session(pool, project_id)?;
    Ok(row.map(Session::from_row))
}

/// Get a compact context summary for the project.
pub fn get_context_summary(pool: &DbPool, project_id: &str) -> CwaResult<ContextSummary> {
    // Get project info
    let project = project_queries::get_project(pool, project_id)?;

    // Get current task
    let current_task = task::get_current_task(pool, project_id)?;

    // Get active spec
    let active_spec = spec::get_active_spec(pool, project_id)?;

    // Get recent decisions (accepted only)
    let decisions = decision::list_accepted_decisions(pool, project_id)?;

    // Get task counts by status
    let tasks = task::list_tasks(pool, project_id)?;
    let task_counts = TaskCounts {
        backlog: tasks.iter().filter(|t| t.status == task::model::TaskStatus::Backlog).count(),
        todo: tasks.iter().filter(|t| t.status == task::model::TaskStatus::Todo).count(),
        in_progress: tasks.iter().filter(|t| t.status == task::model::TaskStatus::InProgress).count(),
        review: tasks.iter().filter(|t| t.status == task::model::TaskStatus::Review).count(),
        done: tasks.iter().filter(|t| t.status == task::model::TaskStatus::Done).count(),
    };

    // Get recent memory entries
    let memory = list_memory(pool, project_id, Some(10))?;

    Ok(ContextSummary {
        project_name: project.name,
        current_task: current_task.map(|t| format!("{}: {}", t.id, t.title)),
        active_spec: active_spec.map(|s| format!("{}: {}", s.id, s.title)),
        task_counts,
        recent_decisions: decisions.into_iter().take(3).map(|d| d.title).collect(),
        recent_insights: memory.into_iter()
            .filter(|m| m.entry_type == "insight")
            .take(5)
            .map(|m| m.content)
            .collect(),
    })
}

/// Suggest next steps based on current state.
pub fn suggest_next_steps(pool: &DbPool, project_id: &str) -> CwaResult<Vec<String>> {
    let mut suggestions = Vec::new();

    // Check if there's a task in progress
    let current_task = task::get_current_task(pool, project_id)?;
    if current_task.is_none() {
        // No task in progress, suggest starting one
        let tasks = cwa_db::queries::tasks::list_tasks_by_status(pool, project_id, "todo")?;
        if !tasks.is_empty() {
            suggestions.push(format!(
                "Start working on task: {} ({})",
                tasks[0].title, tasks[0].id
            ));
        } else {
            suggestions.push("No tasks in 'todo' column. Consider moving tasks from backlog.".to_string());
        }
    } else {
        let task = current_task.unwrap();
        suggestions.push(format!("Continue working on: {} ({})", task.title, task.id));

        // Check if task has been in progress for a while
        if let Some(_started) = &task.started_at {
            suggestions.push("Consider: Is this task blocked? Does it need to be split?".to_string());
        }
    }

    // Check for specs without tasks
    let specs = spec::list_specs(pool, project_id)?;
    for spec in specs.iter().filter(|s| s.status == spec::model::SpecStatus::Active) {
        suggestions.push(format!("Active spec '{}' may need tasks created", spec.title));
    }

    Ok(suggestions)
}

/// Clean up expired memory entries.
pub fn cleanup_memory(pool: &DbPool) -> CwaResult<usize> {
    let count = queries::cleanup_expired_memory(pool)?;
    Ok(count)
}

/// Task count statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskCounts {
    pub backlog: usize,
    pub todo: usize,
    pub in_progress: usize,
    pub review: usize,
    pub done: usize,
}
