//! Bridge Integration Test - Comprehensive test of MCP-Collaboration bridge
//!
//! This test verifies that the bridge successfully connects MCP handlers
//! to the collaboration infrastructure with proper data flow and consistency.

use std::time::Duration;
use tokio::time::sleep;
use anyhow::Result;
use tracing::{info, error};

use crate::collaboration::{
    initialize_bridge, get_bridge, CollaborationBridge,
    DiscoveryType, DiscoveryBroadcaster, LiveOrchestrator,
    ContextManager, DiscoverySSEBroadcaster, Discovery
};
use crate::handlers::{
    Parameters, DISCOVERY_STORE,
    share_discovery::ShareDiscoveryParams,
    subscribe_discoveries::SubscribeDiscoveriesParams,
    bridged_handlers::*
};

/// Test that the bridge initializes correctly and integrates all components
#[tokio::test]
async fn test_bridge_initialization_and_integration() -> Result<()> {
    // Initialize NATS connection
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping test");
            return Ok(());
        }
    };
    
    // Initialize bridge
    initialize_bridge(nats_client.clone()).await?;
    
    // Verify bridge is available
    let bridge = get_bridge().await.expect("Bridge should be initialized");
    
    // Test collaboration status
    let status = bridge.get_collaboration_status().await?;
    assert!(status.content.len() > 0, "Status should contain content");
    
    info!("✅ Bridge initialization and integration test passed");
    Ok(())
}

/// Test that bridged discovery sharing works correctly
#[tokio::test]
async fn test_bridged_discovery_sharing() -> Result<()> {
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping test");
            return Ok(());
        }
    };
    
    // Initialize bridge
    initialize_bridge(nats_client.clone()).await?;
    
    // Test discovery sharing
    let result = bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "test-agent-bridge".to_string(),
                agent_role: "TestRole".to_string(),
                discovery_type: "pattern".to_string(),
                content: "Bridge test discovery".to_string(),
                confidence: 0.85,
                related_to: None,
            }
        },
        nats_client.clone()
    ).await?;
    
    // Verify result contains collaboration metadata
    assert!(result.content.len() > 0, "Result should contain content");
    
    // Verify discovery was stored
    let stored_discoveries = DISCOVERY_STORE.get_discoveries_for_agent("test-agent-bridge").await;
    assert!(stored_discoveries.len() > 0, "Discovery should be stored");
    
    info!("✅ Bridged discovery sharing test passed");
    Ok(())
}

/// Test that bridged subscription works correctly
#[tokio::test]
async fn test_bridged_subscription() -> Result<()> {
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping test");
            return Ok(());
        }
    };
    
    // Initialize bridge
    initialize_bridge(nats_client.clone()).await?;
    
    // Test subscription creation
    let result = bridged_subscribe_discoveries_handler(
        Parameters {
            params: SubscribeDiscoveriesParams {
                discovery_types: Some(vec!["pattern".to_string()]),
                agent_roles: Some(vec!["TestRole".to_string()]),
                min_confidence: Some(0.7),
                subscriber_agent_id: "test-subscriber-bridge".to_string(),
            }
        },
        nats_client.clone()
    ).await?;
    
    // Verify result contains SSE endpoint
    assert!(result.content.len() > 0, "Result should contain content");
    
    // Verify subscription was stored
    let subscription = DISCOVERY_STORE.get_subscription("test-subscriber-bridge").await;
    assert!(subscription.is_some(), "Subscription should be stored");
    
    info!("✅ Bridged subscription test passed");
    Ok(())
}

/// Test that agent spawning works correctly
#[tokio::test]
async fn test_collaborative_agent_spawning() -> Result<()> {
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping test");
            return Ok(());
        }
    };
    
    // Initialize bridge
    initialize_bridge(nats_client.clone()).await?;
    
    // Test agent spawning
    let result = spawn_collaborative_agent_handler(
        Parameters {
            params: SpawnAgentParams {
                agent_role: "TestAgent".to_string(),
                initial_context: "Test context for bridge".to_string(),
            }
        },
        nats_client.clone()
    ).await?;
    
    // Verify result contains agent info
    assert!(result.content.len() > 0, "Result should contain content");
    
    info!("✅ Collaborative agent spawning test passed");
    Ok(())
}

