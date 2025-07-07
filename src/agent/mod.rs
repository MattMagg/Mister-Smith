//! Agent Architecture Module
//!
//! Core agent types and lifecycle management for Claude CLI process orchestration.
//! Each "agent" is a managed Claude CLI process with supervision and state management.

pub mod agent;
pub mod lifecycle;
pub mod state;
pub mod pool;

pub use agent::*;
pub use lifecycle::*;
pub use state::*;
pub use pool::*;
