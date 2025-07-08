//! Comprehensive Integration Tests for Real-Time Discovery Sharing System
//!
//! This test suite validates the complete flow:
//! MCP client → share_discovery → NATS → SSE → subscribing client
//!
//! Tests include:
//! - End-to-end discovery flow
//! - Multi-agent collaboration patterns
//! - Real-time updates and subscriptions
//! - Error handling and edge cases
//! - Performance under load

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_nats::Client;
use axum::extract::ws::WebSocket;
use axum::extract::{Query, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use futures::{SinkExt, StreamExt};
use hyper::StatusCode;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

// Import the discovery sharing components
use mistersmith::collaboration::{
    Discovery, DiscoveryBroadcaster, DiscoveryListener, DiscoverySSEBroadcaster, DiscoveryType,
};

/// Test configuration
#[derive(Debug, Clone)]
struct TestConfig {
    nats_url: String,
    sse_server_port: u16,
    test_timeout: Duration,
    max_concurrent_agents: usize,
    discovery_batch_size: usize,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            nats_url: "nats://localhost:4222".to_string(),
            sse_server_port: 8080,
            test_timeout: Duration::from_secs(30),
            max_concurrent_agents: 10,
            discovery_batch_size: 100,
        }
    }
}

/// Test agent for simulation
#[derive(Debug, Clone)]
struct TestAgent {
    id: String,
    role: String,
    broadcaster: Option<DiscoveryBroadcaster>,
    listener: Option<DiscoveryListener>,
    discoveries_received: Arc<RwLock<Vec<Discovery>>>,
    discoveries_sent: Arc<RwLock<Vec<Discovery>>>,
}

