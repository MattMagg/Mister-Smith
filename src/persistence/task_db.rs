//! Task assignment and management database operations
//! 
//! Provides persistence layer for task routing and assignment in the supervision system.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, Row};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::connection::DbPool;

/// Task type enum matching database
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskType {
    Research,
    Code,
    Analysis,
    Communication,
    Custom(String),
}

impl TaskType {
    /// Convert to database string representation
    pub fn to_db_string(&self) -> (&str, Option<&str>) {
        match self {
            TaskType::Research => ("Research", None),
            TaskType::Code => ("Code", None),
            TaskType::Analysis => ("Analysis", None),
            TaskType::Communication => ("Communication", None),
            TaskType::Custom(name) => ("Custom", Some(name.as_str())),
        }
    }
}

/// Task status enum matching database
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    /// Convert to database string
    pub fn to_db_string(&self) -> &str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Assigned => "assigned",
            TaskStatus::Processing => "processing",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }
    
    /// Convert from database string
    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TaskStatus::Pending),
            "assigned" => Some(TaskStatus::Assigned),
            "processing" => Some(TaskStatus::Processing),
            "completed" => Some(TaskStatus::Completed),
            "failed" => Some(TaskStatus::Failed),
            "cancelled" => Some(TaskStatus::Cancelled),
            _ => None,
        }
    }
}

