//! Multi-Agent Coordination Patterns
//!
//! Implements advanced coordination patterns for Phase 3 including
//! agent discovery, task distribution, and workflow orchestration.

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;
use async_nats::Client;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, debug};
use uuid::Uuid;
use futures::StreamExt;

use crate::transport::{subjects, NatsTransport};

/// Agent discovery and registration system
pub struct AgentDiscovery {
    client: Client,
    /// Currently discovered agents
    discovered_agents: Arc<RwLock<HashMap<String, DiscoveredAgent>>>,
}

impl AgentDiscovery {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            discovered_agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Announce this agent to the system
    pub async fn announce_agent(&self, announcement: AgentAnnouncement) -> Result<()> {
        debug!("Announcing agent: {}", announcement.agent_id);
        
        let payload = serde_json::to_vec(&announcement)?;
        self.client
            .publish(subjects::discovery::ANNOUNCE, payload.into())
            .await?;
        
        Ok(())
    }

    /// Query for agents with specific capabilities
    pub async fn query_agents(&self, query: CapabilityQuery) -> Result<Vec<DiscoveredAgent>> {
        debug!("Querying for agents with capabilities: {:?}", query.required_capabilities);
        
        let payload = serde_json::to_vec(&query)?;
        
        // Use request-reply pattern with timeout
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            self.client.request(subjects::discovery::QUERY, payload.into())
        ).await??;
        
        let agents: Vec<DiscoveredAgent> = serde_json::from_slice(&response.payload)?;
        Ok(agents)
    }

    /// Start discovery listener
    pub async fn start_discovery_listener(&self) -> Result<()> {
        let mut subscriber = self.client
            .subscribe(subjects::discovery::ANNOUNCE)
            .await?;
        
        let discovered_agents = self.discovered_agents.clone();
        
        tokio::spawn(async move {
            while let Some(msg) = subscriber.next().await {
                if let Ok(announcement) = serde_json::from_slice::<AgentAnnouncement>(&msg.payload) {
                    let mut agents = discovered_agents.write().await;
                    agents.insert(
                        announcement.agent_id.clone(),
                        DiscoveredAgent {
                            agent_id: announcement.agent_id,
                            agent_type: announcement.agent_type,
                            capabilities: announcement.capabilities,
                            capacity: announcement.capacity,
                            last_seen: Utc::now(),
                        }
                    );
                }
            }
        });
        
        info!("Agent discovery listener started");
        Ok(())
    }

    /// Get currently discovered agents
    pub async fn get_discovered_agents(&self) -> Vec<DiscoveredAgent> {
        self.discovered_agents.read().await.values().cloned().collect()
    }
}

/// Task distribution system with queue groups
pub struct TaskDistributor {
    client: Client,
    /// Task tracking
    active_tasks: Arc<RwLock<HashMap<String, DistributedTask>>>,
}

