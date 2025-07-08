//! Supervision persistence layer for MisterSmith Phase 3
//! 
//! Provides database operations for supervision tree state management.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, FromRow, Row};
use tracing::{info, warn};
use uuid::Uuid;

use crate::supervision::{
    FailureReason, NodeType, SupervisionStrategy, RestartPolicy, BackoffStrategy,
};

use super::connection::DbPool;

/// Database representation of a supervision node
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupervisionNodeDb {
    pub id: Uuid,
    pub node_id: String,
    pub node_type: String, // Will be converted to NodeType enum
    pub node_type_detail: Option<String>,
    pub status: String,
    pub parent_node_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub configuration: JsonValue,
    pub metadata: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub terminated_at: Option<DateTime<Utc>>,
}

/// Database representation of a supervision policy
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupervisionPolicyDb {
    pub id: Uuid,
    pub node_id: Uuid,
    pub supervision_strategy: String,
    pub max_restarts: i32,
    pub time_window_seconds: i32,
    pub backoff_strategy: String,
    pub backoff_factor: f64,
    pub min_restart_delay_ms: i32,
    pub max_restart_delay_ms: i32,
    pub escalate_on_limit: bool,
    pub escalate_on_error_types: Vec<String>,
    pub escalation_timeout_seconds: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database representation of a supervision failure
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SupervisionFailureDb {
    pub id: Uuid,
    pub node_id: Uuid,
    pub failure_reason: String,
    pub failure_detail: String,
    pub stack_trace: Option<String>,
    pub context: JsonValue,
    pub occurred_at: DateTime<Utc>,
    pub restart_attempted: bool,
    pub escalated_to_parent: bool,
    pub recovery_action: Option<String>,
    pub correlation_id: Option<Uuid>,
}

/// Database representation of child state
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChildStateDb {
    pub id: Uuid,
    pub node_id: Uuid,
    pub child_id: String,
    pub child_node_id: Option<Uuid>,
    pub instance_id: Option<Uuid>,
    pub state: String,
    pub state_data: JsonValue,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub error_count: i32,
    pub restart_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Supervision database operations
pub struct SupervisionDb {
    pool: DbPool,
}

impl SupervisionDb {
    /// Create a new supervision database instance
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
    
    /// Create a new supervision node
    pub async fn create_node(
        &self,
        node_id: &str,
        node_type: NodeType,
        parent_id: Option<Uuid>,
        agent_id: Option<Uuid>,
        configuration: JsonValue,
    ) -> Result<SupervisionNodeDb> {
        let node_type_str = match node_type {
            NodeType::RootSupervisor => "RootSupervisor",
            NodeType::AgentSupervisor(_) => "AgentSupervisor",
            NodeType::Worker(_) => "Worker",
            NodeType::Service(_) => "Service",
        };
        
        let node_type_detail = match &node_type {
            NodeType::RootSupervisor => None,
            NodeType::AgentSupervisor(t) => Some(t.clone()),
            NodeType::Worker(t) => Some(t.clone()),
            NodeType::Service(n) => Some(n.clone()),
        };
        
        let node = sqlx::query_as::<_, SupervisionNodeDb>(
            r#"
            INSERT INTO supervision_nodes (
                node_id, node_type, node_type_detail, parent_node_id, 
                agent_id, configuration, status
            )
            VALUES ($1, $2::node_type, $3, $4, $5, $6, 'inactive'::node_status)
            RETURNING *
            "#,
        )
        .bind(node_id)
        .bind(node_type_str)
        .bind(node_type_detail)
        .bind(parent_id)
        .bind(agent_id)
        .bind(&configuration)
        .fetch_one(self.pool.pool())
        .await
        .context("Failed to create supervision node")?;
        
        info!("Created supervision node: {} ({})", node_id, node_type_str);
        Ok(node)
    }
    
    /// Update node status
    pub async fn update_node_status(&self, node_id: Uuid, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE supervision_nodes 
            SET status = $1::node_status, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(status)
        .bind(node_id)
        .execute(self.pool.pool())
        .await
        .context("Failed to update node status")?;
        
        Ok(())
    }
    
    /// Get node by ID
    pub async fn get_node(&self, node_id: Uuid) -> Result<Option<SupervisionNodeDb>> {
        let node = sqlx::query_as::<_, SupervisionNodeDb>(
            "SELECT * FROM supervision_nodes WHERE id = $1",
        )
        .bind(node_id)
        .fetch_optional(self.pool.pool())
        .await
        .context("Failed to fetch supervision node")?;
        
        Ok(node)
    }
    
    /// Get node by string node_id
    pub async fn get_node_by_string_id(&self, node_id: &str) -> Result<Option<SupervisionNodeDb>> {
        let node = sqlx::query_as::<_, SupervisionNodeDb>(
            "SELECT * FROM supervision_nodes WHERE node_id = $1",
        )
        .bind(node_id)
        .fetch_optional(self.pool.pool())
        .await
        .context("Failed to fetch supervision node by string ID")?;
        
        Ok(node)
    }
    
    /// Get children of a node
    pub async fn get_children(&self, parent_id: Uuid) -> Result<Vec<SupervisionNodeDb>> {
        let children = sqlx::query_as::<_, SupervisionNodeDb>(
            "SELECT * FROM supervision_nodes WHERE parent_node_id = $1 ORDER BY created_at",
        )
        .bind(parent_id)
        .fetch_all(self.pool.pool())
        .await
        .context("Failed to fetch child nodes")?;
        
        Ok(children)
    }
    
    /// Create or update supervision policy
    pub async fn upsert_policy(
        &self,
        node_id: Uuid,
        strategy: SupervisionStrategy,
        restart_policy: &RestartPolicy,
    ) -> Result<SupervisionPolicyDb> {
        let strategy_str = match strategy {
            SupervisionStrategy::OneForOne => "OneForOne",
            SupervisionStrategy::OneForAll => "OneForAll",
            SupervisionStrategy::RestForOne => "RestForOne",
            SupervisionStrategy::Escalate => "Escalate",
        };
        
        let backoff_str = match restart_policy.backoff_strategy {
            BackoffStrategy::Fixed => "Fixed",
            BackoffStrategy::Linear => "Linear",
            BackoffStrategy::Exponential { .. } => "Exponential",
        };
        
        let policy = sqlx::query_as::<_, SupervisionPolicyDb>(
            r#"
            INSERT INTO supervision_policies (
                node_id, supervision_strategy, max_restarts, time_window_seconds,
                backoff_strategy, backoff_factor, min_restart_delay_ms, max_restart_delay_ms,
                escalate_on_limit, escalate_on_error_types, escalation_timeout_seconds
            )
            VALUES ($1, $2::supervision_strategy, $3, $4, $5::backoff_strategy, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (node_id) DO UPDATE SET
                supervision_strategy = EXCLUDED.supervision_strategy,
                max_restarts = EXCLUDED.max_restarts,
                time_window_seconds = EXCLUDED.time_window_seconds,
                backoff_strategy = EXCLUDED.backoff_strategy,
                backoff_factor = EXCLUDED.backoff_factor,
                min_restart_delay_ms = EXCLUDED.min_restart_delay_ms,
                max_restart_delay_ms = EXCLUDED.max_restart_delay_ms,
                escalate_on_limit = EXCLUDED.escalate_on_limit,
                escalate_on_error_types = EXCLUDED.escalate_on_error_types,
                escalation_timeout_seconds = EXCLUDED.escalation_timeout_seconds,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(node_id)
        .bind(strategy_str)
        .bind(restart_policy.max_restarts as i32)
        .bind(restart_policy.time_window.as_secs() as i32)
        .bind(backoff_str)
        .bind(match restart_policy.backoff_strategy {
            BackoffStrategy::Exponential { factor } => factor,
            _ => 2.0, // Default factor for non-exponential strategies
        })
        .bind(restart_policy.min_restart_delay.as_millis() as i32)
        .bind(restart_policy.max_restart_delay.as_millis() as i32)
        .bind(true) // escalate_on_limit
        .bind(Vec::<String>::new()) // escalate_on_error_types
        .bind(10) // escalation_timeout_seconds
        .fetch_one(self.pool.pool())
        .await
        .context("Failed to upsert supervision policy")?;
        
        Ok(policy)
    }
    
    /// Get policy for a node
    pub async fn get_policy(&self, node_id: Uuid) -> Result<Option<SupervisionPolicyDb>> {
        let policy = sqlx::query_as::<_, SupervisionPolicyDb>(
            "SELECT * FROM supervision_policies WHERE node_id = $1",
        )
        .bind(node_id)
        .fetch_optional(self.pool.pool())
        .await
        .context("Failed to fetch supervision policy")?;
        
        Ok(policy)
    }
    
    /// Record a failure
    pub async fn record_failure(
        &self,
        node_id: Uuid,
        reason: FailureReason,
        detail: &str,
        context: JsonValue,
    ) -> Result<Uuid> {
        let reason_str = match reason {
            FailureReason::Crash(_) => "Crash",
            FailureReason::HeartbeatTimeout => "HeartbeatTimeout",
            FailureReason::ResourceExhaustion(_) => "ResourceExhaustion",
            FailureReason::UnhandledError(_) => "UnhandledError",
            FailureReason::ExternalFailure(_) => "ExternalFailure",
        };
        
        let failure_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO supervision_failures (
                node_id, failure_reason, failure_detail, context
            )
            VALUES ($1, $2::failure_reason, $3, $4)
            RETURNING id
            "#,
        )
        .bind(node_id)
        .bind(reason_str)
        .bind(detail)
        .bind(&context)
        .fetch_one(self.pool.pool())
        .await
        .context("Failed to record supervision failure")?;
        
        warn!("Recorded failure for node {}: {} - {}", node_id, reason_str, detail);
        Ok(failure_id)
    }
    
    /// Get recent failures for restart policy check
    pub async fn get_recent_failures(
        &self,
        node_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<SupervisionFailureDb>> {
        let failures = sqlx::query_as::<_, SupervisionFailureDb>(
            r#"
            SELECT * FROM supervision_failures 
            WHERE node_id = $1 AND occurred_at > $2
            ORDER BY occurred_at DESC
            "#,
        )
        .bind(node_id)
        .bind(since)
        .fetch_all(self.pool.pool())
        .await
        .context("Failed to fetch recent failures")?;
        
        Ok(failures)
    }
    
    /// Record a restart attempt
    pub async fn record_restart(
        &self,
        node_id: Uuid,
        failure_id: Option<Uuid>,
        attempt_number: i32,
        delay_ms: i32,
    ) -> Result<Uuid> {
        let restart_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO supervision_restarts (
                node_id, failure_id, restart_attempt_number, restart_delay_ms
            )
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(node_id)
        .bind(failure_id)
        .bind(attempt_number)
        .bind(delay_ms)
        .fetch_one(self.pool.pool())
        .await
        .context("Failed to record restart attempt")?;
        
        info!("Recorded restart attempt {} for node {}", attempt_number, node_id);
        Ok(restart_id)
    }
    
    /// Complete a restart attempt
    pub async fn complete_restart(
        &self,
        restart_id: Uuid,
        success: bool,
        error_message: Option<&str>,
        new_instance_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE supervision_restarts
            SET completed_at = NOW(), success = $1, error_message = $2, new_instance_id = $3
            WHERE id = $4
            "#,
        )
        .bind(success)
        .bind(error_message)
        .bind(new_instance_id)
        .bind(restart_id)
        .execute(self.pool.pool())
        .await
        .context("Failed to complete restart record")?;
        
        Ok(())
    }
    
    /// Update or create child state
    pub async fn upsert_child_state(
        &self,
        parent_node_id: Uuid,
        child_id: &str,
        state: &str,
        instance_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO supervision_child_states (
                node_id, child_id, state, instance_id
            )
            VALUES ($1, $2, $3::child_state, $4)
            ON CONFLICT (node_id, child_id) DO UPDATE SET
                state = EXCLUDED.state,
                instance_id = EXCLUDED.instance_id,
                updated_at = NOW()
            "#,
        )
        .bind(parent_node_id)
        .bind(child_id)
        .bind(state)
        .bind(instance_id)
        .execute(self.pool.pool())
        .await
        .context("Failed to upsert child state")?;
        
        Ok(())
    }
    
    /// Get child states for a parent node
    pub async fn get_child_states(&self, parent_node_id: Uuid) -> Result<Vec<ChildStateDb>> {
        let states = sqlx::query_as::<_, ChildStateDb>(
            "SELECT * FROM supervision_child_states WHERE node_id = $1",
        )
        .bind(parent_node_id)
        .fetch_all(self.pool.pool())
        .await
        .context("Failed to fetch child states")?;
        
        Ok(states)
    }
    
    /// Get supervision tree hierarchy
    pub async fn get_supervision_tree(&self, root_id: Option<Uuid>) -> Result<Vec<TreeNode>> {
        let rows = if let Some(id) = root_id {
            sqlx::query(
                "SELECT * FROM get_supervision_tree($1)",
            )
            .bind(id)
            .fetch_all(self.pool.pool())
            .await?
        } else {
            sqlx::query(
                "SELECT * FROM get_supervision_tree(NULL)",
            )
            .fetch_all(self.pool.pool())
            .await?
        };
        
        let nodes = rows
            .into_iter()
            .map(|row: PgRow| TreeNode {
                node_id: row.get("node_id"),
                node_name: row.get("node_name"),
                node_type: row.get("node_type"),
                parent_id: row.get("parent_id"),
                level: row.get("level"),
                path: row.get("path"),
            })
            .collect();
        
        Ok(nodes)
    }
    
    /// Check if restart should be allowed
    pub async fn should_restart(&self, node_id: Uuid) -> Result<bool> {
        let should_restart = sqlx::query_scalar::<_, bool>(
            "SELECT should_restart($1)",
        )
        .bind(node_id)
        .fetch_one(self.pool.pool())
        .await
        .context("Failed to check restart eligibility")?;
        
        Ok(should_restart)
    }
    
    /// Calculate restart delay
    pub async fn calculate_restart_delay(
        &self,
        policy_id: Uuid,
        attempt_number: i32,
    ) -> Result<i32> {
        let delay_ms = sqlx::query_scalar::<_, i32>(
            "SELECT calculate_restart_delay($1, $2)",
        )
        .bind(policy_id)
        .bind(attempt_number)
        .fetch_one(self.pool.pool())
        .await
        .context("Failed to calculate restart delay")?;
        
        Ok(delay_ms)
    }
}

/// Tree node representation from database function
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub node_id: Uuid,
    pub node_name: String,
    pub node_type: String,
    pub parent_id: Option<Uuid>,
    pub level: i32,
    pub path: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: Full integration tests would require a test database
    // These are basic unit tests for type conversions
    
    #[test]
    fn test_node_type_conversion() {
        let node_type = NodeType::Worker("test_worker".to_string());
        let (type_str, detail) = match node_type {
            NodeType::RootSupervisor => ("RootSupervisor", None),
            NodeType::AgentSupervisor(ref t) => ("AgentSupervisor", Some(t.as_str())),
            NodeType::Worker(ref t) => ("Worker", Some(t.as_str())),
            NodeType::Service(ref n) => ("Service", Some(n.as_str())),
        };
        
        assert_eq!(type_str, "Worker");
        assert_eq!(detail, Some("test_worker"));
    }
}