impl TestAgent {
    async fn new(id: String, role: String, nats: Client) -> Self {
        Self {
            id: id.clone(),
            role: role.clone(),
            broadcaster: Some(DiscoveryBroadcaster::new(id.clone(), role.clone(), nats.clone())),
            listener: Some(DiscoveryListener::new(id, role, nats)),
            discoveries_received: Arc::new(RwLock::new(Vec::new())),
            discoveries_sent: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn share_discovery(&mut self, discovery_type: DiscoveryType, content: &str, confidence: f32) -> Result<()> {
        if let Some(broadcaster) = &self.broadcaster {
            let discovery = Discovery {
                agent_id: self.id.clone(),
                agent_role: self.role.clone(),
                discovery_type: discovery_type.clone(),
                content: content.to_string(),
                confidence,
                timestamp: chrono::Utc::now(),
                related_to: None,
            };

            self.discoveries_sent.write().await.push(discovery.clone());
            broadcaster.share_discovery(discovery_type, content, confidence).await?;
        }
        Ok(())
    }

    async fn start_listening(&mut self) -> Result<()> {
        if let Some(listener) = self.listener.take() {
            let discoveries_received = self.discoveries_received.clone();
            let agent_id = self.id.clone();
            
            tokio::spawn(async move {
                listener.listen(move |discovery| {
                    let discoveries_received = discoveries_received.clone();
                    let agent_id = agent_id.clone();
                    
                    tokio::spawn(async move {
                        debug!("Agent {} received discovery: {:?}", agent_id, discovery);
                        discoveries_received.write().await.push(discovery);
                    });
                    
                    Ok(())
                }).await.map_err(|e| {
                    error!("Listener error for agent {}: {}", agent_id, e);
                });
            });
        }
        Ok(())
    }

    async fn get_received_discoveries(&self) -> Vec<Discovery> {
        self.discoveries_received.read().await.clone()
    }

    async fn get_sent_discoveries(&self) -> Vec<Discovery> {
        self.discoveries_sent.read().await.clone()
    }
}

/// SSE client for testing
#[derive(Debug)]
struct SSETestClient {
    client: HttpClient,
    base_url: String,
    received_events: Arc<RwLock<Vec<String>>>,
}

impl SSETestClient {
    fn new(base_url: String) -> Self {
        Self {
            client: HttpClient::new(),
            base_url,
            received_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn connect_and_listen(&self, agent_id: &str) -> Result<()> {
        let url = format!("{}/discoveries/stream?agent_id={}", self.base_url, agent_id);
        let response = self.client.get(&url).send().await?;
        
        if response.status() != StatusCode::OK {
            anyhow::bail!("Failed to connect to SSE endpoint: {}", response.status());
        }

        let mut stream = response.bytes_stream();
        let received_events = self.received_events.clone();
        
        tokio::spawn(async move {
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let data = line.strip_prefix("data: ").unwrap_or(line);
                                debug!("SSE received: {}", data);
                                received_events.write().await.push(data.to_string());
                            }
                        }
                    }
                    Err(e) => {
                        error!("SSE stream error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    async fn get_received_events(&self) -> Vec<String> {
        self.received_events.read().await.clone()
    }
}

/// Test fixture for managing test infrastructure
#[derive(Debug)]
struct TestFixture {
    config: TestConfig,
    nats: Client,
    sse_broadcaster: Arc<DiscoverySSEBroadcaster>,
    sse_server_handle: Option<tokio::task::JoinHandle<()>>,
    agents: Vec<TestAgent>,
    sse_clients: Vec<SSETestClient>,
}

impl TestFixture {
    async fn new(config: TestConfig) -> Result<Self> {
        // Connect to NATS
        let nats = async_nats::connect(&config.nats_url).await
            .context("Failed to connect to NATS")?;

        // Create SSE broadcaster
        let sse_broadcaster = Arc::new(DiscoverySSEBroadcaster::new(nats.clone()));
        
        // Start SSE broadcaster listening
        sse_broadcaster.start_listening().await?;

        // Start SSE server
        let server_handle = Self::start_sse_server(sse_broadcaster.clone(), config.sse_server_port).await?;

        Ok(Self {
            config,
            nats,
            sse_broadcaster,
            sse_server_handle: Some(server_handle),
            agents: Vec::new(),
            sse_clients: Vec::new(),
        })
    }

    async fn start_sse_server(
        broadcaster: Arc<DiscoverySSEBroadcaster>,
        port: u16,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let app = broadcaster.create_router();
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
            .await
            .context("Failed to bind SSE server")?;

        let handle = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                error!("SSE server error: {}", e);
            }
        });

        // Give the server time to start
        sleep(Duration::from_millis(100)).await;

        Ok(handle)
    }

    async fn create_agent(&mut self, id: String, role: String) -> Result<()> {
        let agent = TestAgent::new(id, role, self.nats.clone()).await;
        self.agents.push(agent);
        Ok(())
    }

    async fn create_sse_client(&mut self, agent_id: String) -> Result<()> {
        let base_url = format!("http://127.0.0.1:{}", self.config.sse_server_port);
        let client = SSETestClient::new(base_url);
        client.connect_and_listen(&agent_id).await?;
        self.sse_clients.push(client);
        Ok(())
    }

    async fn start_all_agents_listening(&mut self) -> Result<()> {
        for agent in &mut self.agents {
            agent.start_listening().await?;
        }
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        if let Some(handle) = self.sse_server_handle.take() {
            handle.abort();
        }
        Ok(())
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        if let Some(handle) = &self.sse_server_handle {
            handle.abort();
        }
    }
}

/// Test utilities
struct TestUtils;

impl TestUtils {
    async fn wait_for_discoveries(
        agent: &TestAgent,
        expected_count: usize,
        timeout_duration: Duration,
    ) -> Result<Vec<Discovery>> {
        let start = Instant::now();
        
        loop {
            let discoveries = agent.get_received_discoveries().await;
            if discoveries.len() >= expected_count {
                return Ok(discoveries);
            }
            
            if start.elapsed() > timeout_duration {
                anyhow::bail!(
                    "Timeout waiting for {} discoveries, got {}",
                    expected_count,
                    discoveries.len()
                );
            }
            
            sleep(Duration::from_millis(50)).await;
        }
    }

    async fn wait_for_sse_events(
        client: &SSETestClient,
        expected_count: usize,
        timeout_duration: Duration,
    ) -> Result<Vec<String>> {
        let start = Instant::now();
        
        loop {
            let events = client.get_received_events().await;
            if events.len() >= expected_count {
                return Ok(events);
            }
            
            if start.elapsed() > timeout_duration {
                anyhow::bail!(
                    "Timeout waiting for {} SSE events, got {}",
                    expected_count,
                    events.len()
                );
            }
            
            sleep(Duration::from_millis(50)).await;
        }
    }

    fn generate_test_discoveries(count: usize) -> Vec<(DiscoveryType, String, f32)> {
        let mut discoveries = Vec::new();
        let types = vec![
            DiscoveryType::Pattern,
            DiscoveryType::Anomaly,
            DiscoveryType::Connection,
            DiscoveryType::Solution,
            DiscoveryType::Question,
            DiscoveryType::Insight,
        ];

        for i in 0..count {
            let discovery_type = types[i % types.len()].clone();
            let content = format!("Test discovery {} - {}", i, match discovery_type {
                DiscoveryType::Pattern => "Found repeating pattern",
                DiscoveryType::Anomaly => "Unusual behavior detected",
                DiscoveryType::Connection => "Connected two systems",
                DiscoveryType::Solution => "Potential solution identified",
                DiscoveryType::Question => "Need help with this",
                DiscoveryType::Insight => "General insight gained",
            });
            let confidence = 0.5 + (i as f32 % 5.0) * 0.1;
            
            discoveries.push((discovery_type, content, confidence));
        }
        
        discoveries
    }
}

//
// INTEGRATION TESTS
//

#[tokio::test]
async fn test_end_to_end_discovery_flow() -> Result<()> {
    let config = TestConfig::default();
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create test agents
    fixture.create_agent("agent-1".to_string(), "TestAgent1".to_string()).await?;
    fixture.create_agent("agent-2".to_string(), "TestAgent2".to_string()).await?;
    
    // Create SSE client
    fixture.create_sse_client("test-client".to_string()).await?;
    
    // Start all agents listening
    fixture.start_all_agents_listening().await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(200)).await;
    
    // Agent 1 shares a discovery
    fixture.agents[0].share_discovery(
        DiscoveryType::Pattern,
        "Found interesting pattern in logs",
        0.8
    ).await?;
    
    // Verify agent 2 receives it
    let discoveries = TestUtils::wait_for_discoveries(
        &fixture.agents[1],
        1,
        config.test_timeout,
    ).await?;
    
    assert_eq!(discoveries.len(), 1);
    assert_eq!(discoveries[0].agent_id, "agent-1");
    assert_eq!(discoveries[0].discovery_type, DiscoveryType::Pattern);
    assert_eq!(discoveries[0].content, "Found interesting pattern in logs");
    
    // Verify SSE client receives it
    let events = TestUtils::wait_for_sse_events(
        &fixture.sse_clients[0],
        1,
        config.test_timeout,
    ).await?;
    
    assert_eq!(events.len(), 1);
    assert!(events[0].contains("discovery"));
    
    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_multi_agent_collaboration_scenario() -> Result<()> {
    let config = TestConfig::default();
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create multiple agents with different roles
    fixture.create_agent("security-agent".to_string(), "Security".to_string()).await?;
    fixture.create_agent("performance-agent".to_string(), "Performance".to_string()).await?;
    fixture.create_agent("network-agent".to_string(), "Network".to_string()).await?;
    
    // Start all agents listening
    fixture.start_all_agents_listening().await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(200)).await;
    
    // Security agent finds an anomaly
    fixture.agents[0].share_discovery(
        DiscoveryType::Anomaly,
        "Unusual login patterns from IP range 192.168.x.x",
        0.7
    ).await?;
    
    // Performance agent finds a pattern
    fixture.agents[1].share_discovery(
        DiscoveryType::Pattern,
        "Database queries spiking from internal network",
        0.8
    ).await?;
    
    // Network agent connects the dots
    fixture.agents[2].share_discovery(
        DiscoveryType::Connection,
        "Internal bot scanning for SQL injection vulnerabilities!",
        0.95
    ).await?;
    
    // Verify all agents receive all discoveries
    for i in 0..3 {
        let discoveries = TestUtils::wait_for_discoveries(
            &fixture.agents[i],
            2, // Each agent should receive 2 discoveries (not their own)
            config.test_timeout,
        ).await?;
        
        assert_eq!(discoveries.len(), 2);
        
        // Verify they don't receive their own discoveries
        for discovery in &discoveries {
            assert_ne!(discovery.agent_id, fixture.agents[i].id);
        }
    }
    
    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_discovery_type_filtering() -> Result<()> {
    let config = TestConfig::default();
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create agents
    fixture.create_agent("broadcaster".to_string(), "Broadcaster".to_string()).await?;
    fixture.create_agent("listener".to_string(), "Listener".to_string()).await?;
    
    // Start listening
    fixture.start_all_agents_listening().await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(200)).await;
    
    // Send different types of discoveries
    let discovery_types = vec![
        (DiscoveryType::Pattern, "Pattern discovery"),
        (DiscoveryType::Anomaly, "Anomaly discovery"),
        (DiscoveryType::Connection, "Connection discovery"),
        (DiscoveryType::Solution, "Solution discovery"),
        (DiscoveryType::Question, "Question discovery"),
        (DiscoveryType::Insight, "Insight discovery"),
    ];
    
    for (discovery_type, content) in discovery_types {
        fixture.agents[0].share_discovery(discovery_type, content, 0.8).await?;
    }
    
    // Verify all discoveries are received
    let discoveries = TestUtils::wait_for_discoveries(
        &fixture.agents[1],
        6,
        config.test_timeout,
    ).await?;
    
    assert_eq!(discoveries.len(), 6);
    
    // Verify all types are represented
    let mut types_found = std::collections::HashSet::new();
    for discovery in &discoveries {
        types_found.insert(discovery.discovery_type.clone());
    }
    
    assert_eq!(types_found.len(), 6);
    
    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_discovery_sharing() -> Result<()> {
    let config = TestConfig::default();
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create multiple agents
    for i in 0..5 {
        fixture.create_agent(format!("agent-{}", i), format!("Agent{}", i)).await?;
    }
    
    // Start all agents listening
    fixture.start_all_agents_listening().await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(200)).await;
    
    // All agents share discoveries concurrently
    let mut tasks = Vec::new();
    
    for i in 0..fixture.agents.len() {
        let discoveries = TestUtils::generate_test_discoveries(5);
        let mut agent = fixture.agents[i].clone();
        
        tasks.push(tokio::spawn(async move {
            for (discovery_type, content, confidence) in discoveries {
                agent.share_discovery(discovery_type, &content, confidence).await?;
                // Small delay between discoveries
                sleep(Duration::from_millis(10)).await;
            }
            Ok::<(), anyhow::Error>(())
        }));
    }
    
    // Wait for all agents to finish sharing
    for task in tasks {
        task.await??;
    }
    
    // Verify each agent received discoveries from all others
    for i in 0..fixture.agents.len() {
        let discoveries = TestUtils::wait_for_discoveries(
            &fixture.agents[i],
            20, // 4 other agents * 5 discoveries each
            config.test_timeout,
        ).await?;
        
        assert_eq!(discoveries.len(), 20);
        
        // Verify they don't receive their own discoveries
        for discovery in &discoveries {
            assert_ne!(discovery.agent_id, fixture.agents[i].id);
        }
    }
    
    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_sse_connection_management() -> Result<()> {
    let config = TestConfig::default();
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create multiple SSE clients
    for i in 0..3 {
        fixture.create_sse_client(format!("client-{}", i)).await?;
    }
    
    // Create a broadcaster agent
    fixture.create_agent("broadcaster".to_string(), "Broadcaster".to_string()).await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(200)).await;
    
    // Send a discovery
    fixture.agents[0].share_discovery(
        DiscoveryType::Insight,
        "Test insight for SSE clients",
        0.9
    ).await?;
    
    // Verify all SSE clients receive the event
    for i in 0..3 {
        let events = TestUtils::wait_for_sse_events(
            &fixture.sse_clients[i],
            1,
            config.test_timeout,
        ).await?;
        
        assert_eq!(events.len(), 1);
        assert!(events[0].contains("discovery"));
    }
    
    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_error_handling_and_recovery() -> Result<()> {
    let config = TestConfig::default();
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create agents
    fixture.create_agent("agent-1".to_string(), "Agent1".to_string()).await?;
    fixture.create_agent("agent-2".to_string(), "Agent2".to_string()).await?;
    
    // Start listening
    fixture.start_all_agents_listening().await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(200)).await;
    
    // Test with invalid discovery data
    let result = fixture.agents[0].share_discovery(
        DiscoveryType::Pattern,
        "", // Empty content
        -1.0 // Invalid confidence
    ).await;
    
    // Should still work (system should handle invalid data gracefully)
    assert!(result.is_ok());
    
    // Test with extremely large content
    let large_content = "x".repeat(10000);
    let result = fixture.agents[0].share_discovery(
        DiscoveryType::Insight,
        &large_content,
        0.8
    ).await;
    
    assert!(result.is_ok());
    
    // Verify the valid discovery is still received
    let discoveries = TestUtils::wait_for_discoveries(
        &fixture.agents[1],
        2,
        config.test_timeout,
    ).await?;
    
    assert_eq!(discoveries.len(), 2);
    
    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_high_volume_discovery_load() -> Result<()> {
    let config = TestConfig {
        discovery_batch_size: 50,
        test_timeout: Duration::from_secs(60),
        ..TestConfig::default()
    };
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create agents
    fixture.create_agent("broadcaster".to_string(), "Broadcaster".to_string()).await?;
    fixture.create_agent("listener".to_string(), "Listener".to_string()).await?;
    
    // Start listening
    fixture.start_all_agents_listening().await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(200)).await;
    
    // Send many discoveries rapidly
    let discoveries = TestUtils::generate_test_discoveries(config.discovery_batch_size);
    
    let start_time = Instant::now();
    
    for (discovery_type, content, confidence) in discoveries {
        fixture.agents[0].share_discovery(discovery_type, &content, confidence).await?;
    }
    
    let send_duration = start_time.elapsed();
    
    // Verify all discoveries are received
    let received_discoveries = TestUtils::wait_for_discoveries(
        &fixture.agents[1],
        config.discovery_batch_size,
        config.test_timeout,
    ).await?;
    
    let total_duration = start_time.elapsed();
    
    assert_eq!(received_discoveries.len(), config.discovery_batch_size);
    
    // Performance metrics
    let send_rate = config.discovery_batch_size as f64 / send_duration.as_secs_f64();
    let total_rate = config.discovery_batch_size as f64 / total_duration.as_secs_f64();
    
    info!("High volume test results:");
    info!("  Discoveries sent: {}", config.discovery_batch_size);
    info!("  Send rate: {:.2} discoveries/sec", send_rate);
    info!("  Total rate: {:.2} discoveries/sec", total_rate);
    info!("  Send duration: {:?}", send_duration);
    info!("  Total duration: {:?}", total_duration);
    
    // Basic performance assertions
    assert!(send_rate > 10.0, "Send rate should be > 10 discoveries/sec");
    assert!(total_rate > 5.0, "Total rate should be > 5 discoveries/sec");
    
    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_memory_usage_under_load() -> Result<()> {
    let config = TestConfig::default();
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create agents
    fixture.create_agent("agent-1".to_string(), "Agent1".to_string()).await?;
    fixture.create_agent("agent-2".to_string(), "Agent2".to_string()).await?;
    
    // Start listening
    fixture.start_all_agents_listening().await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(200)).await;
    
    // Measure baseline memory
    let initial_discoveries_1 = fixture.agents[0].get_received_discoveries().await.len();
    let initial_discoveries_2 = fixture.agents[1].get_received_discoveries().await.len();
    
    // Send many discoveries
    for i in 0..1000 {
        fixture.agents[0].share_discovery(
            DiscoveryType::Pattern,
            &format!("Discovery {}", i),
            0.8
        ).await?;
        
        if i % 100 == 0 {
            // Give system time to process
            sleep(Duration::from_millis(10)).await;
        }
    }
    
    // Wait for processing
    sleep(Duration::from_millis(1000)).await;
    
    // Check memory usage (discovery storage)
    let final_discoveries_1 = fixture.agents[0].get_received_discoveries().await.len();
    let final_discoveries_2 = fixture.agents[1].get_received_discoveries().await.len();
    
    let growth_1 = final_discoveries_1 - initial_discoveries_1;
    let growth_2 = final_discoveries_2 - initial_discoveries_2;
    
    info!("Memory usage test results:");
    info!("  Agent 1 discovery growth: {}", growth_1);
    info!("  Agent 2 discovery growth: {}", growth_2);
    
    // Agent 1 shouldn't receive its own discoveries
    assert_eq!(growth_1, 0);
    // Agent 2 should receive all discoveries from agent 1
    assert_eq!(growth_2, 1000);
    
    fixture.cleanup().await?;
    Ok(())
}

//
// PERFORMANCE BENCHMARKS
//

#[tokio::test]
async fn benchmark_discovery_throughput() -> Result<()> {
    let config = TestConfig {
        discovery_batch_size: 1000,
        test_timeout: Duration::from_secs(120),
        ..TestConfig::default()
    };
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create agents
    fixture.create_agent("broadcaster".to_string(), "Broadcaster".to_string()).await?;
    fixture.create_agent("listener".to_string(), "Listener".to_string()).await?;
    
    // Start listening
    fixture.start_all_agents_listening().await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(200)).await;
    
