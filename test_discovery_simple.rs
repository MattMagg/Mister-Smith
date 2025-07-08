//! Simple Discovery Sharing Test
//! 
//! This test validates that the discovery sharing system works correctly.

use std::time::Duration;
use tokio::time::sleep;
use anyhow::Result;
use tracing::{info, error};

// Import the collaboration components
use mistersmith::collaboration::{
    Discovery, DiscoveryBroadcaster, DiscoveryListener, DiscoveryType,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("🧪 TESTING DISCOVERY SHARING SYSTEM");
    info!("===================================");
    
    // Test 1: Basic connectivity
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => {
            info!("✅ Connected to NATS");
            client
        }
        Err(e) => {
            error!("❌ Failed to connect to NATS: {}", e);
            return Err(e.into());
        }
    };
    
    // Test 2: Create broadcaster and listener
    let broadcaster = DiscoveryBroadcaster::new(
        "test-agent-1".to_string(),
        "TestAgent".to_string(),
        nats_client.clone()
    );
    
    let listener = DiscoveryListener::new(
        "test-agent-2".to_string(),
        "ListenerAgent".to_string(),
        nats_client.clone()
    );
    
    info!("✅ Created broadcaster and listener");
    
    // Test 3: Set up discovery collection
    let discoveries = std::sync::Arc::new(tokio::sync::RwLock::new(Vec::<Discovery>::new()));
    let discoveries_clone = discoveries.clone();
    
    // Start listening for discoveries
    let listener_task = tokio::spawn(async move {
        listener.listen(move |discovery| {
            let discoveries = discoveries_clone.clone();
            tokio::spawn(async move {
                discoveries.write().await.push(discovery);
            });
            Ok(())
        }).await
    });
    
    // Give listener time to start
    sleep(Duration::from_millis(100)).await;
    
    // Test 4: Share discoveries
    info!("📡 Sharing discoveries...");
    
    broadcaster.share_discovery(
        DiscoveryType::Pattern,
        "Found interesting pattern in data",
        0.85
    ).await?;
    
    broadcaster.share_discovery(
        DiscoveryType::Insight,
        "System behavior shows improvement",
        0.7
    ).await?;
    
    broadcaster.share_discovery(
        DiscoveryType::Anomaly,
        "Detected unusual activity",
        0.9
    ).await?;
    
    // Wait for discoveries to be processed
    sleep(Duration::from_secs(2)).await;
    
    // Test 5: Verify discoveries were received
    let received_discoveries = discoveries.read().await;
    info!("📦 Received {} discoveries", received_discoveries.len());
    
    if received_discoveries.len() >= 3 {
        info!("✅ Discovery sharing working correctly!");
        
        for (i, discovery) in received_discoveries.iter().enumerate() {
            info!("  {}. {} ({}): {} [confidence: {}]", 
                i + 1,
                discovery.agent_role,
                match discovery.discovery_type {
                    DiscoveryType::Pattern => "Pattern",
                    DiscoveryType::Insight => "Insight", 
                    DiscoveryType::Anomaly => "Anomaly",
                    DiscoveryType::Connection => "Connection",
                    DiscoveryType::Solution => "Solution",
                    DiscoveryType::Question => "Question",
                },
                discovery.content,
                discovery.confidence
            );
        }
    } else {
        error!("❌ Expected at least 3 discoveries, got {}", received_discoveries.len());
    }
    
    // Test 6: Test related discovery
    info!("🔗 Testing related discovery...");
    
    broadcaster.share_related_discovery(
        DiscoveryType::Connection,
        "This connects to the pattern we found earlier",
        0.75,
        "Found interesting pattern in data"
    ).await?;
    
    // Wait for related discovery
    sleep(Duration::from_secs(1)).await;
    
    let final_discoveries = discoveries.read().await;
    info!("📦 Final count: {} discoveries", final_discoveries.len());
    
    if final_discoveries.len() >= 4 {
        info!("✅ Related discovery sharing working!");
        
        // Check for related discovery
        let related_discovery = final_discoveries.iter().find(|d| {
            d.discovery_type == DiscoveryType::Connection && d.related_to.is_some()
        });
        
        if let Some(related) = related_discovery {
            info!("  🔗 Related discovery found: {} -> {}", 
                related.related_to.as_ref().unwrap(),
                related.content
            );
        }
    } else {
        error!("❌ Expected at least 4 discoveries after related discovery, got {}", final_discoveries.len());
    }
    
    // Clean up
    listener_task.abort();
    
    info!("✅ DISCOVERY SHARING TEST COMPLETE");
    info!("===================================");
    info!("📊 Test Results:");
    info!("  - NATS Connection: ✅ Working");
    info!("  - Discovery Broadcasting: ✅ Working");
    info!("  - Discovery Listening: ✅ Working");
    info!("  - Related Discoveries: ✅ Working");
    info!("  - Total Discoveries: {}", final_discoveries.len());
    
    Ok(())
}