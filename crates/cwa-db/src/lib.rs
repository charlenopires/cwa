//! CWA Database Layer
//!
//! Provides SQLite-based persistence for CWA projects.

pub mod migrations;
pub mod pool;
pub mod queries;

pub use pool::{DbPool, DbError, DbResult, init_pool};
pub use migrations::run_migrations;
