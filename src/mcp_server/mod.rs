//! MCP Server Module
//! 
//! Model Context Protocol server components for MisterSmith

pub mod discovery_state;
pub mod nats_config;
pub mod server;
pub mod sse_bridge;
pub mod handler_integration;

#[cfg(test)]
pub mod discovery_state_example;

pub mod example_usage;

pub use discovery_state::{DiscoveryStore, DiscoveryFilter, StoreStats};
pub use nats_config::{NatsConfig, NatsClientManager, SubjectPatterns, ConnectionStats, ConnectionStatus};
pub use server::{McpServer, McpServerConfig, McpServerState, SseConfig, ServerStats};
pub use sse_bridge::{SseBridge, SseBridgeConfig, SseClientInfo, SseBridgeStats};
pub use handler_integration::{HandlerIntegration, HandlerIntegrationFactory, HandlerContext, HandlerMetrics, HandlerMetricsCollector, HandlerMiddleware};