/// Database representation of a task definition
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskDefinitionDb {
    pub id: Uuid,
    pub task_type: String,
    pub custom_type_name: Option<String>,
    pub description: Option<String>,
    pub routing_rules: JsonValue,
    pub required_capabilities: Vec<String>,
    pub priority_weight: i32,
    pub timeout_seconds: Option<i32>,
    pub retry_policy: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database representation of a task assignment
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskAssignmentDb {
    pub id: Uuid,
    pub task_id: Uuid,
    pub task_type: String,
    pub custom_type_name: Option<String>,
    pub task_payload: JsonValue,
    pub priority: i32,
    pub assigned_to_node_id: Option<Uuid>,
    pub assigned_by_node_id: Option<Uuid>,
    pub status: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub created_at: DateTime<Utc>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deadline_at: Option<DateTime<Utc>>,
    pub result_payload: Option<JsonValue>,
    pub error_message: Option<String>,
    pub message_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
}

/// Task database operations
pub struct TaskDb {
    pool: DbPool,
}

impl TaskDb {
    /// Create a new task database instance
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
    
    /// Create or update a task definition
    pub async fn upsert_task_definition(
        &self,
        task_type: TaskType,
        description: Option<&str>,
        routing_rules: JsonValue,
        required_capabilities: Vec<String>,
        priority_weight: i32,
        timeout_seconds: Option<i32>,
    ) -> Result<TaskDefinitionDb> {
        let (type_str, custom_name) = task_type.to_db_string();
        
        let definition = sqlx::query_as::<_, TaskDefinitionDb>(
            r#"
            INSERT INTO task_definitions (
                task_type, custom_type_name, description, routing_rules,
                required_capabilities, priority_weight, timeout_seconds
            )
            VALUES ($1::task_type, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (task_type, custom_type_name) DO UPDATE SET
                description = EXCLUDED.description,
                routing_rules = EXCLUDED.routing_rules,
                required_capabilities = EXCLUDED.required_capabilities,
                priority_weight = EXCLUDED.priority_weight,
                timeout_seconds = EXCLUDED.timeout_seconds,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(type_str)
        .bind(custom_name)
        .bind(description)
        .bind(&routing_rules)
        .bind(&required_capabilities)
        .bind(priority_weight)
        .bind(timeout_seconds)
        .fetch_one(self.pool.pool())
        .await
        .context("Failed to upsert task definition")?;
        
        info!("Upserted task definition: {:?}", task_type);
        Ok(definition)
    }
    
    /// Get task definition by type
    pub async fn get_task_definition(
        &self,
        task_type: TaskType,
    ) -> Result<Option<TaskDefinitionDb>> {
        let (type_str, custom_name) = task_type.to_db_string();
        
        let definition = sqlx::query_as::<_, TaskDefinitionDb>(
            r#"
            SELECT * FROM task_definitions 
            WHERE task_type = $1 
            AND (($2::TEXT IS NULL AND custom_type_name IS NULL) OR custom_type_name = $2)
            "#,
        )
        .bind(type_str)
        .bind(custom_name)
        .fetch_optional(self.pool.pool())
        .await
        .context("Failed to fetch task definition")?;
        
        Ok(definition)
    }
    
    /// Create a new task assignment
    pub async fn create_task(
        &self,
        task_id: Uuid,
        task_type: TaskType,
        payload: JsonValue,
        priority: i32,
        deadline: Option<DateTime<Utc>>,
        correlation_id: Option<Uuid>,
    ) -> Result<TaskAssignmentDb> {
        let (type_str, custom_name) = task_type.to_db_string();
        
        let assignment = sqlx::query_as::<_, TaskAssignmentDb>(
            r#"
            INSERT INTO task_assignments (
                task_id, task_type, custom_type_name, task_payload,
                priority, deadline_at, correlation_id
            )
            VALUES ($1, $2::task_type, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(task_id)
        .bind(type_str)
        .bind(custom_name)
        .bind(&payload)
        .bind(priority)
        .bind(deadline)
        .bind(correlation_id)
        .fetch_one(self.pool.pool())
        .await
        .context("Failed to create task assignment")?;
        
        info!("Created task: {} ({:?})", task_id, task_type);
        Ok(assignment)
    }
    
    /// Get pending tasks for assignment
    pub async fn get_pending_tasks(&self, limit: i64) -> Result<Vec<TaskAssignmentDb>> {
        let tasks = sqlx::query_as::<_, TaskAssignmentDb>(
            r#"
            SELECT * FROM task_assignments
            WHERE status = 'pending'::task_status
            ORDER BY priority DESC, created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self.pool.pool())
        .await
        .context("Failed to fetch pending tasks")?;
        
        Ok(tasks)
    }
    
    /// Assign task to a node
    pub async fn assign_task(
        &self,
        task_id: Uuid,
        assigned_to: Uuid,
        assigned_by: Uuid,
    ) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE task_assignments
            SET assigned_to_node_id = $1,
                assigned_by_node_id = $2,
                assigned_at = NOW(),
                status = 'assigned'::task_status
            WHERE task_id = $3 AND status = 'pending'::task_status
            "#,
        )
        .bind(assigned_to)
        .bind(assigned_by)
        .bind(task_id)
        .execute(self.pool.pool())
        .await
        .context("Failed to assign task")?
        .rows_affected();
        
        if rows_affected == 0 {
            warn!("Task {} was not in pending status", task_id);
        } else {
            info!("Assigned task {} to node {}", task_id, assigned_to);
        }
        
        Ok(())
    }
    
    /// Update task status
    pub async fn update_task_status(
        &self,
        task_id: Uuid,
        status: TaskStatus,
        result: Option<JsonValue>,
        error: Option<&str>,
    ) -> Result<()> {
        let status_str = status.to_db_string();
        
        // Add timestamp updates based on status
        let query = match status {
            TaskStatus::Processing => {
                sqlx::query(
                    r#"
                    UPDATE task_assignments
                    SET status = $1::task_status,
                        result_payload = $2,
                        error_message = $3,
                        started_at = NOW()
                    WHERE task_id = $4
                    "#,
                )
            }
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                sqlx::query(
                    r#"
                    UPDATE task_assignments
                    SET status = $1::task_status,
                        result_payload = $2,
                        error_message = $3,
                        completed_at = NOW()
                    WHERE task_id = $4
                    "#,
                )
            }
            _ => {
                sqlx::query(
                    r#"
                    UPDATE task_assignments
                    SET status = $1::task_status,
                        result_payload = $2,
                        error_message = $3
                    WHERE task_id = $4
                    "#,
                )
            }
        };
        
        query
            .bind(status_str)
            .bind(result)
            .bind(error)
            .bind(task_id)
            .execute(self.pool.pool())
            .await
            .context("Failed to update task status")?;
        
        info!("Updated task {} status to {:?}", task_id, status);
        Ok(())
    }
    
    /// Get tasks assigned to a node
    pub async fn get_node_tasks(
        &self,
        node_id: Uuid,
        include_completed: bool,
    ) -> Result<Vec<TaskAssignmentDb>> {
        let query = if include_completed {
            sqlx::query_as::<_, TaskAssignmentDb>(
                r#"
                SELECT * FROM task_assignments
                WHERE assigned_to_node_id = $1
                ORDER BY created_at DESC
                "#,
            )
        } else {
            sqlx::query_as::<_, TaskAssignmentDb>(
                r#"
                SELECT * FROM task_assignments
                WHERE assigned_to_node_id = $1
                AND status NOT IN ('completed', 'failed', 'cancelled')
                ORDER BY priority DESC, created_at ASC
                "#,
            )
        };
        
        let tasks = query
            .bind(node_id)
            .fetch_all(self.pool.pool())
            .await
            .context("Failed to fetch node tasks")?;
        
        Ok(tasks)
    }
    
    /// Call the assign_task_to_worker database function
    pub async fn auto_assign_task(
        &self,
        task_id: Uuid,
        supervisor_id: Uuid,
    ) -> Result<Option<Uuid>> {
        let assigned_worker = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT assign_task_to_worker($1, $2)",
        )
        .bind(task_id)
        .bind(supervisor_id)
        .fetch_one(self.pool.pool())
        .await
        .context("Failed to auto-assign task")?;
        
        if let Some(worker_id) = assigned_worker {
            info!("Auto-assigned task {} to worker {}", task_id, worker_id);
        } else {
            warn!("No available worker for task {}", task_id);
        }
        
        Ok(assigned_worker)
    }
    
    /// Get task metrics for a time window
    pub async fn get_task_metrics(
        &self,
        since: DateTime<Utc>,
    ) -> Result<TaskMetrics> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) FILTER (WHERE status = 'pending') as pending_count,
                COUNT(*) FILTER (WHERE status = 'assigned') as assigned_count,
                COUNT(*) FILTER (WHERE status = 'processing') as processing_count,
                COUNT(*) FILTER (WHERE status = 'completed') as completed_count,
                COUNT(*) FILTER (WHERE status = 'failed') as failed_count,
                COUNT(*) FILTER (WHERE status = 'cancelled') as cancelled_count,
                AVG(EXTRACT(EPOCH FROM (completed_at - created_at))) 
                    FILTER (WHERE status = 'completed') as avg_completion_seconds
            FROM task_assignments
            WHERE created_at >= $1
            "#,
        )
        .bind(since)
        .fetch_one(self.pool.pool())
        .await
        .context("Failed to fetch task metrics")?;
        
        Ok(TaskMetrics {
            pending_count: row.get::<i64, _>("pending_count") as u64,
            assigned_count: row.get::<i64, _>("assigned_count") as u64,
            processing_count: row.get::<i64, _>("processing_count") as u64,
            completed_count: row.get::<i64, _>("completed_count") as u64,
            failed_count: row.get::<i64, _>("failed_count") as u64,
            cancelled_count: row.get::<i64, _>("cancelled_count") as u64,
            avg_completion_seconds: row.get::<Option<f64>, _>("avg_completion_seconds"),
        })
    }
}

/// Task metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub pending_count: u64,
    pub assigned_count: u64,
    pub processing_count: u64,
    pub completed_count: u64,
    pub failed_count: u64,
    pub cancelled_count: u64,
    pub avg_completion_seconds: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_task_type_conversion() {
        let task_type = TaskType::Custom("test_task".to_string());
        let (type_str, custom_name) = task_type.to_db_string();
        assert_eq!(type_str, "Custom");
        assert_eq!(custom_name, Some("test_task"));
        
        let task_type = TaskType::Research;
        let (type_str, custom_name) = task_type.to_db_string();
        assert_eq!(type_str, "Research");
        assert_eq!(custom_name, None);
    }
    
    #[test]
    fn test_task_status_conversion() {
        assert_eq!(TaskStatus::Pending.to_db_string(), "pending");
        assert_eq!(TaskStatus::from_db_string("pending"), Some(TaskStatus::Pending));
        assert_eq!(TaskStatus::from_db_string("invalid"), None);
    }
}