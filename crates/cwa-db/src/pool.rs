//! Database connection pool management.

use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Database error types.
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database connection error: {0}")]
    Connection(#[from] rusqlite::Error),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Lock poisoned")]
    LockPoisoned,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

/// Result type for database operations.
pub type DbResult<T> = Result<T, DbError>;

/// Thread-safe database connection pool.
///
/// Uses a single connection with WAL mode for better concurrency.
#[derive(Clone)]
pub struct DbPool {
    conn: Arc<Mutex<Connection>>,
}

impl DbPool {
    /// Create a new database pool from a file path.
    pub fn new(path: &Path) -> DbResult<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrent access
        conn.pragma_update(None, "journal_mode", "WAL")?;

        // Enable foreign key constraints
        conn.pragma_update(None, "foreign_keys", "ON")?;

        // Set busy timeout to 5 seconds
        conn.pragma_update(None, "busy_timeout", 5000)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory database (for testing).
    pub fn in_memory() -> DbResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Execute a function with access to the database connection.
    pub fn with_conn<F, T>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(&Connection) -> DbResult<T>,
    {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        f(&conn)
    }

    /// Execute a function with mutable access to the database connection.
    pub fn with_conn_mut<F, T>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(&mut Connection) -> DbResult<T>,
    {
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        f(&mut conn)
    }
}

/// Initialize a database pool at the given path, running migrations.
pub fn init_pool(db_path: &Path) -> DbResult<DbPool> {
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            DbError::Migration(format!("Failed to create database directory: {}", e))
        })?;
    }

    let pool = DbPool::new(db_path)?;
    crate::migrations::run_migrations(&pool)?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_pool() {
        let pool = DbPool::in_memory().unwrap();
        pool.with_conn(|conn| {
            conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", [])?;
            Ok(())
        })
        .unwrap();
    }
}