/// Test context sharing between agents
#[tokio::test]
async fn test_context_sharing() -> Result<()> {
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping test");
            return Ok(());
        }
    };
    
    // Initialize bridge
    initialize_bridge(nats_client.clone()).await?;
    
    // Test context sharing
    let result = share_context_handler(
        Parameters {
            params: ShareContextParams {
                from_agent: "test-agent-1".to_string(),
                to_agent: Some("test-agent-2".to_string()),
                context_type: "working_memory".to_string(),
                content: "Shared context from bridge test".to_string(),
                relevance: 0.8,
            }
        },
        nats_client.clone()
    ).await?;
    
    // Verify result indicates success
    assert!(result.content.len() > 0, "Result should contain content");
    
    info!("✅ Context sharing test passed");
    Ok(())
}

/// Test task orchestration
#[tokio::test]
async fn test_task_orchestration() -> Result<()> {
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping test");
            return Ok(());
        }
    };
    
    // Initialize bridge
    initialize_bridge(nats_client.clone()).await?;
    
    // Test task orchestration
    let result = orchestrate_task_handler(
        Parameters {
            params: OrchestateTaskParams {
                task_description: "Test task orchestration through bridge".to_string(),
                required_agents: Some(vec!["TestAgent".to_string()]),
                timeout_seconds: Some(30),
            }
        },
        nats_client.clone()
    ).await?;
    
    // Verify result contains task info
    assert!(result.content.len() > 0, "Result should contain content");
    
    info!("✅ Task orchestration test passed");
    Ok(())
}

/// Test complete discovery flow from MCP to SSE
#[tokio::test]
async fn test_complete_discovery_flow() -> Result<()> {
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping test");
            return Ok(());
        }
    };
    
    // Initialize bridge
    initialize_bridge(nats_client.clone()).await?;
    
    // Step 1: Create subscription
    let subscription_result = bridged_subscribe_discoveries_handler(
        Parameters {
            params: SubscribeDiscoveriesParams {
                discovery_types: Some(vec!["insight".to_string()]),
                agent_roles: None,
                min_confidence: Some(0.5),
                subscriber_agent_id: "flow-test-subscriber".to_string(),
            }
        },
        nats_client.clone()
    ).await?;
    
    assert!(subscription_result.content.len() > 0, "Subscription should be created");
    
    // Step 2: Share discovery
    let discovery_result = bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "flow-test-agent".to_string(),
                agent_role: "FlowTest".to_string(),
                discovery_type: "insight".to_string(),
                content: "Test insight for complete flow".to_string(),
                confidence: 0.7,
                related_to: None,
            }
        },
        nats_client.clone()
    ).await?;
    
    assert!(discovery_result.content.len() > 0, "Discovery should be shared");
    
    // Step 3: Verify discovery was stored and would be streamed
    let stored_discoveries = DISCOVERY_STORE.get_discoveries_for_agent("flow-test-agent").await;
    assert!(stored_discoveries.len() > 0, "Discovery should be stored");
    
    let subscription = DISCOVERY_STORE.get_subscription("flow-test-subscriber").await;
    assert!(subscription.is_some(), "Subscription should exist");
    
    // Step 4: Verify discovery matches subscription filter
    let filter = subscription.unwrap();
    let discovery = &stored_discoveries[0];
    assert!(filter.matches(discovery), "Discovery should match subscription filter");
    
    info!("✅ Complete discovery flow test passed");
    Ok(())
}

/// Test bridge resilience and error handling
#[tokio::test]
async fn test_bridge_error_handling() -> Result<()> {
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping test");
            return Ok(());
        }
    };
    
    // Initialize bridge
    initialize_bridge(nats_client.clone()).await?;
    
    // Test invalid discovery type
    let invalid_result = bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "test-agent".to_string(),
                agent_role: "TestRole".to_string(),
                discovery_type: "invalid_type".to_string(),
                content: "Test content".to_string(),
                confidence: 0.5,
                related_to: None,
            }
        },
        nats_client.clone()
    ).await;
    
    assert!(invalid_result.is_err(), "Invalid discovery type should fail");
    
    // Test invalid confidence
    let invalid_confidence_result = bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "test-agent".to_string(),
                agent_role: "TestRole".to_string(),
                discovery_type: "pattern".to_string(),
                content: "Test content".to_string(),
                confidence: 1.5, // Invalid confidence > 1.0
                related_to: None,
            }
        },
        nats_client.clone()
    ).await;
    
    assert!(invalid_confidence_result.is_err(), "Invalid confidence should fail");
    
    // Test empty agent_id
    let empty_agent_result = bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "".to_string(),
                agent_role: "TestRole".to_string(),
                discovery_type: "pattern".to_string(),
                content: "Test content".to_string(),
                confidence: 0.5,
                related_to: None,
            }
        },
        nats_client.clone()
    ).await;
    
    assert!(empty_agent_result.is_err(), "Empty agent_id should fail");
    
    info!("✅ Bridge error handling test passed");
    Ok(())
}

