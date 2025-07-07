//! Concrete supervisor implementations
//!
//! This module provides ready-to-use supervisor implementations for common patterns.

use std::sync::Arc;
use async_trait::async_trait;
use tracing::{info, debug, error};

use crate::supervision::types::*;

/// Root supervisor at the top of the supervision tree
#[derive(Debug)]
pub struct RootSupervisor {
    strategy: SupervisionStrategy,
    restart_policy: RestartPolicy,
    escalation_policy: EscalationPolicy,
}

impl RootSupervisor {
    /// Create a new root supervisor
    pub fn new(strategy: SupervisionStrategy) -> Self {
        Self {
            strategy,
            restart_policy: RestartPolicy::exponential_backoff(),
            escalation_policy: EscalationPolicy::default(),
        }
    }

    /// Create with custom policies
    pub fn with_policies(
        strategy: SupervisionStrategy,
        restart_policy: RestartPolicy,
        escalation_policy: EscalationPolicy,
    ) -> Self {
        Self {
            strategy,
            restart_policy,
            escalation_policy,
        }
    }
}

#[async_trait]
impl Supervisor for RootSupervisor {
    type Child = ChildRef;

    async fn supervise(&self, children: Vec<Self::Child>) -> SupervisionResult {
        info!("Root supervisor starting with {} children", children.len());
        
        // In a real implementation, this would manage the lifecycle of children
        // For now, we just mark as running
        SupervisionResult::Running
    }

    fn supervision_strategy(&self) -> SupervisionStrategy {
        self.strategy
    }

    fn restart_policy(&self) -> RestartPolicy {
        self.restart_policy.clone()
    }

    fn escalation_policy(&self) -> EscalationPolicy {
        self.escalation_policy.clone()
    }

    async fn route_task(&self, task: Task) -> Option<ChildId> {
        // Root supervisor routes to appropriate child supervisor based on task type
        debug!("Root supervisor routing task: {:?}", task.task_type);
        
        match task.task_type {
            TaskType::Research => Some(ChildId("research-supervisor".to_string())),
            TaskType::Code => Some(ChildId("code-supervisor".to_string())),
            TaskType::Analysis => Some(ChildId("analysis-supervisor".to_string())),
            TaskType::Communication => Some(ChildId("comm-supervisor".to_string())),
            TaskType::Custom(ref custom_type) => {
                Some(ChildId(format!("{}-supervisor", custom_type)))
            }
        }
    }

    async fn handle_child_failure(
        &self,
        child_id: ChildId,
        error: FailureReason,
    ) -> SupervisionDecision {
        error!("Root supervisor handling child failure: {} - {:?}", child_id.0, error);
        
        // Root supervisor typically handles all failures locally
        // Only catastrophic failures would cause shutdown
        match error {
            FailureReason::ResourceExhaustion(_) => {
                // Critical system resource issue
                SupervisionDecision::Stop
            }
            _ => {
                // Handle other failures with configured strategy
                SupervisionDecision::Handled
            }
        }
    }
}

/// Supervisor for managing agents of a specific type
#[derive(Debug)]
pub struct AgentSupervisor {
    agent_type: String,
    strategy: SupervisionStrategy,
    restart_policy: RestartPolicy,
    escalation_policy: EscalationPolicy,
    max_concurrent_tasks: usize,
}

impl AgentSupervisor {
    /// Create a new agent supervisor
    pub fn new(
        agent_type: &str,
        strategy: SupervisionStrategy,
        restart_policy: RestartPolicy,
    ) -> Self {
        Self {
            agent_type: agent_type.to_string(),
            strategy,
            restart_policy,
            escalation_policy: EscalationPolicy::default(),
            max_concurrent_tasks: 10,
        }
    }

    /// Set maximum concurrent tasks
    pub fn with_max_concurrent_tasks(mut self, max: usize) -> Self {
        self.max_concurrent_tasks = max;
        self
    }
}

#[async_trait]
impl Supervisor for AgentSupervisor {
    type Child = ChildRef;

    async fn supervise(&self, children: Vec<Self::Child>) -> SupervisionResult {
        info!(
            "Agent supervisor ({}) starting with {} children",
            self.agent_type,
            children.len()
        );
        
        SupervisionResult::Running
    }

    fn supervision_strategy(&self) -> SupervisionStrategy {
        self.strategy
    }

