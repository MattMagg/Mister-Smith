use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use anyhow::Result;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub subject: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Default)]
pub struct EventMetrics {
    pub total_events: usize,
    pub events_by_type: HashMap<String, usize>,
    pub unique_nodes: usize,
    pub failure_rate: f64,
    pub events_per_second: f64,
}

/// Records and analyzes supervision events for testing
pub struct EventRecorder {
    transport: Arc<mistersmith::transport::nats::NatsTransport>,
    events: Arc<RwLock<Vec<RecordedEvent>>>,
    recording: Arc<RwLock<bool>>,
}

impl EventRecorder {
    pub fn new(transport: Arc<mistersmith::transport::nats::NatsTransport>) -> Self {
        Self {
            transport,
            events: Arc::new(RwLock::new(Vec::new())),
            recording: Arc::new(RwLock::new(false)),
        }
    }

    /// Start recording events matching the pattern
    pub async fn start_recording(&self, pattern: &str) -> Result<()> {
        *self.recording.write().await = true;
        
        let mut subscriber = self.transport.subscribe(pattern).await?;
        let events = self.events.clone();
        let recording = self.recording.clone();
        let pattern = pattern.to_string();

        tokio::spawn(async move {
            while *recording.read().await {
                if let Some(message) = subscriber.next().await {
                    if let Ok(event_str) = String::from_utf8(message.clone()) {
                        // Try to parse as supervision event
                        if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event_str) {
                            let event_type = payload.get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let recorded = RecordedEvent {
                                timestamp: chrono::Utc::now(),
                                subject: pattern.clone(),
                                event_type,
                                payload,
                            };

                            events.write().await.push(recorded);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop recording events
    pub async fn stop_recording(&self) {
        *self.recording.write().await = false;
    }

    /// Get all recorded events
    pub async fn get_recorded_events(&self) -> Vec<RecordedEvent> {
        self.events.read().await.clone()
    }

    /// Clear recorded events
    pub async fn clear_events(&self) {
        self.events.write().await.clear();
    }

    /// Get events by type
    pub async fn get_events_by_type(&self, event_type: &str) -> Vec<RecordedEvent> {
        self.events.read().await
            .iter()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Aggregate metrics from recorded events
    pub fn aggregate_metrics(&self, events: &[RecordedEvent]) -> EventMetrics {
        let mut metrics = EventMetrics::default();
        metrics.total_events = events.len();

        // Count by type
        for event in events {
            *metrics.events_by_type
                .entry(event.event_type.clone())
                .or_insert(0) += 1;
        }

        // Count unique nodes
        let mut unique_nodes = std::collections::HashSet::new();
        for event in events {
            if let Some(node_id) = event.payload.get("node_id").and_then(|n| n.as_str()) {
                unique_nodes.insert(node_id.to_string());
            }
            if let Some(child_id) = event.payload.get("child_id").and_then(|n| n.as_str()) {
                unique_nodes.insert(child_id.to_string());
            }
        }
        metrics.unique_nodes = unique_nodes.len();

        // Calculate failure rate
        let failure_count = *metrics.events_by_type.get("node_failure").unwrap_or(&0);
        if metrics.total_events > 0 {
            metrics.failure_rate = failure_count as f64 / metrics.total_events as f64;
        }

        // Calculate events per second
        if events.len() >= 2 {
            let first = &events[0].timestamp;
            let last = &events[events.len() - 1].timestamp;
            let duration = (*last - *first).num_seconds() as f64;
            if duration > 0.0 {
                metrics.events_per_second = events.len() as f64 / duration;
            }
        }

        metrics
    }

    /// Wait for a specific number of events
    pub async fn wait_for_events(&self, count: usize, timeout: std::time::Duration) -> Result<()> {
        let start = std::time::Instant::now();
        
        while self.events.read().await.len() < count {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for {} events, only got {}",
                    count,
                    self.events.read().await.len()
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Verify event sequence
    pub async fn verify_sequence(&self, expected_types: &[&str]) -> Result<()> {
        let events = self.events.read().await;
        
        if events.len() < expected_types.len() {
            return Err(anyhow::anyhow!(
                "Not enough events: expected {}, got {}",
                expected_types.len(),
                events.len()
            ));
        }

        for (i, expected_type) in expected_types.iter().enumerate() {
            if events[i].event_type != *expected_type {
                return Err(anyhow::anyhow!(
                    "Event sequence mismatch at position {}: expected '{}', got '{}'",
                    i,
                    expected_type,
                    events[i].event_type
                ));
            }
        }

        Ok(())
    }
}

/// Mock supervision event types for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SupervisionEvent {
    NodeAdded {
        parent_id: mistersmith::supervision::NodeId,
        child_id: mistersmith::supervision::ChildId,
        node_type: mistersmith::supervision::NodeType,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    StateChanged {
        node_id: mistersmith::supervision::NodeId,
        child_id: mistersmith::supervision::ChildId,
        old_state: mistersmith::supervision::ChildState,
        new_state: mistersmith::supervision::ChildState,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    NodeFailure {
        node_id: mistersmith::supervision::NodeId,
        child_id: mistersmith::supervision::ChildId,
        reason: mistersmith::supervision::FailureReason,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    RestartInitiated {
        node_id: mistersmith::supervision::NodeId,
        child_id: mistersmith::supervision::ChildId,
        delay: std::time::Duration,
        attempt: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    RestartCompleted {
        node_id: mistersmith::supervision::NodeId,
        child_id: mistersmith::supervision::ChildId,
        success: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

/// Extension trait for publishing events in tests
#[async_trait::async_trait]
pub trait TestEventPublisher {
    async fn publish_event(&self, event: SupervisionEvent) -> Result<()>;
}

#[async_trait::async_trait]
impl TestEventPublisher for mistersmith::supervision::SupervisionTree {
    async fn publish_event(&self, event: SupervisionEvent) -> Result<()> {
        let subject = match &event {
            SupervisionEvent::NodeAdded { .. } => "supervision.nodes.added",
            SupervisionEvent::StateChanged { .. } => "supervision.states.changed",
            SupervisionEvent::NodeFailure { .. } => "supervision.failures.occurred",
            SupervisionEvent::RestartInitiated { .. } => "supervision.restarts.initiated",
            SupervisionEvent::RestartCompleted { .. } => "supervision.restarts.completed",
        };

        let payload = serde_json::to_vec(&event)?;
        self.publish_event_raw(subject, &payload).await
    }
}