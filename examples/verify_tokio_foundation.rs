//! Verification script for the basic Tokio runtime foundation
//!
//! This example demonstrates that the foundation can be built upon
//! and extended with additional async capabilities.

use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::info;

/// Demonstrate building upon the basic foundation
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("=== Verifying Tokio Foundation is Ready for Extension ===");
    
    // Test 1: Foundation works
    verify_foundation().await?;
    
    // Test 2: Can be extended with channels
    verify_channel_extension().await?;
    
    // Test 3: Can be extended with multiple concurrent tasks
    verify_multi_task_extension().await?;
    
    info!("✅ Foundation verification complete - ready for MisterSmith features!");
    
    Ok(())
}

/// Verify the basic foundation works
async fn verify_foundation() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n📋 1. Foundation Verification");
    
    // Basic async operation
    let result = timeout(Duration::from_millis(100), async {
        sleep(Duration::from_millis(50)).await;
        "Foundation working"
    }).await?;
    
    info!("   ✅ {}", result);
    
    Ok(())
}

/// Verify foundation can be extended with channels
async fn verify_channel_extension() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n📡 2. Channel Extension");
    
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);
    
    // Producer task
    let producer = tokio::spawn(async move {
        for i in 1..=3 {
            let message = format!("Message {}", i);
            tx.send(message).await.unwrap();
            sleep(Duration::from_millis(10)).await;
        }
    });
    
    // Consumer task
    let consumer = tokio::spawn(async move {
        let mut messages = Vec::new();
        while let Some(msg) = rx.recv().await {
            messages.push(msg);
            if messages.len() == 3 {
                break;
            }
        }
        messages
    });
    
    // Wait for both tasks
    producer.await?;
    let messages = consumer.await?;
    
    info!("   ✅ Received {} messages via channel", messages.len());
    for msg in messages {
        info!("   📨 {}", msg);
    }
    
    Ok(())
}

/// Verify foundation can be extended with multiple concurrent tasks
async fn verify_multi_task_extension() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🚀 3. Multi-Task Extension");
    
    // Spawn multiple tasks concurrently
    let tasks: Vec<_> = (1..=5).map(|i| {
        tokio::spawn(async move {
            let delay = Duration::from_millis(i * 10);
            sleep(delay).await;
            format!("Task {} completed", i)
        })
    }).collect();
    
    // Wait for all tasks to complete
    let mut results = Vec::new();
    for task in tasks {
        let result = task.await?;
        results.push(result);
    }
    
    info!("   ✅ All {} tasks completed concurrently", results.len());
    for result in results {
        info!("   🎯 {}", result);
    }
    
    Ok(())
}