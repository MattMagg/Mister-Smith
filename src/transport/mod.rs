//! Transport Layer Module
//!
//! NATS and JetStream transport implementation for agent communication.
//! Provides connection management, stream configuration, and message transport.

pub mod nats;
// pub mod jetstream;
pub mod connection;
pub mod supervision_events;
pub mod multi_agent_coordination;

pub use nats::*;
// pub use jetstream::*;
pub use connection::*;
pub use supervision_events::*;
pub use multi_agent_coordination::*;
