//! Integration tests for AgentPool + SupervisionTree
//!
//! Tests the Phase 3 integration between agent pool management
//! and hierarchical supervision.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

use crate::agent::{Agent, AgentConfig, AgentPool};
use crate::supervision::{
    AgentPoolSupervisor, SupervisionTree, SupervisionTreeConfig,
    SupervisorNode, NodeId, NodeType,
};

#[tokio::test]
async fn test_agent_pool_supervisor_basic_integration() {
    // Create agent pool
    let pool = Arc::new(AgentPool::new(5));
    
    // Create pool supervisor
    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(pool.clone()));
    
    // Create supervision tree
    let tree_config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(tree_config);
    
    // Create supervisor node
    let supervisor_node = SupervisorNode::new(
        NodeId("agent-pool-supervisor".to_string()),
        NodeType::Service("agent-pool".to_string()),
        None,
        pool_supervisor.clone(),
    );
    
    // Set as root (for simplicity in this test)
    tree.set_root(Arc::new(supervisor_node)).await
        .expect("Failed to set root supervisor");
    
    // Start supervision tree
    tree.start().await.expect("Failed to start supervision tree");
    
    info!("Supervision tree started with agent pool supervisor");
    
    // Create and register an agent
    let agent_config = AgentConfig::default();
    let agent = Agent::new(agent_config);
    let agent_id = agent.id.clone();
    
    // Register agent with supervised pool
    pool_supervisor.register_agent(agent).await
        .expect("Failed to register agent");
    
    info!("Agent {} registered with supervised pool", agent_id);
    
    // Verify pool status
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 1);
    assert_eq!(status.max_capacity, 5);
    assert_eq!(status.available_slots, 4);
    
    // Test health check
    use crate::supervision::HealthCheck;
    let health = pool_supervisor.check_health().await;
    match health {
        crate::supervision::HealthResult::Healthy => {
            info!("Pool supervisor is healthy");
        }
        _ => panic!("Expected healthy pool supervisor"),
    }
    
    // Unregister agent
    pool_supervisor.unregister_agent(&agent_id).await
        .expect("Failed to unregister agent");
    
    // Verify pool is empty
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 0);
    
    // Stop supervision tree
    tree.stop().await.expect("Failed to stop supervision tree");
    
    info!("Test completed successfully");
}

#[tokio::test]
async fn test_agent_state_synchronization() {
    // Create agent pool
    let pool = Arc::new(AgentPool::new(3));
    
    // Create pool supervisor
    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(pool.clone()));
    
    // Create and register multiple agents
    let mut agent_ids = Vec::new();
    
    for i in 0..3 {
        let mut config = AgentConfig::default();
        config.name = format!("test-agent-{}", i);
        
        let agent = Agent::new(config);
        let agent_id = agent.id.clone();
        agent_ids.push(agent_id.clone());
        
        pool_supervisor.register_agent(agent).await
            .expect("Failed to register agent");
    }
    
    info!("Registered {} agents", agent_ids.len());
    
    // Verify all agents are tracked
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 3);
    assert_eq!(status.agents.len(), 3);
    
    // Test state synchronization
    pool_supervisor.sync_agent_states().await
        .expect("Failed to sync agent states");
    
    info!("State synchronization completed");
    
    // Clean up
    for agent_id in &agent_ids {
        pool_supervisor.unregister_agent(agent_id).await
            .expect("Failed to unregister agent");
    }
    
    // Verify pool is empty
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 0);
}

#[tokio::test]
async fn test_pool_capacity_limits_with_supervision() {
    // Create small pool for testing limits
    let pool = Arc::new(AgentPool::new(2));
    
    // Create pool supervisor
    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(pool.clone()));
    
    // Register agents up to capacity
    let agent1 = Agent::new(AgentConfig::default());
    let agent2 = Agent::new(AgentConfig::default());
    
    pool_supervisor.register_agent(agent1).await
        .expect("Failed to register first agent");
    pool_supervisor.register_agent(agent2).await
        .expect("Failed to register second agent");
    
    // Verify pool is at capacity
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 2);
    assert_eq!(status.available_slots, 0);
    
    // Try to register beyond capacity (should block/timeout)
    let agent3 = Agent::new(AgentConfig::default());
    
    let register_result = tokio::time::timeout(
        Duration::from_millis(100),
        pool_supervisor.register_agent(agent3)
    ).await;
    
    // Should timeout since pool is full
    assert!(register_result.is_err(), "Expected timeout when pool is full");
    
    info!("Pool capacity limits working correctly with supervision");
}

#[cfg(test)]
mod test_helpers {
    use super::*;
    
    /// Helper to create a supervised agent pool setup
    pub async fn create_supervised_pool(capacity: usize) -> (Arc<AgentPool>, Arc<AgentPoolSupervisor>) {
        let pool = Arc::new(AgentPool::new(capacity));
        let supervisor = Arc::new(AgentPoolSupervisor::new(pool.clone()));
        (pool, supervisor)
    }
}