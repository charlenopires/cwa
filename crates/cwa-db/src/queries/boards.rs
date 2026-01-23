//! Board, column, and card database queries for the web Kanban UI.

use crate::pool::{DbPool, DbResult, DbError};
use rusqlite::params;

/// Board row from database.
#[derive(Debug, Clone)]
pub struct BoardRow {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Column row from database.
#[derive(Debug, Clone)]
pub struct ColumnRow {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub position: i32,
    pub color: Option<String>,
    pub wip_limit: Option<i32>,
}

/// Card row from database.
#[derive(Debug, Clone)]
pub struct CardRow {
    pub id: String,
    pub column_id: String,
    pub title: String,
    pub description: Option<String>,
    pub position: i32,
    pub priority: Option<String>,
    pub due_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// Label row from database.
#[derive(Debug, Clone)]
pub struct LabelRow {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub color: String,
}

/// Card history row.
#[derive(Debug, Clone)]
pub struct CardHistoryRow {
    pub id: i64,
    pub card_id: String,
    pub action: String,
    pub from_column_id: Option<String>,
    pub to_column_id: Option<String>,
    pub timestamp: String,
}

// ============================================================
// BOARDS
// ============================================================

/// Create a new board.
pub fn create_board(
    pool: &DbPool,
    id: &str,
    project_id: &str,
    name: &str,
    description: Option<&str>,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO boards (id, project_id, name, description)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, project_id, name, description],
        )?;
        Ok(())
    })
}

/// Get a board by ID.
pub fn get_board(pool: &DbPool, id: &str) -> DbResult<BoardRow> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, project_id, name, description, created_at, updated_at
             FROM boards WHERE id = ?1",
            params![id],
            |row| {
                Ok(BoardRow {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Board: {}", id)),
            e => DbError::Connection(e),
        })
    })
}

/// List boards for a project.
pub fn list_boards(pool: &DbPool, project_id: &str) -> DbResult<Vec<BoardRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, description, created_at, updated_at
             FROM boards WHERE project_id = ?1
             ORDER BY created_at ASC",
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok(BoardRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Delete a board (cascades to columns and cards).
pub fn delete_board(pool: &DbPool, id: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute("DELETE FROM boards WHERE id = ?1", params![id])?;
        Ok(())
    })
}

// ============================================================
// COLUMNS
// ============================================================

/// Create a new column.
pub fn create_column(
    pool: &DbPool,
    id: &str,
    board_id: &str,
    name: &str,
    position: i32,
    color: Option<&str>,
    wip_limit: Option<i32>,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO columns (id, board_id, name, position, color, wip_limit)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, board_id, name, position, color, wip_limit],
        )?;
        Ok(())
    })
}

/// List columns for a board (ordered by position).
pub fn list_columns(pool: &DbPool, board_id: &str) -> DbResult<Vec<ColumnRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, board_id, name, position, color, wip_limit
             FROM columns WHERE board_id = ?1
             ORDER BY position ASC",
        )?;

        let rows = stmt.query_map(params![board_id], |row| {
            Ok(ColumnRow {
                id: row.get(0)?,
                board_id: row.get(1)?,
                name: row.get(2)?,
                position: row.get(3)?,
                color: row.get(4)?,
                wip_limit: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Get a column by ID.
pub fn get_column(pool: &DbPool, id: &str) -> DbResult<ColumnRow> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, board_id, name, position, color, wip_limit
             FROM columns WHERE id = ?1",
            params![id],
            |row| {
                Ok(ColumnRow {
                    id: row.get(0)?,
                    board_id: row.get(1)?,
                    name: row.get(2)?,
                    position: row.get(3)?,
                    color: row.get(4)?,
                    wip_limit: row.get(5)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Column: {}", id)),
            e => DbError::Connection(e),
        })
    })
}

/// Count cards in a column (for WIP limit checks).
pub fn count_cards_in_column(pool: &DbPool, column_id: &str) -> DbResult<i32> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM cards WHERE column_id = ?1 AND completed_at IS NULL",
            params![column_id],
            |row| row.get(0),
        )
        .map_err(DbError::from)
    })
}

/// Delete a column.
pub fn delete_column(pool: &DbPool, id: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute("DELETE FROM columns WHERE id = ?1", params![id])?;
        Ok(())
    })
}

// ============================================================
// CARDS
// ============================================================

/// Create a new card.
pub fn create_card(
    pool: &DbPool,
    id: &str,
    column_id: &str,
    title: &str,
    description: Option<&str>,
    position: i32,
    priority: Option<&str>,
    due_date: Option<&str>,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO cards (id, column_id, title, description, position, priority, due_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, column_id, title, description, position, priority, due_date],
        )?;
        // Record history
        conn.execute(
            "INSERT INTO card_history (card_id, action, to_column_id)
             VALUES (?1, 'created', ?2)",
            params![id, column_id],
        )?;
        Ok(())
    })
}

