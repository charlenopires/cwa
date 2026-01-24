//! Memory and context management.

pub mod model;
pub mod observation;

use crate::error::CwaResult;
use crate::task;
use crate::spec;
use crate::decision;
use cwa_db::DbPool;
use cwa_db::queries::memory as queries;
use cwa_db::queries::observations as obs_queries;
use cwa_db::queries::projects as project_queries;
use model::{MemoryEntry, Session, ContextSummary};
use observation::{Observation, ObservationIndex, Summary, ObservationType};
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

// --- Observation Functions ---

/// Add a new observation.
pub fn add_observation(
    pool: &DbPool,
    project_id: &str,
    obs_type: &str,
    title: &str,
    narrative: Option<&str>,
    facts: &[String],
    concepts: &[String],
    files_modified: &[String],
    files_read: &[String],
    session_id: Option<&str>,
    confidence: f64,
) -> CwaResult<Observation> {
    // Validate obs_type
    ObservationType::from_str(obs_type)
        .ok_or_else(|| crate::error::CwaError::ValidationError(
            format!("Invalid observation type: '{}'. Use: {}", obs_type, ObservationType::all_variants().join(", "))
        ))?;

    let id = Uuid::new_v4().to_string();

    let facts_json = if facts.is_empty() { None } else { Some(serde_json::to_string(facts).unwrap()) };
    let concepts_json = if concepts.is_empty() { None } else { Some(serde_json::to_string(concepts).unwrap()) };
    let files_mod_json = if files_modified.is_empty() { None } else { Some(serde_json::to_string(files_modified).unwrap()) };
    let files_read_json = if files_read.is_empty() { None } else { Some(serde_json::to_string(files_read).unwrap()) };

    obs_queries::create_observation(
        pool, &id, project_id, session_id, obs_type, title, narrative,
        facts_json.as_deref(), concepts_json.as_deref(),
        files_mod_json.as_deref(), files_read_json.as_deref(),
        None, None, confidence,
    )?;

    let row = obs_queries::get_observation(pool, &id)?
        .ok_or_else(|| crate::error::CwaError::NotFound("Observation just created not found".to_string()))?;

    Ok(Observation::from_row(row))
}

/// Get an observation by ID.
pub fn get_observation(pool: &DbPool, id: &str) -> CwaResult<Option<Observation>> {
    let row = obs_queries::get_observation(pool, id)?;
    Ok(row.map(Observation::from_row))
}

/// Get multiple observations by IDs.
pub fn get_observations_batch(pool: &DbPool, ids: &[&str]) -> CwaResult<Vec<Observation>> {
    let rows = obs_queries::get_observations_batch(pool, ids)?;
    Ok(rows.into_iter().map(Observation::from_row).collect())
}

/// Get timeline of observations (compact index).
pub fn get_timeline(pool: &DbPool, project_id: &str, days_back: i64, limit: i64) -> CwaResult<Vec<ObservationIndex>> {
    let rows = obs_queries::list_observations_timeline(pool, project_id, days_back, limit)?;
    Ok(rows.into_iter().map(ObservationIndex::from_row).collect())
}

/// Get high-confidence observations (full details).
pub fn get_high_confidence_observations(pool: &DbPool, project_id: &str, min_confidence: f64, limit: i64) -> CwaResult<Vec<Observation>> {
    let rows = obs_queries::list_high_confidence(pool, project_id, min_confidence, limit)?;
    Ok(rows.into_iter().map(Observation::from_row).collect())
}

/// Create a summary from recent observations.
pub fn create_summary(
    pool: &DbPool,
    project_id: &str,
    session_id: Option<&str>,
    content: &str,
    key_facts: &[String],
    observations_count: i64,
) -> CwaResult<Summary> {
    let id = Uuid::new_v4().to_string();
    let key_facts_json = if key_facts.is_empty() { None } else { Some(serde_json::to_string(key_facts).unwrap()) };

    obs_queries::create_summary(
        pool, &id, project_id, session_id, content,
        observations_count, key_facts_json.as_deref(), None, None,
    )?;

    let summaries = obs_queries::get_recent_summaries(pool, project_id, 1)?;
    let row = summaries.into_iter().next()
        .ok_or_else(|| crate::error::CwaError::NotFound("Summary just created not found".to_string()))?;

    Ok(Summary::from_row(row))
}

/// Get recent summaries.
pub fn get_recent_summaries(pool: &DbPool, project_id: &str, limit: i64) -> CwaResult<Vec<Summary>> {
    let rows = obs_queries::get_recent_summaries(pool, project_id, limit)?;
    Ok(rows.into_iter().map(Summary::from_row).collect())
}

/// Boost confidence of an observation (cap at 1.0).
pub fn boost_confidence(pool: &DbPool, id: &str, amount: f64) -> CwaResult<()> {
    if let Some(row) = obs_queries::get_observation(pool, id)? {
        let new_confidence = (row.confidence + amount).min(1.0);
        obs_queries::update_confidence(pool, id, new_confidence)?;
    }
    Ok(())
}

/// Decay confidence for all observations in a project.
pub fn decay_confidence(pool: &DbPool, project_id: &str, factor: f64) -> CwaResult<usize> {
    let count = obs_queries::decay_all_confidence(pool, project_id, factor)?;
    Ok(count)
}

/// Remove observations below a confidence threshold.
pub fn remove_low_confidence_observations(pool: &DbPool, project_id: &str, min_confidence: f64) -> CwaResult<Vec<String>> {
    let ids = obs_queries::remove_low_confidence(pool, project_id, min_confidence)?;
    Ok(ids)
}
