//! Supervisor node implementation
//!
//! This module implements individual supervisor nodes that manage child processes
//! and apply supervision strategies.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{sleep, timeout};
use tracing::{info, warn, error, debug};
use anyhow::Result;

use crate::supervision::types::*;

/// Individual supervisor node in the supervision tree
#[derive(Debug)]
pub struct SupervisorNode {
    /// Unique identifier for this node
    node_id: NodeId,
    /// Type of this node
    node_type: NodeType,
    /// Parent supervisor ID (None for root)
    parent_id: Option<NodeId>,
    /// Children managed by this supervisor
    children: Arc<RwLock<Vec<ChildRef>>>,
    /// Reference to the actual supervisor implementation
    supervisor: Arc<dyn Supervisor<Child = ChildRef> + Send + Sync>,
    /// Current state of this node
    state: Arc<Mutex<SupervisorNodeState>>,
    /// Metrics for this node
    metrics: Arc<RwLock<SupervisionMetrics>>,
    /// Failure history for restart decisions
    failure_history: Arc<RwLock<Vec<FailureRecord>>>,
}

impl SupervisorNode {
    /// Create a new supervisor node
    pub fn new(
        node_id: NodeId,
        node_type: NodeType,
        parent_id: Option<NodeId>,
        supervisor: Arc<dyn Supervisor<Child = ChildRef> + Send + Sync>,
    ) -> Self {
        Self {
            node_id,
            node_type,
            parent_id,
            children: Arc::new(RwLock::new(Vec::new())),
            supervisor,
            state: Arc::new(Mutex::new(SupervisorNodeState::Created)),
            metrics: Arc::new(RwLock::new(SupervisionMetrics::default())),
            failure_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the node ID
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get the parent ID
    pub fn parent_id(&self) -> &Option<NodeId> {
        &self.parent_id
    }

    /// Start supervising
    pub async fn start(&self) -> Result<()> {
        info!("Starting supervisor node: {}", self.node_id);
        
        *self.state.lock().await = SupervisorNodeState::Starting;
        
        // Initialize children if any
        let children = self.children.read().await.clone();
        
        // Start supervision
        let result = self.supervisor.supervise(children).await;
        
        match result {
            SupervisionResult::Running => {
                *self.state.lock().await = SupervisorNodeState::Active;
                info!("Supervisor node {} started successfully", self.node_id);
                Ok(())
            }
            SupervisionResult::Failed(err) => {
                *self.state.lock().await = SupervisorNodeState::Failed;
                error!("Supervisor node {} failed to start: {}", self.node_id, err);
                Err(SupervisionError::InitializationFailed(err).into())
            }
            SupervisionResult::Stopped => {
                *self.state.lock().await = SupervisorNodeState::Stopped;
                Ok(())
            }
        }
    }

    /// Stop supervising
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping supervisor node: {}", self.node_id);
        
        *self.state.lock().await = SupervisorNodeState::Stopping;
        
        // Stop all children
        let children = self.children.read().await.clone();
        for child in children {
            self.stop_child(&child.id).await?;
        }
        
        *self.state.lock().await = SupervisorNodeState::Stopped;
        Ok(())
    }

    /// Add a child to supervise
    pub async fn add_child(&self, child: ChildRef) -> Result<()> {
        debug!("Adding child {} to supervisor {}", child.id.0, self.node_id);
        
        self.children.write().await.push(child);
        self.metrics.write().await.child_count += 1;
        
        Ok(())
    }

    /// Remove a child
    pub async fn remove_child(&self, child_id: &ChildId) -> Result<()> {
        debug!("Removing child {} from supervisor {}", child_id.0, self.node_id);
        
        let mut children = self.children.write().await;
        children.retain(|c| c.id != *child_id);
        self.metrics.write().await.child_count = children.len();
        
        Ok(())
    }

    /// Handle a child failure
    pub async fn handle_child_failure(
        &self,
        child_id: ChildId,
        error: FailureReason,
    ) -> Result<SupervisionDecision> {
        error!("Child {} failed: {:?}", child_id.0, error);
        
        // Record failure
        self.record_failure(error.clone()).await;
        
        // Let supervisor decide
        let decision = self.supervisor.handle_child_failure(child_id.clone(), error.clone()).await;
        
        // Apply supervision strategy
        match decision {
            SupervisionDecision::Handled => {
                let strategy = self.supervisor.supervision_strategy();
                self.apply_supervision_strategy(strategy, child_id, error).await?;
            }
            _ => {}
        }
        
        Ok(decision)
    }

    /// Apply supervision strategy after a child failure
    async fn apply_supervision_strategy(
        &self,
        strategy: SupervisionStrategy,
        failed_child_id: ChildId,
        _error: FailureReason,
    ) -> Result<()> {
        match strategy {
            SupervisionStrategy::OneForOne => {
                // Restart only the failed child
                info!("Applying OneForOne strategy for child {}", failed_child_id.0);
                self.restart_child(&failed_child_id).await?;
            }
            SupervisionStrategy::OneForAll => {
                // Restart all children
                info!("Applying OneForAll strategy - restarting all children");
                self.restart_all_children().await?;
            }
            SupervisionStrategy::RestForOne => {
                // Restart failed child and all started after it
                info!("Applying RestForOne strategy for child {}", failed_child_id.0);
                self.restart_child_and_siblings(&failed_child_id).await?;
            }
            SupervisionStrategy::Escalate => {
                // This is handled by the decision itself
                info!("Escalating failure to parent");
            }
        }
        
        Ok(())
    }

    /// Restart a single child
    async fn restart_child(&self, child_id: &ChildId) -> Result<()> {
        let children = self.children.read().await;
        
        if let Some(child_ref) = children.iter().find(|c| c.id == *child_id) {
            let restart_policy = self.supervisor.restart_policy();
            
            // Check if restart is allowed
            let history = self.failure_history.read().await;
            if !restart_policy.should_restart(&history) {
                error!("Restart limit exceeded for child {}", child_id.0);
                return Err(SupervisionError::RestartLimitExceeded {
                    node_id: self.node_id.clone(),
                }
                .into());
            }
            
            // Calculate restart delay
            let attempt = history.len() as u32;
            let delay = restart_policy.calculate_delay(attempt);
            
            info!("Restarting child {} after {:?} delay", child_id.0, delay);
            sleep(delay).await;
            
            // Stop the old child
            self.stop_child_internal(child_ref).await?;
            
            // Start new child (simplified - in real implementation would create new process)
            *child_ref.state.write().await = ChildState::Starting;
            
            // Simulate restart
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            
            *child_ref.state.write().await = ChildState::Running;
            
            // Update metrics
            self.metrics.write().await.restarts_total += 1;
            
            info!("Child {} restarted successfully", child_id.0);
        } else {
            warn!("Child {} not found for restart", child_id.0);
        }
        
        Ok(())
    }

    /// Restart all children
    async fn restart_all_children(&self) -> Result<()> {
        let children = self.children.read().await.clone();
        
        info!("Restarting all {} children", children.len());
        
        // Stop all children
        for child in &children {
            self.stop_child_internal(&child).await?;
        }
        
        // Apply restart delay
        let restart_policy = self.supervisor.restart_policy();
        let delay = restart_policy.calculate_delay(0);
        sleep(delay).await;
        
        // Start all children
        for child in &children {
            *child.state.write().await = ChildState::Starting;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            *child.state.write().await = ChildState::Running;
            
            self.metrics.write().await.restarts_total += 1;
        }
        
        info!("All children restarted successfully");
        Ok(())
    }

    /// Restart failed child and siblings started after it
    async fn restart_child_and_siblings(&self, failed_child_id: &ChildId) -> Result<()> {
        let children = self.children.read().await.clone();
        
        // Find position of failed child
        let failed_index = children
            .iter()
            .position(|c| c.id == *failed_child_id)
            .ok_or_else(|| {
                SupervisionError::OperationFailed(format!("Child {} not found", failed_child_id.0))
            })?;
        
        info!(
            "Restarting child {} and {} younger siblings",
            failed_child_id.0,
            children.len() - failed_index - 1
        );
        
        // Restart failed child and all after it
        for i in failed_index..children.len() {
            let child = &children[i];
            self.stop_child_internal(child).await?;
            
            // Apply restart delay
            let restart_policy = self.supervisor.restart_policy();
            let delay = restart_policy.calculate_delay(0);
            sleep(delay).await;
            
            *child.state.write().await = ChildState::Starting;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            *child.state.write().await = ChildState::Running;
            
            self.metrics.write().await.restarts_total += 1;
        }
        
        Ok(())
    }

    /// Stop a child process
    async fn stop_child(&self, child_id: &ChildId) -> Result<()> {
        let children = self.children.read().await;
        
        if let Some(child_ref) = children.iter().find(|c| c.id == *child_id) {
            self.stop_child_internal(child_ref).await?;
        }
        
        Ok(())
    }

    /// Internal method to stop a child
    async fn stop_child_internal(&self, child_ref: &ChildRef) -> Result<()> {
        debug!("Stopping child {}", child_ref.id.0);
        
        *child_ref.state.write().await = ChildState::Stopped;
        
        Ok(())
    }

    /// Route a task to appropriate child
    pub async fn route_task(&self, task: Task) -> Option<ChildId> {
        self.supervisor.route_task(task).await
    }

    /// Get children
    pub async fn get_children(&self) -> Vec<ChildRef> {
        self.children.read().await.clone()
    }

    /// Get metrics
    pub async fn get_metrics(&self) -> SupervisionMetrics {
        self.metrics.read().await.clone()
    }

    /// Record a failure
    async fn record_failure(&self, reason: FailureReason) {
        let mut history = self.failure_history.write().await;
        let mut metrics = self.metrics.write().await;
        
        history.push(FailureRecord {
            timestamp: Instant::now(),
            reason,
        });
        
        metrics.failures_total += 1;
        metrics.last_failure_time = Some(Instant::now());
        
        // Keep history bounded
        if history.len() > 100 {
            history.remove(0);
        }
    }
}

/// State of a supervisor node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SupervisorNodeState {
    Created,
    Starting,
    Active,
    Stopping,
    Stopped,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Test implementation of Supervisor trait for testing
    #[derive(Debug)]
    struct TestSupervisor {
        strategy: SupervisionStrategy,
        restart_policy: RestartPolicy,
    }
    
