//! Example Usage of MCP Server with NATS Integration
//!
//! Demonstrates how to use the complete MCP server with NATS messaging,
//! discovery sharing, and SSE broadcasting.

use std::time::Duration;
use anyhow::Result;
use serde_json::json;
use tokio::time;
use tracing::{info, warn};

use super::{
    McpServer, McpServerConfig, NatsConfig, SseBridgeConfig, SseConfig,
    HandlerIntegration, HandlerIntegrationFactory, HandlerContext,
};
use crate::handlers::{
    share_discovery::ShareDiscoveryParams,
    subscribe_discoveries::SubscribeDiscoveriesParams,
};
use crate::mcp_server::discovery_state::DiscoveryFilter;
use crate::collaboration::DiscoveryType;

/// Complete example of MCP server with NATS integration
pub async fn run_complete_example() -> Result<()> {
    info!("🚀 Starting complete MCP server with NATS integration example");

    // Configure NATS connection
    let nats_config = NatsConfig {
        server_url: "nats://localhost:4222".to_string(),
        ping_interval: Duration::from_secs(30),
        max_reconnects: Some(10),
        ..Default::default()
    };

    // Configure SSE bridge
    let sse_config = SseConfig {
        endpoint_prefix: "/sse".to_string(),
        max_connections: 100,
        connection_timeout: Duration::from_secs(300),
        keep_alive_interval: Duration::from_secs(30),
    };

    // Create server configuration
    let server_config = McpServerConfig {
        bind_addr: "127.0.0.1".to_string(),
        port: 8080,
        nats_config,
        sse_config,
        request_timeout: Duration::from_secs(30),
        enable_cors: true,
    };

    // Create and start the MCP server
    let server = McpServer::new(server_config).await?;
    
    info!("✅ MCP server initialized successfully");
    info!("🌐 Server will be available at: http://127.0.0.1:8080");
    info!("📋 Available endpoints:");
    info!("  • POST /mcp - MCP JSON-RPC endpoint");
    info!("  • GET  /health - Health check");
    info!("  • GET  /info - Server information");
    info!("  • GET  /discoveries/stream - SSE discovery stream");

    // In a real application, you would call server.serve().await
    // For this example, we'll demonstrate the functionality
    info!("🔄 Example completed - server configured but not started");
    
    Ok(())
}

/// Example of using handler integration directly
pub async fn handler_integration_example() -> Result<()> {
    info!("🔗 Demonstrating handler integration");

    // This would typically be done inside the MCP server
    // For this example, we'll show how to use the handler integration directly
    warn!("⚠️  This example requires a running NATS server at localhost:4222");
    warn!("⚠️  Start NATS server with: nats-server");

    // Note: In a real scenario, this would be created by the MCP server
    info!("📝 Example shows handler integration pattern");
    info!("   - Handlers receive NATS client via dependency injection");
    info!("   - Discovery sharing publishes to subjects like 'discoveries.agent-id.type'");
    info!("   - SSE bridge subscribes to 'discoveries.>' and streams to clients");
    info!("   - All components use the same NATS client for consistency");

    Ok(())
}

