//! Core supervision types and definitions
//!
//! This module defines the fundamental types used throughout the supervision system.

use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unique identifier for nodes in the supervision tree
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for child processes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChildId(pub String);

/// Type of node in the supervision tree
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    /// Root supervisor at the top of the tree
    RootSupervisor,
    /// Agent supervisor managing a group of agents
    AgentSupervisor(String), // agent type
    /// Individual worker node
    Worker(String), // worker type
    /// Service node
    Service(String), // service name
}

/// Supervision strategy determining how to handle child failures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisionStrategy {
    /// Restart only the failed child
    OneForOne,
    /// Restart all children when one fails
    OneForAll,
    /// Restart failed child and all started after it
    RestForOne,
    /// Propagate failure to parent supervisor
    Escalate,
}

/// Restart policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartPolicy {
    /// Maximum number of restarts allowed
    pub max_restarts: u32,
    /// Time window for restart counting
    pub time_window: Duration,
    /// Backoff strategy
    pub backoff_strategy: BackoffStrategy,
    /// Minimum delay between restarts
    pub min_restart_delay: Duration,
    /// Maximum delay between restarts
    pub max_restart_delay: Duration,
}

impl RestartPolicy {
    /// Create a policy with exponential backoff
    pub fn exponential_backoff() -> Self {
        Self {
            max_restarts: 5,
            time_window: Duration::from_secs(60),
            backoff_strategy: BackoffStrategy::Exponential { factor: 2.0 },
            min_restart_delay: Duration::from_millis(100),
            max_restart_delay: Duration::from_secs(30),
        }
    }

    /// Create a policy with maximum restart limit
    pub fn max_restarts(max: u32) -> Self {
        Self {
            max_restarts: max,
            time_window: Duration::from_secs(60),
            backoff_strategy: BackoffStrategy::Linear,
            min_restart_delay: Duration::from_secs(1),
            max_restart_delay: Duration::from_secs(5),
        }
    }

    /// Create a policy that always restarts
    pub fn always_restart() -> Self {
        Self {
            max_restarts: u32::MAX,
            time_window: Duration::from_secs(3600), // 1 hour
            backoff_strategy: BackoffStrategy::Fixed,
            min_restart_delay: Duration::from_secs(1),
            max_restart_delay: Duration::from_secs(1),
        }
    }

    /// Check if restart should be allowed based on failure history
    pub fn should_restart(&self, failure_history: &[FailureRecord]) -> bool {
        let now = Instant::now();
        let recent_failures = failure_history
            .iter()
            .filter(|f| now.duration_since(f.timestamp) < self.time_window)
            .count();
        
        recent_failures < self.max_restarts as usize
    }

    /// Calculate restart delay based on attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = match self.backoff_strategy {
            BackoffStrategy::Fixed => self.min_restart_delay,
            BackoffStrategy::Linear => {
                self.min_restart_delay + Duration::from_secs(attempt as u64)
            }
            BackoffStrategy::Exponential { factor } => {
                let secs = self.min_restart_delay.as_secs_f64() * factor.powi(attempt as i32);
                Duration::from_secs_f64(secs)
            }
        };

        // Cap at maximum delay
        delay.min(self.max_restart_delay)
    }
}

/// Backoff strategy for restart delays
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between restarts
    Fixed,
    /// Linear increase in delay
    Linear,
    /// Exponential increase in delay
    Exponential { factor: f64 },
}

/// Record of a failure event
#[derive(Debug, Clone)]
pub struct FailureRecord {
    pub timestamp: Instant,
    pub reason: FailureReason,
}

/// Reason for node failure
#[derive(Debug, Clone, Error)]
pub enum FailureReason {
    #[error("Process crashed: {0}")]
    Crash(String),
    
    #[error("Heartbeat timeout")]
    HeartbeatTimeout,
    
    #[error("Resource exhaustion: {0}")]
    ResourceExhaustion(String),
    
    #[error("Unhandled error: {0}")]
    UnhandledError(String),
    
    #[error("External failure: {0}")]
    ExternalFailure(String),
}

