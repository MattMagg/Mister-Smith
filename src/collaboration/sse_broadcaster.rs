//! SSE Broadcaster - Real-time discovery updates via Server-Sent Events
//! 
//! Bridges NATS discovery messages to SSE clients for real-time updates

use crate::collaboration::discovery_sharing::Discovery;
use async_nats::Client;
use axum::{
    extract::{Query, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::get,
    Router,
};
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::Infallible,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, warn};

/// JSON-RPC notification format for SSE messages
#[derive(Debug, Clone, Serialize)]
struct ResourceNotification {
    jsonrpc: String,
    method: String,
    params: NotificationParams,
}

#[derive(Debug, Clone, Serialize)]
struct NotificationParams {
    uri: String,
}

/// Query parameters for SSE endpoint
#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    agent_id: String,
}

/// SSE client connection info
#[derive(Debug, Clone)]
pub struct SseClient {
    agent_id: String,
    connected_at: chrono::DateTime<chrono::Utc>,
}

/// Manages SSE broadcasting for discovery updates
#[derive(Debug)]
pub struct DiscoverySSEBroadcaster {
    /// Connected SSE clients indexed by unique connection ID
    sse_clients: Arc<RwLock<HashMap<String, SseClient>>>,
    /// Broadcast channel for discovery updates
    discovery_tx: broadcast::Sender<Discovery>,
    /// NATS client for subscribing to discoveries
    nats: Client,
}

impl DiscoverySSEBroadcaster {
    /// Create a new SSE broadcaster
    pub fn new(nats: Client) -> Self {
        // Create broadcast channel with buffer for rapid updates
        let (discovery_tx, _) = broadcast::channel(1024);
        
        Self {
            sse_clients: Arc::new(RwLock::new(HashMap::new())),
            discovery_tx,
            nats,
        }
    }
    
