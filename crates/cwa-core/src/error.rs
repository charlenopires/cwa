//! Centralized error types for CWA.

use thiserror::Error;

/// Main error type for CWA operations.
#[derive(Error, Debug)]
pub enum CwaError {
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Specification not found: {0}")]
    SpecNotFound(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Bounded context not found: {0}")]
    ContextNotFound(String),

    #[error("Decision not found: {0}")]
    DecisionNotFound(String),

    #[error("Board not found: {0}")]
    BoardNotFound(String),

    #[error("Card not found: {0}")]
    CardNotFound(String),

    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    #[error("Invalid state transition: cannot move from '{from}' to '{to}'")]
    InvalidStateTransition { from: String, to: String },

    #[error("WIP limit exceeded for column '{column}': limit is {limit}, current count is {current}")]
    WipLimitExceeded {
        column: String,
        limit: i64,
        current: i64,
    },

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    Database(#[from] cwa_db::DbError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not initialized: {0}")]
    NotInitialized(String),
}

/// Result type for CWA operations.
pub type CwaResult<T> = Result<T, CwaError>;

impl CwaError {
    /// Create a validation error.
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationError(msg.into())
    }

    /// Create a not initialized error.
    pub fn not_initialized(msg: impl Into<String>) -> Self {
        Self::NotInitialized(msg.into())
    }
}