/// Escalation policy for handling failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    /// Whether to escalate to parent on restart limit exceeded
    pub escalate_on_limit: bool,
    /// Whether to escalate on specific error types
    pub escalate_on_error_types: Vec<String>,
    /// Timeout before escalating
    pub escalation_timeout: Duration,
}

impl Default for EscalationPolicy {
    fn default() -> Self {
        Self {
            escalate_on_limit: true,
            escalate_on_error_types: vec![
                "ResourceExhaustion".to_string(),
                "SystemFailure".to_string(),
            ],
            escalation_timeout: Duration::from_secs(10),
        }
    }
}

/// Result of a supervision operation
#[derive(Debug, Clone)]
pub enum SupervisionResult {
    /// Supervision is running normally
    Running,
    /// Supervision has stopped
    Stopped,
    /// Supervision failed with error
    Failed(String),
}

/// Decision made by supervisor for handling a failure
#[derive(Debug)]
pub enum SupervisionDecision {
    /// Failure was handled locally
    Handled,
    /// Failure needs to be escalated
    Escalate(String), // Changed from Box<dyn Error> to String for cloneability
    /// Stop supervision
    Stop,
}

/// Reference to a child process
#[derive(Debug, Clone)]
pub struct ChildRef {
    pub id: ChildId,
    pub node_type: NodeType,
    pub state: Arc<tokio::sync::RwLock<ChildState>>,
}

/// State of a child process
#[derive(Debug, Clone)]
pub enum ChildState {
    /// Child is starting up
    Starting,
    /// Child is running normally
    Running,
    /// Child has failed
    Failed(FailureReason),
    /// Child is stopped
    Stopped,
}

/// Reference to a supervisor
pub type SupervisorRef = Arc<dyn Supervisor<Child = ChildRef> + Send + Sync>;

/// Core supervisor trait
#[async_trait::async_trait]
pub trait Supervisor: std::fmt::Debug {
    /// Type of children this supervisor manages
    type Child;
    
    /// Supervise the given children
    async fn supervise(&self, children: Vec<Self::Child>) -> SupervisionResult;
    
    /// Get the supervision strategy
    fn supervision_strategy(&self) -> SupervisionStrategy;
    
    /// Get the restart policy
    fn restart_policy(&self) -> RestartPolicy;
    
    /// Get the escalation policy
    fn escalation_policy(&self) -> EscalationPolicy;
    
    /// Route a task to an appropriate child (hub-and-spoke pattern)
    async fn route_task(&self, task: Task) -> Option<ChildId>;
    
    /// Handle a child failure
    async fn handle_child_failure(&self, child_id: ChildId, error: FailureReason) -> SupervisionDecision;
}

/// Task to be routed to agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub payload: serde_json::Value,
    pub priority: u8,
}

/// Type of task for routing decisions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    Research,
    Code,
    Analysis,
    Communication,
    Custom(String),
}

/// Supervision metrics for monitoring
#[derive(Debug, Clone, Default)]
pub struct SupervisionMetrics {
    pub restarts_total: u64,
    pub failures_total: u64,
    pub uptime_seconds: u64,
    pub last_failure_time: Option<Instant>,
    pub child_count: usize,
}

/// Errors that can occur in the supervision system
#[derive(Debug, Error)]
pub enum SupervisionError {
    #[error("Supervision tree initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("Child process failed to start: {0}")]
    ChildStartFailed(String),
    
    #[error("Restart limit exceeded for {node_id}")]
    RestartLimitExceeded { node_id: NodeId },
    
    #[error("Supervisor not found: {0}")]
    SupervisorNotFound(NodeId),
    
    #[error("Invalid supervision configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Supervision operation failed: {0}")]
    OperationFailed(String),
}

/// Health check result for supervised components
#[derive(Debug, Clone)]
pub enum HealthResult {
    /// Component is healthy
    Healthy,
    /// Component is degraded but functional
    Degraded(String),
    /// Component is unhealthy
    Unhealthy(String),
}

/// Health check trait for supervised components
#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// Check the health of the component
    async fn check_health(&self) -> HealthResult;
}