/// Get a card by ID.
pub fn get_card(pool: &DbPool, id: &str) -> DbResult<CardRow> {
    pool.with_conn(|conn| {
        conn.query_row(
            "SELECT id, column_id, title, description, position, priority, due_date,
                    created_at, updated_at, completed_at
             FROM cards WHERE id = ?1",
            params![id],
            |row| {
                Ok(CardRow {
                    id: row.get(0)?,
                    column_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    position: row.get(4)?,
                    priority: row.get(5)?,
                    due_date: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    completed_at: row.get(9)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Card: {}", id)),
            e => DbError::Connection(e),
        })
    })
}

/// List cards in a column (ordered by position).
pub fn list_cards_in_column(pool: &DbPool, column_id: &str) -> DbResult<Vec<CardRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, column_id, title, description, position, priority, due_date,
                    created_at, updated_at, completed_at
             FROM cards WHERE column_id = ?1
             ORDER BY position ASC",
        )?;

        let rows = stmt.query_map(params![column_id], |row| {
            Ok(CardRow {
                id: row.get(0)?,
                column_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                position: row.get(4)?,
                priority: row.get(5)?,
                due_date: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                completed_at: row.get(9)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Move a card to a new column and position.
pub fn move_card(
    pool: &DbPool,
    card_id: &str,
    target_column_id: &str,
    new_position: i32,
) -> DbResult<String> {
    pool.with_conn(|conn| {
        // Get current column
        let from_column_id: String = conn.query_row(
            "SELECT column_id FROM cards WHERE id = ?1",
            params![card_id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Card: {}", card_id)),
            e => DbError::Connection(e),
        })?;

        // Update card
        conn.execute(
            "UPDATE cards SET column_id = ?1, position = ?2, updated_at = datetime('now')
             WHERE id = ?3",
            params![target_column_id, new_position, card_id],
        )?;

        // Record history
        conn.execute(
            "INSERT INTO card_history (card_id, action, from_column_id, to_column_id)
             VALUES (?1, 'moved', ?2, ?3)",
            params![card_id, from_column_id, target_column_id],
        )?;

        Ok(from_column_id)
    })
}

/// Update a card's title and description.
pub fn update_card(
    pool: &DbPool,
    id: &str,
    title: &str,
    description: Option<&str>,
    priority: Option<&str>,
    due_date: Option<&str>,
) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE cards SET title = ?1, description = ?2, priority = ?3, due_date = ?4,
                    updated_at = datetime('now')
             WHERE id = ?5",
            params![title, description, priority, due_date, id],
        )?;
        conn.execute(
            "INSERT INTO card_history (card_id, action)
             VALUES (?1, 'updated')",
            params![id],
        )?;
        Ok(())
    })
}

/// Mark a card as completed.
pub fn complete_card(pool: &DbPool, id: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "UPDATE cards SET completed_at = datetime('now'), updated_at = datetime('now')
             WHERE id = ?1",
            params![id],
        )?;
        conn.execute(
            "INSERT INTO card_history (card_id, action)
             VALUES (?1, 'completed')",
            params![id],
        )?;
        Ok(())
    })
}

/// Delete (archive) a card.
pub fn delete_card(pool: &DbPool, id: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO card_history (card_id, action)
             VALUES (?1, 'archived')",
            params![id],
        )?;
        conn.execute("DELETE FROM cards WHERE id = ?1", params![id])?;
        Ok(())
    })
}

/// Reorder cards within a column by updating positions.
pub fn reorder_cards(pool: &DbPool, column_id: &str, card_ids: &[String]) -> DbResult<()> {
    pool.with_conn(|conn| {
        for (position, card_id) in card_ids.iter().enumerate() {
            conn.execute(
                "UPDATE cards SET position = ?1, updated_at = datetime('now')
                 WHERE id = ?2 AND column_id = ?3",
                params![position as i32, card_id, column_id],
            )?;
        }
        Ok(())
    })
}

// ============================================================
// LABELS
// ============================================================

/// Create a label for a board.
pub fn create_label(pool: &DbPool, id: &str, board_id: &str, name: &str, color: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO labels (id, board_id, name, color) VALUES (?1, ?2, ?3, ?4)",
            params![id, board_id, name, color],
        )?;
        Ok(())
    })
}

/// List labels for a board.
pub fn list_labels(pool: &DbPool, board_id: &str) -> DbResult<Vec<LabelRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, board_id, name, color FROM labels WHERE board_id = ?1 ORDER BY name",
        )?;

        let rows = stmt.query_map(params![board_id], |row| {
            Ok(LabelRow {
                id: row.get(0)?,
                board_id: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Get labels for a card.
pub fn get_card_labels(pool: &DbPool, card_id: &str) -> DbResult<Vec<LabelRow>> {
    pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT l.id, l.board_id, l.name, l.color
             FROM labels l
             INNER JOIN card_labels cl ON cl.label_id = l.id
             WHERE cl.card_id = ?1
             ORDER BY l.name",
        )?;

        let rows = stmt.query_map(params![card_id], |row| {
            Ok(LabelRow {
                id: row.get(0)?,
                board_id: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    })
}

/// Attach a label to a card.
pub fn add_label_to_card(pool: &DbPool, card_id: &str, label_id: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT OR IGNORE INTO card_labels (card_id, label_id) VALUES (?1, ?2)",
            params![card_id, label_id],
        )?;
        Ok(())
    })
}

/// Remove a label from a card.
pub fn remove_label_from_card(pool: &DbPool, card_id: &str, label_id: &str) -> DbResult<()> {
    pool.with_conn(|conn| {
        conn.execute(
            "DELETE FROM card_labels WHERE card_id = ?1 AND label_id = ?2",
            params![card_id, label_id],
        )?;
        Ok(())
    })
}

/// Get the next available position for a card in a column.
pub fn next_card_position(pool: &DbPool, column_id: &str) -> DbResult<i32> {
    pool.with_conn(|conn| {
        let max: Option<i32> = conn.query_row(
            "SELECT MAX(position) FROM cards WHERE column_id = ?1",
            params![column_id],
            |row| row.get(0),
        )
        .map_err(DbError::from)?;

        Ok(max.unwrap_or(-1) + 1)
    })
}
