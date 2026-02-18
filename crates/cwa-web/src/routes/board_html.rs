//! HTMX-driven Kanban board route handlers.
//!
//! Returns HTML fragments for HTMX partial page updates.

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;

use crate::state::{AppState, WebSocketMessage};
use cwa_core::board::{self, Card, Column, Priority};

// ============================================================
// TEMPLATES
// ============================================================

#[derive(Template)]
#[template(path = "board.html")]
struct BoardTemplate {
    board_id: String,
    board_name: String,
    columns: Vec<ColumnView>,
}

#[derive(Template)]
#[template(path = "partials/column.html")]
struct ColumnTemplate {
    column: ColumnView,
}

/// View model for a column (with computed fields).
struct ColumnView {
    id: String,
    name: String,
    position: i32,
    color: Option<String>,
    wip_limit: Option<i32>,
    wip_exceeded: bool,
    cards: Vec<CardView>,
}

/// View model for a card.
struct CardView {
    id: String,
    title: String,
    description: Option<String>,
    priority: Option<Priority>,
    due_date: Option<String>,
    labels: Vec<LabelView>,
}

struct LabelView {
    name: String,
    color: String,
}

impl CardView {
    fn from_card(card: &Card) -> Self {
        Self {
            id: card.id.clone(),
            title: card.title.clone(),
            description: card.description.clone(),
            priority: card.priority.clone(),
            due_date: card.due_date.clone(),
            labels: card.labels.iter().map(|l| LabelView {
                name: l.name.clone(),
                color: l.color.clone(),
            }).collect(),
        }
    }
}

impl ColumnView {
    fn from_column(col: &Column) -> Self {
        let card_count = col.cards.len() as i32;
        let wip_exceeded = col.wip_limit.map_or(false, |limit| card_count >= limit);
        Self {
            id: col.id.clone(),
            name: col.name.clone(),
            position: col.position,
            color: col.color.clone(),
            wip_limit: col.wip_limit,
            wip_exceeded,
            cards: col.cards.iter().map(CardView::from_card).collect(),
        }
    }
}

// ============================================================
// REQUEST TYPES
// ============================================================

#[derive(Deserialize)]
pub struct CreateCardForm {
    pub board_id: String,
    pub column_id: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub due_date: Option<String>,
}

#[derive(Deserialize)]
pub struct MoveCardForm {
    pub target_column_id: String,
    pub position: i32,
}

// ============================================================
// HANDLERS
// ============================================================

/// GET / - Redirect to the default board.
pub async fn index(State(state): State<AppState>) -> Response {
    // Find the default project and board
    let project = match cwa_db::queries::projects::get_default_project(&state.db).await {
        Ok(Some(p)) => p,
        Ok(None) | Err(_) => return (StatusCode::NOT_FOUND, Html("No project found. Run 'cwa init' first.".to_string())).into_response(),
    };

    let board = match board::get_or_create_default_board(&state.db, &project.id).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("Error: {}", e))).into_response(),
    };

    Redirect::to(&format!("/boards/{}", board.id)).into_response()
}

/// GET /boards - List all boards (redirects to first one).
pub async fn list_boards(State(state): State<AppState>) -> Response {
    index(State(state)).await
}

/// GET /boards/{id} - Render full board page.
pub async fn get_board(
    State(state): State<AppState>,
    Path(board_id): Path<String>,
) -> Response {
    let board = match board::get_board(&state.db, &board_id).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::NOT_FOUND, Html("Board not found".to_string())).into_response(),
    };

    let columns: Vec<ColumnView> = board.columns.iter().map(ColumnView::from_column).collect();

    let template = BoardTemplate {
        board_id: board.id,
        board_name: board.name,
        columns,
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("Template error: {}", e))).into_response(),
    }
}

/// POST /cards - Create a new card. Returns updated board columns.
pub async fn create_card(
    State(state): State<AppState>,
    Form(form): Form<CreateCardForm>,
) -> Response {
    let priority = form.priority.as_deref().filter(|s| !s.is_empty());
    let due_date = form.due_date.as_deref().filter(|s| !s.is_empty());
    let description = form.description.as_deref().filter(|s| !s.is_empty());

    if let Err(e) = board::create_card(
        &state.db,
        &form.column_id,
        &form.title,
        description,
        priority,
        due_date,
    ).await {
        return (StatusCode::BAD_REQUEST, Html(format!("Error: {}", e))).into_response();
    }

    state.broadcast(WebSocketMessage::BoardRefresh);
    render_board_columns(&state, &form.board_id).await
}

/// PATCH /cards/{id}/move - Move a card to a new column/position.
pub async fn move_card(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
    Form(form): Form<MoveCardForm>,
) -> Response {
    // Get the card to find its board
    let card = match cwa_db::queries::boards::get_card(&state.db, &card_id).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, Html("Card not found".to_string())).into_response(),
    };

    if let Err(e) = board::move_card(&state.db, &card_id, &form.target_column_id, form.position).await {
        return (StatusCode::BAD_REQUEST, Html(format!("Error: {}", e))).into_response();
    }

    state.broadcast(WebSocketMessage::BoardRefresh);

    // Find board_id from column
    let column = match cwa_db::queries::boards::get_column(&state.db, &card.column_id).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Column not found".to_string())).into_response(),
    };

    render_board_columns(&state, &column.board_id).await
}

/// DELETE /cards/{id} - Delete a card.
pub async fn delete_card(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
) -> Response {
    // Get card's column to find board
    let card = match cwa_db::queries::boards::get_card(&state.db, &card_id).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, Html("Card not found".to_string())).into_response(),
    };

    let column = match cwa_db::queries::boards::get_column(&state.db, &card.column_id).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Column not found".to_string())).into_response(),
    };

    if let Err(e) = board::delete_card(&state.db, &card_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("Error: {}", e))).into_response();
    }

    state.broadcast(WebSocketMessage::BoardRefresh);
    render_board_columns(&state, &column.board_id).await
}

// ============================================================
// HELPERS
// ============================================================

/// Render just the board columns HTML (for HTMX swaps after mutations).
async fn render_board_columns(state: &AppState, board_id: &str) -> Response {
    let board = match board::get_board(&state.db, board_id).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("Error: {}", e))).into_response(),
    };

    let columns: Vec<ColumnView> = board.columns.iter().map(ColumnView::from_column).collect();

    let mut html = String::new();
    for column in &columns {
        let tmpl = ColumnTemplate { column: column_view_ref(column) };
        match tmpl.render() {
            Ok(rendered) => html.push_str(&rendered),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("Template error: {}", e))).into_response(),
        }
    }

    Html(html).into_response()
}

/// Create a ColumnView reference for template rendering.
/// Askama templates need owned data, so we clone for partial renders.
fn column_view_ref(col: &ColumnView) -> ColumnView {
    ColumnView {
        id: col.id.clone(),
        name: col.name.clone(),
        position: col.position,
        color: col.color.clone(),
        wip_limit: col.wip_limit,
        wip_exceeded: col.wip_exceeded,
        cards: col.cards.iter().map(|c| CardView {
            id: c.id.clone(),
            title: c.title.clone(),
            description: c.description.clone(),
            priority: c.priority.clone(),
            due_date: c.due_date.clone(),
            labels: c.labels.iter().map(|l| LabelView {
                name: l.name.clone(),
                color: l.color.clone(),
            }).collect(),
        }).collect(),
    }
}
