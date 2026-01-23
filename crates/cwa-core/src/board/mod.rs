//! Board domain logic for the web Kanban UI.

pub mod model;

pub use model::{Board, Card, Column, Label, Priority, DEFAULT_COLUMNS};

use crate::error::{CwaError, CwaResult};
use cwa_db::DbPool;
use cwa_db::queries::boards;

/// Create a new board with default columns.
pub fn create_board(
    pool: &DbPool,
    project_id: &str,
    name: &str,
    description: Option<&str>,
) -> CwaResult<Board> {
    let board_id = uuid::Uuid::new_v4().to_string();
    boards::create_board(pool, &board_id, project_id, name, description)?;

    // Create default columns
    for (position, (col_name, wip_limit, color)) in DEFAULT_COLUMNS.iter().enumerate() {
        let col_id = uuid::Uuid::new_v4().to_string();
        boards::create_column(
            pool,
            &col_id,
            &board_id,
            col_name,
            position as i32,
            Some(color),
            *wip_limit,
        )?;
    }

    get_board(pool, &board_id)
}

/// Get a full board with columns and cards.
pub fn get_board(pool: &DbPool, board_id: &str) -> CwaResult<Board> {
    let board_row = boards::get_board(pool, board_id)
        .map_err(|_| CwaError::BoardNotFound(board_id.to_string()))?;

    let column_rows = boards::list_columns(pool, board_id)?;

    let mut columns = Vec::with_capacity(column_rows.len());
    for col_row in column_rows {
        let card_rows = boards::list_cards_in_column(pool, &col_row.id)?;
        let cards: Vec<Card> = card_rows
            .into_iter()
            .map(|c| {
                let labels = boards::get_card_labels(pool, &c.id).unwrap_or_default();
                Card {
                    id: c.id,
                    column_id: c.column_id,
                    title: c.title,
                    description: c.description,
                    position: c.position,
                    priority: c.priority.as_deref().and_then(Priority::from_str),
                    due_date: c.due_date,
                    labels: labels.into_iter().map(|l| Label {
                        id: l.id,
                        board_id: l.board_id,
                        name: l.name,
                        color: l.color,
                    }).collect(),
                    created_at: c.created_at,
                    updated_at: c.updated_at,
                    completed_at: c.completed_at,
                }
            })
            .collect();

        columns.push(Column {
            id: col_row.id,
            board_id: col_row.board_id,
            name: col_row.name,
            position: col_row.position,
            color: col_row.color,
            wip_limit: col_row.wip_limit,
            cards,
        });
    }

    Ok(Board {
        id: board_row.id,
        project_id: board_row.project_id,
        name: board_row.name,
        description: board_row.description,
        columns,
        created_at: board_row.created_at,
        updated_at: board_row.updated_at,
    })
}

/// List all boards for a project.
pub fn list_boards(pool: &DbPool, project_id: &str) -> CwaResult<Vec<Board>> {
    let board_rows = boards::list_boards(pool, project_id)?;
    let mut result = Vec::with_capacity(board_rows.len());
    for row in board_rows {
        result.push(get_board(pool, &row.id)?);
    }
    Ok(result)
}

/// Get or create a default board for a project.
pub fn get_or_create_default_board(pool: &DbPool, project_id: &str) -> CwaResult<Board> {
    let existing = boards::list_boards(pool, project_id)?;
    if let Some(first) = existing.into_iter().next() {
        get_board(pool, &first.id)
    } else {
        create_board(pool, project_id, "Default Board", None)
    }
}