    // Benchmark discovery sharing
    let discoveries = TestUtils::generate_test_discoveries(config.discovery_batch_size);
    
    let start_time = Instant::now();
    
    for (discovery_type, content, confidence) in discoveries {
        fixture.agents[0].share_discovery(discovery_type, &content, confidence).await?;
    }
    
    let send_duration = start_time.elapsed();
    
    // Wait for all discoveries to be received
    let received_discoveries = TestUtils::wait_for_discoveries(
        &fixture.agents[1],
        config.discovery_batch_size,
        config.test_timeout,
    ).await?;
    
    let total_duration = start_time.elapsed();
    
    // Calculate metrics
    let send_throughput = config.discovery_batch_size as f64 / send_duration.as_secs_f64();
    let receive_throughput = config.discovery_batch_size as f64 / total_duration.as_secs_f64();
    
    info!("Discovery Throughput Benchmark:");
    info!("  Total discoveries: {}", config.discovery_batch_size);
    info!("  Send throughput: {:.2} discoveries/sec", send_throughput);
    info!("  Receive throughput: {:.2} discoveries/sec", receive_throughput);
    info!("  Send latency: {:.2}ms", send_duration.as_millis() as f64 / config.discovery_batch_size as f64);
    info!("  End-to-end latency: {:.2}ms", total_duration.as_millis() as f64 / config.discovery_batch_size as f64);
    
