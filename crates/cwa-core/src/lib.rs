//! CWA Core Library
//!
//! Domain models and business logic for the Claude Workflow Architect.

pub mod analysis;
pub mod decision;
pub mod domain;
pub mod error;
pub mod memory;
pub mod project;
pub mod spec;
pub mod task;

pub use error::{CwaError, CwaResult};
