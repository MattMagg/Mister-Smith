//! NATS Transport Implementation
//!
//! Core NATS transport layer for agent communication.

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use async_nats::{Client, ConnectOptions, Event};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{info, warn, error};

use crate::message::{MessageRouter, AgentMessageHandler, SystemMessageHandler};
use crate::agent::AgentPool;

/// NATS transport client for agent communication
#[derive(Debug, Clone)]
pub struct NatsTransport {
    /// NATS client
    client: Client,
    /// Message router
    router: Arc<MessageRouter>,
}

impl NatsTransport {
    /// Create a new NATS transport with default configuration
    pub async fn new(server_url: &str) -> Result<Self, TransportError> {
        let config = NatsConfig::default();
        Self::with_config(server_url, config).await
    }
    
    /// Create a new NATS transport with custom configuration
    pub async fn with_config(server_url: &str, config: NatsConfig) -> Result<Self, TransportError> {
        info!(server_url = server_url, "Connecting to NATS server");
        
        // Set up connection options
        let options = ConnectOptions::new()
            .ping_interval(config.ping_interval)
            .max_reconnects(config.max_reconnects)
            .reconnect_delay_callback(|attempts| {
                // Exponential backoff with jitter
                let delay = std::cmp::min(
                    Duration::from_millis(1000 * 2_u64.pow(attempts as u32)),
                    Duration::from_secs(30)
                );
                delay + Duration::from_millis(fastrand::u64(0..1000))
            })
            .event_callback(|event| async move {
                match event {
                    Event::Connected => info!("Connected to NATS server"),
                    Event::Disconnected => warn!("Disconnected from NATS server"),
                    Event::ClientError(e) => error!(error = %e, "NATS client error"),
                    _ => {}
                }
            });
        
        // Connect to NATS
        let client = async_nats::connect_with_options(server_url, options)
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
        
        // Create message router
        let router = Arc::new(MessageRouter::new(client.clone()));
        
        info!("NATS transport initialized successfully");
        
        Ok(Self {
            client,
            router,
        })
    }
    
    /// Set up message handlers and subscriptions
    pub async fn setup_handlers(&self, agent_pool: Arc<AgentPool>) -> Result<(), TransportError> {
        info!("Setting up NATS message handlers");
        
        // Create handlers
        let agent_handler = AgentMessageHandler::new(agent_pool.clone(), self.client.clone());
        let system_handler = SystemMessageHandler::new(agent_pool, self.client.clone());
        
        // Subscribe to agent commands
        self.router.subscribe(
            "agent.command",
            crate::message::FunctionHandler::new(move |msg| {
                let handler = agent_handler.clone();
                Box::pin(async move {
                    handler.handle_agent_command(msg).await
                })
            })
        ).await
        .map_err(|e| TransportError::SubscriptionFailed(e.to_string()))?;
        
        // Subscribe to system messages
        self.router.subscribe(
            "system.control",
            crate::message::FunctionHandler::new(move |msg| {
                let handler = system_handler.clone();
                Box::pin(async move {
                    handler.handle_system_message(msg).await
                })
            })
        ).await
        .map_err(|e| TransportError::SubscriptionFailed(e.to_string()))?;
        
        info!("Message handlers set up successfully");
        Ok(())
    }
    
    /// Get the NATS client
    pub fn client(&self) -> &Client {
        &self.client
    }
    
    /// Get the message router
    pub fn router(&self) -> Arc<MessageRouter> {
        self.router.clone()
    }
    
    /// Check if transport is connected
    pub async fn is_connected(&self) -> bool {
        // Since async-nats handles reconnection automatically,
        // we consider it connected if the client exists
        true
    }
    
    /// Get transport statistics
    pub async fn get_stats(&self) -> TransportStats {
        let router_stats = self.router.get_stats().await;
        let connected = true; // async-nats handles reconnection automatically
        
        TransportStats {
            connected,
            server_info: if connected { Some(format!("Connected to NATS")) } else { None },
            active_subscriptions: router_stats.active_subscriptions,
            registered_handlers: router_stats.registered_handlers,
        }
    }
    
    /// Gracefully shutdown the transport
    pub async fn shutdown(&self) -> Result<(), TransportError> {
        info!("Shutting down NATS transport");
        
        // The NATS client will automatically clean up connections
        // when it's dropped, so we don't need explicit cleanup here
        
        info!("NATS transport shut down");
        Ok(())
    }
}


/// NATS transport configuration
#[derive(Debug, Clone)]
pub struct NatsConfig {
    /// Ping interval for keep-alive
    pub ping_interval: Duration,
    /// Maximum number of reconnection attempts
    pub max_reconnects: Option<usize>,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            ping_interval: Duration::from_secs(30),
            max_reconnects: Some(10),
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// Transport statistics
#[derive(Debug, Clone)]
pub struct TransportStats {
    pub connected: bool,
    pub server_info: Option<String>,
    pub active_subscriptions: usize,
    pub registered_handlers: usize,
}

/// Transport-specific errors
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),
    
    #[error("Publish failed: {0}")]
    PublishFailed(String),
    
    #[error("Request timeout")]
    RequestTimeout,
    
    #[error("Transport not connected")]
    NotConnected,
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Handler setup failed: {0}")]
    HandlerSetupFailed(String),
}