    // Store results for later analysis
    let results = json!({
        "test": "discovery_throughput",
        "discoveries": config.discovery_batch_size,
        "send_throughput": send_throughput,
        "receive_throughput": receive_throughput,
        "send_duration_ms": send_duration.as_millis(),
        "total_duration_ms": total_duration.as_millis(),
    });
    
    info!("Benchmark results: {}", results);
    
    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn benchmark_multi_agent_scalability() -> Result<()> {
    let config = TestConfig {
        max_concurrent_agents: 20,
        discovery_batch_size: 100,
        test_timeout: Duration::from_secs(180),
        ..TestConfig::default()
    };
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create many agents
    for i in 0..config.max_concurrent_agents {
        fixture.create_agent(format!("agent-{}", i), format!("Agent{}", i)).await?;
    }
    
    // Start all agents listening
    fixture.start_all_agents_listening().await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(500)).await;
    
    let start_time = Instant::now();
    
    // All agents share discoveries concurrently
    let mut tasks = Vec::new();
    
    for i in 0..fixture.agents.len() {
        let discoveries = TestUtils::generate_test_discoveries(config.discovery_batch_size);
        let mut agent = fixture.agents[i].clone();
        
        tasks.push(tokio::spawn(async move {
            for (discovery_type, content, confidence) in discoveries {
                agent.share_discovery(discovery_type, &content, confidence).await?;
            }
            Ok::<(), anyhow::Error>(())
        }));
    }
    