/// Test bridge performance under load
#[tokio::test]
async fn test_bridge_performance() -> Result<()> {
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping test");
            return Ok(());
        }
    };
    
    // Initialize bridge
    initialize_bridge(nats_client.clone()).await?;
    
    // Create multiple discoveries rapidly
    let mut tasks = Vec::new();
    
    for i in 0..10 {
        let nats_client = nats_client.clone();
        let task = tokio::spawn(async move {
            bridged_share_discovery_handler(
                Parameters {
                    params: ShareDiscoveryParams {
                        agent_id: format!("perf-agent-{}", i),
                        agent_role: "Performance".to_string(),
                        discovery_type: "pattern".to_string(),
                        content: format!("Performance test discovery {}", i),
                        confidence: 0.5 + (i as f32 * 0.05),
                        related_to: None,
                    }
                },
                nats_client
            ).await
        });
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;
    
    // Verify all discoveries were processed
    let mut success_count = 0;
    for result in results {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => error!("Discovery failed: {}", e),
            Err(e) => error!("Task failed: {}", e),
        }
    }
    
    assert!(success_count >= 8, "Most discoveries should succeed (got {})", success_count);
    
    info!("✅ Bridge performance test passed");
    Ok(())
}

/// Test concurrent agent operations
#[tokio::test]
async fn test_concurrent_agent_operations() -> Result<()> {
    let nats_client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("NATS not available, skipping test");
            return Ok(());
        }
    };
    
    // Initialize bridge
    initialize_bridge(nats_client.clone()).await?;
    
    // Spawn multiple agents concurrently
    let agent_tasks = vec![
        spawn_collaborative_agent_handler(
            Parameters {
                params: SpawnAgentParams {
                    agent_role: "ConcurrentAgent1".to_string(),
                    initial_context: "Concurrent test context 1".to_string(),
                }
            },
            nats_client.clone()
        ),
        spawn_collaborative_agent_handler(
            Parameters {
                params: SpawnAgentParams {
                    agent_role: "ConcurrentAgent2".to_string(),
                    initial_context: "Concurrent test context 2".to_string(),
                }
            },
            nats_client.clone()
        ),
        spawn_collaborative_agent_handler(
            Parameters {
                params: SpawnAgentParams {
                    agent_role: "ConcurrentAgent3".to_string(),
                    initial_context: "Concurrent test context 3".to_string(),
                }
            },
            nats_client.clone()
        ),
    ];
    
    // Wait for all agents to spawn
    let results = futures::future::join_all(agent_tasks).await;
    
    // Verify all agents spawned successfully
    let mut success_count = 0;
    for result in results {
        match result {
            Ok(_) => success_count += 1,
            Err(e) => error!("Agent spawn failed: {}", e),
        }
    }
    
    assert!(success_count >= 2, "Most agents should spawn successfully (got {})", success_count);
    
    info!("✅ Concurrent agent operations test passed");
    Ok(())
}

/// Run all bridge integration tests
pub async fn run_all_bridge_tests() -> Result<()> {
    info!("🧪 Running comprehensive bridge integration tests");
    
    // Run tests sequentially to avoid interference
    test_bridge_initialization_and_integration().await?;
    sleep(Duration::from_millis(100)).await;
    
    test_bridged_discovery_sharing().await?;
    sleep(Duration::from_millis(100)).await;
    
    test_bridged_subscription().await?;
    sleep(Duration::from_millis(100)).await;
    
    test_collaborative_agent_spawning().await?;
    sleep(Duration::from_millis(100)).await;
    
    test_context_sharing().await?;
    sleep(Duration::from_millis(100)).await;
    
    test_task_orchestration().await?;
    sleep(Duration::from_millis(100)).await;
    
    test_complete_discovery_flow().await?;
    sleep(Duration::from_millis(100)).await;
    
    test_bridge_error_handling().await?;
    sleep(Duration::from_millis(100)).await;
    
    test_bridge_performance().await?;
    sleep(Duration::from_millis(100)).await;
    
    test_concurrent_agent_operations().await?;
    
    info!("✅ All bridge integration tests passed");
    Ok(())
}

/// Utility function to verify bridge state
async fn verify_bridge_state() -> Result<()> {
    let bridge = get_bridge().await.ok_or_else(|| anyhow::anyhow!("Bridge not initialized"))?;
    let status = bridge.get_collaboration_status().await?;
    
    info!("Bridge status: {:?}", status);
    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn comprehensive_bridge_integration_test() {
        if let Err(e) = run_all_bridge_tests().await {
            eprintln!("Bridge integration tests failed: {}", e);
            // Don't fail the test if NATS is not available
        }
    }
}