    #[async_trait::async_trait]
    impl Supervisor for TestSupervisor {
        type Child = ChildRef;
        
        async fn supervise(&self, _children: Vec<Self::Child>) -> SupervisionResult {
            SupervisionResult::Running
        }
        
        fn supervision_strategy(&self) -> SupervisionStrategy {
            self.strategy
        }
        
        fn restart_policy(&self) -> RestartPolicy {
            self.restart_policy.clone()
        }
        
        fn escalation_policy(&self) -> EscalationPolicy {
            EscalationPolicy::default()
        }
        
        async fn route_task(&self, _task: Task) -> Option<ChildId> {
            None
        }
        
        async fn handle_child_failure(
            &self,
            _child_id: ChildId,
            _error: FailureReason,
        ) -> SupervisionDecision {
            SupervisionDecision::Handled
        }
    }
    
    #[tokio::test]
    async fn test_supervisor_node_creation() {
        let supervisor = Arc::new(TestSupervisor {
            strategy: SupervisionStrategy::OneForOne,
            restart_policy: RestartPolicy::exponential_backoff(),
        });
        
        let node = SupervisorNode::new(
            NodeId("test-node".to_string()),
            NodeType::AgentSupervisor("test".to_string()),
            None,
            supervisor,
        );
        
        assert_eq!(node.node_id().0, "test-node");
        assert!(node.parent_id().is_none());
    }
}