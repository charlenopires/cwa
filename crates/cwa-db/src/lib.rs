//! CWA Database Layer — Thin wrapper over cwa-redis.
//!
//! All persistence is now handled by Redis via the cwa-redis crate.
//! This crate provides backwards-compatible type aliases and re-exports.

pub mod queries;

// Re-export core types with backwards-compatible aliases
pub use cwa_redis::RedisPool as DbPool;
pub use cwa_redis::RedisError as DbError;
pub use cwa_redis::RedisResult as DbResult;
pub use cwa_redis::{
    BroadcastReceiver, BroadcastSender, WebSocketMessage, create_broadcast_channel,
};

/// Initialize a database pool from a Redis URL.
///
/// The URL is read from the `REDIS_URL` environment variable if not provided.
/// Falls back to `redis://127.0.0.1:6379`.
pub async fn init_pool(redis_url: &str) -> DbResult<DbPool> {
    cwa_redis::init_pool(redis_url).await
}

/// Initialize a pool reading REDIS_URL from environment (or default).
pub async fn init_pool_from_env() -> DbResult<DbPool> {
    let url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    cwa_redis::init_pool(&url).await
}
