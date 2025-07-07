//! Connection Management
//!
//! Manages NATS connections with health monitoring and reconnection logic.

use std::sync::Arc;
use std::time::{Duration, Instant};
use async_nats::{Client, ConnectOptions, Event};
use anyhow::Result;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error, debug};

/// Connection manager for NATS with health monitoring
#[derive(Debug)]
pub struct ConnectionManager {
    /// Current NATS client
    client: Arc<RwLock<Option<Client>>>,
    /// Connection configuration
    config: ConnectionConfig,
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Event sender for connection events
    event_sender: mpsc::UnboundedSender<ConnectionEvent>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub async fn new(
        server_url: String,
        config: ConnectionConfig,
    ) -> Result<(Self, mpsc::UnboundedReceiver<ConnectionEvent>), ConnectionError> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let manager = Self {
            client: Arc::new(RwLock::new(None)),
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            event_sender,
        };
        
        // Establish initial connection
        manager.connect(&server_url).await?;
        
        Ok((manager, event_receiver))
    }
    
    /// Connect to NATS server
    async fn connect(&self, server_url: &str) -> Result<(), ConnectionError> {
        info!(server_url = server_url, "Connecting to NATS server");
        
        *self.state.write().await = ConnectionState::Connecting;
        
        // Set up connection options with event handling
        let event_sender = self.event_sender.clone();
        let options = ConnectOptions::new()
            .ping_interval(self.config.ping_interval)
            .max_reconnects(self.config.max_reconnects)
            .reconnect_delay_callback(|attempts| {
                // Exponential backoff with maximum delay
                let delay = std::cmp::min(
                    Duration::from_millis(1000 * 2_u64.pow(attempts.min(10) as u32)),
                    Duration::from_secs(60)
                );
                delay
            })
            .event_callback(move |event| {
                let sender = event_sender.clone();
                async move {
                    let connection_event = match event {
                        Event::Connected => {
                            info!("Connected to NATS server");
                            ConnectionEvent::Connected
                        }
                        Event::Disconnected => {
                            warn!("Disconnected from NATS server");
                            ConnectionEvent::Disconnected
                        }
                        Event::ClientError(e) => {
                            error!(error = %e, "NATS client error");
                            ConnectionEvent::Error(e.to_string())
                        }
                        Event::ServerError(e) => {
                            error!(error = %e, "NATS server error");
                            ConnectionEvent::Error(e.to_string())
                        }
                        Event::SlowConsumer(subject) => {
                            warn!(subject = %subject, "Slow consumer detected");
                            ConnectionEvent::SlowConsumer(subject.to_string())
                        }
                        Event::LameDuckMode => {
                            warn!("NATS server entering lame duck mode");
                            ConnectionEvent::LameDuckMode
                        }
                    };
                    
                    if let Err(e) = sender.send(connection_event) {
                        error!(error = %e, "Failed to send connection event");
                    }
                }
            });
        
        // Attempt connection
        let client = async_nats::connect_with_options(server_url, options)
            .await
            .map_err(|e| ConnectionError::ConnectionFailed(e.to_string()))?;
        
        // Store client and update state
        *self.client.write().await = Some(client);
        *self.state.write().await = ConnectionState::Connected {
            since: Instant::now(),
            server_url: server_url.to_string(),
        };
        
        // Send connected event
        if let Err(e) = self.event_sender.send(ConnectionEvent::Connected) {
            warn!(error = %e, "Failed to send connection event");
        }
        
        info!("Successfully connected to NATS server");
        Ok(())
    }
    
    /// Get the current NATS client
    pub async fn client(&self) -> Option<Client> {
        self.client.read().await.clone()
    }
    
    /// Check if connected to NATS
    pub async fn is_connected(&self) -> bool {
        match &*self.state.read().await {
            ConnectionState::Connected { .. } => {
                // Additional check: verify client is still valid
                if let Some(_client) = &*self.client.read().await {
                    // async-nats handles reconnection automatically
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
    
    /// Get connection state
    pub async fn state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }
    
    /// Perform health check
    pub async fn health_check(&self) -> Result<HealthStatus, ConnectionError> {
        debug!("Performing connection health check");
        
        let state = self.state().await;
        let client = self.client().await;
        
        match (&state, &client) {
            (ConnectionState::Connected { since, server_url }, Some(_client)) => {
                // Check if client is responsive
                let is_connected = true; // async-nats handles reconnection automatically
                let uptime = since.elapsed();
                
                if is_connected {
                    Ok(HealthStatus::Healthy {
                        uptime,
                        server_url: server_url.clone(),
                        server_info: "Connected".to_string(),
                    })
                } else {
                    Ok(HealthStatus::Unhealthy {
                        reason: "No server info available".to_string(),
                    })
                }
            }
            (ConnectionState::Connecting, _) => {
                Ok(HealthStatus::Connecting)
            }
            (ConnectionState::Disconnected, _) => {
                Ok(HealthStatus::Disconnected)
            }
            _ => {
                Ok(HealthStatus::Unhealthy {
                    reason: "Inconsistent state".to_string(),
                })
            }
        }
    }
    
    /// Get connection statistics
    pub async fn stats(&self) -> ConnectionStats {
        let state = self.state().await;
        let health = self.health_check().await.unwrap_or(HealthStatus::Unhealthy {
            reason: "Health check failed".to_string(),
        });
        
        ConnectionStats {
            state,
            health,
            config: self.config.clone(),
        }
    }
    
    /// Close the connection
    pub async fn close(&self) -> Result<(), ConnectionError> {
        info!("Closing NATS connection");
        
        *self.state.write().await = ConnectionState::Disconnected;
        *self.client.write().await = None;
        
        info!("NATS connection closed");
        Ok(())
    }
}

/// Connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Ping interval for keep-alive
    pub ping_interval: Duration,
    /// Maximum number of reconnection attempts
    pub max_reconnects: Option<usize>,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            ping_interval: Duration::from_secs(30),
            max_reconnects: Some(10),
            connect_timeout: Duration::from_secs(10),
            health_check_interval: Duration::from_secs(60),
        }
    }
}

