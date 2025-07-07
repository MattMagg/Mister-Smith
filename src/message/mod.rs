//! Message System Module
//!
//! NATS messaging system for agent communication.
//! Provides message types, routing, and serialization for agent coordination.

pub mod types;
pub mod router;
pub mod handler;

pub use types::*;
pub use router::*;
pub use handler::*;
