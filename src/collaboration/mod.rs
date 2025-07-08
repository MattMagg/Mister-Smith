//! Real-time Agent Collaboration Module
//! 
//! This is the SECRET SAUCE - agents that talk DURING work, not after

pub mod orchestrator;
pub mod discovery_sharing;
pub mod context_protocol;
pub mod sse_broadcaster;
pub mod bridge;
pub mod bridge_usage_example;

pub use orchestrator::*;
pub use discovery_sharing::*;
pub use context_protocol::*;
pub use sse_broadcaster::*;
pub use bridge::*;