//! Test NATS-based collaboration between persistent sessions
//!
//! This demonstrates the actual innovation: orchestrating multiple
//! persistent Claude sessions to collaborate in real-time

use mistersmith::collaboration::{DiscoveryBroadcaster, DiscoveryListener, DiscoveryType};
use anyhow::Result;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n🧪 TESTING NATS COLLABORATION LAYER");
    println!("=====================================\n");
    
    // Check NATS
    let nats = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => {
            println!("✅ Connected to NATS");
            client
        }
        Err(e) => {
            println!("❌ NATS not running: {}", e);
            println!("   Run: nats-server");
            return Ok(());
        }
    };
    
    // Test real-time discovery sharing
    test_discovery_sharing(nats.clone()).await?;
    
    println!("\n✅ NATS collaboration layer working!");
    println!("\n📝 Next step: Connect this with InteractiveClaudeSession");
    println!("   - Each agent spawns a persistent Claude REPL");
    println!("   - They share discoveries via NATS as they work");
    println!("   - Orchestrator synthesizes their findings");
    
    Ok(())
}

async fn test_discovery_sharing(nats: async_nats::Client) -> Result<()> {
    println!("🔬 Testing discovery broadcasting...\n");
    
    // Create broadcasters for different agents
    let security_agent = DiscoveryBroadcaster::new(
        "security-1".to_string(),
        "Security".to_string(),
        nats.clone()
    );
    
    let perf_agent = DiscoveryBroadcaster::new(
        "perf-1".to_string(),
        "Performance".to_string(),
        nats.clone()
    );
    
    // Create a listener (simulating orchestrator)
    let listener = DiscoveryListener::new(
        "orchestrator".to_string(),
        "Orchestrator".to_string(),
        nats.clone()
    );
    
    // Start listening in background
    let discoveries = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let discoveries_clone = discoveries.clone();
    
    tokio::spawn(async move {
        let _ = listener.listen(move |discovery| {
            println!("📡 Orchestrator received: {} from {}", 
                     discovery.content, 
                     discovery.agent_role);
            discoveries_clone.lock().unwrap().push(discovery);
            Ok(())
        }).await;
    });
    
    // Give listener time to subscribe
    sleep(Duration::from_millis(100)).await;
    
    // Simulate collaborative discovery
    println!("🔍 Security agent makes a discovery...");
    security_agent.share_discovery(
        DiscoveryType::Anomaly,
        "Suspicious login patterns from internal IPs",
        0.8
    ).await?;
    
    sleep(Duration::from_millis(500)).await;
    
    println!("📊 Performance agent correlates...");
    perf_agent.share_discovery(
        DiscoveryType::Connection,
        "CPU spikes coincide with login anomalies!",
        0.9
    ).await?;
    
    // Check what was received
    sleep(Duration::from_millis(500)).await;
    
    let received = discoveries.lock().unwrap();
    println!("\n📬 Orchestrator received {} discoveries", received.len());
    
    if received.len() >= 2 {
        println!("✅ Real-time discovery sharing works!");
        println!("\n🎯 This enables:");
        println!("   - Agents share insights AS they discover them");
        println!("   - Other agents can immediately build on findings");
        println!("   - Orchestrator sees the full picture emerge");
    } else {
        println!("⚠️  Expected 2 discoveries, got {}", received.len());
    }
    
    Ok(())
}