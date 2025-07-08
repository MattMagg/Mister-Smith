//! MCP Server for Discovery Sharing - Complete Test Implementation
//!
//! This module implements a complete MCP server with discovery sharing tools
//! that can be used for integration testing of the real-time discovery system.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_nats::Client;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

// Import the discovery sharing components
use mistersmith::collaboration::{
    Discovery, DiscoveryBroadcaster, DiscoveryListener, DiscoverySSEBroadcaster, DiscoveryType,
};

/// MCP JSON-RPC 2.0 request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// MCP JSON-RPC 2.0 response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<McpError>,
}

/// MCP error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// MCP notification structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

/// Discovery tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareDiscoveryParams {
    pub discovery_type: String,
    pub content: String,
    pub confidence: f64,
    pub agent_id: Option<String>,
    pub agent_role: Option<String>,
}

/// Discovery query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDiscoveriesParams {
    pub agent_id: Option<String>,
    pub discovery_type: Option<String>,
    pub min_confidence: Option<f64>,
    pub limit: Option<usize>,
}

/// Discovery stream parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDiscoveriesParams {
    pub agent_id: String,
    pub discovery_types: Option<Vec<String>>,
    pub min_confidence: Option<f64>,
}

/// MCP Server state
#[derive(Debug)]
pub struct McpDiscoveryServer {
    /// Server identifier
    pub server_id: String,
    /// NATS client for discovery sharing
    pub nats: Client,
    /// Discovery broadcaster for sharing discoveries
    pub broadcaster: DiscoveryBroadcaster,
    /// Discovery listener for receiving discoveries
    pub listener: DiscoveryListener,
    /// SSE broadcaster for real-time updates
    pub sse_broadcaster: Arc<DiscoverySSEBroadcaster>,
    /// Connected MCP clients
    pub connected_clients: Arc<RwLock<HashMap<String, McpClientInfo>>>,
    /// Stored discoveries for querying
    pub stored_discoveries: Arc<RwLock<Vec<Discovery>>>,
    /// Broadcast channel for real-time updates
    pub update_tx: broadcast::Sender<Discovery>,
    /// Server statistics
    pub stats: Arc<RwLock<ServerStats>>,
}

/// MCP client information
#[derive(Debug, Clone)]
pub struct McpClientInfo {
    pub client_id: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub agent_id: Option<String>,
    pub agent_role: Option<String>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Server statistics
#[derive(Debug, Clone, Default)]
pub struct ServerStats {
    pub total_discoveries_shared: u64,
    pub total_discoveries_received: u64,
    pub total_mcp_requests: u64,
    pub total_mcp_notifications: u64,
    pub connected_clients: u64,
    pub sse_connections: u64,
}

impl McpDiscoveryServer {
    /// Create a new MCP discovery server
    pub async fn new(server_id: String, nats: Client) -> Result<Self> {
        let broadcaster = DiscoveryBroadcaster::new(
            server_id.clone(),
            "MCP-Server".to_string(),
            nats.clone(),
        );

        let listener = DiscoveryListener::new(
            server_id.clone(),
            "MCP-Server".to_string(),
            nats.clone(),
        );

        let sse_broadcaster = Arc::new(DiscoverySSEBroadcaster::new(nats.clone()));

        let (update_tx, _) = broadcast::channel(1024);

        let server = Self {
            server_id,
            nats,
            broadcaster,
            listener,
            sse_broadcaster,
            connected_clients: Arc::new(RwLock::new(HashMap::new())),
            stored_discoveries: Arc::new(RwLock::new(Vec::new())),
            update_tx,
            stats: Arc::new(RwLock::new(ServerStats::default())),
        };

        // Start the SSE broadcaster
        server.sse_broadcaster.start_listening().await?;

        Ok(server)
    }

    /// Start listening for discoveries
    pub async fn start_discovery_listener(&self) -> Result<()> {
        let stored_discoveries = self.stored_discoveries.clone();
        let update_tx = self.update_tx.clone();
        let stats = self.stats.clone();
        let server_id = self.server_id.clone();

        // Clone the listener for the background task
        let listener = DiscoveryListener::new(
            self.server_id.clone(),
            "MCP-Server".to_string(),
            self.nats.clone(),
        );

        tokio::spawn(async move {
            if let Err(e) = listener.listen(move |discovery| {
                let stored_discoveries = stored_discoveries.clone();
                let update_tx = update_tx.clone();
                let stats = stats.clone();
                let server_id = server_id.clone();

                tokio::spawn(async move {
                    // Store the discovery
                    stored_discoveries.write().await.push(discovery.clone());

                    // Update statistics
                    stats.write().await.total_discoveries_received += 1;

                    // Broadcast to connected clients
                    if let Err(e) = update_tx.send(discovery.clone()) {
                        debug!("No active subscribers for discovery update: {}", e);
                    }

                    info!(
                        "MCP Server {} received discovery from {}: {}",
                        server_id, discovery.agent_id, discovery.content
                    );
                });

                Ok(())
            }).await {
                error!("Discovery listener error: {}", e);
            }
        });

        Ok(())
    }

