//! Agent Pool Supervisor - Bridge between AgentPool and SupervisionTree
//!
//! This module implements the supervisor for the agent pool, enabling fault-tolerant
//! multi-agent orchestration by integrating the resource pool pattern with hierarchical
//! supervision.

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{info, warn, error, debug};
use async_trait::async_trait;

use crate::agent::{Agent, AgentId, AgentState};
use crate::agent::pool::{AgentPool, PoolError};
use crate::supervision::types::*;

/// Supervisor implementation for the agent pool
#[derive(Debug)]
pub struct AgentPoolSupervisor {
    /// Reference to the agent pool
    pool: Arc<AgentPool>,
    /// Supervision strategy (OneForOne for independent agents)
    supervision_strategy: SupervisionStrategy,
    /// Restart policy configuration
    restart_policy: RestartPolicy,
    /// Escalation policy configuration  
    escalation_policy: EscalationPolicy,
    /// Mapping from AgentId to ChildId for tracking
    agent_child_mapping: Arc<RwLock<HashMap<AgentId, ChildId>>>,
    /// Mapping from ChildId to AgentId for reverse lookup
    child_agent_mapping: Arc<RwLock<HashMap<ChildId, AgentId>>>,
    /// Child references managed by this supervisor
    children: Arc<RwLock<Vec<ChildRef>>>,
}

