//! Task management (Kanban).

pub mod model;

use crate::error::{CwaError, CwaResult};
use cwa_db::DbPool;
use cwa_db::queries::tasks as queries;
use model::{Task, TaskStatus, Board, BoardColumn, WipStatus};
use uuid::Uuid;

/// Default Kanban columns.
/// WIP limits: only in_progress is limited (solo dev mode).
/// Use `cwa task wip-set` to adjust limits as needed.
const DEFAULT_COLUMNS: &[(&str, Option<i64>)] = &[
    ("backlog", None),
    ("todo", None),           // No limit - queue of ready tasks
    ("in_progress", Some(1)), // WIP limit of 1 for focused work
    ("review", Some(3)),      // Allow a few items in review
    ("done", None),
];

/// Create a new task.
pub async fn create_task(
    pool: &DbPool,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    spec_id: Option<&str>,
    priority: &str,
) -> CwaResult<Task> {
    let id = Uuid::new_v4().to_string();

    queries::create_task(pool, &id, project_id, title, description, spec_id, priority).await?;

    let row = queries::get_task(pool, &id).await?;
    Ok(Task::from_row(row))
}

/// Get a task by ID.
pub async fn get_task(pool: &DbPool, id: &str) -> CwaResult<Task> {
    let row = queries::get_task(pool, id).await?;
    Ok(Task::from_row(row))
}

/// Get the current in-progress task.
pub async fn get_current_task(pool: &DbPool, project_id: &str) -> CwaResult<Option<Task>> {
    let row = queries::get_current_task(pool, project_id).await?;
    Ok(row.map(Task::from_row))
}

/// List all tasks for a project.
pub async fn list_tasks(pool: &DbPool, project_id: &str) -> CwaResult<Vec<Task>> {
    let rows = queries::list_tasks(pool, project_id).await?;
    Ok(rows.into_iter().map(Task::from_row).collect())
}

/// List tasks linked to a specific spec.
pub async fn list_tasks_by_spec(pool: &DbPool, spec_id: &str) -> CwaResult<Vec<Task>> {
    let rows = queries::list_tasks_by_spec(pool, spec_id).await?;
    Ok(rows.into_iter().map(Task::from_row).collect())
}

/// Move a task to a new status.
pub async fn move_task(pool: &DbPool, project_id: &str, task_id: &str, new_status: &str) -> CwaResult<()> {
    let task = queries::get_task(pool, task_id).await?;
    let current_status = TaskStatus::from_str(&task.status);
    let target_status = TaskStatus::from_str(new_status);

    // Validate transition
    if !current_status.can_transition_to(&target_status) {
        return Err(CwaError::InvalidStateTransition {
            from: task.status,
            to: new_status.to_string(),
        });
    }

    // Check WIP limit
    if let Some(limit) = queries::get_wip_limit(pool, project_id, new_status).await? {
        let current_count = queries::count_tasks_by_status(pool, project_id, new_status).await?;
        if current_count >= limit {
            return Err(CwaError::WipLimitExceeded {
                column: new_status.to_string(),
                limit,
                current: current_count,
            });
        }
    }

    queries::update_task_status(pool, task_id, new_status).await?;
    Ok(())
}

/// Get the Kanban board for a project.
pub async fn get_board(pool: &DbPool, project_id: &str) -> CwaResult<Board> {
    let tasks = queries::list_tasks(pool, project_id).await?;

    let mut columns = Vec::new();
    for (name, default_limit) in DEFAULT_COLUMNS {
        let wip_limit = queries::get_wip_limit(pool, project_id, name).await?
            .or(*default_limit);

        let column_tasks: Vec<Task> = tasks
            .iter()
            .filter(|t| t.status == *name)
            .cloned()
            .map(Task::from_row)
            .collect();

        columns.push(BoardColumn {
            name: name.to_string(),
            wip_limit,
            tasks: column_tasks,
        });
    }

    Ok(Board { columns })
}

