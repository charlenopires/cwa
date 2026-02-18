//! CWA Redis Data Layer
//!
//! Provides async Redis-based persistence for CWA projects.
//! Replaces the SQLite-based cwa-db crate.

pub mod broadcast;
pub mod client;
pub mod queries;

pub use broadcast::{
    BroadcastReceiver, BroadcastSender, WebSocketMessage, create_broadcast_channel,
};
pub use client::{RedisError, RedisPool, RedisResult, init_pool};
pub use queries::boards;
pub use queries::decisions;
pub use queries::domains;
pub use queries::glossary;
pub use queries::memory;
pub use queries::observations;
pub use queries::projects;
pub use queries::specs;
pub use queries::tasks;
