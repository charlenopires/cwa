//! Architectural Decision Records (ADR).

pub mod model;

use crate::error::CwaResult;
use cwa_db::DbPool;
use cwa_db::queries::decisions as queries;
use model::Decision;
use uuid::Uuid;

/// Create a new decision.
pub async fn create_decision(
    pool: &DbPool,
    project_id: &str,
    title: &str,
    context: &str,
    decision: &str,
) -> CwaResult<Decision> {
    let id = Uuid::new_v4().to_string();

    queries::create_decision(pool, &id, project_id, title, context, decision).await?;

    let row = queries::get_decision(pool, &id).await?;
    Ok(Decision::from_row(row))
}

/// Get a decision by ID.
pub async fn get_decision(pool: &DbPool, id: &str) -> CwaResult<Decision> {
    let row = queries::get_decision(pool, id).await?;
    Ok(Decision::from_row(row))
}

/// List all decisions for a project.
pub async fn list_decisions(pool: &DbPool, project_id: &str) -> CwaResult<Vec<Decision>> {
    let rows = queries::list_decisions(pool, project_id).await?;
    Ok(rows.into_iter().map(Decision::from_row).collect())
}

/// List accepted decisions.
pub async fn list_accepted_decisions(pool: &DbPool, project_id: &str) -> CwaResult<Vec<Decision>> {
    let rows = queries::list_accepted_decisions(pool, project_id).await?;
    Ok(rows.into_iter().map(Decision::from_row).collect())
}

/// Accept a decision.
pub async fn accept_decision(pool: &DbPool, id: &str) -> CwaResult<()> {
    queries::update_decision_status(pool, id, "accepted").await?;
    Ok(())
}

/// Deprecate a decision.
pub async fn deprecate_decision(pool: &DbPool, id: &str) -> CwaResult<()> {
    queries::update_decision_status(pool, id, "deprecated").await?;
    Ok(())
}

/// Supersede a decision with a new one.
pub async fn supersede_decision(pool: &DbPool, old_id: &str, new_id: &str) -> CwaResult<()> {
    queries::supersede_decision(pool, old_id, new_id).await?;
    Ok(())
}

/// Format decisions as markdown for context summary.
pub fn format_decisions_summary(decisions: &[Decision]) -> String {
    if decisions.is_empty() {
        return "No architectural decisions recorded.".to_string();
    }

    let mut output = String::new();
    for decision in decisions.iter().take(5) {
        output.push_str(&format!(
            "- **{}** ({}): {}\n",
            decision.title,
            decision.status.as_str(),
            decision.decision.chars().take(100).collect::<String>()
        ));
    }

    if decisions.len() > 5 {
        output.push_str(&format!("... and {} more decisions\n", decisions.len() - 5));
    }

    output
}
