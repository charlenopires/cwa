//! Kanban board queries — Redis implementation.

use crate::client::{RedisError, RedisPool, RedisResult};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardRow {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnRow {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub position: i32,
    pub color: Option<String>,
    pub wip_limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelRow {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub color: String,
}

// ─────────────────────────────── BOARDS ────────────────────────────────

pub async fn create_board(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    name: &str,
    description: Option<&str>,
) -> RedisResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let row = BoardRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        name: name.to_string(),
        description: description.map(str::to_string),
        created_at: now.clone(),
        updated_at: now,
    };
    let mut conn = pool.clone();
    let key = format!("cwa:{}:board:{}", project_id, id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    let set_key = format!("cwa:{}:boards:all", project_id);
    conn.sadd::<_, _, ()>(&set_key, id).await?;
    Ok(())
}

pub async fn get_board(pool: &RedisPool, board_id: &str) -> RedisResult<BoardRow> {
    let mut conn = pool.clone();
    let pattern = format!("cwa:*:board:{}", board_id);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    if let Some(key) = scan.next_item().await {
        drop(scan);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(serde_json::from_str(&j)?);
        }
    }
    Err(RedisError::NotFound(format!("Board not found: {}", board_id)))
}

pub async fn list_boards(pool: &RedisPool, project_id: &str) -> RedisResult<Vec<BoardRow>> {
    let mut conn = pool.clone();
    let set_key = format!("cwa:{}:boards:all", project_id);
    let ids: Vec<String> = conn.smembers(&set_key).await?;
    let mut boards = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:board:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<BoardRow>(&j) {
                boards.push(row);
            }
        }
    }
    Ok(boards)
}

pub async fn delete_board(pool: &RedisPool, board_id: &str) -> RedisResult<()> {
    let board = get_board(pool, board_id).await?;
    let mut conn = pool.clone();
    let key = format!("cwa:{}:board:{}", board.project_id, board_id);
    conn.del::<_, ()>(&key).await?;
    let set_key = format!("cwa:{}:boards:all", board.project_id);
    conn.srem::<_, _, ()>(&set_key, board_id).await?;
    Ok(())
}

// ─────────────────────────────── COLUMNS ───────────────────────────────

pub async fn create_column(
    pool: &RedisPool,
    id: &str,
    board_id: &str,
    name: &str,
    position: i32,
    color: Option<&str>,
    wip_limit: Option<i32>,
) -> RedisResult<()> {
    let board = get_board(pool, board_id).await?;
    let row = ColumnRow {
        id: id.to_string(),
        board_id: board_id.to_string(),
        name: name.to_string(),
        position,
        color: color.map(str::to_string),
        wip_limit,
    };
    let mut conn = pool.clone();
    let key = format!("cwa:{}:column:{}", board.project_id, id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    conn.hset::<_, _, _, ()>(&key, "board_id", board_id).await?;
    let set_key = format!("cwa:{}:columns:board:{}", board.project_id, board_id);
    conn.sadd::<_, _, ()>(&set_key, id).await?;
    Ok(())
}

pub async fn list_columns(pool: &RedisPool, board_id: &str) -> RedisResult<Vec<ColumnRow>> {
    let board = get_board(pool, board_id).await?;
    let mut conn = pool.clone();
    let set_key = format!("cwa:{}:columns:board:{}", board.project_id, board_id);
    let ids: Vec<String> = conn.smembers(&set_key).await?;
    let mut columns = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:column:{}", board.project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<ColumnRow>(&j) {
                columns.push(row);
            }
        }
    }
    columns.sort_by_key(|c| c.position);
    Ok(columns)
}

pub async fn get_column(pool: &RedisPool, column_id: &str) -> RedisResult<ColumnRow> {
    let mut conn = pool.clone();
    let pattern = format!("cwa:*:column:{}", column_id);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    if let Some(key) = scan.next_item().await {
        drop(scan);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(serde_json::from_str(&j)?);
        }
    }
    Err(RedisError::NotFound(format!("Column not found: {}", column_id)))
}

pub async fn count_cards_in_column(pool: &RedisPool, column_id: &str) -> RedisResult<i32> {
    let col = get_column(pool, column_id).await?;
    let board = get_board(pool, &col.board_id).await?;
    let mut conn = pool.clone();
    let set_key = format!("cwa:{}:cards:column:{}", board.project_id, column_id);
    let count: i64 = conn.scard(&set_key).await?;
    Ok(count as i32)
}

