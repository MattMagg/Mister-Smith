//! Unit tests for AgentPoolSupervisor

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::sync::Arc;
    use crate::agent::{Agent, AgentConfig, AgentPool, AgentState};
    
    #[tokio::test]
    async fn test_agent_pool_supervisor_creation() {
        // Create agent pool
        let pool = Arc::new(AgentPool::new(5));
        
        // Create pool supervisor
        let supervisor = AgentPoolSupervisor::new(pool.clone());
        
        // Verify supervisor properties
        assert!(matches!(supervisor.supervision_strategy(), SupervisionStrategy::OneForOne));
        
        // Verify restart policy
        let policy = supervisor.restart_policy();
        assert_eq!(policy.max_restarts, 5);
    }
    
    #[test]
    fn test_state_mapping() {
        // Test all state mappings
        assert!(matches!(
            AgentPoolSupervisor::map_agent_state(AgentState::Created),
            ChildState::Starting
        ));
        
        assert!(matches!(
            AgentPoolSupervisor::map_agent_state(AgentState::Starting),
            ChildState::Starting
        ));
        
        assert!(matches!(
            AgentPoolSupervisor::map_agent_state(AgentState::Running),
            ChildState::Running
        ));
        
        assert!(matches!(
            AgentPoolSupervisor::map_agent_state(AgentState::Idle),
            ChildState::Running
        ));
        
        assert!(matches!(
            AgentPoolSupervisor::map_agent_state(AgentState::Processing),
            ChildState::Running
        ));
        
        assert!(matches!(
            AgentPoolSupervisor::map_agent_state(AgentState::Error),
            ChildState::Failed(_)
        ));
        
        assert!(matches!(
            AgentPoolSupervisor::map_agent_state(AgentState::Terminated),
            ChildState::Stopped
        ));
        
        assert!(matches!(
            AgentPoolSupervisor::map_agent_state(AgentState::Stopping),
            ChildState::Stopped
        ));
        
        assert!(matches!(
            AgentPoolSupervisor::map_agent_state(AgentState::Paused),
            ChildState::Running
        ));
    }
    
    #[tokio::test]
    async fn test_register_unregister_agent() {
        // Create pool and supervisor
        let pool = Arc::new(AgentPool::new(3));
        let supervisor = AgentPoolSupervisor::new(pool.clone());
        
        // Create an agent
        let agent = Agent::new(AgentConfig::default());
        let agent_id = agent.id.clone();
        
        // Register agent
        supervisor.register_agent(agent).await
            .expect("Failed to register agent");
        
        // Verify pool contains the agent
        let status = pool.get_status().await;
        assert_eq!(status.active_count, 1);
        assert!(status.agents.contains(&agent_id));
        
        // Unregister agent
        supervisor.unregister_agent(&agent_id).await
            .expect("Failed to unregister agent");
        
        // Verify agent removed
        let status = pool.get_status().await;
        assert_eq!(status.active_count, 0);
        assert!(!status.agents.contains(&agent_id));
    }
    
    #[tokio::test]
    async fn test_health_check() {
        use crate::supervision::HealthCheck;
        
        // Test empty pool
        let pool = Arc::new(AgentPool::new(5));
        let supervisor = AgentPoolSupervisor::new(pool.clone());
        
        let health = supervisor.check_health().await;
        assert!(matches!(health, HealthResult::Unhealthy(_)));
        
        // Add an agent
        let agent = Agent::new(AgentConfig::default());
        supervisor.register_agent(agent).await
            .expect("Failed to register agent");
        
        // Should be healthy now
        let health = supervisor.check_health().await;
        assert!(matches!(health, HealthResult::Healthy));
        
        // Fill pool to capacity
        for _ in 1..5 {
            let agent = Agent::new(AgentConfig::default());
            supervisor.register_agent(agent).await
                .expect("Failed to register agent");
        }
        
        // Should be degraded at capacity
        let health = supervisor.check_health().await;
        assert!(matches!(health, HealthResult::Degraded(_)));
    }
    
    #[tokio::test]
    async fn test_create_child_ref() {
        let pool = Arc::new(AgentPool::new(2));
        let supervisor = AgentPoolSupervisor::new(pool.clone());
        
        // Create an agent
        let agent = Agent::new(AgentConfig::default());
        let agent_id = agent.id.clone();
        
        // Create child ref
        let child_ref = supervisor.create_child_ref(&agent).await
            .expect("Failed to create child ref");
        
        // Verify child ref properties
        assert_eq!(child_ref.id.0, format!("child-{}", agent_id));
        assert!(matches!(child_ref.node_type, NodeType::Worker(_)));
        
        // Verify state mapping
        let state = child_ref.state.read().await;
        assert!(matches!(*state, ChildState::Starting));
    }
}