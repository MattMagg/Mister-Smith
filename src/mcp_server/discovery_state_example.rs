//! Usage example for the Discovery State management

use super::discovery_state::{DiscoveryStore, DiscoveryFilter};
use crate::collaboration::discovery_sharing::{Discovery, DiscoveryType};
use chrono::Utc;
use std::sync::Arc;

/// Example demonstrating how to use the DiscoveryStore
pub async fn discovery_store_example() -> anyhow::Result<()> {
    // Create a shared discovery store
    let store = Arc::new(DiscoveryStore::new());
    
    // Example 1: Store discoveries from multiple agents
    println!("=== Storing Discoveries ===");
    
    // Security agent finds an anomaly
    let security_discovery = Discovery {
        agent_id: "security-agent-1".to_string(),
        agent_role: "SecurityAnalyzer".to_string(),
        discovery_type: DiscoveryType::Anomaly,
        content: "Detected unusual login patterns from IP 192.168.1.100".to_string(),
        confidence: 0.85,
        timestamp: Utc::now(),
        related_to: None,
    };
    
    let id1 = store.store_discovery(security_discovery).await?;
    println!("Stored security discovery with ID: {}", id1);
    
    // Performance agent finds a pattern
    let perf_discovery = Discovery {
        agent_id: "perf-agent-1".to_string(),
        agent_role: "PerformanceMonitor".to_string(),
        discovery_type: DiscoveryType::Pattern,
        content: "Database query latency increased by 300% in last 10 minutes".to_string(),
        confidence: 0.92,
        timestamp: Utc::now(),
        related_to: None,
    };
    
    let id2 = store.store_discovery(perf_discovery).await?;
    println!("Stored performance discovery with ID: {}", id2);
    
    // Network agent connects the dots
    let network_discovery = Discovery {
        agent_id: "network-agent-1".to_string(),
        agent_role: "NetworkAnalyzer".to_string(),
        discovery_type: DiscoveryType::Connection,
        content: "Correlation detected: Login attempts coincide with database load spikes".to_string(),
        confidence: 0.95,
        timestamp: Utc::now(),
        related_to: Some(id1.clone()), // Links to the security discovery
    };
    
    let id3 = store.store_discovery(network_discovery).await?;
    println!("Stored network discovery with ID: {}", id3);
    
    // Example 2: Query discoveries by agent
    println!("\n=== Querying by Agent ===");
    let security_discoveries = store.get_discoveries_for_agent("security-agent-1").await;
    println!("Security agent has {} discoveries", security_discoveries.len());
    
    // Example 3: Query discoveries by type
    println!("\n=== Querying by Type ===");
    let anomalies = store.get_discoveries_by_type(DiscoveryType::Anomaly).await;
    println!("Found {} anomalies", anomalies.len());
    
    let patterns = store.get_discoveries_by_type(DiscoveryType::Pattern).await;
    println!("Found {} patterns", patterns.len());
    
    // Example 4: Set up subscriptions
    println!("\n=== Setting Up Subscriptions ===");
    
    // Incident response agent wants high-confidence anomalies and connections
    let incident_filter = DiscoveryFilter {
        types: Some(vec![DiscoveryType::Anomaly, DiscoveryType::Connection]),
        agent_roles: None,
        min_confidence: Some(0.8),
        keywords: None,
    };
    
    store.add_subscription("incident-response-1".to_string(), incident_filter).await;
    println!("Added subscription for incident response agent");
    
    // Analysis agent wants all patterns from specific roles
    let analysis_filter = DiscoveryFilter {
        types: Some(vec![DiscoveryType::Pattern]),
        agent_roles: Some(vec!["PerformanceMonitor".to_string(), "NetworkAnalyzer".to_string()]),
        min_confidence: None,
        keywords: None,
    };
    
    store.add_subscription("analysis-agent-1".to_string(), analysis_filter).await;
    println!("Added subscription for analysis agent");
    
    // Example 5: Test subscription matching
    println!("\n=== Testing Subscriptions ===");
    
    // New high-confidence anomaly
    let new_anomaly = Discovery {
        agent_id: "security-agent-2".to_string(),
        agent_role: "SecurityAnalyzer".to_string(),
        discovery_type: DiscoveryType::Anomaly,
        content: "Potential SQL injection attempt detected".to_string(),
        confidence: 0.9,
        timestamp: Utc::now(),
        related_to: None,
    };
    
    let subscribers = store.get_subscribed_agents(&new_anomaly).await;
    println!("Agents to notify about anomaly: {:?}", subscribers);
    
    // Example 6: Get recent discoveries
    println!("\n=== Recent Discoveries ===");
    let recent = store.get_recent_discoveries(5).await;
    for discovery in recent {
        println!("- {} ({}): {}", 
            discovery.agent_role, 
            match discovery.discovery_type {
                DiscoveryType::Pattern => "Pattern",
                DiscoveryType::Anomaly => "Anomaly",
                DiscoveryType::Connection => "Connection",
                DiscoveryType::Solution => "Solution",
                DiscoveryType::Question => "Question",
                DiscoveryType::Insight => "Insight",
            },
            discovery.content
        );
    }
    
    // Example 7: Check store statistics
    println!("\n=== Store Statistics ===");
    let stats = store.get_stats().await;
    println!("Total discoveries: {}", stats.total_discoveries);
    println!("Unique agents: {}", stats.unique_agents);
    println!("Discovery types: {}", stats.discovery_types);
    println!("Active subscriptions: {}", stats.active_subscriptions);
    println!("Capacity used: {:.1}%", stats.capacity_used);
    
    Ok(())
}

