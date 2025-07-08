//! Example: SSE Discovery Streaming
//! 
//! Demonstrates how to set up an SSE server that streams discovery updates
//! to connected clients in real-time.

use anyhow::Result;
use axum::{routing::get, Router};
use mistersmith::collaboration::{
    Discovery, DiscoveryBroadcaster, DiscoverySSEBroadcaster, DiscoveryType,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting SSE Discovery Stream Example");
    
    // Connect to NATS
    let nats = async_nats::connect("nats://localhost:4222").await?;
    info!("✅ Connected to NATS");
    
    // Create SSE broadcaster
    let broadcaster = Arc::new(DiscoverySSEBroadcaster::new(nats.clone()));
    
    // Start listening for NATS discoveries
    broadcaster.start_listening().await?;
    info!("📡 SSE Broadcaster listening for discoveries");
    
    // Create health check endpoint
    let health_router = Router::new()
        .route("/health", get(|| async { "OK" }));
    
    // Combine routers
    let app = Router::new()
        .merge(broadcaster.create_router())
        .merge(health_router);
    
    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("🌐 SSE server listening on http://127.0.0.1:3000");
    info!("📍 SSE endpoint: http://127.0.0.1:3000/discoveries/stream?agent_id=YOUR_AGENT_ID");
    
    // Spawn example discovery generator
    tokio::spawn(generate_example_discoveries(nats.clone()));
    
    // Run server
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Generate example discoveries to demonstrate streaming
async fn generate_example_discoveries(nats: async_nats::Client) -> Result<()> {
    // Wait a bit for clients to connect
    sleep(Duration::from_secs(5)).await;
    
    // Create different agent broadcasters
    let security_agent = DiscoveryBroadcaster::new(
        "security-agent-001".to_string(),
        "Security Analyzer".to_string(),
        nats.clone(),
    );
    
    let performance_agent = DiscoveryBroadcaster::new(
        "perf-agent-002".to_string(),
        "Performance Monitor".to_string(),
        nats.clone(),
    );
    
    let network_agent = DiscoveryBroadcaster::new(
        "network-agent-003".to_string(),
        "Network Inspector".to_string(),
        nats.clone(),
    );
    
    info!("🎯 Starting discovery generation...");
    
    // Generate discoveries periodically
    loop {
        // Security discovery
        security_agent
            .share_discovery(
                DiscoveryType::Anomaly,
                "Detected unusual authentication attempts from internal subnet",
                0.75,
            )
            .await?;
        
        sleep(Duration::from_secs(3)).await;
        
        // Performance discovery
        performance_agent
            .share_discovery(
                DiscoveryType::Pattern,
                "CPU usage spike correlated with database query load",
                0.82,
            )
            .await?;
        
        sleep(Duration::from_secs(2)).await;
        
        // Network discovery
        network_agent
            .share_discovery(
                DiscoveryType::Insight,
                "Bandwidth utilization optimized by 15% through route adjustment",
                0.90,
            )
            .await?;
        
        sleep(Duration::from_secs(5)).await;
        
        // Question from security
        security_agent
            .share_discovery(
                DiscoveryType::Question,
                "Is the increased database load related to the auth attempts?",
                0.60,
            )
            .await?;
        
        sleep(Duration::from_secs(4)).await;
        
        // Connection discovery
        network_agent
            .share_related_discovery(
                DiscoveryType::Connection,
                "Confirmed: Auth attempts originating from same IP range as DB queries",
                0.95,
                "security-question-001",
            )
            .await?;
        
        sleep(Duration::from_secs(10)).await;
    }
}

// Example curl commands to test the SSE stream:
// 
// ```bash
// # Connect as a specific agent
// curl -N "http://localhost:3000/discoveries/stream?agent_id=monitor-001"
// 
// # Connect with headers for better formatting
// curl -N -H "Accept: text/event-stream" \
//      "http://localhost:3000/discoveries/stream?agent_id=analyzer-002"
// 
// # Using EventSource in JavaScript:
// const source = new EventSource('http://localhost:3000/discoveries/stream?agent_id=web-001');
// source.addEventListener('discovery', (e) => {
//     const notification = JSON.parse(e.data);
//     console.log('Discovery:', notification.params.uri);
// });
// ```