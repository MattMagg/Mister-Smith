//! NATS Configuration for MCP Server
//!
//! Provides configuration structures for NATS client integration
//! with connection management, retry policies, and health monitoring.

use std::time::Duration;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use async_nats::{Client, ConnectOptions, Event};
use tracing::{info, warn, error};

/// NATS configuration for MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// NATS server URL
    pub server_url: String,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Ping interval for keep-alive
    pub ping_interval: Duration,
    /// Maximum number of reconnection attempts
    pub max_reconnects: Option<usize>,
    /// Initial reconnection delay
    pub reconnect_delay: Duration,
    /// Maximum reconnection delay
    pub max_reconnect_delay: Duration,
    /// Subject patterns for discovery publishing
    pub subject_patterns: SubjectPatterns,
    /// Connection pool settings
    pub connection_pool: ConnectionPoolConfig,
    /// Health check configuration
    pub health_check: HealthCheckConfig,
}

/// Subject patterns for NATS messaging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectPatterns {
    /// Discovery subject pattern: discoveries.{agent_id}.{type}
    pub discovery_pattern: String,
    /// Discovery wildcard for subscriptions: discoveries.>
    pub discovery_wildcard: String,
    /// SSE subscription pattern: sse.subscriptions.{id}
    pub sse_subscription_pattern: String,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum number of connections in pool
    pub max_connections: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Connection validation interval
    pub validation_interval: Duration,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check interval
    pub interval: Duration,
    /// Health check timeout
    pub timeout: Duration,
    /// Maximum consecutive failures before marking unhealthy
    pub max_failures: usize,
    /// Recovery check interval when unhealthy
    pub recovery_interval: Duration,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            server_url: "nats://localhost:4222".to_string(),
            connect_timeout: Duration::from_secs(10),
            ping_interval: Duration::from_secs(30),
            max_reconnects: Some(10),
            reconnect_delay: Duration::from_millis(500),
            max_reconnect_delay: Duration::from_secs(30),
            subject_patterns: SubjectPatterns::default(),
            connection_pool: ConnectionPoolConfig::default(),
            health_check: HealthCheckConfig::default(),
        }
    }
}

impl Default for SubjectPatterns {
    fn default() -> Self {
        Self {
            discovery_pattern: "discoveries.{}.{}".to_string(),
            discovery_wildcard: "discoveries.>".to_string(),
            sse_subscription_pattern: "sse.subscriptions.{}".to_string(),
        }
    }
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            validation_interval: Duration::from_secs(60), // 1 minute
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            max_failures: 3,
            recovery_interval: Duration::from_secs(10),
        }
    }
}

/// NATS client manager with connection pooling and health monitoring
pub struct NatsClientManager {
    /// NATS client
    client: Client,
    /// Configuration
    config: NatsConfig,
    /// Connection health status
    is_healthy: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Connection statistics
    stats: std::sync::Arc<std::sync::RwLock<ConnectionStats>>,
}

/// Connection statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct ConnectionStats {
    /// Total messages published
    pub messages_published: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total connection failures
    pub connection_failures: u64,
    /// Total reconnection attempts
    pub reconnection_attempts: u64,
    /// Last health check timestamp
    #[serde(skip)]
    pub last_health_check: Option<std::time::Instant>,
    /// Current connection status
    pub connection_status: ConnectionStatus,
}

/// Connection status enumeration
#[derive(Debug, Clone, Default, Serialize)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

impl NatsClientManager {
    /// Create a new NATS client manager
    pub async fn new(config: NatsConfig) -> Result<Self> {
        info!("Initializing NATS client manager with server: {}", config.server_url);
        
        // Create connection options with retry and health monitoring
        let options = ConnectOptions::new()
            .ping_interval(config.ping_interval)
            .max_reconnects(config.max_reconnects)
            .reconnect_delay_callback({
                let initial_delay = config.reconnect_delay;
                let max_delay = config.max_reconnect_delay;
                move |attempts| {
                    let delay = std::cmp::min(
                        initial_delay.mul_f64(2.0_f64.powi(attempts as i32)),
                        max_delay
                    );
                    // Add jitter to prevent thundering herd
                    let jitter = Duration::from_millis(fastrand::u64(0..100));
                    delay + jitter
                }
            })
            .event_callback(|event| async move {
                match event {
                    Event::Connected => {
                        info!("✅ NATS client connected successfully");
                    }
                    Event::Disconnected => {
                        warn!("⚠️  NATS client disconnected");
                    }
                    Event::ClientError(ref e) => {
                        error!("❌ NATS client error: {}", e);
                    }
                    Event::ServerError(ref e) => {
                        error!("🖥️  NATS server error: {}", e);
                    }
                    Event::SlowConsumer(_) => {
                        warn!("🐌 NATS slow consumer detected");
                    }
                    Event::LameDuckMode => {
                        warn!("🦆 NATS server entering lame duck mode");
                    }
                }
            });

        // Connect to NATS server
        let client = async_nats::connect_with_options(&config.server_url, options).await?;
        
        let is_healthy = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let stats = std::sync::Arc::new(std::sync::RwLock::new(ConnectionStats::default()));
        
        // Update initial connection status
        {
            let mut stats = stats.write().unwrap();
            stats.connection_status = ConnectionStatus::Connected;
            stats.last_health_check = Some(std::time::Instant::now());
        }

        let manager = Self {
            client,
            config,
            is_healthy,
            stats,
        };

        // Start health monitoring
        manager.start_health_monitoring();

        info!("🎯 NATS client manager initialized successfully");
        Ok(manager)
    }