/// Example of concurrent access to the store
pub async fn concurrent_access_example() -> anyhow::Result<()> {
    let store = Arc::new(DiscoveryStore::new());
    
    // Spawn multiple tasks that access the store concurrently
    let mut handles = vec![];
    
    // Multiple agents storing discoveries
    for i in 0..5 {
        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            let discovery = Discovery {
                agent_id: format!("agent-{}", i),
                agent_role: "Worker".to_string(),
                discovery_type: DiscoveryType::Insight,
                content: format!("Discovery from agent {}", i),
                confidence: 0.7 + (i as f32 * 0.05),
                timestamp: Utc::now(),
                related_to: None,
            };
            
            store_clone.store_discovery(discovery).await
        });
        handles.push(handle);
    }
    
    // Wait for all stores to complete
    for handle in handles {
        handle.await??;
    }
    
    // Multiple readers querying concurrently
    let mut read_handles = vec![];
    
    for i in 0..3 {
        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            let discoveries = store_clone.get_recent_discoveries(10).await;
            println!("Reader {} found {} recent discoveries", i, discoveries.len());
        });
        read_handles.push(handle);
    }
    
    for handle in read_handles {
        handle.await?;
    }
    
    Ok(())
}

/// Example of subscription-based notification system
pub async fn subscription_notification_example() -> anyhow::Result<()> {
    let store = Arc::new(DiscoveryStore::new());
    
    // Set up various agent subscriptions
    
    // Security team wants all security-related discoveries
    let security_filter = DiscoveryFilter {
        types: None, // All types
        agent_roles: Some(vec!["SecurityAnalyzer".to_string()]),
        min_confidence: None,
        keywords: Some(vec!["security".to_string(), "attack".to_string(), "vulnerability".to_string()]),
    };
    store.add_subscription("security-team".to_string(), security_filter).await;
    
    // Ops team wants high-confidence anomalies and solutions
    let ops_filter = DiscoveryFilter {
        types: Some(vec![DiscoveryType::Anomaly, DiscoveryType::Solution]),
        agent_roles: None,
        min_confidence: Some(0.85),
        keywords: None,
    };
    store.add_subscription("ops-team".to_string(), ops_filter).await;
    
    // Research team wants all questions and insights
    let research_filter = DiscoveryFilter {
        types: Some(vec![DiscoveryType::Question, DiscoveryType::Insight]),
        agent_roles: None,
        min_confidence: None,
        keywords: None,
    };
    store.add_subscription("research-team".to_string(), research_filter).await;
    
    // Simulate various discoveries and check who gets notified
    let test_discoveries = vec![
        Discovery {
            agent_id: "sec-1".to_string(),
            agent_role: "SecurityAnalyzer".to_string(),
            discovery_type: DiscoveryType::Anomaly,
            content: "Detected potential security vulnerability in authentication module".to_string(),
            confidence: 0.9,
            timestamp: Utc::now(),
            related_to: None,
        },
        Discovery {
            agent_id: "perf-1".to_string(),
            agent_role: "PerformanceMonitor".to_string(),
            discovery_type: DiscoveryType::Question,
            content: "Why is CPU usage spiking every 30 minutes?".to_string(),
            confidence: 0.6,
            timestamp: Utc::now(),
            related_to: None,
        },
        Discovery {
            agent_id: "ml-1".to_string(),
            agent_role: "MLAnalyzer".to_string(),
            discovery_type: DiscoveryType::Solution,
            content: "Implementing request rate limiting could prevent the DoS attacks".to_string(),
            confidence: 0.88,
            timestamp: Utc::now(),
            related_to: None,
        },
    ];
    
    for discovery in test_discoveries {
        let id = store.store_discovery(discovery.clone()).await?;
        let subscribers = store.get_subscribed_agents(&discovery).await;
        
        println!("\nDiscovery: {} ({})", 
            discovery.content,
            match discovery.discovery_type {
                DiscoveryType::Anomaly => "Anomaly",
                DiscoveryType::Question => "Question",
                DiscoveryType::Solution => "Solution",
                _ => "Other",
            }
        );
        println!("Will notify: {:?}", subscribers);
    }
    
    Ok(())
}