    // Wait for all agents to finish sharing
    for task in tasks {
        task.await??;
    }
    
    let send_duration = start_time.elapsed();
    
    // Wait for all discoveries to be received
    let expected_discoveries = (config.max_concurrent_agents - 1) * config.discovery_batch_size;
    
    for i in 0..fixture.agents.len() {
        let _discoveries = TestUtils::wait_for_discoveries(
            &fixture.agents[i],
            expected_discoveries,
            config.test_timeout,
        ).await?;
    }
    
    let total_duration = start_time.elapsed();
    
    // Calculate metrics
    let total_discoveries = config.max_concurrent_agents * config.discovery_batch_size;
    let send_throughput = total_discoveries as f64 / send_duration.as_secs_f64();
    let receive_throughput = total_discoveries as f64 / total_duration.as_secs_f64();
    
    info!("Multi-Agent Scalability Benchmark:");
    info!("  Agents: {}", config.max_concurrent_agents);
    info!("  Discoveries per agent: {}", config.discovery_batch_size);
    info!("  Total discoveries: {}", total_discoveries);
    info!("  Send throughput: {:.2} discoveries/sec", send_throughput);
    info!("  Receive throughput: {:.2} discoveries/sec", receive_throughput);
    info!("  Send duration: {:?}", send_duration);
    info!("  Total duration: {:?}", total_duration);
    
