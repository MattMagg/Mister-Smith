//! Agent Module Tests
//!
//! Test suites for agent lifecycle, state management, and pool operations.

#[cfg(test)]
mod test_agent_lifecycle {
    use crate::agent::{Agent, AgentConfig, AgentState};
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_agent_creation() {
        let config = AgentConfig::default();
        let agent = Agent::new(config.clone());
        
        // Verify initial state
        assert_eq!(agent.get_state().await, AgentState::Created);
        assert!(!agent.id.uuid().is_nil());
        assert_eq!(agent.config.model, config.model);
    }

    #[tokio::test]
    async fn test_agent_state_transitions() {
        let agent = Agent::new(AgentConfig::default());
        
        // Test Created -> Starting -> Running
        assert_eq!(agent.get_state().await, AgentState::Created);
        
        agent.set_state(AgentState::Starting).await;
        assert_eq!(agent.get_state().await, AgentState::Starting);
        
        agent.set_state(AgentState::Running).await;
        assert_eq!(agent.get_state().await, AgentState::Running);
        assert!(agent.is_active().await);
        
        // Test Running -> Stopping -> Terminated
        agent.set_state(AgentState::Stopping).await;
        assert_eq!(agent.get_state().await, AgentState::Stopping);
        
        agent.set_state(AgentState::Terminated).await;
        assert_eq!(agent.get_state().await, AgentState::Terminated);
        assert!(!agent.is_active().await);
    }

    #[tokio::test]
    async fn test_agent_uptime() {
        let agent = Agent::new(AgentConfig::default());
        
        // Initial uptime should be near zero
        let initial_uptime = agent.uptime();
        assert!(initial_uptime.as_millis() < 100);
        
        // Wait and check uptime increased
        sleep(Duration::from_millis(100)).await;
        let later_uptime = agent.uptime();
        assert!(later_uptime.as_millis() >= 100);
    }
}

#[cfg(test)]
mod test_agent_pool {
    use crate::agent::{Agent, AgentConfig, AgentPool};
    
    #[tokio::test]
    async fn test_pool_capacity() {
        let pool = AgentPool::new(2);
        
        // Check initial status
        let status = pool.get_status().await;
        assert_eq!(status.max_capacity, 2);
        assert_eq!(status.active_count, 0);
        assert_eq!(status.available_slots, 2);
        
        // Register agents
        let agent1 = Agent::new(AgentConfig::default());
        let agent2 = Agent::new(AgentConfig::default());
        
        let _permit1 = pool.register(agent1).await.unwrap();
        let _permit2 = pool.register(agent2).await.unwrap();
        
        // Check pool is full
        let status = pool.get_status().await;
        assert_eq!(status.active_count, 2);
        assert_eq!(status.available_slots, 0);
        assert!(!pool.has_capacity());
    }
    
    #[tokio::test]
    async fn test_pool_agent_retrieval() {
        let pool = AgentPool::new(5);
        let agent = Agent::new(AgentConfig::default());
        let agent_id = agent.id.clone();
        
        // Register agent
        let _permit = pool.register(agent).await.unwrap();
        
        // Retrieve agent
        let retrieved = pool.get(&agent_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, agent_id);
        
        // Unregister agent
        pool.unregister(&agent_id).await.unwrap();
        
        // Verify agent no longer in pool
        let retrieved = pool.get(&agent_id).await;
        assert!(retrieved.is_none());
    }
}