/// Example JSON-RPC requests for the MCP server
pub async fn json_rpc_examples() -> Result<()> {
    info!("📨 Example JSON-RPC requests for MCP server");

    // Example 1: List available tools
    let list_tools_request = json!({
        "jsonrpc": "2.0",
        "method": "list_tools",
        "id": "1"
    });

    info!("📤 List tools request:");
    info!("{}", serde_json::to_string_pretty(&list_tools_request)?);

    // Example 2: Share a discovery
    let share_discovery_request = json!({
        "jsonrpc": "2.0",
        "method": "call_tool",
        "params": {
            "name": "share_discovery",
            "arguments": {
                "agent_id": "security-agent-001",
                "agent_role": "Security Analyst",
                "discovery_type": "anomaly",
                "content": "Unusual login patterns detected from IP range 192.168.1.0/24",
                "confidence": 0.85,
                "related_to": null
            }
        },
        "id": "2"
    });

    info!("📤 Share discovery request:");
    info!("{}", serde_json::to_string_pretty(&share_discovery_request)?);

    // Example 3: Subscribe to discoveries
    let subscribe_request = json!({
        "jsonrpc": "2.0",
        "method": "call_tool",
        "params": {
            "name": "subscribe_discoveries",
            "arguments": {
                "discovery_types": ["anomaly", "solution"],
                "agent_roles": ["Security Analyst", "System Administrator"],
                "min_confidence": 0.7,
                "subscriber_agent_id": "monitoring-agent-001"
            }
        },
        "id": "3"
    });

    info!("📤 Subscribe discoveries request:");
    info!("{}", serde_json::to_string_pretty(&subscribe_request)?);

    // Example response format
    let example_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "Discovery shared successfully: uuid-123"
                }
            ]
        },
        "id": "2"
    });

    info!("📥 Example response:");
    info!("{}", serde_json::to_string_pretty(&example_response)?);

    Ok(())
}

/// Example of SSE client connection
pub async fn sse_client_example() -> Result<()> {
    info!("🌊 Example SSE client connection");

    // SSE endpoint URL with query parameters
    let sse_url = "http://127.0.0.1:8080/discoveries/stream?agent_id=client-agent-001";
    
    info!("📡 SSE Connection URL: {}", sse_url);
    info!("🔄 Client would connect and receive events like:");
    
    // Example SSE events
    let connection_event = "event: connected\ndata: {\"client_id\":\"uuid-123\",\"agent_id\":\"client-agent-001\"}\n\n";
    let discovery_event = "event: discovery\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/resources/updated\",\"params\":{\"uri\":\"discovery://agents/security-agent-001/discoveries/1234567890-anomaly\"}}\n\n";
    let keepalive_event = ": keep-alive\n\n";

    info!("📨 Connection event:");
    info!("{}", connection_event);
    info!("📨 Discovery event:");
    info!("{}", discovery_event);
    info!("📨 Keep-alive event:");
    info!("{}", keepalive_event);

    Ok(())
}

/// Example of discovery filtering
pub async fn discovery_filtering_example() -> Result<()> {
    info!("🔍 Example discovery filtering");

    // Example filter for high-confidence security anomalies
    let security_filter = DiscoveryFilter {
        types: Some(vec![DiscoveryType::Anomaly, DiscoveryType::Solution]),
        agent_roles: Some(vec!["Security Analyst".to_string()]),
        min_confidence: Some(0.8),
        keywords: Some(vec!["security".to_string(), "threat".to_string()]),
    };

    info!("🔧 Security filter configuration:");
    info!("   - Types: {:?}", security_filter.types);
    info!("   - Agent roles: {:?}", security_filter.agent_roles);
    info!("   - Min confidence: {:?}", security_filter.min_confidence);
    info!("   - Keywords: {:?}", security_filter.keywords);

    // Example discovery that would match
    let matching_discovery = crate::collaboration::Discovery {
        agent_id: "security-agent-001".to_string(),
        agent_role: "Security Analyst".to_string(),
        discovery_type: DiscoveryType::Anomaly,
        content: "Critical security vulnerability detected in authentication system".to_string(),
        confidence: 0.95,
        timestamp: chrono::Utc::now(),
        related_to: None,
    };

    let matches = security_filter.matches(&matching_discovery);
    info!("✅ Discovery matches filter: {}", matches);

    Ok(())
}