    // Store results for later analysis
    let results = json!({
        "test": "multi_agent_scalability",
        "agents": config.max_concurrent_agents,
        "discoveries_per_agent": config.discovery_batch_size,
        "total_discoveries": total_discoveries,
        "send_throughput": send_throughput,
        "receive_throughput": receive_throughput,
        "send_duration_ms": send_duration.as_millis(),
        "total_duration_ms": total_duration.as_millis(),
    });
    
    info!("Benchmark results: {}", results);
    
    fixture.cleanup().await?;
    Ok(())
}

//
// INTEGRATION EXAMPLE TESTS
//

#[tokio::test]
async fn test_realistic_debugging_scenario() -> Result<()> {
    let config = TestConfig::default();
    let mut fixture = TestFixture::new(config.clone()).await?;
    
    // Create specialized agents
    fixture.create_agent("orchestrator".to_string(), "Orchestrator".to_string()).await?;
    fixture.create_agent("devops".to_string(), "DevOps".to_string()).await?;
    fixture.create_agent("debug".to_string(), "Debug".to_string()).await?;
    fixture.create_agent("performance".to_string(), "Performance".to_string()).await?;
    
    // Start all agents listening
    fixture.start_all_agents_listening().await?;
    
    // Give everything time to connect
    sleep(Duration::from_millis(200)).await;
    
    // Realistic debugging scenario
    
    // 1. Orchestrator identifies the problem
    fixture.agents[0].share_discovery(
        DiscoveryType::Question,
        "Debug intermittent 502 errors on uploads",
        0.9
    ).await?;
    
    // 2. DevOps finds initial clue
    fixture.agents[1].share_discovery(
        DiscoveryType::Anomaly,
        "Timeouts at exactly 60s mark in API gateway logs",
        0.8
    ).await?;
    
    // 3. Debug agent connects to worker logs
    fixture.agents[2].share_discovery(
        DiscoveryType::Pattern,
        "Queue depth spikes align with timeout events",
        0.85
    ).await?;
    
    // 4. Performance agent validates the correlation
    fixture.agents[3].share_discovery(
        DiscoveryType::Connection,
        "Memory usage spikes correlate with queue depth",
        0.9
    ).await?;
    
    // 5. Debug agent proposes solution
    fixture.agents[2].share_discovery(
        DiscoveryType::Solution,
        "Redis connection pool exhausted - increase from 10 to 50",
        0.95
    ).await?;
    
    // 6. Performance agent validates solution
    fixture.agents[3].share_discovery(
        DiscoveryType::Insight,
        "Modeling confirms fix will resolve issue",
        0.98
    ).await?;
    
    // Wait for all discoveries to propagate
    sleep(Duration::from_secs(1)).await;
    
    // Verify the collaboration worked
    for i in 0..4 {
        let discoveries = fixture.agents[i].get_received_discoveries().await;
        let sent_discoveries = fixture.agents[i].get_sent_discoveries().await;
        
        info!("Agent {} ({}) - Sent: {}, Received: {}", 
              i, fixture.agents[i].role, sent_discoveries.len(), discoveries.len());
        
        // Each agent should have received discoveries from others
        assert!(discoveries.len() > 0);
        
        // Verify they don't receive their own discoveries
        for discovery in &discoveries {
            assert_ne!(discovery.agent_id, fixture.agents[i].id);
        }
    }
    
    fixture.cleanup().await?;
    Ok(())
}