    /// Create the HTTP router for the MCP server
    pub fn create_router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/mcp", post(Self::handle_mcp_request))
            .route("/mcp/ws", get(Self::handle_websocket_upgrade))
            .route("/discoveries/stream", get(Self::handle_sse_stream))
            .route("/stats", get(Self::get_server_stats))
            .route("/health", get(Self::health_check))
            .merge(self.sse_broadcaster.create_router())
            .with_state(self)
    }

    /// Handle MCP JSON-RPC requests
    async fn handle_mcp_request(
        State(server): State<Arc<McpDiscoveryServer>>,
        Json(request): Json<McpRequest>,
    ) -> Json<McpResponse> {
        // Update statistics
        server.stats.write().await.total_mcp_requests += 1;

        let response = match server.process_mcp_request(request.clone()).await {
            Ok(result) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(result),
                error: None,
            },
            Err(e) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: e.to_string(),
                    data: None,
                }),
            },
        };

        Json(response)
    }

    /// Process an MCP request
    async fn process_mcp_request(&self, request: McpRequest) -> Result<Value> {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(request.params).await,
            "notifications/initialized" => self.handle_initialized().await,
            "resources/list" => self.handle_resources_list().await,
            "resources/read" => self.handle_resources_read(request.params).await,
            _ => Err(anyhow::anyhow!("Method not found: {}", request.method)),
        }
    }

    /// Handle initialize request
    async fn handle_initialize(&self, _params: Option<Value>) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {},
                "notifications": {
                    "discovery_shared": {},
                    "discovery_received": {}
                }
            },
            "serverInfo": {
                "name": "MCP Discovery Server",
                "version": "1.0.0",
                "description": "Real-time discovery sharing server for MisterSmith"
            }
        }))
    }

    /// Handle tools/list request
    async fn handle_tools_list(&self) -> Result<Value> {
        Ok(json!({
            "tools": [
                {
                    "name": "share_discovery",
                    "description": "Share a discovery with other agents in real-time",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "discovery_type": {
                                "type": "string",
                                "enum": ["Pattern", "Anomaly", "Connection", "Solution", "Question", "Insight"],
                                "description": "Type of discovery being shared"
                            },
                            "content": {
                                "type": "string",
                                "description": "Content of the discovery"
                            },
                            "confidence": {
                                "type": "number",
                                "minimum": 0.0,
                                "maximum": 1.0,
                                "description": "Confidence level of the discovery"
                            },
                            "agent_id": {
                                "type": "string",
                                "description": "Optional agent identifier"
                            },
                            "agent_role": {
                                "type": "string",
                                "description": "Optional agent role"
                            }
                        },
                        "required": ["discovery_type", "content", "confidence"]
                    }
                },
                {
                    "name": "query_discoveries",
                    "description": "Query stored discoveries with optional filters",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "agent_id": {
                                "type": "string",
                                "description": "Filter by agent ID"
                            },
                            "discovery_type": {
                                "type": "string",
                                "enum": ["Pattern", "Anomaly", "Connection", "Solution", "Question", "Insight"],
                                "description": "Filter by discovery type"
                            },
                            "min_confidence": {
                                "type": "number",
                                "minimum": 0.0,
                                "maximum": 1.0,
                                "description": "Minimum confidence threshold"
                            },
                            "limit": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 100,
                                "description": "Maximum number of results"
                            }
                        }
                    }
                },
                {
                    "name": "stream_discoveries",
                    "description": "Start streaming real-time discovery updates",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "agent_id": {
                                "type": "string",
                                "description": "Agent identifier for the stream"
                            },
                            "discovery_types": {
                                "type": "array",
                                "items": {
                                    "type": "string",
                                    "enum": ["Pattern", "Anomaly", "Connection", "Solution", "Question", "Insight"]
                                },
                                "description": "Filter by discovery types"
                            },
                            "min_confidence": {
                                "type": "number",
                                "minimum": 0.0,
                                "maximum": 1.0,
                                "description": "Minimum confidence threshold"
                            }
                        },
                        "required": ["agent_id"]
                    }
                }
            ]
        }))
    }

    /// Handle tools/call request
    async fn handle_tools_call(&self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| anyhow::anyhow!("Missing parameters"))?;
        
        let name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;

        let arguments = params.get("arguments")
            .ok_or_else(|| anyhow::anyhow!("Missing tool arguments"))?;

        match name {
            "share_discovery" => self.tool_share_discovery(arguments).await,
            "query_discoveries" => self.tool_query_discoveries(arguments).await,
            "stream_discoveries" => self.tool_stream_discoveries(arguments).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }

    /// Handle initialized notification
    async fn handle_initialized(&self) -> Result<Value> {
        Ok(json!({}))
    }

    /// Handle resources/list request
    async fn handle_resources_list(&self) -> Result<Value> {
        let discoveries = self.stored_discoveries.read().await;
        let mut resources = Vec::new();

        for (i, discovery) in discoveries.iter().enumerate() {
            resources.push(json!({
                "uri": format!("discovery://agents/{}/discoveries/{}", discovery.agent_id, i),
                "name": format!("Discovery {} from {}", i, discovery.agent_id),
                "description": discovery.content,
                "mimeType": "application/json"
            }));
        }

        Ok(json!({
            "resources": resources
        }))
    }

    /// Handle resources/read request
    async fn handle_resources_read(&self, params: Option<Value>) -> Result<Value> {
        let params = params.ok_or_else(|| anyhow::anyhow!("Missing parameters"))?;
        
        let uri = params.get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing resource URI"))?;

        // Parse URI to extract discovery index
        if let Some(index_str) = uri.split('/').last() {
            if let Ok(index) = index_str.parse::<usize>() {
                let discoveries = self.stored_discoveries.read().await;
                if let Some(discovery) = discoveries.get(index) {
                    return Ok(json!({
                        "contents": [
                            {
                                "uri": uri,
                                "mimeType": "application/json",
                                "text": serde_json::to_string_pretty(discovery)?
                            }
                        ]
                    }));
                }
            }
        }

        Err(anyhow::anyhow!("Resource not found: {}", uri))
    }

    /// Tool: share_discovery
    async fn tool_share_discovery(&self, arguments: &Value) -> Result<Value> {
        let params: ShareDiscoveryParams = serde_json::from_value(arguments.clone())?;

        let discovery_type = match params.discovery_type.as_str() {
            "Pattern" => DiscoveryType::Pattern,
            "Anomaly" => DiscoveryType::Anomaly,
            "Connection" => DiscoveryType::Connection,
            "Solution" => DiscoveryType::Solution,
            "Question" => DiscoveryType::Question,
            "Insight" => DiscoveryType::Insight,
            _ => return Err(anyhow::anyhow!("Invalid discovery type: {}", params.discovery_type)),
        };

        // Share the discovery
        self.broadcaster.share_discovery(
            discovery_type,
            &params.content,
            params.confidence as f32,
        ).await?;

        // Update statistics
        self.stats.write().await.total_discoveries_shared += 1;

        Ok(json!({
            "success": true,
            "message": "Discovery shared successfully",
            "discovery_type": params.discovery_type,
            "content": params.content,
            "confidence": params.confidence
        }))
    }

    /// Tool: query_discoveries
    async fn tool_query_discoveries(&self, arguments: &Value) -> Result<Value> {
        let params: QueryDiscoveriesParams = serde_json::from_value(arguments.clone())?;

        let discoveries = self.stored_discoveries.read().await;
        let mut filtered_discoveries = Vec::new();

        for discovery in discoveries.iter() {
            // Apply filters
            if let Some(agent_id) = &params.agent_id {
                if discovery.agent_id != *agent_id {
                    continue;
                }
            }

            if let Some(discovery_type) = &params.discovery_type {
                let type_matches = match discovery_type.as_str() {
                    "Pattern" => discovery.discovery_type == DiscoveryType::Pattern,
                    "Anomaly" => discovery.discovery_type == DiscoveryType::Anomaly,
                    "Connection" => discovery.discovery_type == DiscoveryType::Connection,
                    "Solution" => discovery.discovery_type == DiscoveryType::Solution,
                    "Question" => discovery.discovery_type == DiscoveryType::Question,
                    "Insight" => discovery.discovery_type == DiscoveryType::Insight,
                    _ => false,
                };
                if !type_matches {
                    continue;
                }
            }

            if let Some(min_confidence) = params.min_confidence {
                if discovery.confidence < min_confidence as f32 {
                    continue;
                }
            }

            filtered_discoveries.push(discovery.clone());
        }

        // Apply limit
        if let Some(limit) = params.limit {
            filtered_discoveries.truncate(limit);
        }

        Ok(json!({
            "discoveries": filtered_discoveries,
            "total_count": filtered_discoveries.len(),
            "filtered_from": discoveries.len()
        }))
    }

    /// Tool: stream_discoveries
    async fn tool_stream_discoveries(&self, arguments: &Value) -> Result<Value> {
        let params: StreamDiscoveriesParams = serde_json::from_value(arguments.clone())?;

        // For now, return information about how to connect to the stream
        // In a real implementation, this would set up a streaming connection
        Ok(json!({
            "stream_url": format!("/discoveries/stream?agent_id={}", params.agent_id),
            "message": "Connect to the SSE endpoint to receive real-time discovery updates",
            "filters": {
                "discovery_types": params.discovery_types,
                "min_confidence": params.min_confidence
            }
        }))
    }

    /// Handle WebSocket upgrade for real-time communication
    async fn handle_websocket_upgrade(
        State(server): State<Arc<McpDiscoveryServer>>,
        Query(params): Query<HashMap<String, String>>,
        ws: WebSocketUpgrade,
    ) -> Response {
        ws.on_upgrade(move |socket| server.handle_websocket(socket, params))
    }

    /// Handle WebSocket connection
    async fn handle_websocket(
        &self,
        socket: WebSocket,
        params: HashMap<String, String>,
    ) {
        let client_id = params.get("client_id")
            .cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        info!("WebSocket client connected: {}", client_id);

        // Register client
        {
            let mut clients = self.connected_clients.write().await;
            clients.insert(client_id.clone(), McpClientInfo {
                client_id: client_id.clone(),
                connected_at: chrono::Utc::now(),
                agent_id: params.get("agent_id").cloned(),
                agent_role: params.get("agent_role").cloned(),
                last_activity: chrono::Utc::now(),
            });
        }

        // Update statistics
        self.stats.write().await.connected_clients += 1;

        // Handle WebSocket messages
        let (mut sender, mut receiver) = socket.split();
        let update_rx = self.update_tx.subscribe();

        // Task to send discoveries to client
        let sender_task = {
            let client_id = client_id.clone();
            tokio::spawn(async move {
                let mut rx = update_rx;
                while let Ok(discovery) = rx.recv().await {
                    let notification = McpNotification {
                        jsonrpc: "2.0".to_string(),
                        method: "notifications/discovery_received".to_string(),
                        params: Some(serde_json::to_value(discovery).unwrap()),
                    };

                    let message = serde_json::to_string(&notification).unwrap();
                    if let Err(e) = sender.send(Message::Text(message)).await {
                        error!("Failed to send message to client {}: {}", client_id, e);
                        break;
                    }
                }
            })
        };

        // Task to receive messages from client
        let receiver_task = {
            let client_id = client_id.clone();
            let server = Arc::clone(&self);
            tokio::spawn(async move {
                while let Some(msg) = receiver.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            // Handle MCP requests over WebSocket
                            if let Ok(request) = serde_json::from_str::<McpRequest>(&text) {
                                match server.process_mcp_request(request.clone()).await {
                                    Ok(result) => {
                                        let response = McpResponse {
                                            jsonrpc: "2.0".to_string(),
                                            id: request.id,
                                            result: Some(result),
                                            error: None,
                                        };
                                        if let Ok(response_text) = serde_json::to_string(&response) {
                                            if let Err(e) = sender.send(Message::Text(response_text)).await {
                                                error!("Failed to send response to client {}: {}", client_id, e);
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let response = McpResponse {
                                            jsonrpc: "2.0".to_string(),
                                            id: request.id,
                                            result: None,
                                            error: Some(McpError {
                                                code: -32603,
                                                message: e.to_string(),
                                                data: None,
                                            }),
                                        };
                                        if let Ok(response_text) = serde_json::to_string(&response) {
                                            if let Err(e) = sender.send(Message::Text(response_text)).await {
                                                error!("Failed to send error response to client {}: {}", client_id, e);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Ok(Message::Close(_)) => {
                            info!("WebSocket client disconnected: {}", client_id);
                            break;
                        }
                        Err(e) => {
                            error!("WebSocket error for client {}: {}", client_id, e);
                            break;
                        }
                        _ => {}
                    }
                }
            })
        };

        // Wait for either task to complete
        tokio::select! {
            _ = sender_task => {},
            _ = receiver_task => {},
        }

        // Clean up client
        {
            let mut clients = self.connected_clients.write().await;
            clients.remove(&client_id);
        }
        self.stats.write().await.connected_clients -= 1;

        info!("WebSocket client disconnected and cleaned up: {}", client_id);
    }

    /// Handle SSE stream for real-time updates
    async fn handle_sse_stream(
        State(server): State<Arc<McpDiscoveryServer>>,
        Query(params): Query<HashMap<String, String>>,
    ) -> Response {
        // Delegate to the SSE broadcaster
        server.sse_broadcaster.handle_sse_connection(
            axum::extract::State(server.sse_broadcaster.clone()),
            axum::extract::Query(mistersmith::collaboration::StreamQuery {
                agent_id: params.get("agent_id").cloned().unwrap_or_default(),
            }),
        ).await.into_response()
    }

    /// Get server statistics
    async fn get_server_stats(
        State(server): State<Arc<McpDiscoveryServer>>,
    ) -> Json<Value> {
        let stats = server.stats.read().await;
        let connected_clients = server.connected_clients.read().await;
        let sse_connections = server.sse_broadcaster.get_connected_clients().await;

        Json(json!({
            "server_id": server.server_id,
            "stats": {
                "total_discoveries_shared": stats.total_discoveries_shared,
                "total_discoveries_received": stats.total_discoveries_received,
                "total_mcp_requests": stats.total_mcp_requests,
                "total_mcp_notifications": stats.total_mcp_notifications,
                "connected_mcp_clients": connected_clients.len(),
                "sse_connections": sse_connections.len(),
                "stored_discoveries": server.stored_discoveries.read().await.len(),
            },
            "connected_clients": connected_clients.values().collect::<Vec<_>>(),
            "sse_connections": sse_connections.len(),
        }))
    }

    /// Health check endpoint
    async fn health_check() -> Json<Value> {
        Json(json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now(),
            "version": "1.0.0"
        }))
    }
}

/// Test utilities for the MCP server
pub mod test_utils {
    use super::*;
    use reqwest::Client as HttpClient;
    use std::time::Duration;

    /// MCP client for testing
    pub struct McpTestClient {
        pub client: HttpClient,
        pub base_url: String,
        pub client_id: String,
    }

    impl McpTestClient {
        pub fn new(base_url: String) -> Self {
            Self {
                client: HttpClient::new(),
                base_url,
                client_id: uuid::Uuid::new_v4().to_string(),
            }
        }

        /// Initialize the MCP client
        pub async fn initialize(&self) -> Result<Value> {
            let request = McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(1)),
                method: "initialize".to_string(),
                params: Some(json!({
                    "clientInfo": {
                        "name": "Test Client",
                        "version": "1.0.0"
                    },
                    "capabilities": {}
                })),
            };

            let response = self.client
                .post(&format!("{}/mcp", self.base_url))
                .json(&request)
                .send()
                .await?;

            let mcp_response: McpResponse = response.json().await?;
            
            if let Some(error) = mcp_response.error {
                return Err(anyhow::anyhow!("MCP error: {}", error.message));
            }

            mcp_response.result.ok_or_else(|| anyhow::anyhow!("No result in response"))
        }

        /// List available tools
        pub async fn list_tools(&self) -> Result<Value> {
            let request = McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(2)),
                method: "tools/list".to_string(),
                params: None,
            };

            let response = self.client
                .post(&format!("{}/mcp", self.base_url))
                .json(&request)
                .send()
                .await?;

            let mcp_response: McpResponse = response.json().await?;
            
            if let Some(error) = mcp_response.error {
                return Err(anyhow::anyhow!("MCP error: {}", error.message));
            }

            mcp_response.result.ok_or_else(|| anyhow::anyhow!("No result in response"))
        }

        /// Call a tool
        pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
            let request = McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(3)),
                method: "tools/call".to_string(),
                params: Some(json!({
                    "name": name,
                    "arguments": arguments
                })),
            };

            let response = self.client
                .post(&format!("{}/mcp", self.base_url))
                .json(&request)
                .send()
                .await?;

            let mcp_response: McpResponse = response.json().await?;
            
            if let Some(error) = mcp_response.error {
                return Err(anyhow::anyhow!("MCP error: {}", error.message));
            }

            mcp_response.result.ok_or_else(|| anyhow::anyhow!("No result in response"))
        }

        /// Share a discovery
        pub async fn share_discovery(
            &self,
            discovery_type: &str,
            content: &str,
            confidence: f64,
        ) -> Result<Value> {
            self.call_tool("share_discovery", json!({
                "discovery_type": discovery_type,
                "content": content,
                "confidence": confidence
            })).await
        }

        /// Query discoveries
        pub async fn query_discoveries(
            &self,
            agent_id: Option<&str>,
            discovery_type: Option<&str>,
            min_confidence: Option<f64>,
            limit: Option<usize>,
        ) -> Result<Value> {
            let mut args = json!({});
            
            if let Some(agent_id) = agent_id {
                args["agent_id"] = json!(agent_id);
            }
            if let Some(discovery_type) = discovery_type {
                args["discovery_type"] = json!(discovery_type);
            }
            if let Some(min_confidence) = min_confidence {
                args["min_confidence"] = json!(min_confidence);
            }
            if let Some(limit) = limit {
                args["limit"] = json!(limit);
            }

            self.call_tool("query_discoveries", args).await
        }

        /// Get server statistics
        pub async fn get_stats(&self) -> Result<Value> {
            let response = self.client
                .get(&format!("{}/stats", self.base_url))
                .send()
                .await?;

            Ok(response.json().await?)
        }

        /// Health check
        pub async fn health_check(&self) -> Result<Value> {
            let response = self.client
                .get(&format!("{}/health", self.base_url))
                .send()
                .await?;

            Ok(response.json().await?)
        }
    }

    /// Start a test MCP server
    pub async fn start_test_server(port: u16) -> Result<Arc<McpDiscoveryServer>> {
        let nats = async_nats::connect("nats://localhost:4222").await
            .context("Failed to connect to NATS")?;

        let server = Arc::new(McpDiscoveryServer::new(
            "test-server".to_string(),
            nats,
        ).await?);

        // Start discovery listener
        server.start_discovery_listener().await?;

        // Start HTTP server
        let app = server.create_router();
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .context("Failed to bind test server")?;

        let server_clone = server.clone();
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                error!("Test server error: {}", e);
            }
        });

        // Give the server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(server)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_utils::*;

    #[tokio::test]
    async fn test_mcp_server_initialization() -> Result<()> {
        let server = start_test_server(8081).await?;
        let client = McpTestClient::new("http://127.0.0.1:8081".to_string());

        // Test initialization
        let init_result = client.initialize().await?;
        assert!(init_result.get("protocolVersion").is_some());

        // Test health check
        let health_result = client.health_check().await?;
        assert_eq!(health_result.get("status").unwrap(), "healthy");

        Ok(())
    }

    #[tokio::test]
    async fn test_mcp_tools() -> Result<()> {
        let server = start_test_server(8082).await?;
        let client = McpTestClient::new("http://127.0.0.1:8082".to_string());

        // Initialize client
        client.initialize().await?;

        // List tools
        let tools_result = client.list_tools().await?;
        let tools = tools_result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 3);

        // Test share_discovery tool
        let share_result = client.share_discovery("Pattern", "Test discovery", 0.8).await?;
        assert_eq!(share_result.get("success").unwrap(), true);

        // Test query_discoveries tool
        let query_result = client.query_discoveries(
            Some("test-server"),
            Some("Pattern"),
            Some(0.7),
            Some(10),
        ).await?;
        
        let discoveries = query_result.get("discoveries").unwrap().as_array().unwrap();
        assert_eq!(discoveries.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_discovery_sharing_flow() -> Result<()> {
        let server = start_test_server(8083).await?;
        let client1 = McpTestClient::new("http://127.0.0.1:8083".to_string());
        let client2 = McpTestClient::new("http://127.0.0.1:8083".to_string());

        // Initialize clients
        client1.initialize().await?;
        client2.initialize().await?;

        // Client 1 shares a discovery
        client1.share_discovery("Insight", "Important insight", 0.9).await?;

        // Give time for discovery to propagate
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Client 2 queries discoveries
        let query_result = client2.query_discoveries(
            Some("test-server"),
            None,
            Some(0.8),
            Some(10),
        ).await?;
        
        let discoveries = query_result.get("discoveries").unwrap().as_array().unwrap();
        assert_eq!(discoveries.len(), 1);

        Ok(())
    }
}