    /// Start listening for NATS discovery messages
    pub async fn start_listening(&self) -> anyhow::Result<()> {
        let mut sub = self.nats.subscribe("discoveries.>").await?;
        let tx = self.discovery_tx.clone();
        
        info!("📡 SSE Broadcaster listening for discoveries");
        
        // Spawn background task to bridge NATS to broadcast channel
        tokio::spawn(async move {
            while let Some(msg) = sub.next().await {
                match serde_json::from_slice::<Discovery>(&msg.payload) {
                    Ok(discovery) => {
                        debug!(
                            "Broadcasting discovery from {} to SSE clients",
                            discovery.agent_id
                        );
                        
                        // Send to all SSE clients via broadcast channel
                        if let Err(e) = tx.send(discovery) {
                            // This is OK if no receivers are connected
                            debug!("No SSE clients connected: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse discovery message: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Handle SSE client connection
    pub async fn handle_sse_connection(
        State(broadcaster): State<Arc<DiscoverySSEBroadcaster>>,
        Query(params): Query<StreamQuery>,
    ) -> impl IntoResponse {
        let client_id = uuid::Uuid::new_v4().to_string();
        let agent_id = params.agent_id.clone();
        
        info!("🔌 SSE client connected: agent={}, id={}", agent_id, client_id);
        
        // Register client
        {
            let mut clients = broadcaster.sse_clients.write().await;
            clients.insert(
                client_id.clone(),
                SseClient {
                    agent_id: agent_id.clone(),
                    connected_at: chrono::Utc::now(),
                },
            );
        }
        
        // Create subscription to broadcast channel
        let rx = broadcaster.discovery_tx.subscribe();
        let stream = BroadcastStream::new(rx);
        
        // Clone values for the async block
        let client_id_clone = client_id.clone();
        let clients_ref = broadcaster.sse_clients.clone();
        
        // Send initial connection confirmation
        let initial_event = Event::default()
            .event("connected")
            .data(format!(r#"{{"agent_id":"{}","client_id":"{}"}}"#, params.agent_id, client_id));
        
        // Create a stream that starts with the initial event
        let initial_stream = futures::stream::once(async move {
            Ok::<_, Infallible>(initial_event)
        });
        
        // Create SSE stream for discoveries
        let discovery_stream = stream
            .filter_map(move |result| {
                let agent_id = agent_id.clone();
                async move {
                    match result {
                        Ok(discovery) => {
                            // Create JSON-RPC notification
                            let notification = ResourceNotification {
                                jsonrpc: "2.0".to_string(),
                                method: "notifications/resources/updated".to_string(),
                                params: NotificationParams {
                                    uri: format!(
                                        "discovery://agents/{}/discoveries/{}",
                                        discovery.agent_id,
                                        // Generate discovery ID from timestamp and type
                                        format!(
                                            "{}-{:?}-{}",
                                            discovery.timestamp.timestamp(),
                                            discovery.discovery_type,
                                            &discovery.content[..20.min(discovery.content.len())]
                                                .replace(' ', "-")
                                                .to_lowercase()
                                        )
                                    ),
                                },
                            };
                            
                            // Convert to SSE event
                            match serde_json::to_string(&notification) {
                                Ok(data) => {
                                    Some(Ok::<_, Infallible>(Event::default()
                                        .event("discovery")
                                        .data(data)))
                                }
                                Err(e) => {
                                    error!("Failed to serialize notification: {}", e);
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Broadcast receive error: {}", e);
                            None
                        }
                    }
                }
            });
        
        // Combine initial event with discovery stream
        let combined_stream = initial_stream.chain(discovery_stream);
        
        // Clone for disconnect handler
        let client_id_for_disconnect = client_id.clone();
        let clients_for_disconnect = broadcaster.sse_clients.clone();
        
        // Spawn task to handle cleanup on disconnect
        tokio::spawn(async move {
            // Note: In production, you'd want a more sophisticated way to detect disconnection
            // This is simplified for the example
            tokio::time::sleep(Duration::from_secs(3600)).await; // 1 hour timeout
            info!("🔌 SSE client timeout: {}", client_id_for_disconnect);
            clients_for_disconnect.write().await.remove(&client_id_for_disconnect);
        });
        
        // Create SSE response with keep-alive
        Sse::new(combined_stream)
            .keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
    }
    
    /// Get current client connections
    pub async fn get_connected_clients(&self) -> Vec<(String, SseClient)> {
        self.sse_clients
            .read()
            .await
            .iter()
            .map(|(id, client)| (id.clone(), client.clone()))
            .collect()
    }
    
    /// Create Axum router for SSE endpoints
    pub fn create_router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/discoveries/stream", get(Self::handle_sse_connection))
            .layer(CorsLayer::permissive())
            .with_state(self)
    }
}

/// Example SSE event stream output
#[cfg(test)]
mod example_output {
    /// Example SSE event stream that would be sent to clients:
    /// ```text
    /// event: connected
    /// data: {"agent_id":"agent-123","client_id":"550e8400-e29b-41d4-a716-446655440000"}
    /// 
    /// event: discovery
    /// data: {"jsonrpc":"2.0","method":"notifications/resources/updated","params":{"uri":"discovery://agents/agent-456/discoveries/1234567890-Pattern-unusual-login-patte"}}
    /// 
    /// : keep-alive
    /// 
    /// event: discovery  
    /// data: {"jsonrpc":"2.0","method":"notifications/resources/updated","params":{"uri":"discovery://agents/agent-789/discoveries/1234567895-Anomaly-database-queries-sp"}}
    /// 
    /// : keep-alive
    /// ```
    const _: &str = "";
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_sse_broadcaster_creation() {
        let nats = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS connection required for test");
            
        let broadcaster = Arc::new(DiscoverySSEBroadcaster::new(nats));
        
        // Verify initial state
        let clients = broadcaster.get_connected_clients().await;
        assert_eq!(clients.len(), 0, "Should start with no clients");
    }
}