//! Redis connection pool management.

use redis::aio::ConnectionManager;
use thiserror::Error;

/// Redis error types.
#[derive(Error, Debug)]
pub enum RedisError {
    #[error("Redis connection error: {0}")]
    Connection(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// Result type for Redis operations.
pub type RedisResult<T> = Result<T, RedisError>;

/// Redis connection pool — ConnectionManager handles multiplexing internally.
/// It is Clone, so callers clone it to get a mutable handle for each operation.
pub type RedisPool = ConnectionManager;

/// Initialize a Redis connection pool from a URL.
///
/// Example URL: `redis://127.0.0.1:6379`
pub async fn init_pool(redis_url: &str) -> RedisResult<RedisPool> {
    let client = redis::Client::open(redis_url)?;
    let manager = ConnectionManager::new(client).await?;
    Ok(manager)
}