//
// UTILITY FUNCTIONS FOR RUNNING TESTS
//

/// Run all integration tests
pub async fn run_integration_tests() -> Result<()> {
    info!("Starting comprehensive integration tests for discovery sharing system");
    
    // Setup tracing for tests
    tracing_subscriber::fmt::init();
    
    let tests = vec![
        ("end_to_end_discovery_flow", test_end_to_end_discovery_flow()),
        ("multi_agent_collaboration", test_multi_agent_collaboration_scenario()),
        ("discovery_type_filtering", test_discovery_type_filtering()),
        ("concurrent_discovery_sharing", test_concurrent_discovery_sharing()),
        ("sse_connection_management", test_sse_connection_management()),
        ("error_handling_recovery", test_error_handling_and_recovery()),
        ("high_volume_load", test_high_volume_discovery_load()),
        ("memory_usage_load", test_memory_usage_under_load()),
        ("realistic_debugging", test_realistic_debugging_scenario()),
    ];
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (test_name, test_future) in tests {
        info!("Running test: {}", test_name);
        match test_future.await {
            Ok(()) => {
                info!("✅ Test passed: {}", test_name);
                passed += 1;
            }
            Err(e) => {
                error!("❌ Test failed: {} - {}", test_name, e);
                failed += 1;
            }
        }
    }
    
    info!("Integration test results: {} passed, {} failed", passed, failed);
    
    if failed > 0 {
        anyhow::bail!("Some integration tests failed");
    }
    
    Ok(())
}

