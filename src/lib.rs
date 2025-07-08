//! MisterSmith Multi-Agent Orchestration Framework
//!
//! A distributed AI agent orchestration framework that manages Claude CLI processes
//! as agents with NATS-based communication and supervision.

pub mod agent;
pub mod runtime;
pub mod message;
pub mod transport;
// pub mod config;
// pub mod core;
// pub mod data;
// pub mod operations;
// pub mod security;
pub mod supervision;
pub mod metrics;
pub mod persistence;

// Re-export core types for convenience
pub use agent::{Agent, AgentId, AgentState, AgentConfig, AgentPool};
pub use runtime::{ProcessManager, ClaudeCommand};
pub use message::{MessageEnvelope, AgentCommand, AgentEvent, MessageRouter, Subject};
pub use transport::{NatsTransport};

/// Framework version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Framework name
pub const NAME: &str = "MisterSmith";

/// Framework description
pub const DESCRIPTION: &str = "Multi-agent orchestration framework for distributed AI systems";

#[cfg(test)]
pub mod tests;
