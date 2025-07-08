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
use crate::agent::{AgentPool, AgentState};
use crate::transport::supervision_events::{
    SupervisionEventPublisher, SupervisionEventSubscriber,
    SupervisionDecisionHandler, LifecycleEventType, LifecycleMetadata,
};

/// Subject hierarchy patterns for Phase 3
pub mod subjects {
    /// Agent communication subjects
    pub mod agents {
        pub const COMMANDS: &str = "agents.{}.commands.{}";
        pub const STATUS: &str = "agents.{}.status.{}";
        pub const HEARTBEAT: &str = "agents.{}.heartbeat";
        pub const CAPABILITIES: &str = "agents.{}.capabilities";
        pub const METRICS: &str = "agents.{}.metrics.{}";
        pub const EVENTS: &str = "agents.{}.events.{}";
        
        // Legacy compatibility
        pub const LEGACY_COMMAND: &str = "agent.command";
    }
    
    /// Task distribution subjects
    pub mod tasks {
        pub const ASSIGNMENT: &str = "tasks.{}.assignment";
        pub const QUEUE: &str = "tasks.{}.queue.{}";
        pub const PROGRESS: &str = "tasks.{}.progress";
        pub const RESULT: &str = "tasks.{}.result";
    }
    
    /// System management subjects
    pub mod system {
        pub const CONTROL: &str = "system.control.{}";
        pub const DISCOVERY: &str = "system.discovery.{}";
        pub const HEALTH: &str = "system.health.{}";
        pub const CONFIG: &str = "system.config.{}.{}";
        
        // Legacy compatibility
        pub const LEGACY_CONTROL: &str = "system.control";
    }
    
    /// Agent discovery subjects
    pub mod discovery {
        pub const ANNOUNCE: &str = "agents.discovery.announce";
        pub const QUERY: &str = "agents.discovery.query";
        pub const RESPONSE: &str = "agents.discovery.response";
        pub const DEPARTING: &str = "agents.discovery.departing";
    }
}

/// NATS transport client for agent communication
#[derive(Debug, Clone)]
pub struct NatsTransport {
    /// NATS client
    client: Client,
    /// Message router
    router: Arc<MessageRouter>,
    /// Supervision event publisher
    event_publisher: Option<Arc<SupervisionEventPublisher>>,
    /// Supervision event subscriber
    event_subscriber: Option<Arc<SupervisionEventSubscriber>>,
    /// Enable Phase 3 features
    phase3_enabled: bool,
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
            event_publisher: None,
            event_subscriber: None,
            phase3_enabled: config.enable_phase3_features,
        })
    }
    
    /// Set up message handlers and subscriptions
    pub async fn setup_handlers(&mut self, agent_pool: Arc<AgentPool>) -> Result<(), TransportError> {
        info!("Setting up NATS message handlers (Phase {} mode)", 
            if self.phase3_enabled { "3" } else { "2" });
        
        // Create handlers
        let agent_handler = AgentMessageHandler::new(agent_pool.clone(), self.client.clone());
        let system_handler = SystemMessageHandler::new(agent_pool.clone(), self.client.clone());
        
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
        
        // Set up Phase 3 supervision events if enabled
        if self.phase3_enabled {
            self.setup_supervision_events(agent_pool).await?;
        }
        
        info!("Message handlers set up successfully");
        Ok(())
    }
    
    /// Set up Phase 3 supervision event system
    async fn setup_supervision_events(&mut self, agent_pool: Arc<AgentPool>) -> Result<(), TransportError> {
        info!("Setting up Phase 3 supervision event system");
        
        // Create event publisher
        let publisher = Arc::new(SupervisionEventPublisher::new(
            self.client.clone(),
            agent_pool.clone(),
        ));
        self.event_publisher = Some(publisher.clone());
        
        // Create event subscriber
        let subscriber = Arc::new(SupervisionEventSubscriber::new(self.client.clone()));
        self.event_subscriber = Some(subscriber.clone());
        
        // Create decision handler
        let decision_handler = Arc::new(SupervisionDecisionHandler::new(
            agent_pool.clone(),
            (*publisher).clone(),
        ));
        
        // Subscribe to lifecycle events
        let handler = decision_handler.clone();
        subscriber.subscribe_lifecycle_events(move |event| {
            let handler = handler.clone();
            tokio::spawn(async move {
                handler.handle_lifecycle_change(event).await;
            });
        }).await
        .map_err(|e| TransportError::SubscriptionFailed(e.to_string()))?;
        
        // Subscribe to health events
        let handler = decision_handler.clone();
        subscriber.subscribe_health_events(None, move |event| {
            let handler = handler.clone();
            tokio::spawn(async move {
                handler.handle_health_degradation(event).await;
            });
        }).await
        .map_err(|e| TransportError::SubscriptionFailed(e.to_string()))?;
        
        info!("Supervision event system initialized");
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
    
    /// Subscribe with queue group for load balancing (Phase 3)
    pub async fn queue_subscribe(
        &self,
        subject: &str,
        queue_group: &str,
    ) -> Result<async_nats::Subscriber, TransportError> {
        if !self.phase3_enabled {
            return Err(TransportError::ConfigError(
                "Queue groups require Phase 3 features to be enabled".to_string()
            ));
        }
        
        self.client
            .queue_subscribe(subject.to_string(), queue_group.to_string())
            .await
            .map_err(|e| TransportError::SubscriptionFailed(e.to_string()))
    }
    
    /// Build a hierarchical subject (Phase 3)
    pub fn build_subject(pattern: &str, components: &[&str]) -> String {
        let mut subject = pattern.to_string();
        for component in components {
            subject = subject.replacen("{}", component, 1);
        }
        subject
    }
    
    /// Publish agent lifecycle event (Phase 3)
    pub async fn publish_lifecycle_event(
        &self,
        agent_id: &str,
        event_type: LifecycleEventType,
        previous_state: AgentState,
        new_state: AgentState,
        trigger: String,
    ) -> Result<(), TransportError> {
        if let Some(publisher) = &self.event_publisher {
            publisher.publish_lifecycle_event(
                agent_id,
                event_type,
                previous_state,
                new_state,
                LifecycleMetadata {
                    trigger,
                    supervisor_id: None,
                    related_task_id: None,
                    error_details: None,
                },
            ).await
            .map_err(|e| TransportError::PublishFailed(e.to_string()))?;
        }
        Ok(())
    }
    
    /// Get transport statistics
    pub async fn get_stats(&self) -> TransportStats {
        let router_stats = self.router.get_stats().await;
        let connected = true; // async-nats handles reconnection automatically
        
        TransportStats {
            connected,
            server_info: if connected { 
                Some(format!("Connected to NATS (Phase {} mode)", 
                    if self.phase3_enabled { "3" } else { "2" }))
            } else { 
                None 
            },
            active_subscriptions: router_stats.active_subscriptions,
            registered_handlers: router_stats.registered_handlers,
            phase3_enabled: self.phase3_enabled,
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
    /// Enable Phase 3 features (hierarchical subjects, supervision events)
    pub enable_phase3_features: bool,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            ping_interval: Duration::from_secs(30),
            max_reconnects: Some(10),
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            enable_phase3_features: false, // Default to Phase 2 for backward compatibility
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
    pub phase3_enabled: bool,
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
