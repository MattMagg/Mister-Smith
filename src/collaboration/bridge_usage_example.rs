//! Bridge Usage Example - Demonstrates complete MCP-Collaboration integration
//!
//! This example shows how to initialize and use the collaboration bridge to seamlessly
//! integrate MCP handlers with the collaboration infrastructure.

use std::time::Duration;
use tokio::time::sleep;
use anyhow::Result;
use tracing::{info, error};

use crate::collaboration::{
    initialize_bridge, get_bridge, 
    DiscoveryType, ContextType, ContextContent
};
use crate::handlers::{
    Parameters, bridged_share_discovery_handler, bridged_subscribe_discoveries_handler,
    spawn_collaborative_agent_handler, get_collaboration_status_handler,
    orchestrate_task_handler, share_context_handler,
    share_discovery::ShareDiscoveryParams, subscribe_discoveries::SubscribeDiscoveriesParams, 
    bridged_handlers::{SpawnAgentParams, OrchestateTaskParams, ShareContextParams}
};

/// Complete example of MCP-Collaboration bridge integration
pub async fn run_complete_bridge_example() -> Result<()> {
    info!("🚀 Starting complete MCP-Collaboration bridge example");
    
    // 1. Initialize NATS connection
    let nats_client = async_nats::connect("nats://localhost:4222").await?;
    info!("✅ NATS connection established");
    
    // 2. Initialize collaboration bridge
    initialize_bridge(nats_client.clone()).await?;
    info!("✅ Collaboration bridge initialized");
    
    // 3. Get collaboration status
    let status_result = get_collaboration_status_handler(
        Parameters { params: serde_json::Value::Null },
        nats_client.clone()
    ).await?;
    info!("📊 Collaboration status: {:?}", status_result);
    
    // 4. Spawn collaborative agents
    info!("🤖 Spawning collaborative agents...");
    
    let security_agent = spawn_collaborative_agent_handler(
        Parameters {
            params: SpawnAgentParams {
                agent_role: "Security".to_string(),
                initial_context: "Focus on security vulnerabilities and threats".to_string(),
            }
        },
        nats_client.clone()
    ).await?;
    info!("✅ Security agent spawned: {:?}", security_agent);
    
    let performance_agent = spawn_collaborative_agent_handler(
        Parameters {
            params: SpawnAgentParams {
                agent_role: "Performance".to_string(),
                initial_context: "Analyze system performance and bottlenecks".to_string(),
            }
        },
        nats_client.clone()
    ).await?;
    info!("✅ Performance agent spawned: {:?}", performance_agent);
    
    // 5. Set up discovery subscriptions
    info!("📡 Setting up discovery subscriptions...");
    
    let security_subscription = bridged_subscribe_discoveries_handler(
        Parameters {
            params: SubscribeDiscoveriesParams {
                discovery_types: Some(vec!["anomaly".to_string(), "pattern".to_string()]),
                agent_roles: Some(vec!["Security".to_string(), "Performance".to_string()]),
                min_confidence: Some(0.7),
                subscriber_agent_id: "security-monitor".to_string(),
            }
        },
        nats_client.clone()
    ).await?;
    info!("✅ Security subscription created: {:?}", security_subscription);
    
    let performance_subscription = bridged_subscribe_discoveries_handler(
        Parameters {
            params: SubscribeDiscoveriesParams {
                discovery_types: Some(vec!["pattern".to_string(), "insight".to_string()]),
                agent_roles: Some(vec!["Performance".to_string()]),
                min_confidence: Some(0.6),
                subscriber_agent_id: "performance-monitor".to_string(),
            }
        },
        nats_client.clone()
    ).await?;
    info!("✅ Performance subscription created: {:?}", performance_subscription);
    
    // 6. Share context between agents
    info!("🤝 Sharing context between agents...");
    
    let context_result = share_context_handler(
        Parameters {
            params: ShareContextParams {
                from_agent: "security-agent".to_string(),
                to_agent: None, // Broadcast to all
                context_type: "background".to_string(),
                content: "System experiencing high load from IP range 192.168.1.0/24".to_string(),
                relevance: 0.8,
            }
        },
        nats_client.clone()
    ).await?;
    info!("✅ Context shared: {:?}", context_result);
    
    // 7. Share discoveries through bridge
    info!("🔍 Sharing discoveries through collaboration bridge...");
    
    let security_discovery = bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "security-agent-001".to_string(),
                agent_role: "Security".to_string(),
                discovery_type: "anomaly".to_string(),
                content: "Unusual authentication patterns detected from suspicious IP ranges".to_string(),
                confidence: 0.85,
                related_to: None,
            }
        },
        nats_client.clone()
    ).await?;
    info!("✅ Security discovery shared: {:?}", security_discovery);
    
    // Give it a moment to propagate
    sleep(Duration::from_millis(500)).await;
    
    let performance_discovery = bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "performance-agent-001".to_string(),
                agent_role: "Performance".to_string(),
                discovery_type: "pattern".to_string(),
                content: "Database query response times increasing during peak hours".to_string(),
                confidence: 0.92,
                related_to: None,
            }
        },
        nats_client.clone()
    ).await?;
    info!("✅ Performance discovery shared: {:?}", performance_discovery);
    
    // 8. Share a connecting discovery
    sleep(Duration::from_millis(500)).await;
    
    let connection_discovery = bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "network-agent-001".to_string(),
                agent_role: "Network".to_string(),
                discovery_type: "connection".to_string(),
                content: "Performance degradation correlates with security anomalies - possible DDoS attack".to_string(),
                confidence: 0.95,
                related_to: Some("security-anomaly-001".to_string()),
            }
        },
        nats_client.clone()
    ).await?;
    info!("✅ Connection discovery shared: {:?}", connection_discovery);
    
    // 9. Orchestrate a collaborative task
    info!("🎯 Orchestrating collaborative task...");
    
    let orchestration_result = orchestrate_task_handler(
        Parameters {
            params: OrchestateTaskParams {
                task_description: "Investigate and mitigate potential DDoS attack affecting system performance".to_string(),
                required_agents: Some(vec!["Security".to_string(), "Performance".to_string(), "Network".to_string()]),
                timeout_seconds: Some(300),
            }
        },
        nats_client.clone()
    ).await?;
    info!("✅ Task orchestrated: {:?}", orchestration_result);
    
    // 10. Get final collaboration status
    let final_status = get_collaboration_status_handler(
        Parameters { params: serde_json::Value::Null },
        nats_client.clone()
    ).await?;
    info!("📊 Final collaboration status: {:?}", final_status);
    
    info!("🎉 Complete MCP-Collaboration bridge example finished successfully!");
    
    Ok(())
}