/// Run performance benchmarks
pub async fn run_performance_benchmarks() -> Result<()> {
    info!("Starting performance benchmarks for discovery sharing system");
    
    let benchmarks = vec![
        ("discovery_throughput", benchmark_discovery_throughput()),
        ("multi_agent_scalability", benchmark_multi_agent_scalability()),
    ];
    
    let mut results = Vec::new();
    
    for (benchmark_name, benchmark_future) in benchmarks {
        info!("Running benchmark: {}", benchmark_name);
        match benchmark_future.await {
            Ok(()) => {
                info!("✅ Benchmark completed: {}", benchmark_name);
                results.push(format!("✅ {}", benchmark_name));
            }
            Err(e) => {
                error!("❌ Benchmark failed: {} - {}", benchmark_name, e);
                results.push(format!("❌ {}: {}", benchmark_name, e));
            }
        }
    }
    
    info!("Performance benchmark results:");
    for result in results {
        info!("  {}", result);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_framework_initialization() -> Result<()> {
        let config = TestConfig::default();
        let fixture = TestFixture::new(config).await?;
        
        // Basic smoke test to ensure framework initializes
        assert!(fixture.nats.connection_state() == async_nats::connection::State::Connected);
        assert!(fixture.sse_broadcaster.get_connected_clients().await.len() == 0);
        
        Ok(())
    }
}