//! SSE Bridge Implementation
//!
//! Bridges NATS discovery messages to SSE clients with subscription management,
//! filtering, and connection pooling.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use async_nats::Client;
use axum::{
    extract::{Query, State},
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse},
    routing::get,
    Router,
};
use futures::{stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::collaboration::Discovery;
use crate::mcp_server::DiscoveryFilter;

/// SSE subscription query parameters
#[derive(Debug, Deserialize)]
pub struct SseSubscriptionQuery {
    /// Agent ID for the subscription
    pub agent_id: String,
    /// Subscription ID (optional, generated if not provided)
    pub subscription_id: Option<String>,
}

/// SSE client connection information
#[derive(Debug, Clone)]
pub struct SseClientInfo {
    /// Client ID (unique)
    pub client_id: String,
    /// Agent ID
    pub agent_id: String,
    /// Connection timestamp
    pub connected_at: chrono::DateTime<chrono::Utc>,
    /// Subscription filter
    pub filter: Option<DiscoveryFilter>,
    /// Number of messages sent
    pub messages_sent: u64,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// SSE bridge configuration
#[derive(Debug, Clone)]
pub struct SseBridgeConfig {
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
    /// Message buffer size
    pub message_buffer_size: usize,
    /// Client cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for SseBridgeConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            connection_timeout: Duration::from_secs(300), // 5 minutes
            keep_alive_interval: Duration::from_secs(30),
            message_buffer_size: 1024,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// SSE bridge that connects NATS to SSE clients
pub struct SseBridge {
    /// NATS client
    nats_client: Client,
    /// Connected clients
    clients: Arc<RwLock<HashMap<String, SseClientInfo>>>,
    /// Broadcast channel for discoveries
    discovery_tx: broadcast::Sender<Discovery>,
    /// Subscription filters by client ID
    filters: Arc<RwLock<HashMap<String, DiscoveryFilter>>>,
    /// Configuration
    config: SseBridgeConfig,
}

impl SseBridge {
    /// Create a new SSE bridge
    pub fn new(nats_client: Client, config: SseBridgeConfig) -> Self {
        let (discovery_tx, _) = broadcast::channel(config.message_buffer_size);
        
        let bridge = Self {
            nats_client,
            clients: Arc::new(RwLock::new(HashMap::new())),
            discovery_tx,
            filters: Arc::new(RwLock::new(HashMap::new())),
            config,
        };

        bridge.start_nats_listener();
        bridge.start_client_cleanup();

        bridge
    }

    /// Start listening for NATS discovery messages
    fn start_nats_listener(&self) {
        let nats_client = self.nats_client.clone();
        let discovery_tx = self.discovery_tx.clone();
        
        tokio::spawn(async move {
            info!("🔄 SSE Bridge: Starting NATS listener for discoveries");
            
            // Subscribe to all discovery messages
            let mut sub = match nats_client.subscribe("discoveries.>").await {
                Ok(sub) => sub,
                Err(e) => {
                    error!("❌ Failed to subscribe to NATS discoveries: {}", e);
                    return;
                }
            };

            while let Some(msg) = sub.next().await {
                // Parse discovery message
                match serde_json::from_slice::<Discovery>(&msg.payload) {
                    Ok(discovery) => {
                        debug!(
                            "📡 SSE Bridge: Received discovery from agent {}: {}",
                            discovery.agent_id, discovery.content
                        );
                        
                        // Broadcast to all SSE clients
                        if let Err(e) = discovery_tx.send(discovery) {
                            debug!("No SSE clients connected: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("❌ Failed to parse discovery message: {}", e);
                    }
                }
            }
        });
    }

    /// Start client cleanup task
    fn start_client_cleanup(&self) {
        let clients = self.clients.clone();
        let filters = self.filters.clone();
        let cleanup_interval = self.config.cleanup_interval;
        let connection_timeout = self.config.connection_timeout;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            
            loop {
                interval.tick().await;
                
                let now = chrono::Utc::now();
                let mut to_remove = Vec::new();
                
                // Find expired connections
                {
                    let clients_read = clients.read().await;
                    for (client_id, client_info) in clients_read.iter() {
                        let elapsed = now.signed_duration_since(client_info.last_activity);
                        if elapsed > chrono::Duration::from_std(connection_timeout).unwrap() {
                            to_remove.push(client_id.clone());
                        }
                    }
                }
                
                // Remove expired connections
                if !to_remove.is_empty() {
                    let mut clients_write = clients.write().await;
                    let mut filters_write = filters.write().await;
                    
                    for client_id in to_remove {
                        clients_write.remove(&client_id);
                        filters_write.remove(&client_id);
                        info!("🧹 SSE Bridge: Cleaned up expired client {}", client_id);
                    }
                }
            }
        });
    }

    /// Handle SSE client connection
    pub async fn handle_sse_connection(
        State(bridge): State<Arc<SseBridge>>,
        Query(query): Query<SseSubscriptionQuery>,
    ) -> impl axum::response::IntoResponse {
        let client_id = query.subscription_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let agent_id = query.agent_id.clone();
        
        info!("🔌 SSE Bridge: Client connecting - agent={}, client={}", agent_id, client_id);
        
        // Check connection limits
        {
            let clients = bridge.clients.read().await;
            if clients.len() >= bridge.config.max_connections {
                warn!("⚠️  SSE Bridge: Max connections reached, rejecting client {}", client_id);
                return axum::response::Response::builder()
                    .status(429)
                    .body(axum::body::Body::from("Too many connections"))
                    .unwrap();
            }
        }
        
        // Register client
        let client_info = SseClientInfo {
            client_id: client_id.clone(),
            agent_id: agent_id.clone(),
            connected_at: chrono::Utc::now(),
            filter: None, // Will be set via subscription
            messages_sent: 0,
            last_activity: chrono::Utc::now(),
        };
        
        {
            let mut clients = bridge.clients.write().await;
            clients.insert(client_id.clone(), client_info);
        }
        
        // Create subscription to broadcast channel
        let rx = bridge.discovery_tx.subscribe();
        let stream = BroadcastStream::new(rx);
        
        // Clone values for the stream
        let bridge_clone = bridge.clone();
        let client_id_clone = client_id.clone();
        let agent_id_clone = agent_id.clone();
        
        // Send initial connection event
        let initial_event = Event::default()
            .event("connected")
            .data(format!(
                r#"{{"client_id":"{}","agent_id":"{}","timestamp":"{}"}}"#,
                client_id,
                agent_id,
                chrono::Utc::now().to_rfc3339()
            ));
        
        // Create discovery stream with filtering
        let discovery_stream = stream
            .filter_map(move |result| {
                let bridge = bridge_clone.clone();
                let client_id = client_id_clone.clone();
                let agent_id = agent_id_clone.clone();
                
                async move {
                    match result {
                        Ok(discovery) => {
                            // Don't send discoveries from the same agent back to itself
                            if discovery.agent_id == agent_id {
                                return None;
                            }
                            
                            // Apply filter if one is set
                            let should_include = {
                                let filters = bridge.filters.read().await;
                                match filters.get(&client_id) {
                                    Some(filter) => filter.matches(&discovery),
                                    None => true, // No filter means include all
                                }
                            };
                            
                            if !should_include {
                                return None;
                            }
                            
                            // Update client activity
                            {
                                let mut clients = bridge.clients.write().await;
                                if let Some(client_info) = clients.get_mut(&client_id) {
                                    client_info.messages_sent += 1;
                                    client_info.last_activity = chrono::Utc::now();
                                }
                            }
                            
                            // Create JSON-RPC notification
                            let notification = serde_json::json!({
                                "jsonrpc": "2.0",
                                "method": "notifications/resources/updated",
                                "params": {
                                    "uri": format!(
                                        "discovery://agents/{}/discoveries/{}-{:?}",
                                        discovery.agent_id,
                                        discovery.timestamp.timestamp(),
                                        discovery.discovery_type
                                    )
                                }
                            });
                            
                            match serde_json::to_string(&notification) {
                                Ok(data) => Some(Ok::<Event, std::convert::Infallible>(Event::default()
                                    .event("discovery")
                                    .data(data))),
                                Err(e) => {
                                    error!("❌ Failed to serialize notification: {}", e);
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            warn!("⚠️  Broadcast receive error: {}", e);
                            None
                        }
                    }
                }
            });
        
        // Combine initial event with discovery stream
        let combined_stream = stream::once(async move { Ok(initial_event) })
            .chain(discovery_stream);
        
        // Set up disconnect cleanup
        let bridge_for_cleanup = bridge.clone();
        let client_id_for_cleanup = client_id.clone();
        
        tokio::spawn(async move {
            // Wait for connection timeout or manual disconnect
            tokio::time::sleep(bridge_for_cleanup.config.connection_timeout).await;
            
            // Clean up client
            {
                let mut clients = bridge_for_cleanup.clients.write().await;
                let mut filters = bridge_for_cleanup.filters.write().await;
                clients.remove(&client_id_for_cleanup);
                filters.remove(&client_id_for_cleanup);
            }
            
            info!("🔌 SSE Bridge: Client disconnected - {}", client_id_for_cleanup);
        });
        
        // Create SSE response
        Sse::new(combined_stream)
            .keep_alive(KeepAlive::new().interval(bridge.config.keep_alive_interval))
            .into_response()
    }

    /// Set filter for a client
    pub async fn set_client_filter(&self, client_id: &str, filter: DiscoveryFilter) {
        let mut filters = self.filters.write().await;
        filters.insert(client_id.to_string(), filter.clone());
        
        // Update client info
        let mut clients = self.clients.write().await;
        if let Some(client_info) = clients.get_mut(client_id) {
            client_info.filter = Some(filter);
        }
        
        debug!("🔧 SSE Bridge: Set filter for client {}", client_id);
    }

    /// Get connected clients
    pub async fn get_connected_clients(&self) -> Vec<SseClientInfo> {
        let clients = self.clients.read().await;
        clients.values().cloned().collect()
    }

    /// Get client statistics
    pub async fn get_stats(&self) -> SseBridgeStats {
        let clients = self.clients.read().await;
        let total_connections = clients.len();
        let total_messages_sent = clients.values().map(|c| c.messages_sent).sum();
        
        SseBridgeStats {
            total_connections,
            total_messages_sent,
            max_connections: self.config.max_connections,
            connection_timeout_seconds: self.config.connection_timeout.as_secs(),
        }
    }

    /// Create Axum router for SSE endpoints
    pub fn create_router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/discoveries/stream", get(Self::handle_sse_connection))
            .route("/sse/stats", get(Self::handle_sse_stats))
            .with_state(self)
    }

    /// Handle SSE statistics endpoint
    async fn handle_sse_stats(
        State(bridge): State<Arc<SseBridge>>,
    ) -> impl axum::response::IntoResponse {
        let stats = bridge.get_stats().await;
        axum::Json(stats)
    }
}

/// SSE bridge statistics
#[derive(Debug, Clone, Serialize)]
pub struct SseBridgeStats {
    /// Total active connections
    pub total_connections: usize,
    /// Total messages sent across all connections
    pub total_messages_sent: u64,
    /// Maximum allowed connections
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_sse_bridge_creation() {
        let nats_client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS connection required for test");
        
        let bridge = SseBridge::new(nats_client, SseBridgeConfig::default());
        
        assert_eq!(bridge.get_connected_clients().await.len(), 0);
        assert_eq!(bridge.get_stats().await.total_connections, 0);
    }

    #[test]
    fn test_sse_bridge_config() {
        let config = SseBridgeConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.connection_timeout, Duration::from_secs(300));
        assert_eq!(config.keep_alive_interval, Duration::from_secs(30));
    }

    #[test]
    fn test_sse_client_info() {
        let client_info = SseClientInfo {
            client_id: "test-client".to_string(),
            agent_id: "test-agent".to_string(),
            connected_at: chrono::Utc::now(),
            filter: None,
            messages_sent: 0,
            last_activity: chrono::Utc::now(),
        };
        
        assert_eq!(client_info.client_id, "test-client");
        assert_eq!(client_info.agent_id, "test-agent");
        assert_eq!(client_info.messages_sent, 0);
    }
}