/// Example of monitoring server health
pub async fn health_monitoring_example() -> Result<()> {
    info!("🏥 Example health monitoring");

    // Example health check response
    let health_response = json!({
        "status": "healthy",
        "timestamp": "2024-01-01T12:00:00Z",
        "components": {
            "nats": {
                "healthy": true,
                "connection_status": "Connected",
                "last_health_check": 5
            },
            "discovery_store": {
                "healthy": true,
                "total_discoveries": 150,
                "capacity_used": 15.0
            }
        }
    });

    info!("📊 Health check response:");
    info!("{}", serde_json::to_string_pretty(&health_response)?);

    // Example NATS statistics
    let nats_stats = json!({
        "connection_status": "Connected",
        "messages_published": 1250,
        "messages_received": 890,
        "connection_failures": 2,
        "reconnection_attempts": 3,
        "last_health_check": 10,
        "healthy": true
    });

    info!("📈 NATS statistics:");
    info!("{}", serde_json::to_string_pretty(&nats_stats)?);

    Ok(())
}

/// Complete workflow example
pub async fn complete_workflow_example() -> Result<()> {
    info!("🔄 Complete workflow example");

    info!("Step 1: Agent starts MCP server with NATS integration");
    info!("   - Server connects to NATS at localhost:4222");
    info!("   - SSE broadcaster starts listening to 'discoveries.>' subject");
    info!("   - HTTP server starts on port 8080");

    info!("Step 2: Agent creates subscription via JSON-RPC");
    info!("   - POST /mcp with subscribe_discoveries method");
    info!("   - Server stores subscription filter in discovery store");
    info!("   - Server returns SSE endpoint URL");

    info!("Step 3: Agent connects to SSE endpoint");
    info!("   - GET /discoveries/stream?agent_id=client-001");
    info!("   - Server registers SSE client with filter");
    info!("   - Client receives connection confirmation event");

    info!("Step 4: Another agent shares discovery via JSON-RPC");
    info!("   - POST /mcp with share_discovery method");
    info!("   - Server stores discovery in discovery store");
    info!("   - Server publishes to NATS subject 'discoveries.agent-id.type'");

    info!("Step 5: SSE bridge delivers to subscribed clients");
    info!("   - SSE broadcaster receives message from NATS");
    info!("   - Filters message based on client subscriptions");
    info!("   - Sends discovery event to matching SSE clients");

    info!("Step 6: Client receives real-time discovery");
    info!("   - SSE client receives 'discovery' event");
    info!("   - Event contains JSON-RPC notification format");
    info!("   - Client can process discovery in real-time");

    info!("✅ Complete workflow demonstrates:");
    info!("   - NATS integration for reliable messaging");
    info!("   - Discovery sharing via JSON-RPC");
    info!("   - Real-time updates via SSE");
    info!("   - Filtering and subscription management");
    info!("   - Health monitoring and statistics");

    Ok(())
}

/// Run all examples
pub async fn run_all_examples() -> Result<()> {
    info!("🎯 Running all MCP server with NATS integration examples");
    
    // Run examples in sequence
    handler_integration_example().await?;
    json_rpc_examples().await?;
    sse_client_example().await?;
    discovery_filtering_example().await?;
    health_monitoring_example().await?;
    complete_workflow_example().await?;
    
    // Note: We're not running the complete example as it would start a server
    info!("📝 To run the complete server example, use: run_complete_example()");
    
    info!("✅ All examples completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_examples() {
        // Test that examples run without errors
        assert!(json_rpc_examples().await.is_ok());
        assert!(sse_client_example().await.is_ok());
        assert!(discovery_filtering_example().await.is_ok());
        assert!(health_monitoring_example().await.is_ok());
        assert!(complete_workflow_example().await.is_ok());
    }

    #[tokio::test]
    async fn test_handler_integration_example() {
        // Test handler integration example
        assert!(handler_integration_example().await.is_ok());
    }

    #[test]
    fn test_discovery_filter_creation() {
        let filter = DiscoveryFilter {
            types: Some(vec![DiscoveryType::Pattern]),
            agent_roles: Some(vec!["TestRole".to_string()]),
            min_confidence: Some(0.7),
            keywords: None,
        };
        
        assert!(filter.types.is_some());
        assert!(filter.agent_roles.is_some());
        assert_eq!(filter.min_confidence, Some(0.7));
    }
}