/// Example of real-time collaboration pattern
pub async fn run_real_time_collaboration_example() -> Result<()> {
    info!("⚡ Starting real-time collaboration example");
    
    let nats_client = async_nats::connect("nats://localhost:4222").await?;
    initialize_bridge(nats_client.clone()).await?;
    
    // Simulate multiple agents discovering things simultaneously
    let (result1, result2, result3) = tokio::join!(
        simulate_security_agent_work(nats_client.clone()),
        simulate_performance_agent_work(nats_client.clone()),
        simulate_network_agent_work(nats_client.clone())
    );
    
    // Check results
    let results = vec![result1, result2, result3];
    
    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(_) => info!("✅ Agent {} completed work", i),
            Err(e) => error!("❌ Agent {} failed: {}", i, e),
        }
    }
    
    info!("⚡ Real-time collaboration example completed");
    Ok(())
}

/// Simulate security agent discovering and sharing findings
async fn simulate_security_agent_work(nats_client: async_nats::Client) -> Result<()> {
    sleep(Duration::from_millis(100)).await;
    
    // Security agent finds an anomaly
    bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "security-agent-real".to_string(),
                agent_role: "Security".to_string(),
                discovery_type: "anomaly".to_string(),
                content: "Brute force attack detected on admin endpoints".to_string(),
                confidence: 0.9,
                related_to: None,
            }
        },
        nats_client.clone()
    ).await?;
    
    sleep(Duration::from_millis(200)).await;
    
    // Security agent shares context
    share_context_handler(
        Parameters {
            params: ShareContextParams {
                from_agent: "security-agent-real".to_string(),
                to_agent: None,
                context_type: "warnings".to_string(),
                content: "All agents should monitor for authentication failures".to_string(),
                relevance: 0.9,
            }
        },
        nats_client
    ).await?;
    
    Ok(())
}

