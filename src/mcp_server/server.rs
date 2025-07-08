//! MCP Server with NATS Integration
//!
//! Complete MCP server implementation that integrates NATS messaging,
//! discovery sharing handlers, and SSE broadcasting.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use async_nats::Client;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, error, debug};

use super::nats_config::{NatsConfig, NatsClientManager};
use super::discovery_state::DiscoveryStore;
use crate::handlers::{
    share_discovery::{share_discovery_handler, ShareDiscoveryParams},
    subscribe_discoveries::{subscribe_discoveries_handler, SubscribeDiscoveriesParams},
    Parameters, CallToolResult, McpError,
};
use crate::collaboration::DiscoverySSEBroadcaster;

/// JSON-RPC 2.0 request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP server configuration
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    /// Server bind address
    pub bind_addr: String,
    /// Server port
    pub port: u16,
    /// NATS configuration
    pub nats_config: NatsConfig,
    /// Request timeout
    pub request_timeout: Duration,
    /// Enable CORS
    pub enable_cors: bool,
    /// SSE configuration
    pub sse_config: SseConfig,
}

/// SSE configuration
#[derive(Debug, Clone)]
pub struct SseConfig {
    /// SSE endpoint path prefix
    pub endpoint_prefix: String,
    /// Maximum number of concurrent SSE connections
    pub max_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1".to_string(),
            port: 8080,
            nats_config: NatsConfig::default(),
            request_timeout: Duration::from_secs(30),
            enable_cors: true,
            sse_config: SseConfig::default(),
        }
    }
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            endpoint_prefix: "/sse".to_string(),
            max_connections: 1000,
            connection_timeout: Duration::from_secs(300), // 5 minutes
            keep_alive_interval: Duration::from_secs(30),
        }
    }
}

/// MCP server state shared across handlers
#[derive(Clone)]
pub struct McpServerState {
    /// NATS client manager
    pub nats_manager: Arc<NatsClientManager>,
    /// Discovery store
    pub discovery_store: Arc<DiscoveryStore>,
    /// SSE broadcaster
    pub sse_broadcaster: Arc<DiscoverySSEBroadcaster>,
    /// Server configuration
    pub config: McpServerConfig,
    /// Handler registry
    pub handlers: Arc<HandlerRegistry>,
}

/// Handler registry for MCP methods
#[derive(Default)]
pub struct HandlerRegistry {
    /// Registered handler methods
    pub methods: HashMap<String, String>,
}

impl HandlerRegistry {
    /// Register a new handler method
    pub fn register(&mut self, method: String, description: String) {
        self.methods.insert(method, description);
    }

    /// Check if a method is registered
    pub fn is_registered(&self, method: &str) -> bool {
        self.methods.contains_key(method)
    }

    /// Get all registered methods
    pub fn get_methods(&self) -> Vec<String> {
        self.methods.keys().cloned().collect()
    }
}

/// Main MCP server implementation
pub struct McpServer {
    /// Server state
    state: McpServerState,
    /// HTTP server router
    router: Router,
}

impl McpServer {
    /// Create a new MCP server
    pub async fn new(config: McpServerConfig) -> Result<Self> {
        info!("🚀 Initializing MCP server on {}:{}", config.bind_addr, config.port);

        // Initialize NATS client manager
        let nats_manager = Arc::new(NatsClientManager::new(config.nats_config.clone()).await?);
        info!("✅ NATS client manager initialized");

        // Initialize discovery store
        let discovery_store = Arc::new(DiscoveryStore::new());
        info!("✅ Discovery store initialized");

        // Initialize SSE broadcaster
        let sse_broadcaster = Arc::new(DiscoverySSEBroadcaster::new(nats_manager.client().clone()));
        info!("✅ SSE broadcaster initialized");

        // Start SSE broadcaster listening for NATS messages
        sse_broadcaster.start_listening().await?;
        info!("📡 SSE broadcaster started listening for NATS messages");

        // Initialize handler registry
        let mut handler_registry = HandlerRegistry::default();
        handler_registry.register(
            "share_discovery".to_string(),
            "Share a discovery with other agents via NATS".to_string(),
        );
        handler_registry.register(
            "subscribe_discoveries".to_string(),
            "Subscribe to discoveries with filters via SSE".to_string(),
        );
        handler_registry.register(
            "list_tools".to_string(),
            "List available MCP tools".to_string(),
        );
        handler_registry.register(
            "call_tool".to_string(),
            "Call an MCP tool".to_string(),
        );

        // Create server state
        let state = McpServerState {
            nats_manager,
            discovery_store,
            sse_broadcaster: sse_broadcaster.clone(),
            config: config.clone(),
            handlers: Arc::new(handler_registry),
        };

        // Build router
        let router = Self::build_router(state.clone(), sse_broadcaster).await;

        info!("🎯 MCP server initialized successfully");
        Ok(Self { state, router })
    }