pub async fn delete_column(pool: &RedisPool, column_id: &str) -> RedisResult<()> {
    let col = get_column(pool, column_id).await?;
    let board = get_board(pool, &col.board_id).await?;
    let mut conn = pool.clone();
    let key = format!("cwa:{}:column:{}", board.project_id, column_id);
    conn.del::<_, ()>(&key).await?;
    let set_key = format!("cwa:{}:columns:board:{}", board.project_id, col.board_id);
    conn.srem::<_, _, ()>(&set_key, column_id).await?;
    Ok(())
}

// ─────────────────────────────── CARDS ─────────────────────────────────

pub async fn create_card(
    pool: &RedisPool,
    id: &str,
    column_id: &str,
    title: &str,
    description: Option<&str>,
    position: i32,
    priority: Option<&str>,
    due_date: Option<&str>,
) -> RedisResult<()> {
    let col = get_column(pool, column_id).await?;
    let board = get_board(pool, &col.board_id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    let row = CardRow {
        id: id.to_string(),
        column_id: column_id.to_string(),
        title: title.to_string(),
        description: description.map(str::to_string),
        position,
        priority: priority.map(str::to_string),
        due_date: due_date.map(str::to_string),
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    };
    let mut conn = pool.clone();
    let key = format!("cwa:{}:card:{}", board.project_id, id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    conn.hset::<_, _, _, ()>(&key, "column_id", column_id).await?;
    let set_key = format!("cwa:{}:cards:column:{}", board.project_id, column_id);
    conn.sadd::<_, _, ()>(&set_key, id).await?;
    Ok(())
}

pub async fn get_card(pool: &RedisPool, card_id: &str) -> RedisResult<CardRow> {
    let mut conn = pool.clone();
    let pattern = format!("cwa:*:card:{}", card_id);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    if let Some(key) = scan.next_item().await {
        drop(scan);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(serde_json::from_str(&j)?);
        }
    }
    Err(RedisError::NotFound(format!("Card not found: {}", card_id)))
}

pub async fn list_cards_in_column(pool: &RedisPool, column_id: &str) -> RedisResult<Vec<CardRow>> {
    let col = get_column(pool, column_id).await?;
    let board = get_board(pool, &col.board_id).await?;
    let mut conn = pool.clone();
    let set_key = format!("cwa:{}:cards:column:{}", board.project_id, column_id);
    let ids: Vec<String> = conn.smembers(&set_key).await?;
    let mut cards = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:card:{}", board.project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<CardRow>(&j) {
                cards.push(row);
            }
        }
    }
    cards.sort_by_key(|c| c.position);
    Ok(cards)
}

pub async fn move_card(
    pool: &RedisPool,
    card_id: &str,
    new_column_id: &str,
    new_position: i32,
) -> RedisResult<String> {
    let mut card = get_card(pool, card_id).await?;
    let old_column_id = card.column_id.clone();

    // Find project_id
    let col = get_column(pool, new_column_id).await?;
    let board = get_board(pool, &col.board_id).await?;
    let project_id = &board.project_id;

    let old_set = format!("cwa:{}:cards:column:{}", project_id, old_column_id);
    let new_set = format!("cwa:{}:cards:column:{}", project_id, new_column_id);

    let mut conn = pool.clone();
    conn.srem::<_, _, ()>(&old_set, card_id).await?;
    conn.sadd::<_, _, ()>(&new_set, card_id).await?;

    card.column_id = new_column_id.to_string();
    card.position = new_position;
    card.updated_at = chrono::Utc::now().to_rfc3339();

    let key = format!("cwa:{}:card:{}", project_id, card_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&card)?).await?;
    conn.hset::<_, _, _, ()>(&key, "column_id", new_column_id).await?;

    Ok(old_column_id)
}

pub async fn update_card(
    pool: &RedisPool,
    card_id: &str,
    title: &str,
    description: Option<&str>,
    priority: Option<&str>,
    due_date: Option<&str>,
) -> RedisResult<()> {
    let mut card = get_card(pool, card_id).await?;
    let col = get_column(pool, &card.column_id).await?;
    let board = get_board(pool, &col.board_id).await?;

    card.title = title.to_string();
    card.description = description.map(str::to_string);
    card.priority = priority.map(str::to_string);
    card.due_date = due_date.map(str::to_string);
    card.updated_at = chrono::Utc::now().to_rfc3339();

    let mut conn = pool.clone();
    let key = format!("cwa:{}:card:{}", board.project_id, card_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&card)?).await?;
    Ok(())
}