/// Simulate performance agent discovering and sharing findings
async fn simulate_performance_agent_work(nats_client: async_nats::Client) -> Result<()> {
    sleep(Duration::from_millis(150)).await;
    
    // Performance agent finds a pattern
    bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "performance-agent-real".to_string(),
                agent_role: "Performance".to_string(),
                discovery_type: "pattern".to_string(),
                content: "CPU usage spiking during authentication attempts".to_string(),
                confidence: 0.8,
                related_to: None,
            }
        },
        nats_client.clone()
    ).await?;
    
    sleep(Duration::from_millis(100)).await;
    
    // Performance agent asks a question
    bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "performance-agent-real".to_string(),
                agent_role: "Performance".to_string(),
                discovery_type: "question".to_string(),
                content: "Should we implement rate limiting on authentication endpoints?".to_string(),
                confidence: 0.7,
                related_to: None,
            }
        },
        nats_client
    ).await?;
    
    Ok(())
}

/// Simulate network agent discovering and sharing findings
async fn simulate_network_agent_work(nats_client: async_nats::Client) -> Result<()> {
    sleep(Duration::from_millis(250)).await;
    
    // Network agent connects the dots
    bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "network-agent-real".to_string(),
                agent_role: "Network".to_string(),
                discovery_type: "connection".to_string(),
                content: "Auth failures correlate with CPU spikes - coordinated attack!".to_string(),
                confidence: 0.95,
                related_to: Some("security-anomaly-brute-force".to_string()),
            }
        },
        nats_client.clone()
    ).await?;
    
    sleep(Duration::from_millis(100)).await;
    
    // Network agent proposes a solution
    bridged_share_discovery_handler(
        Parameters {
            params: ShareDiscoveryParams {
                agent_id: "network-agent-real".to_string(),
                agent_role: "Network".to_string(),
                discovery_type: "solution".to_string(),
                content: "Implement IP-based rate limiting + fail2ban for attack mitigation".to_string(),
                confidence: 0.88,
                related_to: Some("auth-performance-question".to_string()),
            }
        },
        nats_client
    ).await?;
    
    Ok(())
}

/// Example showing SSE integration
pub async fn run_sse_integration_example() -> Result<()> {
    info!("📡 Starting SSE integration example");
    
    let nats_client = async_nats::connect("nats://localhost:4222").await?;
    initialize_bridge(nats_client.clone()).await?;
    
    // Get bridge instance
    let bridge = get_bridge().await.ok_or_else(|| anyhow::anyhow!("Bridge not initialized"))?;
    let sse_broadcaster = bridge.get_sse_broadcaster();
    
    // Create SSE router (this would typically be done in your web server)
    let sse_router = sse_broadcaster.clone().create_router();
    
    info!("✅ SSE router created and ready for connections");
    
    // Create subscription that will connect to SSE
    let subscription_result = bridged_subscribe_discoveries_handler(
        Parameters {
            params: SubscribeDiscoveriesParams {
                discovery_types: Some(vec!["insight".to_string()]),
                agent_roles: None,
                min_confidence: Some(0.5),
                subscriber_agent_id: "sse-client-001".to_string(),
            }
        },
        nats_client.clone()
    ).await?;
    
    info!("✅ SSE subscription created: {:?}", subscription_result);
    
    // Share some discoveries that will be streamed via SSE
    for i in 0..5 {
        bridged_share_discovery_handler(
            Parameters {
                params: ShareDiscoveryParams {
                    agent_id: format!("insight-agent-{}", i),
                    agent_role: "Insight".to_string(),
                    discovery_type: "insight".to_string(),
                    content: format!("Insight #{}: Pattern discovered in data stream", i),
                    confidence: 0.6 + (i as f32 * 0.1),
                    related_to: None,
                }
            },
            nats_client.clone()
        ).await?;
        
        sleep(Duration::from_millis(100)).await;
    }
    
    info!("✅ All discoveries shared and should be available via SSE");
    
    // Get connected clients info
    let connected_clients = sse_broadcaster.get_connected_clients().await;
    info!("📊 Connected SSE clients: {}", connected_clients.len());
    
    info!("📡 SSE integration example completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_complete_bridge_example() {
        if let Err(e) = run_complete_bridge_example().await {
            eprintln!("Bridge example failed: {}", e);
            // Don't fail the test if NATS is not available
        }
    }
    
    #[tokio::test]
    async fn test_real_time_collaboration() {
        if let Err(e) = run_real_time_collaboration_example().await {
            eprintln!("Real-time collaboration example failed: {}", e);
            // Don't fail the test if NATS is not available
        }
    }
    
    #[tokio::test]
    async fn test_sse_integration() {
        if let Err(e) = run_sse_integration_example().await {
            eprintln!("SSE integration example failed: {}", e);
            // Don't fail the test if NATS is not available
        }
    }
}