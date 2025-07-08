//! Supervision Event System for Multi-Agent Coordination
//!
//! Implements NATS-based event publishing and subscribing for agent supervision,
//! health monitoring, and task allocation in Phase 3.

use std::sync::Arc;
use anyhow::Result;
use async_nats::{Client, Subscriber};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use futures::StreamExt;

use crate::agent::{AgentPool, AgentState};
// TODO: ProcessHandle and ProcessMetrics will be added in future phases
// use crate::supervision::{ProcessHandle, ProcessMetrics};

/// Event publisher for supervision-related events
#[derive(Debug, Clone)]
pub struct SupervisionEventPublisher {
    client: Client,
    agent_pool: Arc<AgentPool>,
}

impl SupervisionEventPublisher {
    /// Create a new supervision event publisher
    pub fn new(client: Client, agent_pool: Arc<AgentPool>) -> Self {
        Self { client, agent_pool }
    }

    /// Publish an agent lifecycle event
    pub async fn publish_lifecycle_event(
        &self,
        agent_id: &str,
        event_type: LifecycleEventType,
        previous_state: AgentState,
        new_state: AgentState,
        metadata: LifecycleMetadata,
    ) -> Result<()> {
        let event = AgentLifecycleEvent {
            agent_id: agent_id.to_string(),
            event_type,
            timestamp: Utc::now(),
            previous_state,
            new_state,
            metadata,
        };

        let subject = format!("supervision.agent.{}.lifecycle.{}", 
            agent_id, 
            event_type.as_str()
        );

        debug!("Publishing lifecycle event: {} -> {}", subject, event.event_type.as_str());
        
        let payload = serde_json::to_vec(&event)?;
        self.client.publish(subject, payload.into()).await?;
        
        Ok(())
    }

    /// Publish a health monitoring event
    pub async fn publish_health_event(
        &self,
        agent_id: &str,
        check_type: HealthCheckType,
        health_score: f64,
        metrics: HealthMetrics,
        alerts: Vec<HealthAlert>,
    ) -> Result<()> {
        let event = HealthMonitoringEvent {
            agent_id: agent_id.to_string(),
            check_type,
            timestamp: Utc::now(),
            health_score,
            metrics,
            alerts,
        };

        let subject = format!("supervision.agent.{}.health.{}", 
            agent_id, 
            check_type.as_str()
        );

        let payload = serde_json::to_vec(&event)?;
        self.client.publish(subject, payload.into()).await?;
        
        Ok(())
    }

    /// Publish a task allocation event
    pub async fn publish_allocation_event(
        &self,
        task_id: &str,
        event_type: AllocationEventType,
        details: AllocationDetails,
        coordination_metadata: CoordinationMetadata,
    ) -> Result<()> {
        let event = TaskAllocationEvent {
            event_type,
            task_id: task_id.to_string(),
            timestamp: Utc::now(),
            allocation_details: details,
            coordination_metadata,
        };

        let subject = format!("supervision.tasks.allocation.{}", event_type.as_str());

        debug!("Publishing allocation event: {} for task {}", subject, task_id);
        
        let payload = serde_json::to_vec(&event)?;
        self.client.publish(subject, payload.into()).await?;
        
        Ok(())
    }

    /// Publish a coordination error event
    pub async fn publish_error_event(
        &self,
        severity: ErrorSeverity,
        component: &str,
        error_info: ErrorInfo,
    ) -> Result<()> {
        let event = CoordinationErrorEvent {
            severity,
            component: component.to_string(),
            timestamp: Utc::now(),
            error_info,
        };

        let subject = format!("supervision.error.{}.{}", severity.as_str(), component);

        warn!("Publishing error event: {} - {}", subject, event.error_info.message);
        
        let payload = serde_json::to_vec(&event)?;
        self.client.publish(subject, payload.into()).await?;
        
        Ok(())
    }
}

/// Event subscriber for handling supervision events
#[derive(Debug)]
pub struct SupervisionEventSubscriber {
    client: Client,
    subscriptions: Arc<RwLock<Vec<Subscriber>>>,
}