pub async fn complete_card(pool: &RedisPool, card_id: &str) -> RedisResult<()> {
    let mut card = get_card(pool, card_id).await?;
    let col = get_column(pool, &card.column_id).await?;
    let board = get_board(pool, &col.board_id).await?;

    let now = chrono::Utc::now().to_rfc3339();
    card.completed_at = Some(now.clone());
    card.updated_at = now;

    let mut conn = pool.clone();
    let key = format!("cwa:{}:card:{}", board.project_id, card_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&card)?).await?;
    Ok(())
}

pub async fn delete_card(pool: &RedisPool, card_id: &str) -> RedisResult<()> {
    let card = get_card(pool, card_id).await?;
    let col = get_column(pool, &card.column_id).await?;
    let board = get_board(pool, &col.board_id).await?;

    let mut conn = pool.clone();
    let key = format!("cwa:{}:card:{}", board.project_id, card_id);
    conn.del::<_, ()>(&key).await?;
    let set_key = format!("cwa:{}:cards:column:{}", board.project_id, card.column_id);
    conn.srem::<_, _, ()>(&set_key, card_id).await?;
    Ok(())
}

pub async fn reorder_cards(
    pool: &RedisPool,
    column_id: &str,
    card_ids: &[String],
) -> RedisResult<()> {
    let col = get_column(pool, column_id).await?;
    let board = get_board(pool, &col.board_id).await?;

    for (pos, card_id) in card_ids.iter().enumerate() {
        let key = format!("cwa:{}:card:{}", board.project_id, card_id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(mut card) = serde_json::from_str::<CardRow>(&j) {
                card.position = pos as i32;
                c.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&card)?).await?;
            }
        }
    }
    Ok(())
}

pub async fn next_card_position(pool: &RedisPool, column_id: &str) -> RedisResult<i32> {
    let count = count_cards_in_column(pool, column_id).await?;
    Ok(count)
}

// ─────────────────────────────── LABELS ────────────────────────────────

pub async fn create_label(
    pool: &RedisPool,
    id: &str,
    board_id: &str,
    name: &str,
    color: &str,
) -> RedisResult<()> {
    let board = get_board(pool, board_id).await?;
    let row = LabelRow {
        id: id.to_string(),
        board_id: board_id.to_string(),
        name: name.to_string(),
        color: color.to_string(),
    };
    let mut conn = pool.clone();
    let key = format!("cwa:{}:label:{}", board.project_id, id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    let set_key = format!("cwa:{}:labels:board:{}", board.project_id, board_id);
    conn.sadd::<_, _, ()>(&set_key, id).await?;
    Ok(())
}

pub async fn list_labels(pool: &RedisPool, board_id: &str) -> RedisResult<Vec<LabelRow>> {
    let board = get_board(pool, board_id).await?;
    let mut conn = pool.clone();
    let set_key = format!("cwa:{}:labels:board:{}", board.project_id, board_id);
    let ids: Vec<String> = conn.smembers(&set_key).await?;
    let mut labels = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:label:{}", board.project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<LabelRow>(&j) {
                labels.push(row);
            }
        }
    }
    Ok(labels)
}

pub async fn get_card_labels(pool: &RedisPool, card_id: &str) -> RedisResult<Vec<LabelRow>> {
    let card = get_card(pool, card_id).await?;
    let col = get_column(pool, &card.column_id).await?;
    let board = get_board(pool, &col.board_id).await?;

    let mut conn = pool.clone();
    let set_key = format!("cwa:{}:card_labels:{}", board.project_id, card_id);
    let label_ids: Vec<String> = conn.smembers(&set_key).await?;
    let mut labels = Vec::new();
    for lid in label_ids {
        let key = format!("cwa:{}:label:{}", board.project_id, lid);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<LabelRow>(&j) {
                labels.push(row);
            }
        }
    }
    Ok(labels)
}

pub async fn add_label_to_card(
    pool: &RedisPool,
    card_id: &str,
    label_id: &str,
) -> RedisResult<()> {
    let card = get_card(pool, card_id).await?;
    let col = get_column(pool, &card.column_id).await?;
    let board = get_board(pool, &col.board_id).await?;
    let mut conn = pool.clone();
    let set_key = format!("cwa:{}:card_labels:{}", board.project_id, card_id);
    conn.sadd::<_, _, ()>(&set_key, label_id).await?;
    Ok(())
}

pub async fn remove_label_from_card(
    pool: &RedisPool,
    card_id: &str,
    label_id: &str,
) -> RedisResult<()> {
    let card = get_card(pool, card_id).await?;
    let col = get_column(pool, &card.column_id).await?;
    let board = get_board(pool, &col.board_id).await?;
    let mut conn = pool.clone();
    let set_key = format!("cwa:{}:card_labels:{}", board.project_id, card_id);
    conn.srem::<_, _, ()>(&set_key, label_id).await?;
    Ok(())
}