/// Get WIP status for a project.
pub async fn get_wip_status(pool: &DbPool, project_id: &str) -> CwaResult<WipStatus> {
    let mut columns = Vec::new();

    for (name, default_limit) in DEFAULT_COLUMNS {
        let wip_limit = queries::get_wip_limit(pool, project_id, name).await?
            .or(*default_limit);
        let current_count = queries::count_tasks_by_status(pool, project_id, name).await?;

        columns.push(model::ColumnWipStatus {
            name: name.to_string(),
            limit: wip_limit,
            current: current_count,
            is_exceeded: wip_limit.map_or(false, |l| current_count > l),
        });
    }

    Ok(WipStatus { columns })
}

/// Result of task generation from a spec.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerateResult {
    pub created: Vec<Task>,
    pub skipped: usize,
}

/// Generate tasks from a spec's acceptance criteria.
///
/// Creates one task per acceptance criterion, skipping criteria that already
/// have a matching task (by title comparison).
pub async fn generate_tasks_from_spec(
    pool: &DbPool,
    project_id: &str,
    spec_id: &str,
    initial_status: &str,
) -> CwaResult<GenerateResult> {
    // Fetch the spec
    let spec = crate::spec::get_spec(pool, project_id, spec_id).await?;

    if spec.acceptance_criteria.is_empty() {
        return Err(CwaError::ValidationError(
            format!("Spec '{}' has no acceptance criteria. Add criteria first with 'cwa spec add-criteria'.", spec.title)
        ));
    }

    // Fetch existing tasks for this spec to avoid duplicates
    let existing_tasks = queries::list_tasks_by_spec(pool, &spec.id).await?;
    let existing_titles: Vec<String> = existing_tasks.iter().map(|t| t.title.clone()).collect();

    let priority = spec.priority.as_str();
    let mut created = Vec::new();
    let mut skipped = 0;

    for criterion in &spec.acceptance_criteria {
        if existing_titles.contains(criterion) {
            skipped += 1;
            continue;
        }

        let task = create_task(
            pool,
            project_id,
            criterion,
            None,
            Some(&spec.id),
            priority,
        ).await?;

        // Move to initial status if not "backlog"
        if initial_status != "backlog" {
            move_task(pool, project_id, &task.id, initial_status).await?;
        }

        created.push(if initial_status != "backlog" {
            get_task(pool, &task.id).await?
        } else {
            task
        });
    }

    Ok(GenerateResult { created, skipped })
}

/// Clear all tasks linked to a spec. Returns the number of deleted tasks.
pub async fn clear_tasks_by_spec(pool: &DbPool, project_id: &str, spec_id: &str) -> CwaResult<usize> {
    let spec = crate::spec::get_spec(pool, project_id, spec_id).await?;
    let count = queries::delete_tasks_by_spec(pool, &spec.id).await?;
    Ok(count)
}

/// Clear all tasks for a project. Returns the number of deleted tasks.
pub async fn clear_all_tasks(pool: &DbPool, project_id: &str) -> CwaResult<usize> {
    let count = queries::delete_all_tasks(pool, project_id).await?;
    Ok(count)
}

/// Initialize default Kanban columns for a project.
pub async fn init_kanban_columns(pool: &DbPool, project_id: &str) -> CwaResult<()> {
    for (i, (name, limit)) in DEFAULT_COLUMNS.iter().enumerate() {
        queries::set_wip_limit(pool, project_id, name, *limit, i as i32).await?;
    }
    Ok(())
}

/// Set WIP limit for a column. Use None to remove limit.
pub async fn set_wip_limit(pool: &DbPool, project_id: &str, column: &str, limit: Option<i64>) -> CwaResult<()> {
    // Validate column name
    let valid_columns: Vec<&str> = DEFAULT_COLUMNS.iter().map(|(name, _)| *name).collect();
    if !valid_columns.contains(&column) {
        return Err(CwaError::validation(format!(
            "Invalid column '{}'. Valid columns: {}",
            column,
            valid_columns.join(", ")
        )));
    }

    // Get column order
    let order = DEFAULT_COLUMNS
        .iter()
        .position(|(name, _)| *name == column)
        .unwrap() as i32;

    queries::set_wip_limit(pool, project_id, column, limit, order).await?;
    Ok(())
}