impl TaskDistributor {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Distribute a task to available agents
    pub async fn distribute_task(
        &self,
        task: TaskRequest,
        queue_group: &str,
    ) -> Result<String> {
        let task_id = Uuid::new_v4().to_string();
        let distributed_task = DistributedTask {
            task_id: task_id.clone(),
            request: task.clone(),
            status: TaskStatus::Queued,
            assigned_agent: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Track the task
        self.active_tasks.write().await.insert(task_id.clone(), distributed_task);
        
        // Build subject based on task type and priority
        let subject = NatsTransport::build_subject(
            subjects::tasks::QUEUE,
            &[&task.task_type, &task.priority.as_str()],
        );
        
        debug!("Distributing task {} to queue group {}", task_id, queue_group);
        
        let task_message = TaskMessage {
            task_id: task_id.clone(),
            request: task,
            timestamp: Utc::now(),
        };
        
        let payload = serde_json::to_vec(&task_message)?;
        self.client.publish(subject, payload.into()).await?;
        
        Ok(task_id)
    }

    /// Update task progress
    pub async fn update_task_progress(
        &self,
        task_id: &str,
        progress: TaskProgress,
    ) -> Result<()> {
        let subject = NatsTransport::build_subject(
            subjects::tasks::PROGRESS,
            &[task_id],
        );
        
        let payload = serde_json::to_vec(&progress)?;
        self.client.publish(subject, payload.into()).await?;
        
        // Update local tracking
        if let Some(task) = self.active_tasks.write().await.get_mut(task_id) {
            task.status = progress.status;
            task.updated_at = Utc::now();
            if let TaskStatus::InProgress = progress.status {
                task.assigned_agent = progress.agent_id;
            }
        }
        
        Ok(())
    }

    /// Complete a task
    pub async fn complete_task(
        &self,
        task_id: &str,
        result: TaskResult,
    ) -> Result<()> {
        let subject = NatsTransport::build_subject(
            subjects::tasks::RESULT,
            &[task_id],
        );
        
        let payload = serde_json::to_vec(&result)?;
        self.client.publish(subject, payload.into()).await?;
        
        // Remove from active tasks
        self.active_tasks.write().await.remove(task_id);
        
        Ok(())
    }
}

/// Workflow coordination system
pub struct WorkflowCoordinator {
    client: Client,
    /// Active workflows
    workflows: Arc<RwLock<HashMap<String, WorkflowState>>>,
}

impl WorkflowCoordinator {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new workflow
    pub async fn create_workflow(&self, definition: WorkflowDefinition) -> Result<String> {
        let workflow_id = Uuid::new_v4().to_string();
        
        let workflow_state = WorkflowState {
            workflow_id: workflow_id.clone(),
            definition: definition.clone(),
            current_stage: 0,
            stage_states: vec![StageState::Pending; definition.stages.len()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        self.workflows.write().await.insert(workflow_id.clone(), workflow_state);
        
        // Publish first stage as ready
        self.publish_stage_ready(&workflow_id, 0).await?;
        
        info!("Created workflow {} with {} stages", workflow_id, definition.stages.len());
        Ok(workflow_id)
    }

    /// Publish stage ready event
    async fn publish_stage_ready(&self, workflow_id: &str, stage_index: usize) -> Result<()> {
        let subject = format!(
            "workflow.{}.stage.{}.ready",
            workflow_id,
            stage_index
        );
        
        let workflows = self.workflows.read().await;
        if let Some(workflow) = workflows.get(workflow_id) {
            if let Some(stage) = workflow.definition.stages.get(stage_index) {
                let stage_ready = StageReadyEvent {
                    workflow_id: workflow_id.to_string(),
                    stage_index,
                    stage_name: stage.name.clone(),
                    required_capabilities: stage.required_capabilities.clone(),
                    estimated_duration_ms: stage.estimated_duration_ms,
                };
                
                let payload = serde_json::to_vec(&stage_ready)?;
                self.client.publish(subject, payload.into()).await?;
            }
        }
        
        Ok(())
    }

    /// Mark stage as completed and advance workflow
    pub async fn complete_stage(
        &self,
        workflow_id: &str,
        stage_index: usize,
        result: StageResult,
    ) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            // Update stage state
            if let Some(state) = workflow.stage_states.get_mut(stage_index) {
                *state = if result.success {
                    StageState::Completed
                } else {
                    StageState::Failed
                };
            }
            
            workflow.updated_at = Utc::now();
            
            // Check if we can advance to next stage
            if result.success && stage_index + 1 < workflow.definition.stages.len() {
                // Check dependencies
                let next_stage = &workflow.definition.stages[stage_index + 1];
                let dependencies_met = next_stage.dependencies.iter().all(|dep| {
                    workflow.stage_states.get(*dep)
                        .map(|s| matches!(s, StageState::Completed))
                        .unwrap_or(false)
                });
                
                if dependencies_met {
                    workflow.current_stage = stage_index + 1;
                    drop(workflows); // Release lock before async call
                    self.publish_stage_ready(workflow_id, stage_index + 1).await?;
                }
            }
        }
        
        Ok(())
    }
}

// Data structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnnouncement {
    pub agent_id: String,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub capacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityQuery {
    pub required_capabilities: Vec<String>,
    pub min_capacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredAgent {
    pub agent_id: String,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub capacity: f64,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_type: String,
    pub priority: TaskPriority,
    pub payload: serde_json::Value,
    pub required_capabilities: Vec<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl TaskPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMessage {
    pub task_id: String,
    pub request: TaskRequest,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTask {
    pub task_id: String,
    pub request: TaskRequest,
    pub status: TaskStatus,
    pub assigned_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub task_id: String,
    pub agent_id: Option<String>,
    pub status: TaskStatus,
    pub progress_percent: Option<f64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub agent_id: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub stages: Vec<WorkflowStage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStage {
    pub name: String,
    pub required_capabilities: Vec<String>,
    pub dependencies: Vec<usize>, // Indices of stages that must complete first
    pub estimated_duration_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct WorkflowState {
    pub workflow_id: String,
    pub definition: WorkflowDefinition,
    pub current_stage: usize,
    pub stage_states: Vec<StageState>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub enum StageState {
    Pending,
    Ready,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageReadyEvent {
    pub workflow_id: String,
    pub stage_index: usize,
    pub stage_name: String,
    pub required_capabilities: Vec<String>,
    pub estimated_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub workflow_id: String,
    pub stage_index: usize,
    pub agent_id: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_priority_serialization() {
        let priority = TaskPriority::High;
        let serialized = serde_json::to_string(&priority).unwrap();
        let deserialized: TaskPriority = serde_json::from_str(&serialized).unwrap();
        assert!(matches!(deserialized, TaskPriority::High));
    }

    #[test]
    fn test_workflow_stage_dependencies() {
        let definition = WorkflowDefinition {
            name: "test_workflow".to_string(),
            stages: vec![
                WorkflowStage {
                    name: "stage1".to_string(),
                    required_capabilities: vec!["cap1".to_string()],
                    dependencies: vec![],
                    estimated_duration_ms: Some(1000),
                },
                WorkflowStage {
                    name: "stage2".to_string(),
                    required_capabilities: vec!["cap2".to_string()],
                    dependencies: vec![0], // Depends on stage1
                    estimated_duration_ms: Some(2000),
                },
            ],
        };
        
        assert_eq!(definition.stages[1].dependencies[0], 0);
    }
}