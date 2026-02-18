//! Database pool — type aliases for backwards compatibility.
//! The actual pool is Redis (cwa-redis crate).

pub use cwa_redis::RedisPool as DbPool;
pub use cwa_redis::RedisError as DbError;
pub use cwa_redis::RedisResult as DbResult;

/// Initialize pool from a Redis URL.
pub async fn init_pool(redis_url: &str) -> DbResult<DbPool> {
    cwa_redis::init_pool(redis_url).await
}
