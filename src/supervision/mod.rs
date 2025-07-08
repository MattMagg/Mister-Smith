//! Supervision tree implementation for fault tolerance
//!
//! This module implements a hierarchical supervision tree pattern inspired by
//! Erlang/OTP, providing fault tolerance and automatic recovery for the
//! MisterSmith agent framework.
//!
//! ## Key Components
//!
//! - **SupervisionTree**: Root structure managing the entire hierarchy
//! - **Supervisor trait**: Defines supervision behavior and strategies
//! - **SupervisorNode**: Individual nodes in the supervision tree
//! - **FailureDetector**: Phi Accrual based adaptive failure detection
//! - **CircuitBreaker**: Prevents cascading failures
//!
//! ## Supervision Strategies
//!
//! - **OneForOne**: Restart only the failed child
//! - **OneForAll**: Restart all children when one fails
//! - **RestForOne**: Restart failed child and younger siblings
//! - **Escalate**: Propagate failure to parent supervisor
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use mister_smith::supervision::*;
//!
//! // Create supervision tree
//! let tree = SupervisionTree::new(SupervisionTreeConfig::default());
//!
//! // Create root supervisor
//! let root = RootSupervisor::new(SupervisionStrategy::OneForAll);
//! let root_node = SupervisorNode::new(
//!     NodeId("root".to_string()),
//!     NodeType::RootSupervisor,
//!     None,
//!     Arc::new(root),
//! );
//!
//! // Set root and start tree
//! tree.set_root(Arc::new(root_node)).await?;
//! tree.start().await?;
//! ```

mod types;
mod tree;
mod node;
mod failure_detector;
mod supervisors;
mod circuit_breaker;
mod agent_pool_supervisor;

#[cfg(test)]
mod test_compile;

// Re-export main types
pub use types::*;
pub use tree::{SupervisionTree, SupervisionTreeConfig, SupervisionTreeMetrics};
pub use node::SupervisorNode;
pub use failure_detector::{FailureDetector, FailureDetectorConfig, FailureEvent};
pub use supervisors::{RootSupervisor, AgentSupervisor, DynamicSupervisor, ChildSpec};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState, CircuitBreakerError};
pub use agent_pool_supervisor::AgentPoolSupervisor;

/// Prelude for common supervision imports
pub mod prelude {
    pub use super::{
        SupervisionTree, SupervisionTreeConfig,
        Supervisor, SupervisorNode,
        SupervisionStrategy, RestartPolicy,
        NodeId, NodeType, ChildId, ChildRef,
        RootSupervisor, AgentSupervisor,
        CircuitBreaker, CircuitBreakerConfig,
    };
}