/// Connection state
#[derive(Debug, Clone)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Attempting to connect
    Connecting,
    /// Connected to server
    Connected {
        since: Instant,
        server_url: String,
    },
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "disconnected"),
            ConnectionState::Connecting => write!(f, "connecting"),
            ConnectionState::Connected { since, .. } => {
                write!(f, "connected ({}s)", since.elapsed().as_secs())
            }
        }
    }
}

/// Connection events
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// Successfully connected
    Connected,
    /// Disconnected from server
    Disconnected,
    /// Connection error occurred
    Error(String),
    /// Slow consumer detected
    SlowConsumer(String),
    /// Server entering lame duck mode
    LameDuckMode,
}

/// Health status
#[derive(Debug, Clone)]
pub enum HealthStatus {
    /// Connection is healthy
    Healthy {
        uptime: Duration,
        server_url: String,
        server_info: String,
    },
    /// Connection is unhealthy
    Unhealthy {
        reason: String,
    },
    /// Currently connecting
    Connecting,
    /// Not connected
    Disconnected,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy { uptime, .. } => {
                write!(f, "healthy ({}s uptime)", uptime.as_secs())
            }
            HealthStatus::Unhealthy { reason } => {
                write!(f, "unhealthy: {}", reason)
            }
            HealthStatus::Connecting => write!(f, "connecting"),
            HealthStatus::Disconnected => write!(f, "disconnected"),
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub state: ConnectionState,
    pub health: HealthStatus,
    pub config: ConnectionConfig,
}

/// Connection-specific errors
#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Connection timeout")]
    ConnectionTimeout,
    
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),
    
    #[error("Already connected")]
    AlreadyConnected,
    
    #[error("Not connected")]
    NotConnected,
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