    /// Build the HTTP router
    async fn build_router(state: McpServerState, sse_broadcaster: Arc<DiscoverySSEBroadcaster>) -> Router {
        let mut router = Router::new()
            // MCP JSON-RPC endpoint
            .route("/mcp", post(Self::handle_mcp_request))
            // Health check endpoint
            .route("/health", get(Self::handle_health_check))
            // Server info endpoint
            .route("/info", get(Self::handle_server_info))
            // NATS statistics endpoint
            .route("/stats/nats", get(Self::handle_nats_stats))
            // Discovery store statistics endpoint
            .route("/stats/discoveries", get(Self::handle_discovery_stats))
            // Share server state
            .with_state(state.clone());

        // Add SSE routes
        let sse_router = sse_broadcaster.create_router();
        router = router.merge(sse_router);

        // Add CORS if enabled
        if state.config.enable_cors {
            router = router.layer(CorsLayer::permissive());
        }

        router
    }

    /// Start the MCP server
    pub async fn serve(self) -> Result<()> {
        let addr = format!("{}:{}", self.state.config.bind_addr, self.state.config.port);
        info!("🌐 Starting MCP server on {}", addr);

        // Start server
        let listener = TcpListener::bind(&addr).await?;
        info!("✅ MCP server listening on {}", addr);

        // Print available endpoints
        info!("📋 Available endpoints:");
        info!("  • POST /mcp - MCP JSON-RPC endpoint");
        info!("  • GET  /health - Health check");
        info!("  • GET  /info - Server information");
        info!("  • GET  /stats/nats - NATS statistics");
        info!("  • GET  /stats/discoveries - Discovery statistics");
        info!("  • GET  /discoveries/stream - SSE discovery stream");

        axum::serve(listener, self.router).await?;

        Ok(())
    }

    /// Handle MCP JSON-RPC requests
    async fn handle_mcp_request(
        State(state): State<McpServerState>,
        Json(request): Json<JsonRpcRequest>,
    ) -> Response {
        debug!("📨 Received MCP request: method={}", request.method);

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return Self::error_response(
                request.id,
                -32600,
                "Invalid Request",
                Some("JSON-RPC version must be 2.0".into()),
            );
        }

        // Check if method is registered
        if !state.handlers.is_registered(&request.method) {
            return Self::error_response(
                request.id,
                -32601,
                "Method not found",
                Some(format!("Method '{}' not found", request.method).into()),
            );
        }

        // Route to appropriate handler
        let result = match request.method.as_str() {
            "list_tools" => Self::handle_list_tools(state, request.params).await,
            "call_tool" => Self::handle_call_tool(state, request.params).await,
            _ => Err(McpError::InvalidParameters(format!("Unknown method: {}", request.method))),
        };