    /// Get the NATS client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get configuration
    pub fn config(&self) -> &NatsConfig {
        &self.config
    }

    /// Check if connection is healthy
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> ConnectionStats {
        self.stats.read().unwrap().clone()
    }

    /// Start health monitoring background task
    fn start_health_monitoring(&self) {
        let client = self.client.clone();
        let config = self.config.clone();
        let is_healthy = self.is_healthy.clone();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.health_check.interval);
            let mut consecutive_failures = 0;

            loop {
                interval.tick().await;
                
                // Simple health check: check connection state
                let is_healthy_now = match client.connection_state() {
                    async_nats::connection::State::Connected => {
                        consecutive_failures = 0;
                        true
                    }
                    _ => {
                        consecutive_failures += 1;
                        error!("NATS connection not healthy: {:?}", client.connection_state());
                        consecutive_failures < config.health_check.max_failures
                    }
                };

                is_healthy.store(is_healthy_now, std::sync::atomic::Ordering::Relaxed);

                // Update stats
                {
                    let mut stats = stats.write().unwrap();
                    stats.last_health_check = Some(std::time::Instant::now());
                    stats.connection_status = if is_healthy_now {
                        ConnectionStatus::Connected
                    } else {
                        ConnectionStatus::Failed
                    };
                }

                if !is_healthy_now {
                    warn!("❌ NATS connection unhealthy (failures: {})", consecutive_failures);
                    // Use recovery interval for faster checks when unhealthy
                    tokio::time::sleep(config.health_check.recovery_interval).await;
                }
            }
        });
    }

    /// Increment published message count
    pub fn increment_published(&self) {
        let mut stats = self.stats.write().unwrap();
        stats.messages_published += 1;
    }

    /// Increment received message count
    pub fn increment_received(&self) {
        let mut stats = self.stats.write().unwrap();
        stats.messages_received += 1;
    }

    /// Format subject with pattern
    pub fn format_subject(&self, pattern: &str, components: &[&str]) -> String {
        let mut subject = pattern.to_string();
        for component in components {
            subject = subject.replacen("{}", component, 1);
        }
        subject
    }

    /// Get discovery subject for agent and type
    pub fn get_discovery_subject(&self, agent_id: &str, discovery_type: &str) -> String {
        self.format_subject(&self.config.subject_patterns.discovery_pattern, &[agent_id, discovery_type])
    }

    /// Get discovery wildcard subject
    pub fn get_discovery_wildcard(&self) -> &str {
        &self.config.subject_patterns.discovery_wildcard
    }
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting"),
            ConnectionStatus::Connected => write!(f, "Connected"),
            ConnectionStatus::Reconnecting => write!(f, "Reconnecting"),
            ConnectionStatus::Failed => write!(f, "Failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NatsConfig::default();
        assert_eq!(config.server_url, "nats://localhost:4222");
        assert_eq!(config.ping_interval, Duration::from_secs(30));
        assert_eq!(config.max_reconnects, Some(10));
    }

    #[test]
    fn test_subject_formatting() {
        let config = NatsConfig::default();
        let manager = NatsClientManager {
            client: async_nats::connect("nats://localhost:4222").await.unwrap(),
            config,
            is_healthy: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
            stats: std::sync::Arc::new(std::sync::RwLock::new(ConnectionStats::default())),
        };
        
        let subject = manager.get_discovery_subject("agent-123", "pattern");
        assert_eq!(subject, "discoveries.agent-123.pattern");
    }

    #[test]
    fn test_connection_status_display() {
        assert_eq!(ConnectionStatus::Connected.to_string(), "Connected");
        assert_eq!(ConnectionStatus::Failed.to_string(), "Failed");
    }

    #[tokio::test]
    async fn test_client_manager_creation() {
        // This test requires a running NATS server
        if let Ok(manager) = NatsClientManager::new(NatsConfig::default()).await {
            assert!(manager.is_healthy());
            assert_eq!(manager.get_stats().messages_published, 0);
        }
    }
}