impl AgentPoolSupervisor {
    /// Create a new agent pool supervisor
    pub fn new(pool: Arc<AgentPool>) -> Self {
        info!("Creating AgentPoolSupervisor");
        
        Self {
            pool,
            supervision_strategy: SupervisionStrategy::OneForOne,
            restart_policy: RestartPolicy::exponential_backoff(),
            escalation_policy: EscalationPolicy::default(),
            agent_child_mapping: Arc::new(RwLock::new(HashMap::new())),
            child_agent_mapping: Arc::new(RwLock::new(HashMap::new())),
            children: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create a ChildRef from an Agent
    pub async fn create_child_ref(&self, agent: &Agent) -> Result<ChildRef> {
        let child_id = ChildId(format!("child-{}", agent.id));
        
        // Map agent state to child state
        let agent_state = agent.state.read().await.clone();
        let child_state = Self::map_agent_state(agent_state);
        
        let child_ref = ChildRef {
            id: child_id.clone(),
            node_type: NodeType::Worker("claude-cli-agent".to_string()),
            state: Arc::new(RwLock::new(child_state)),
        };
        
        // Update mappings
        {
            let mut agent_to_child = self.agent_child_mapping.write().await;
            agent_to_child.insert(agent.id.clone(), child_id.clone());
            
            let mut child_to_agent = self.child_agent_mapping.write().await;
            child_to_agent.insert(child_id, agent.id.clone());
        }
        
        Ok(child_ref)
    }
    
    /// Map AgentState to ChildState
    fn map_agent_state(agent_state: AgentState) -> ChildState {
        match agent_state {
            AgentState::Created | AgentState::Starting => ChildState::Starting,
            AgentState::Running | AgentState::Idle | AgentState::Processing => ChildState::Running,
            AgentState::Error => ChildState::Failed(FailureReason::UnhandledError("Agent error".to_string())),
            AgentState::Terminated | AgentState::Stopping => ChildState::Stopped,
            AgentState::Paused => ChildState::Running, // Paused agents are still technically running
        }
    }
    
    /// Register an agent with supervision
    pub async fn register_agent(&self, agent: Agent) -> Result<()> {
        info!(agent_id = %agent.id, "Registering agent with supervision");
        
        // Create child ref for the agent
        let child_ref = self.create_child_ref(&agent).await?;
        
        // Add to children list
        {
            let mut children = self.children.write().await;
            children.push(child_ref);
        }
        
        // Register with the pool (this handles semaphore/capacity)
        self.pool.register(agent).await?;
        
        info!("Agent registered with supervision successfully");
        Ok(())
    }
    
    /// Unregister an agent from supervision
    pub async fn unregister_agent(&self, agent_id: &AgentId) -> Result<()> {
        info!(agent_id = %agent_id, "Unregistering agent from supervision");
        
        // Find and remove child ref
        let child_id = {
            let mapping = self.agent_child_mapping.read().await;
            mapping.get(agent_id).cloned()
        };
        
        if let Some(child_id) = child_id {
            // Remove from children list
            {
                let mut children = self.children.write().await;
                children.retain(|c| c.id != child_id);
            }
            
            // Clean up mappings
            {
                let mut agent_to_child = self.agent_child_mapping.write().await;
                agent_to_child.remove(agent_id);
                
                let mut child_to_agent = self.child_agent_mapping.write().await;
                child_to_agent.remove(&child_id);
            }
        }
        
        // Unregister from the pool
        self.pool.unregister(agent_id).await?;
        
        info!("Agent unregistered from supervision successfully");
        Ok(())
    }
    
    /// Monitor agent states and update child states accordingly
    pub async fn sync_agent_states(&self) -> Result<()> {
        debug!("Synchronizing agent states with child states");
        
        let children = self.children.read().await;
        let child_to_agent = self.child_agent_mapping.read().await;
        
        for child in children.iter() {
            if let Some(agent_id) = child_to_agent.get(&child.id) {
                if let Some(agent) = self.pool.get(agent_id).await {
                    let agent_state = agent.state.read().await.clone();
                    let new_child_state = Self::map_agent_state(agent_state);
                    
                    let mut child_state = child.state.write().await;
                    *child_state = new_child_state;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle agent restart after failure
    async fn restart_agent(&self, agent_id: &AgentId) -> Result<()> {
        warn!(agent_id = %agent_id, "Attempting to restart failed agent");
        
        // TODO: Implement actual agent restart logic
        // This will involve:
        // 1. Creating a new Agent instance
        // 2. Starting the Claude CLI process
        // 3. Updating the pool
        // 4. Updating supervision mappings
        
        Ok(())
    }
}

#[async_trait]
impl Supervisor for AgentPoolSupervisor {
    type Child = ChildRef;
    
    async fn supervise(&self, children: Vec<Self::Child>) -> SupervisionResult {
        info!("Starting agent pool supervision");
        
        // Store children references
        {
            let mut stored_children = self.children.write().await;
            *stored_children = children;
        }
        
        // Start state synchronization loop
        // Note: In a real implementation, we would store the handle and properly manage this task
        // For now, we're not spawning a background task due to lifetime constraints
        // The sync_agent_states should be called by the supervision tree periodically
        
        SupervisionResult::Running
    }
    
    fn supervision_strategy(&self) -> SupervisionStrategy {
        self.supervision_strategy
    }
    
    fn restart_policy(&self) -> RestartPolicy {
        self.restart_policy.clone()
    }
    
    fn escalation_policy(&self) -> EscalationPolicy {
        self.escalation_policy.clone()
    }
    
    async fn route_task(&self, _task: Task) -> Option<ChildId> {
        // For now, simple round-robin routing to any available agent
        let children = self.children.read().await;
        
        // Find first available (Running) child
        for child in children.iter() {
            let state = child.state.read().await;
            if matches!(*state, ChildState::Running) {
                return Some(child.id.clone());
            }
        }
        
        None
    }
    
    async fn handle_child_failure(&self, child_id: ChildId, error: FailureReason) -> SupervisionDecision {
        error!(child_id = ?child_id, error = %error, "Handling child failure");
        
        // Get the corresponding agent ID
        let agent_id = {
            let mapping = self.child_agent_mapping.read().await;
            mapping.get(&child_id).cloned()
        };
        
        if let Some(agent_id) = agent_id {
            // Check restart policy
            // TODO: Track failure history and check if restart is allowed
            
            // For now, always attempt restart
            if let Err(e) = self.restart_agent(&agent_id).await {
                error!("Failed to restart agent: {}", e);
                return SupervisionDecision::Escalate(format!("Failed to restart agent: {}", e));
            }
            
            SupervisionDecision::Handled
        } else {
            warn!("Child failure for unknown agent");
            SupervisionDecision::Escalate("Unknown child".to_string())
        }
    }
}

/// Health check implementation for the agent pool supervisor
#[async_trait]
impl HealthCheck for AgentPoolSupervisor {
    async fn check_health(&self) -> HealthResult {
        let pool_status = self.pool.get_status().await;
        
        if pool_status.active_count == 0 && pool_status.max_capacity > 0 {
            HealthResult::Unhealthy("No active agents in pool".into())
        } else if pool_status.available_slots == 0 {
            HealthResult::Degraded("Pool at capacity".into())  
        } else {
            HealthResult::Healthy
        }
    }
}

#[cfg(test)]
#[path = "agent_pool_supervisor_test.rs"]
mod tests;