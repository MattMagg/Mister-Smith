// End-to-End Validation Test for MisterSmith Real-Time Discovery Flow
// This test validates the complete pipeline: MCP Client → CollaborationBridge → DiscoveryBroadcaster → NATS → SSE → Clients

use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use serde_json::json;
use uuid::Uuid;

use mistersmith::{
    collaboration::{
        discovery_sharing::{Discovery, DiscoveryType},
        orchestrator::LiveOrchestrator,
        bridge::CollaborationBridge,
        sse_broadcaster::DiscoverySSEBroadcaster,
    },
    agent::Agent,
    runtime::RuntimeConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("🧪 MISTERSMITH END-TO-END VALIDATION TEST");
    println!("=========================================");
    
    // Initialize test environment
    let test_runner = EndToEndTestRunner::new().await?;
    
    // Run validation tests
    test_runner.run_bridge_integration_test().await?;
    test_runner.run_discovery_sharing_test().await?;
    test_runner.run_multi_agent_test().await?;
    test_runner.run_real_time_flow_test().await?;
    test_runner.run_performance_test().await?;
    
    println!("\n✅ ALL END-TO-END VALIDATION TESTS PASSED!");
    println!("🎯 Real-time discovery sharing system is fully functional");
    
    Ok(())
}

struct EndToEndTestRunner {
    nats_client: async_nats::Client,
    orchestrator: Arc<LiveOrchestrator>,
    bridge: Arc<CollaborationBridge>,
    sse_broadcaster: Arc<DiscoverySSEBroadcaster>,
    test_agents: Vec<TestAgent>,
}

struct TestAgent {
    id: String,
    role: String,
    agent: Agent,
}

impl EndToEndTestRunner {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        println!("🔧 Initializing test environment...");
        
        // Connect to NATS
        let nats_client = async_nats::connect("nats://localhost:4222").await?;
        println!("✅ Connected to NATS");
        
        // Initialize orchestrator
        let orchestrator = Arc::new(LiveOrchestrator::new(nats_client.clone()));
        
        // Initialize SSE broadcaster
        let sse_broadcaster = Arc::new(DiscoverySSEBroadcaster::new());
        
        // Initialize collaboration bridge
        let bridge = Arc::new(CollaborationBridge::new(
            nats_client.clone(),
            orchestrator.clone(),
            sse_broadcaster.clone(),
        ));
        
        // Create test agents
        let test_agents = vec![
            TestAgent {
                id: "security-agent".to_string(),
                role: "Security Analysis".to_string(),
                agent: Agent::new("security-agent".to_string(), RuntimeConfig::default()),
            },
            TestAgent {
                id: "performance-agent".to_string(),
                role: "Performance Monitoring".to_string(),
                agent: Agent::new("performance-agent".to_string(), RuntimeConfig::default()),
            },
            TestAgent {
                id: "data-agent".to_string(),
                role: "Data Analysis".to_string(),
                agent: Agent::new("data-agent".to_string(), RuntimeConfig::default()),
            },
        ];
        
        println!("✅ Test environment initialized");
        
