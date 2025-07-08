//! Persistence layer for MisterSmith
//! 
//! Provides database connectivity and operations for supervision state management.

pub mod connection;
pub mod supervision_db;
pub mod task_db;

pub use connection::{create_pool_with_retry, DbConfig, DbPool, PoolStats};
pub use supervision_db::{
    ChildStateDb, SupervisionDb, SupervisionFailureDb, SupervisionNodeDb, SupervisionPolicyDb,
    TreeNode,
};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::supervision::{
    FailureReason, NodeType, SupervisionStrategy, RestartPolicy,
};

/// Trait for supervision persistence operations
#[async_trait]
pub trait SupervisionPersistence: Send + Sync {
    /// Create a new supervision node
    async fn create_node(
        &self,
        node_id: &str,
        node_type: NodeType,
        parent_id: Option<Uuid>,
        agent_id: Option<Uuid>,
        configuration: JsonValue,
    ) -> Result<SupervisionNodeDb>;
    
    /// Update node status
    async fn update_node_status(&self, node_id: Uuid, status: &str) -> Result<()>;
    
    /// Get node by ID
    async fn get_node(&self, node_id: Uuid) -> Result<Option<SupervisionNodeDb>>;
    
    /// Get node by string ID
    async fn get_node_by_string_id(&self, node_id: &str) -> Result<Option<SupervisionNodeDb>>;
    
    /// Get children of a node
    async fn get_children(&self, parent_id: Uuid) -> Result<Vec<SupervisionNodeDb>>;
    
    /// Create or update supervision policy
    async fn upsert_policy(
        &self,
        node_id: Uuid,
        strategy: SupervisionStrategy,
        restart_policy: &RestartPolicy,
    ) -> Result<SupervisionPolicyDb>;
    
    /// Get policy for a node
    async fn get_policy(&self, node_id: Uuid) -> Result<Option<SupervisionPolicyDb>>;
    
    /// Record a failure
    async fn record_failure(
        &self,
        node_id: Uuid,
        reason: FailureReason,
        detail: &str,
        context: JsonValue,
    ) -> Result<Uuid>;
    
    /// Get recent failures
    async fn get_recent_failures(
        &self,
        node_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<SupervisionFailureDb>>;
    
    /// Record a restart attempt
    async fn record_restart(
        &self,
        node_id: Uuid,
        failure_id: Option<Uuid>,
        attempt_number: i32,
        delay_ms: i32,
    ) -> Result<Uuid>;
    
    /// Complete a restart attempt
    async fn complete_restart(
        &self,
        restart_id: Uuid,
        success: bool,
        error_message: Option<&str>,
        new_instance_id: Option<Uuid>,
    ) -> Result<()>;
    
    /// Update or create child state
    async fn upsert_child_state(
        &self,
        parent_node_id: Uuid,
        child_id: &str,
        state: &str,
        instance_id: Option<Uuid>,
    ) -> Result<()>;
    
    /// Get child states
    async fn get_child_states(&self, parent_node_id: Uuid) -> Result<Vec<ChildStateDb>>;
    
    /// Get supervision tree hierarchy
    async fn get_supervision_tree(&self, root_id: Option<Uuid>) -> Result<Vec<TreeNode>>;
    
    /// Check if restart should be allowed
    async fn should_restart(&self, node_id: Uuid) -> Result<bool>;
}

/// Implement the trait for SupervisionDb
#[async_trait]
impl SupervisionPersistence for SupervisionDb {
    async fn create_node(
        &self,
        node_id: &str,
        node_type: NodeType,
        parent_id: Option<Uuid>,
        agent_id: Option<Uuid>,
        configuration: JsonValue,
    ) -> Result<SupervisionNodeDb> {
        self.create_node(node_id, node_type, parent_id, agent_id, configuration).await
    }
    
    async fn update_node_status(&self, node_id: Uuid, status: &str) -> Result<()> {
        self.update_node_status(node_id, status).await
    }
    
    async fn get_node(&self, node_id: Uuid) -> Result<Option<SupervisionNodeDb>> {
        self.get_node(node_id).await
    }
    
    async fn get_node_by_string_id(&self, node_id: &str) -> Result<Option<SupervisionNodeDb>> {
        self.get_node_by_string_id(node_id).await
    }
    
    async fn get_children(&self, parent_id: Uuid) -> Result<Vec<SupervisionNodeDb>> {
        self.get_children(parent_id).await
    }
    
    async fn upsert_policy(
        &self,
        node_id: Uuid,
        strategy: SupervisionStrategy,
        restart_policy: &RestartPolicy,
    ) -> Result<SupervisionPolicyDb> {
        self.upsert_policy(node_id, strategy, restart_policy).await
    }
    
    async fn get_policy(&self, node_id: Uuid) -> Result<Option<SupervisionPolicyDb>> {
        self.get_policy(node_id).await
    }
    
    async fn record_failure(
        &self,
        node_id: Uuid,
        reason: FailureReason,
        detail: &str,
        context: JsonValue,
    ) -> Result<Uuid> {
        self.record_failure(node_id, reason, detail, context).await
    }
    
    async fn get_recent_failures(
        &self,
        node_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<SupervisionFailureDb>> {
        self.get_recent_failures(node_id, since).await
    }
    
    async fn record_restart(
        &self,
        node_id: Uuid,
        failure_id: Option<Uuid>,
        attempt_number: i32,
        delay_ms: i32,
    ) -> Result<Uuid> {
        self.record_restart(node_id, failure_id, attempt_number, delay_ms).await
    }
    
    async fn complete_restart(
        &self,
        restart_id: Uuid,
        success: bool,
        error_message: Option<&str>,
        new_instance_id: Option<Uuid>,
    ) -> Result<()> {
        self.complete_restart(restart_id, success, error_message, new_instance_id).await
    }
    
    async fn upsert_child_state(
        &self,
        parent_node_id: Uuid,
        child_id: &str,
        state: &str,
        instance_id: Option<Uuid>,
    ) -> Result<()> {
        self.upsert_child_state(parent_node_id, child_id, state, instance_id).await
    }
    
    async fn get_child_states(&self, parent_node_id: Uuid) -> Result<Vec<ChildStateDb>> {
        self.get_child_states(parent_node_id).await
    }
    
    async fn get_supervision_tree(&self, root_id: Option<Uuid>) -> Result<Vec<TreeNode>> {
        self.get_supervision_tree(root_id).await
    }
    
    async fn should_restart(&self, node_id: Uuid) -> Result<bool> {
        self.should_restart(node_id).await
    }
}