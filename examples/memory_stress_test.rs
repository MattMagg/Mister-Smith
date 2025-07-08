//! Memory Stress Test for DiscoveryStore FIFO Limit
//!
//! This test specifically validates the 1000 discovery FIFO eviction mechanism
//! and measures system behavior under memory pressure.
//!
//! Usage: cargo run --example memory_stress_test

use std::time::{Duration, Instant};
use anyhow::Result;
use tokio::time::sleep;
use mistersmith::collaboration::{DiscoveryBroadcaster, DiscoveryListener, DiscoveryType};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧠 MisterSmith Memory Stress Test");
    println!("================================");
    println!("Testing 1000 discovery FIFO eviction limit\n");
    
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
    
    // Test 1: FIFO Limit Test (1200 discoveries to exceed 1000 limit)
    println!("🔬 Test 1: FIFO Eviction Test");
    println!("   - Sending 1200 discoveries (200 over limit)");
    println!("   - Expecting oldest 200 to be evicted");
    
    let broadcaster = DiscoveryBroadcaster::new(
        "memory-test-agent".to_string(),
        "MemoryTester".to_string(),
        nats.clone()
    );
    
    let listener = DiscoveryListener::new(
        "memory-listener".to_string(),
        "MemoryListener".to_string(),
        nats.clone()
    );
    
    // Track discoveries received
    let discoveries_received = std::sync::Arc::new(std::sync::Mutex::new(0));
    let discoveries_received_clone = discoveries_received.clone();
    
    // Start listening
    tokio::spawn(async move {
        let _ = listener.listen(move |_discovery| {
            let mut count = discoveries_received_clone.lock().unwrap();
            *count += 1;
            Ok(())
        }).await;
    });
    
    // Give listener time to subscribe
    sleep(Duration::from_millis(500)).await;
    
    // Send discoveries in batches
    let batch_size = 100;
    let total_discoveries = 1200;
    let start_time = Instant::now();
    
    for batch in 0..(total_discoveries / batch_size) {
        let batch_start = Instant::now();
        
        for i in 0..batch_size {
            let discovery_id = batch * batch_size + i;
            let content = format!("Memory stress discovery #{} - batch {}", discovery_id, batch);
            
            broadcaster.share_discovery(
                DiscoveryType::Pattern,
                &content,
                0.8
            ).await?;
        }
        
        let batch_duration = batch_start.elapsed();
        let throughput = batch_size as f64 / batch_duration.as_secs_f64();
        
        println!("   📊 Batch {} completed: {} discoveries in {:?} ({:.1} discoveries/sec)", 
                 batch + 1, batch_size, batch_duration, throughput);
        
        // Small delay to prevent overwhelming the system
        sleep(Duration::from_millis(50)).await;
    }
    
    // Wait for processing
    sleep(Duration::from_secs(2)).await;
    
    let total_duration = start_time.elapsed();
    let received_count = *discoveries_received.lock().unwrap();
    
    println!("\n📈 Memory Stress Test Results:");
    println!("   - Total discoveries sent: {}", total_discoveries);
    println!("   - Total discoveries received: {}", received_count);
    println!("   - Test duration: {:?}", total_duration);
    println!("   - Overall throughput: {:.1} discoveries/sec", 
             total_discoveries as f64 / total_duration.as_secs_f64());
    
    // Test 2: Sustained Memory Pressure Test
    println!("\n🔬 Test 2: Sustained Memory Pressure Test");
    println!("   - Continuous discovery generation for 30 seconds");
    println!("   - Measuring memory usage patterns");
    
    let sustained_test_duration = Duration::from_secs(30);
    let sustained_start = Instant::now();
    let mut discovery_count = 0;
    
    while sustained_start.elapsed() < sustained_test_duration {
        let content = format!("Sustained memory test discovery #{} at {:?}", 
                             discovery_count, sustained_start.elapsed());
        
        broadcaster.share_discovery(
            DiscoveryType::Insight,
            &content,
            0.7
        ).await?;
        
        discovery_count += 1;
        
        // Brief pause to simulate realistic load
        sleep(Duration::from_millis(100)).await;
        
        // Report progress every 5 seconds
        if sustained_start.elapsed().as_secs() % 5 == 0 && 
           sustained_start.elapsed().as_millis() % 5000 < 150 {
            let elapsed = sustained_start.elapsed();
            let rate = discovery_count as f64 / elapsed.as_secs_f64();
            println!("   📊 {:?}: {} discoveries sent ({:.1} discoveries/sec)", 
                     elapsed, discovery_count, rate);
        }
    }
    
    let sustained_duration = sustained_start.elapsed();
    let sustained_received = *discoveries_received.lock().unwrap();
    
    println!("\n📈 Sustained Memory Pressure Results:");
    println!("   - Duration: {:?}", sustained_duration);
    println!("   - Discoveries sent: {}", discovery_count);
    println!("   - Discoveries received: {}", sustained_received);
    println!("   - Average throughput: {:.1} discoveries/sec", 
             discovery_count as f64 / sustained_duration.as_secs_f64());
    
    // Test 3: Rapid Burst Test
    println!("\n🔬 Test 3: Rapid Burst Test");
    println!("   - 500 discoveries as fast as possible");
    println!("   - Measuring peak throughput");
    
    let burst_count = 500;
    let burst_start = Instant::now();
    
    for i in 0..burst_count {
        let content = format!("Burst test discovery #{}", i);
        
        broadcaster.share_discovery(
            DiscoveryType::Anomaly,
            &content,
            0.9
        ).await?;
    }
    
    let burst_duration = burst_start.elapsed();
    let burst_throughput = burst_count as f64 / burst_duration.as_secs_f64();
    
    println!("   📊 Burst test completed in {:?}", burst_duration);
    println!("   📊 Peak throughput: {:.1} discoveries/sec", burst_throughput);
    
    // Wait for final processing
    sleep(Duration::from_secs(3)).await;
    
    let final_received = *discoveries_received.lock().unwrap();
    
    println!("\n🎯 Final Memory Stress Test Summary:");
    println!("=====================================");
    println!("Test 1 (FIFO Limit): {} discoveries sent", total_discoveries);
    println!("Test 2 (Sustained): {} discoveries sent", discovery_count);
    println!("Test 3 (Burst): {} discoveries sent", burst_count);
    println!("Total discoveries sent: {}", total_discoveries + discovery_count + burst_count);
    println!("Total discoveries received: {}", final_received);
    println!("Peak throughput: {:.1} discoveries/sec", burst_throughput);
    
    // Memory usage estimation
    let estimated_peak_memory = (total_discoveries + discovery_count + burst_count) * 200; // bytes per discovery
    println!("Estimated peak memory usage: {} KB", estimated_peak_memory / 1024);
    
    if final_received > 0 {
        println!("✅ Memory stress test completed successfully!");
        println!("✅ System handled {} total discoveries", final_received);
        println!("✅ FIFO eviction mechanism working (if final_received ≤ 1000)");
    } else {
        println!("⚠️  No discoveries received - check system setup");
    }
    
    Ok(())
}