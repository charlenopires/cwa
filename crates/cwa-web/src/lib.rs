//! CWA Web Server
//!
//! Axum-based web server for dashboard and REST API.

pub mod routes;
pub mod state;
pub mod websocket;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use cwa_db::DbPool;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use state::AppState;

/// Create the application router.
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        // Tasks
        .route("/tasks", get(routes::tasks::list_tasks))
        .route("/tasks", post(routes::tasks::create_task))
        .route("/tasks/{id}", get(routes::tasks::get_task))
        .route("/tasks/{id}", put(routes::tasks::update_task))
        .route("/board", get(routes::tasks::get_board))
        // Specs
        .route("/specs", get(routes::specs::list_specs))
        .route("/specs", post(routes::specs::create_spec))
        .route("/specs/{id}", get(routes::specs::get_spec))
        .route("/specs/{id}/generate-tasks", post(routes::specs::generate_tasks))
        // Domains
        .route("/domains", get(routes::domains::list_contexts))
        .route("/domains/{id}", get(routes::domains::get_context))
        // Decisions
        .route("/decisions", get(routes::decisions::list_decisions))
        .route("/decisions", post(routes::decisions::create_decision))
        // Context
        .route("/context/summary", get(routes::context::get_summary))
        .with_state(state.clone());

    // HTMX-driven HTML routes for Kanban board
    let board_routes = Router::new()
        .route("/boards", get(routes::board_html::list_boards))
        .route("/boards/{id}", get(routes::board_html::get_board))
        .route("/cards", post(routes::board_html::create_card))
        .route("/cards/{id}/move", patch(routes::board_html::move_card))
        .route("/cards/{id}", delete(routes::board_html::delete_card))
        .with_state(state.clone());

    Router::new()
        .route("/", get(routes::dashboard::index))
        .nest("/api", api_routes)
        .merge(board_routes)
        .route("/ws", get(websocket::ws_handler))
        .route("/internal/notify", post(routes::internal::notify))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Run the web server.
pub async fn run_server(db: Arc<DbPool>, port: u16) -> anyhow::Result<()> {
    let state = AppState::new(db);
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    tracing::info!("Web server listening on http://127.0.0.1:{}", port);

    axum::serve(listener, app).await?;
    Ok(())
}