        // Format response
        match result {
            Ok(result) => Self::success_response(request.id, result),
            Err(e) => Self::error_response(
                request.id,
                -32603,
                "Internal error",
                Some(e.to_string().into()),
            ),
        }
    }

    /// Handle list_tools request
    async fn handle_list_tools(
        state: McpServerState,
        _params: Option<Value>,
    ) -> Result<Value, McpError> {
        let tools = serde_json::json!({
            "tools": [
                {
                    "name": "share_discovery",
                    "description": "Share a discovery with other agents via NATS",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "agent_id": {"type": "string"},
                            "agent_role": {"type": "string"},
                            "discovery_type": {"type": "string"},
                            "content": {"type": "string"},
                            "confidence": {"type": "number"},
                            "related_to": {"type": "string"}
                        },
                        "required": ["agent_id", "agent_role", "discovery_type", "content", "confidence"]
                    }
                },
                {
                    "name": "subscribe_discoveries",
                    "description": "Subscribe to discoveries with filters via SSE",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "discovery_types": {"type": "array", "items": {"type": "string"}},
                            "agent_roles": {"type": "array", "items": {"type": "string"}},
                            "min_confidence": {"type": "number"},
                            "subscriber_agent_id": {"type": "string"}
                        },
                        "required": ["subscriber_agent_id"]
                    }
                }
            ]
        });

        Ok(tools)
    }

    /// Handle call_tool request
    async fn handle_call_tool(
        state: McpServerState,
        params: Option<Value>,
    ) -> Result<Value, McpError> {
        let params = params.ok_or_else(|| McpError::InvalidParameters("Missing parameters".to_string()))?;
        
        // Extract tool name and arguments
        let tool_name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParameters("Missing tool name".to_string()))?;
        
        let arguments = params.get("arguments")
            .ok_or_else(|| McpError::InvalidParameters("Missing arguments".to_string()))?;

        // Route to appropriate tool handler
        let result = match tool_name {
            "share_discovery" => {
                let params: ShareDiscoveryParams = serde_json::from_value(arguments.clone())?;
                let wrapped_params = Parameters { params };
                share_discovery_handler(wrapped_params, state.nats_manager.client().clone()).await?
            }
            "subscribe_discoveries" => {
                let params: SubscribeDiscoveriesParams = serde_json::from_value(arguments.clone())?;
                let wrapped_params = Parameters { params };
                subscribe_discoveries_handler(wrapped_params, state.nats_manager.client().clone()).await?
            }
            _ => return Err(McpError::InvalidParameters(format!("Unknown tool: {}", tool_name))),
        };

        // Convert CallToolResult to JSON
        Ok(serde_json::to_value(result)?)
    }

    /// Handle health check
    async fn handle_health_check(State(state): State<McpServerState>) -> impl IntoResponse {
        let nats_healthy = state.nats_manager.is_healthy();
        let nats_stats = state.nats_manager.get_stats();
        let discovery_stats = state.discovery_store.get_stats().await;

        let health = serde_json::json!({
            "status": if nats_healthy { "healthy" } else { "unhealthy" },
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "components": {
                "nats": {
                    "healthy": nats_healthy,
                    "connection_status": nats_stats.connection_status.to_string(),
                    "last_health_check": nats_stats.last_health_check.map(|t| t.elapsed().as_secs())
                },
                "discovery_store": {
                    "healthy": true,
                    "total_discoveries": discovery_stats.total_discoveries,
                    "capacity_used": discovery_stats.capacity_used
                }
            }
        });

        if nats_healthy {
            (StatusCode::OK, Json(health))
        } else {
            (StatusCode::SERVICE_UNAVAILABLE, Json(health))
        }
    }

    /// Handle server info
    async fn handle_server_info(State(state): State<McpServerState>) -> impl IntoResponse {
        let info = serde_json::json!({
            "name": "MisterSmith MCP Server",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "MCP server with NATS integration for discovery sharing",
            "capabilities": {
                "tools": true,
                "resources": false,
                "prompts": false
            },
            "available_tools": state.handlers.get_methods(),
            "endpoints": {
                "mcp": "/mcp",
                "health": "/health",
                "info": "/info",
                "sse_discoveries": "/discoveries/stream"
            },
            "nats": {
                "server_url": state.nats_manager.config().server_url,
                "subjects": {
                    "discovery_pattern": state.nats_manager.config().subject_patterns.discovery_pattern,
                    "discovery_wildcard": state.nats_manager.config().subject_patterns.discovery_wildcard
                }
            }
        });

        Json(info)
    }

    /// Handle NATS statistics
    async fn handle_nats_stats(State(state): State<McpServerState>) -> impl IntoResponse {
        let stats = state.nats_manager.get_stats();
        Json(serde_json::json!({
            "connection_status": stats.connection_status.to_string(),
            "messages_published": stats.messages_published,
            "messages_received": stats.messages_received,
            "connection_failures": stats.connection_failures,
            "reconnection_attempts": stats.reconnection_attempts,
            "last_health_check": stats.last_health_check.map(|t| t.elapsed().as_secs()),
            "healthy": state.nats_manager.is_healthy()
        }))
    }

    /// Handle discovery statistics
    async fn handle_discovery_stats(State(state): State<McpServerState>) -> impl IntoResponse {
        let stats = state.discovery_store.get_stats().await;
        Json(serde_json::json!({
            "total_discoveries": stats.total_discoveries,
            "unique_agents": stats.unique_agents,
            "discovery_types": stats.discovery_types,
            "active_subscriptions": stats.active_subscriptions,
            "capacity_used": stats.capacity_used
        }))
    }

    /// Create a success JSON-RPC response
    fn success_response(id: Option<Value>, result: impl Serialize) -> Response {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
            id,
        };

        (StatusCode::OK, Json(response)).into_response()
    }

    /// Create an error JSON-RPC response
    fn error_response(id: Option<Value>, code: i32, message: &str, data: Option<Value>) -> Response {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data,
            }),
            id,
        };

        (StatusCode::BAD_REQUEST, Json(response)).into_response()
    }
}

/// Server statistics
#[derive(Debug, Clone, Serialize)]
pub struct ServerStats {
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Active SSE connections
    pub active_sse_connections: usize,
    /// NATS connection statistics
    pub nats_stats: super::nats_config::ConnectionStats,
    /// Discovery store statistics
    pub discovery_stats: super::discovery_state::StoreStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = McpServerConfig::default();
        assert_eq!(config.bind_addr, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(config.enable_cors);
    }

    #[test]
    fn test_handler_registry() {
        let mut registry = HandlerRegistry::default();
        registry.register("test_method".to_string(), "Test description".to_string());
        
        assert!(registry.is_registered("test_method"));
        assert!(!registry.is_registered("unknown_method"));
        assert_eq!(registry.get_methods().len(), 1);
    }

    #[test]
    fn test_json_rpc_structures() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "test_method".to_string(),
            params: Some(serde_json::json!({"key": "value"})),
            id: Some(serde_json::json!(1)),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.method, "test_method");
        assert_eq!(parsed.jsonrpc, "2.0");
    }
}