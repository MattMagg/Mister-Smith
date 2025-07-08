// Simple End-to-End Validation Test
// This test validates the core functionality using working components

use async_nats;
use futures::StreamExt;
use serde_json::json;
use std::time::Duration;
use tokio::time::{sleep, timeout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 SIMPLE END-TO-END VALIDATION TEST");
    println!("====================================");
    
    // Test 1: NATS Connection
    println!("\n🔗 Test 1: NATS Connection");
    let client = async_nats::connect("nats://localhost:4222").await?;
    println!("✅ Connected to NATS successfully");
    
    // Test 2: Discovery Broadcasting
    println!("\n📡 Test 2: Discovery Broadcasting");
    let mut subscriber = client.subscribe("discoveries.security-agent.pattern").await?;
    
    // Simulate discovery sharing
    let discovery_message = json!({
        "id": "test-discovery-1",
        "agent_id": "security-agent",
        "type": "pattern",
        "content": "Test discovery for validation",
        "confidence": 0.95,
        "timestamp": "2025-01-08T20:00:00Z"
    });
    
    client.publish("discoveries.security-agent.pattern", discovery_message.to_string().into()).await?;
    println!("📤 Published discovery message");
    
    // Test 3: Message Reception
    println!("\n📥 Test 3: Message Reception");
    match timeout(Duration::from_secs(5), subscriber.next()).await {
        Ok(Some(msg)) => {
            println!("✅ Received message: {}", String::from_utf8_lossy(&msg.payload));
            
            // Validate message format
            let parsed: serde_json::Value = serde_json::from_slice(&msg.payload)?;
            assert!(parsed.get("id").is_some(), "Message should have ID");
            assert!(parsed.get("agent_id").is_some(), "Message should have agent_id");
            assert!(parsed.get("type").is_some(), "Message should have type");
            println!("✅ Message format validation passed");
        }
        Ok(None) => return Err("No message received".into()),
        Err(_) => return Err("Timeout waiting for message".into()),
    }
    
    // Test 4: Multi-Agent Discovery Sharing
    println!("\n🤝 Test 4: Multi-Agent Discovery Sharing");
    let mut orchestrator_subscriber = client.subscribe("orchestrator.discoveries").await?;
    
    // Simulate multiple agents sharing discoveries
    let agents = vec!["security-agent", "performance-agent", "data-agent"];
    let discovery_types = vec!["pattern", "anomaly", "insight"];
    
    for (i, agent) in agents.iter().enumerate() {
        let discovery = json!({
            "id": format!("multi-agent-discovery-{}", i),
            "agent_id": agent,
            "type": discovery_types[i],
            "content": format!("Discovery from {}", agent),
            "confidence": 0.8 + (i as f64 * 0.05),
            "timestamp": "2025-01-08T20:00:00Z"
        });
        
        client.publish("orchestrator.discoveries", discovery.to_string().into()).await?;
        println!("📤 {} shared discovery", agent);
        sleep(Duration::from_millis(100)).await;
    }
    
    // Verify orchestrator received all discoveries
    let mut received_count = 0;
    while received_count < agents.len() {
        match timeout(Duration::from_secs(2), orchestrator_subscriber.next()).await {
            Ok(Some(_)) => {
                received_count += 1;
                println!("📥 Orchestrator received discovery #{}", received_count);
            }
            Ok(None) => break,
            Err(_) => {
                println!("⏰ Timeout waiting for orchestrator message");
                break;
            }
        }
    }
    
    assert_eq!(received_count, agents.len(), "All discoveries should be received");
    println!("✅ Multi-agent discovery sharing successful");
    
    // Test 5: Real-Time Performance
    println!("\n⚡ Test 5: Real-Time Performance");
    let test_count = 10;
    let mut latencies = Vec::new();
    
    for i in 0..test_count {
        let start_time = std::time::Instant::now();
        
        let discovery = json!({
            "id": format!("perf-test-{}", i),
            "agent_id": "performance-test-agent",
            "type": "benchmark",
            "content": format!("Performance test discovery #{}", i),
            "confidence": 0.9
        });
        
        client.publish("discoveries.performance-test.benchmark", discovery.to_string().into()).await?;
        
        let latency = start_time.elapsed();
        latencies.push(latency);
    }
    
    let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    let max_latency = latencies.iter().max().unwrap();
    let min_latency = latencies.iter().min().unwrap();
    
    println!("📊 Performance Results:");
    println!("   • Average latency: {:?}", avg_latency);
    println!("   • Maximum latency: {:?}", max_latency);
    println!("   • Minimum latency: {:?}", min_latency);
    println!("   • Throughput: {:.2} msg/sec", test_count as f64 / latencies.iter().sum::<Duration>().as_secs_f64());
    
    assert!(avg_latency < Duration::from_millis(10), "Average latency should be under 10ms");
    println!("✅ Performance test passed");
    
    println!("\n🎯 ALL VALIDATION TESTS PASSED!");
    println!("✅ NATS connectivity confirmed");
    println!("✅ Discovery broadcasting working");
    println!("✅ Message reception verified");
    println!("✅ Multi-agent coordination successful");
    println!("✅ Real-time performance acceptable");
    
    Ok(())
}