impl SupervisionEventSubscriber {
    /// Create a new supervision event subscriber
    pub fn new(client: Client) -> Self {
        Self {
            client,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Subscribe to all lifecycle events
    pub async fn subscribe_lifecycle_events<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(AgentLifecycleEvent) + Send + Sync + 'static,
    {
        let subscriber = self.client
            .subscribe("supervision.agent.*.lifecycle.*")
            .await?;

        info!("Subscribed to agent lifecycle events");

        let handler = Arc::new(handler);
        
        // Spawn handler task
        tokio::spawn(async move {
            let mut subscriber = subscriber;
            while let Some(msg) = subscriber.next().await {
                match serde_json::from_slice::<AgentLifecycleEvent>(&msg.payload) {
                    Ok(event) => {
                        debug!("Received lifecycle event: {:?}", event.event_type);
                        handler(event);
                    }
                    Err(e) => {
                        error!("Failed to deserialize lifecycle event: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Subscribe to health monitoring events for specific check types
    pub async fn subscribe_health_events<F>(
        &self, 
        check_type: Option<HealthCheckType>,
        handler: F
    ) -> Result<()>
    where
        F: Fn(HealthMonitoringEvent) + Send + Sync + 'static,
    {
        let subject = match check_type {
            Some(ct) => format!("supervision.agent.*.health.{}", ct.as_str()),
            None => "supervision.agent.*.health.*".to_string(),
        };

        let subscriber = self.client.subscribe(subject).await?;

        info!("Subscribed to health monitoring events");

        let handler = Arc::new(handler);
        
        tokio::spawn(async move {
            let mut subscriber = subscriber;
            while let Some(msg) = subscriber.next().await {
                match serde_json::from_slice::<HealthMonitoringEvent>(&msg.payload) {
                    Ok(event) => {
                        handler(event);
                    }
                    Err(e) => {
                        error!("Failed to deserialize health event: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Subscribe to task allocation events
    pub async fn subscribe_allocation_events<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(TaskAllocationEvent) + Send + Sync + 'static,
    {
        let subscriber = self.client
            .subscribe("supervision.tasks.allocation.*")
            .await?;

        info!("Subscribed to task allocation events");

        let handler = Arc::new(handler);
        
        tokio::spawn(async move {
            let mut subscriber = subscriber;
            while let Some(msg) = subscriber.next().await {
                match serde_json::from_slice::<TaskAllocationEvent>(&msg.payload) {
                    Ok(event) => {
                        handler(event);
                    }
                    Err(e) => {
                        error!("Failed to deserialize allocation event: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Subscribe to coordination errors of specific severity
    pub async fn subscribe_error_events<F>(
        &self,
        min_severity: ErrorSeverity,
        handler: F
    ) -> Result<()>
    where
        F: Fn(CoordinationErrorEvent) + Send + Sync + 'static,
    {
        let subscriber = self.client
            .subscribe("supervision.error.*.*")
            .await?;

        info!("Subscribed to coordination error events");

        let handler = Arc::new(handler);
        let min_severity = Arc::new(min_severity);
        
        tokio::spawn(async move {
            let mut subscriber = subscriber;
            while let Some(msg) = subscriber.next().await {
                match serde_json::from_slice::<CoordinationErrorEvent>(&msg.payload) {
                    Ok(event) => {
                        if event.severity >= *min_severity {
                            handler(event);
                        }
                    }
                    Err(e) => {
                        error!("Failed to deserialize error event: {}", e);
                    }
                }
            }
        });

        Ok(())
    }
}

// Event Types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLifecycleEvent {
    pub agent_id: String,
    pub event_type: LifecycleEventType,
    pub timestamp: DateTime<Utc>,
    pub previous_state: AgentState,
    pub new_state: AgentState,
    pub metadata: LifecycleMetadata,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LifecycleEventType {
    Created,
    Initialized,
    Running,
    Paused,
    Resumed,
    Terminating,
    Terminated,
    Error,
}

impl LifecycleEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Initialized => "initialized",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Resumed => "resumed",
            Self::Terminating => "terminating",
            Self::Terminated => "terminated",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleMetadata {
    pub trigger: String,
    pub supervisor_id: Option<String>,
    pub related_task_id: Option<String>,
    pub error_details: Option<ErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitoringEvent {
    pub agent_id: String,
    pub check_type: HealthCheckType,
    pub timestamp: DateTime<Utc>,
    pub health_score: f64, // 0.0 to 1.0
    pub metrics: HealthMetrics,
    pub alerts: Vec<HealthAlert>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HealthCheckType {
    Heartbeat,
    ResourceUsage,
    Capacity,
    Performance,
    Connectivity,
}

impl HealthCheckType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Heartbeat => "heartbeat",
            Self::ResourceUsage => "resource_usage",
            Self::Capacity => "capacity",
            Self::Performance => "performance",
            Self::Connectivity => "connectivity",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub active_tasks: u32,
    pub queue_depth: u32,
    pub response_time_ms: u64,
    pub error_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    pub alert_type: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub threshold_value: f64,
    pub actual_value: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAllocationEvent {
    pub event_type: AllocationEventType,
    pub task_id: String,
    pub timestamp: DateTime<Utc>,
    pub allocation_details: AllocationDetails,
    pub coordination_metadata: CoordinationMetadata,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AllocationEventType {
    Requested,
    Assigned,
    Rejected,
    Redistributed,
    Completed,
    Failed,
    Timeout,
}

impl AllocationEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Assigned => "assigned",
            Self::Rejected => "rejected",
            Self::Redistributed => "redistributed",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Timeout => "timeout",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationDetails {
    pub requested_capabilities: Vec<String>,
    pub assigned_agent: Option<String>,
    pub rejection_reasons: Vec<String>,
    pub alternative_agents: Vec<String>,
    pub priority: u8,
    pub estimated_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationMetadata {
    pub workflow_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub dependencies: Vec<String>,
    pub coordination_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationErrorEvent {
    pub severity: ErrorSeverity,
    pub component: String,
    pub timestamp: DateTime<Utc>,
    pub error_info: ErrorInfo,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl ErrorSeverity {
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
pub struct ErrorInfo {
    pub error_code: String,
    pub message: String,
    pub details: Option<String>,
    pub stack_trace: Option<String>,
    pub recovery_suggestions: Vec<String>,
}

// Coordination Handlers

/// Handler for processing supervision decisions based on events
pub struct SupervisionDecisionHandler {
    agent_pool: Arc<AgentPool>,
    event_publisher: SupervisionEventPublisher,
}

impl SupervisionDecisionHandler {
    pub fn new(
        agent_pool: Arc<AgentPool>,
        event_publisher: SupervisionEventPublisher,
    ) -> Self {
        Self {
            agent_pool,
            event_publisher,
        }
    }

    /// Handle agent health degradation
    pub async fn handle_health_degradation(&self, event: HealthMonitoringEvent) {
        if event.health_score < 0.5 {
            warn!(
                "Agent {} health degraded to {:.2}", 
                event.agent_id, 
                event.health_score
            );

            // Publish task redistribution event
            let _ = self.event_publisher.publish_allocation_event(
                &Uuid::new_v4().to_string(),
                AllocationEventType::Redistributed,
                AllocationDetails {
                    requested_capabilities: vec![],
                    assigned_agent: None,
                    rejection_reasons: vec![format!(
                        "Agent {} health below threshold",
                        event.agent_id
                    )],
                    alternative_agents: vec![],
                    priority: 1,
                    estimated_duration_ms: None,
                },
                CoordinationMetadata {
                    workflow_id: None,
                    parent_task_id: None,
                    dependencies: vec![],
                    coordination_strategy: "health_based_redistribution".to_string(),
                },
            ).await;
        }
    }

    /// Handle agent lifecycle changes
    pub async fn handle_lifecycle_change(&self, event: AgentLifecycleEvent) {
        match event.event_type {
            LifecycleEventType::Terminated => {
                info!("Agent {} terminated, checking for task redistribution", event.agent_id);
                // In future phases, redistribute tasks from terminated agent
            }
            LifecycleEventType::Error => {
                error!("Agent {} encountered error: {:?}", event.agent_id, event.metadata.error_details);
                // Trigger recovery procedures
            }
            _ => {
                debug!("Agent {} lifecycle event: {:?}", event.agent_id, event.event_type);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = AgentLifecycleEvent {
            agent_id: "test-agent".to_string(),
            event_type: LifecycleEventType::Running,
            timestamp: Utc::now(),
            previous_state: AgentState::Created,
            new_state: AgentState::Running,
            metadata: LifecycleMetadata {
                trigger: "manual".to_string(),
                supervisor_id: Some("supervisor-1".to_string()),
                related_task_id: None,
                error_details: None,
            },
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: AgentLifecycleEvent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(event.agent_id, deserialized.agent_id);
        assert_eq!(event.event_type, deserialized.event_type);
    }
}