    fn restart_policy(&self) -> RestartPolicy {
        self.restart_policy.clone()
    }

    fn escalation_policy(&self) -> EscalationPolicy {
        self.escalation_policy.clone()
    }

    async fn route_task(&self, task: Task) -> Option<ChildId> {
        // Route to least loaded child agent
        debug!(
            "Agent supervisor ({}) routing task: {:?}",
            self.agent_type, task.id
        );
        
        // In real implementation, would check child load and availability
        // For now, simple round-robin simulation
        Some(ChildId(format!("{}-worker-1", self.agent_type)))
    }

    async fn handle_child_failure(
        &self,
        child_id: ChildId,
        error: FailureReason,
    ) -> SupervisionDecision {
        error!(
            "Agent supervisor ({}) handling child failure: {} - {:?}",
            self.agent_type, child_id.0, error
        );
        
        // Decide based on error type and escalation policy
        match error {
            FailureReason::Crash(_) | FailureReason::UnhandledError(_) => {
                // Normal failures - handle locally
                SupervisionDecision::Handled
            }
            FailureReason::ResourceExhaustion(_) => {
                // May need to escalate if affecting multiple children
                if self.escalation_policy.escalate_on_error_types.contains(&"ResourceExhaustion".to_string()) {
                    SupervisionDecision::Escalate(format!("ResourceExhaustion: {}", error))
                } else {
                    SupervisionDecision::Handled
                }
            }
            _ => SupervisionDecision::Handled,
        }
    }
}

/// Dynamic supervisor that can spawn children on demand
pub struct DynamicSupervisor {
    name: String,
    strategy: SupervisionStrategy,
    restart_policy: RestartPolicy,
    child_spec_factory: Arc<dyn Fn() -> ChildSpec + Send + Sync>,
}

impl std::fmt::Debug for DynamicSupervisor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicSupervisor")
            .field("name", &self.name)
            .field("strategy", &self.strategy)
            .field("restart_policy", &self.restart_policy)
            .field("child_spec_factory", &"<factory function>")
            .finish()
    }
}

/// Specification for creating a child
#[derive(Debug, Clone)]
pub struct ChildSpec {
    pub id: String,
    pub node_type: NodeType,
    pub start_timeout: std::time::Duration,
}

impl DynamicSupervisor {
    /// Create a new dynamic supervisor
    pub fn new<F>(
        name: &str,
        strategy: SupervisionStrategy,
        child_factory: F,
    ) -> Self
    where
        F: Fn() -> ChildSpec + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            strategy,
            restart_policy: RestartPolicy::exponential_backoff(),
            child_spec_factory: Arc::new(child_factory),
        }
    }
}

#[async_trait]
impl Supervisor for DynamicSupervisor {
    type Child = ChildRef;

    async fn supervise(&self, children: Vec<Self::Child>) -> SupervisionResult {
        info!(
            "Dynamic supervisor ({}) starting with {} initial children",
            self.name,
            children.len()
        );
        
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

    async fn route_task(&self, task: Task) -> Option<ChildId> {
        debug!("Dynamic supervisor ({}) routing task: {:?}", self.name, task.id);
        
        // Could spawn new child if needed
        let child_spec = (self.child_spec_factory)();
        Some(ChildId(child_spec.id))
    }

    async fn handle_child_failure(
        &self,
        child_id: ChildId,
        error: FailureReason,
    ) -> SupervisionDecision {
        error!(
            "Dynamic supervisor ({}) handling child failure: {} - {:?}",
            self.name, child_id.0, error
        );
        
        SupervisionDecision::Handled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_root_supervisor_creation() {
        let supervisor = RootSupervisor::new(SupervisionStrategy::OneForAll);
        assert_eq!(supervisor.supervision_strategy(), SupervisionStrategy::OneForAll);
    }

    #[tokio::test]
    async fn test_agent_supervisor_routing() {
        let supervisor = AgentSupervisor::new(
            "test",
            SupervisionStrategy::OneForOne,
            RestartPolicy::max_restarts(3),
        );

        let task = Task {
            id: "test-task".to_string(),
            task_type: TaskType::Research,
            payload: serde_json::Value::Null,
            priority: 5,
        };

        let child_id = supervisor.route_task(task).await;
        assert!(child_id.is_some());
    }
}