/// Create a new card in a column.
pub fn create_card(
    pool: &DbPool,
    column_id: &str,
    title: &str,
    description: Option<&str>,
    priority: Option<&str>,
    due_date: Option<&str>,
) -> CwaResult<Card> {
    // Verify column exists and check WIP limit
    let column = boards::get_column(pool, column_id)
        .map_err(|_| CwaError::ColumnNotFound(column_id.to_string()))?;

    if let Some(limit) = column.wip_limit {
        let current = boards::count_cards_in_column(pool, column_id)?;
        if current >= limit {
            return Err(CwaError::WipLimitExceeded {
                column: column.name,
                limit: limit as i64,
                current: current as i64,
            });
        }
    }

    let card_id = uuid::Uuid::new_v4().to_string();
    let position = boards::next_card_position(pool, column_id)?;

    boards::create_card(
        pool,
        &card_id,
        column_id,
        title,
        description,
        position,
        priority,
        due_date,
    )?;

    let card_row = boards::get_card(pool, &card_id)?;
    Ok(Card {
        id: card_row.id,
        column_id: card_row.column_id,
        title: card_row.title,
        description: card_row.description,
        position: card_row.position,
        priority: card_row.priority.as_deref().and_then(Priority::from_str),
        due_date: card_row.due_date,
        labels: vec![],
        created_at: card_row.created_at,
        updated_at: card_row.updated_at,
        completed_at: card_row.completed_at,
    })
}

/// Move a card to a different column.
pub fn move_card(
    pool: &DbPool,
    card_id: &str,
    target_column_id: &str,
    position: i32,
) -> CwaResult<Card> {
    // Verify target column exists and check WIP limit
    let target_column = boards::get_column(pool, target_column_id)
        .map_err(|_| CwaError::ColumnNotFound(target_column_id.to_string()))?;

    if let Some(limit) = target_column.wip_limit {
        let current = boards::count_cards_in_column(pool, target_column_id)?;
        // Don't count the card being moved if it's already in this column
        let card_row = boards::get_card(pool, card_id)
            .map_err(|_| CwaError::CardNotFound(card_id.to_string()))?;
        let adjustment = if card_row.column_id == target_column_id { 1 } else { 0 };
        if current - adjustment >= limit {
            return Err(CwaError::WipLimitExceeded {
                column: target_column.name,
                limit: limit as i64,
                current: (current - adjustment) as i64,
            });
        }
    }

    boards::move_card(pool, card_id, target_column_id, position)?;

    let card_row = boards::get_card(pool, card_id)?;
    let labels = boards::get_card_labels(pool, card_id).unwrap_or_default();
    Ok(Card {
        id: card_row.id,
        column_id: card_row.column_id,
        title: card_row.title,
        description: card_row.description,
        position: card_row.position,
        priority: card_row.priority.as_deref().and_then(Priority::from_str),
        due_date: card_row.due_date,
        labels: labels.into_iter().map(|l| Label {
            id: l.id,
            board_id: l.board_id,
            name: l.name,
            color: l.color,
        }).collect(),
        created_at: card_row.created_at,
        updated_at: card_row.updated_at,
        completed_at: card_row.completed_at,
    })
}

/// Update a card's details.
pub fn update_card(
    pool: &DbPool,
    card_id: &str,
    title: &str,
    description: Option<&str>,
    priority: Option<&str>,
    due_date: Option<&str>,
) -> CwaResult<Card> {
    boards::update_card(pool, card_id, title, description, priority, due_date)?;

    let card_row = boards::get_card(pool, card_id)
        .map_err(|_| CwaError::CardNotFound(card_id.to_string()))?;
    let labels = boards::get_card_labels(pool, card_id).unwrap_or_default();
    Ok(Card {
        id: card_row.id,
        column_id: card_row.column_id,
        title: card_row.title,
        description: card_row.description,
        position: card_row.position,
        priority: card_row.priority.as_deref().and_then(Priority::from_str),
        due_date: card_row.due_date,
        labels: labels.into_iter().map(|l| Label {
            id: l.id,
            board_id: l.board_id,
            name: l.name,
            color: l.color,
        }).collect(),
        created_at: card_row.created_at,
        updated_at: card_row.updated_at,
        completed_at: card_row.completed_at,
    })
}

/// Delete a card.
pub fn delete_card(pool: &DbPool, card_id: &str) -> CwaResult<()> {
    boards::delete_card(pool, card_id)?;
    Ok(())
}