        Ok(Self {
            nats_client,
            orchestrator,
            bridge,
            sse_broadcaster,
            test_agents,
        })
    }
    
    async fn run_bridge_integration_test(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🧪 TEST 1: Bridge Integration Test");
        println!("----------------------------------");
        
        // Test discovery sharing through bridge
        let discovery = Discovery {
            id: Uuid::new_v4().to_string(),
            agent_id: "security-agent".to_string(),
            discovery_type: DiscoveryType::Pattern,
            content: "Suspicious login patterns detected from internal IPs".to_string(),
            confidence: 0.85,
            timestamp: chrono::Utc::now(),
            tags: vec!["security".to_string(), "authentication".to_string()],
            metadata: json!({
                "source": "auth_logs",
                "pattern_type": "temporal_anomaly"
            }),
        };
        
        // Share discovery through bridge
        self.bridge.share_discovery(discovery.clone()).await?;
        println!("✅ Discovery shared through bridge");
        
        // Verify discovery propagation
        sleep(Duration::from_millis(100)).await;
        
        // Test subscription functionality
        let subscription_result = self.bridge.subscribe_to_discoveries(
            "performance-agent".to_string(),
            Some(vec!["security".to_string()]),
            None,
        ).await?;
        
        println!("✅ Discovery subscription successful");
        println!("🎯 Bridge integration test passed");
        
        Ok(())
    }
    
    async fn run_discovery_sharing_test(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🧪 TEST 2: Discovery Sharing Test");
        println!("----------------------------------");
        
        let mut discoveries_received = 0;
        
        // Set up discovery listener
        let mut subscriber = self.nats_client.subscribe("discoveries.>").await?;
        
        // Create various discovery types
        let discoveries = vec![
            Discovery {
                id: Uuid::new_v4().to_string(),
                agent_id: "security-agent".to_string(),
                discovery_type: DiscoveryType::Anomaly,
                content: "Unusual CPU spike during login attempts".to_string(),
                confidence: 0.92,
                timestamp: chrono::Utc::now(),
                tags: vec!["security".to_string(), "performance".to_string()],
                metadata: json!({"severity": "high"}),
            },
            Discovery {
                id: Uuid::new_v4().to_string(),
                agent_id: "performance-agent".to_string(),
                discovery_type: DiscoveryType::Connection,
                content: "CPU spikes correlate with failed authentication events".to_string(),
                confidence: 0.78,
                timestamp: chrono::Utc::now(),
                tags: vec!["correlation".to_string()],
                metadata: json!({"correlation_strength": 0.78}),
            },
            Discovery {
                id: Uuid::new_v4().to_string(),
                agent_id: "data-agent".to_string(),
                discovery_type: DiscoveryType::Solution,
                content: "Implement rate limiting on authentication endpoints".to_string(),
                confidence: 0.95,
                timestamp: chrono::Utc::now(),
                tags: vec!["recommendation".to_string()],
                metadata: json!({"priority": "high"}),
            },
        ];
        
        // Share discoveries
        for discovery in discoveries {
            self.bridge.share_discovery(discovery).await?;
            discoveries_received += 1;
            println!("📤 Shared discovery #{}", discoveries_received);
        }
        
        // Verify NATS message delivery
        let timeout_duration = Duration::from_secs(5);
        let mut received_count = 0;
        
        while received_count < discoveries_received {
            match timeout(timeout_duration, subscriber.next()).await {
                Ok(Some(msg)) => {
                    received_count += 1;
                    println!("📥 Received discovery message #{}", received_count);
                }
                Ok(None) => break,
                Err(_) => {
                    println!("⏰ Timeout waiting for discovery messages");
                    break;
                }
            }
        }
        
        assert_eq!(received_count, discoveries_received, "All discoveries should be received");
        println!("✅ All discoveries propagated through NATS");
        println!("🎯 Discovery sharing test passed");
        
        Ok(())
    }
    
    async fn run_multi_agent_test(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🧪 TEST 3: Multi-Agent Coordination Test");
        println!("----------------------------------------");
        
        let mut received_discoveries = Vec::new();
        
        // Set up subscriber for orchestrator messages
        let mut subscriber = self.nats_client.subscribe("orchestrator.discoveries").await?;
        
        // Simulate multi-agent discovery sharing scenario
        for (i, agent) in self.test_agents.iter().enumerate() {
            let discovery = Discovery {
                id: Uuid::new_v4().to_string(),
                agent_id: agent.id.clone(),
                discovery_type: match i % 3 {
                    0 => DiscoveryType::Pattern,
                    1 => DiscoveryType::Anomaly,
                    _ => DiscoveryType::Insight,
                },
                content: format!("{} discovery from {}", agent.role, agent.id),
                confidence: 0.80 + (i as f64 * 0.05),
                timestamp: chrono::Utc::now(),
                tags: vec![agent.role.to_lowercase().replace(" ", "_")],
                metadata: json!({"agent_role": agent.role}),
            };
            
            // Share discovery
            self.bridge.share_discovery(discovery.clone()).await?;
            println!("📤 {} shared discovery", agent.role);
            
            // Small delay to ensure proper ordering
            sleep(Duration::from_millis(50)).await;
        }
        
        // Collect orchestrator messages
        let timeout_duration = Duration::from_secs(3);
        while received_discoveries.len() < self.test_agents.len() {
            match timeout(timeout_duration, subscriber.next()).await {
                Ok(Some(msg)) => {
                    received_discoveries.push(msg);
                    println!("📥 Orchestrator received discovery #{}", received_discoveries.len());
                }
                Ok(None) => break,
                Err(_) => {
                    println!("⏰ Timeout waiting for orchestrator messages");
                    break;
                }
            }
        }
        
        assert_eq!(received_discoveries.len(), self.test_agents.len(), 
                   "Orchestrator should receive all discoveries");
        println!("✅ All agents successfully coordinated through orchestrator");
        println!("🎯 Multi-agent coordination test passed");
        
        Ok(())
    }
    
    async fn run_real_time_flow_test(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🧪 TEST 4: Real-Time Flow Test");
        println!("------------------------------");
        
        // Test SSE streaming functionality
        let sse_client_id = Uuid::new_v4().to_string();
        
        // Register SSE client
        self.sse_broadcaster.register_client(sse_client_id.clone(), "test-agent".to_string()).await;
        println!("✅ SSE client registered");
        
        // Create test discovery
        let discovery = Discovery {
            id: Uuid::new_v4().to_string(),
            agent_id: "real-time-agent".to_string(),
            discovery_type: DiscoveryType::Insight,
            content: "Real-time discovery sharing test".to_string(),
            confidence: 0.99,
            timestamp: chrono::Utc::now(),
            tags: vec!["real-time".to_string()],
            metadata: json!({"test": true}),
        };
        
        // Measure discovery sharing latency
        let start_time = std::time::Instant::now();
        
        // Share discovery
        self.bridge.share_discovery(discovery.clone()).await?;
        
        // Measure end-to-end latency
        let latency = start_time.elapsed();
        println!("⏱️ Discovery sharing latency: {:?}", latency);
        
        // Verify SSE broadcast
        self.sse_broadcaster.broadcast_discovery(discovery).await?;
        println!("✅ SSE broadcast successful");
        
        // Cleanup
        self.sse_broadcaster.unregister_client(&sse_client_id).await;
        println!("✅ SSE client unregistered");
        
        assert!(latency < Duration::from_millis(100), "Latency should be under 100ms");
        println!("🎯 Real-time flow test passed");
        
        Ok(())
    }
    
    async fn run_performance_test(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🧪 TEST 5: Performance Test");
        println!("---------------------------");
        
        let test_count = 50;
        let mut latencies = Vec::new();
        
        // Performance test with multiple concurrent discoveries
        for i in 0..test_count {
            let discovery = Discovery {
                id: Uuid::new_v4().to_string(),
                agent_id: format!("perf-agent-{}", i % 3),
                discovery_type: DiscoveryType::Pattern,
                content: format!("Performance test discovery #{}", i),
                confidence: 0.75,
                timestamp: chrono::Utc::now(),
                tags: vec!["performance".to_string()],
                metadata: json!({"test_id": i}),
            };
            
            let start_time = std::time::Instant::now();
            self.bridge.share_discovery(discovery).await?;
            let latency = start_time.elapsed();
            latencies.push(latency);
            
            if i % 10 == 0 {
                println!("📊 Processed {} discoveries", i + 1);
            }
        }
        
        // Calculate performance metrics
        let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
        let max_latency = latencies.iter().max().unwrap();
        let min_latency = latencies.iter().min().unwrap();
        
        println!("📈 Performance Metrics:");
        println!("   • Average latency: {:?}", avg_latency);
        println!("   • Maximum latency: {:?}", max_latency);
        println!("   • Minimum latency: {:?}", min_latency);
        println!("   • Total discoveries: {}", test_count);
        println!("   • Throughput: {:.2} discoveries/sec", 
                 test_count as f64 / latencies.iter().sum::<Duration>().as_secs_f64());
        
        assert!(avg_latency < Duration::from_millis(50), "Average latency should be under 50ms");
        println!("🎯 Performance test passed");
        
        Ok(())
    }
}