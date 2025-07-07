//! Agent-specific metrics collection

use crate::metrics::types::{MetricValue, MetricType, MetricDescriptor, AgentPerformanceMetrics};
use crate::metrics::MetricCollector;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Collector for agent-specific metrics
pub struct AgentMetricsCollector {
    agent_states: Arc<RwLock<HashMap<String, AgentState>>>,
}

#[derive(Debug, Clone)]
struct AgentState {
    agent_id: String,
    agent_type: String,
    metrics: AgentPerformanceMetrics,
}

impl AgentMetricsCollector {
    pub fn new() -> Self {
        Self {
            agent_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Update metrics for a specific agent
    pub async fn update_agent_metrics(
        &self,
        agent_id: &str,
        agent_type: &str,
        metrics: AgentPerformanceMetrics,
    ) {
        let mut states = self.agent_states.write().await;
        states.insert(
            agent_id.to_string(),
            AgentState {
                agent_id: agent_id.to_string(),
                agent_type: agent_type.to_string(),
                metrics,
            },
        );
    }
    
    /// Remove an agent from tracking
    pub async fn remove_agent(&self, agent_id: &str) {
        let mut states = self.agent_states.write().await;
        states.remove(agent_id);
    }
}

#[async_trait::async_trait]
impl MetricCollector for AgentMetricsCollector {
    async fn collect(&self) -> Vec<MetricValue> {
        let states = self.agent_states.read().await;
        let mut metrics = Vec::new();
        
        for (agent_id, state) in states.iter() {
            let labels = HashMap::from([
                ("agent_id".to_string(), agent_id.clone()),
                ("agent_type".to_string(), state.agent_type.clone()),
            ]);
            
            // Operation count
            metrics.push(
                MetricValue::new(
                    "agent_operation_count",
                    MetricType::Counter,
                    state.metrics.operation_count as f64,
                )
                .with_labels(labels.clone()),
            );
            
            // Operation duration
            metrics.push(
                MetricValue::new(
                    "agent_operation_duration_ms",
                    MetricType::Histogram,
                    state.metrics.operation_duration_ms,
                )
                .with_labels(labels.clone()),
            );
            
            // Error count
            metrics.push(
                MetricValue::new(
                    "agent_error_count",
                    MetricType::Counter,
                    state.metrics.error_count as f64,
                )
                .with_labels(labels.clone()),
            );
            
            // Memory usage
            metrics.push(
                MetricValue::new(
                    "agent_memory_usage_mb",
                    MetricType::Gauge,
                    state.metrics.memory_usage_mb,
                )
                .with_labels(labels.clone()),
            );
            
            // CPU usage
            metrics.push(
                MetricValue::new(
                    "agent_cpu_usage_percent",
                    MetricType::Gauge,
                    state.metrics.cpu_usage_percent,
                )
                .with_labels(labels.clone()),
            );
            
            // Queue depth
            metrics.push(
                MetricValue::new(
                    "agent_queue_depth",
                    MetricType::Gauge,
                    state.metrics.queue_depth as f64,
                )
                .with_labels(labels.clone()),
            );
        }
        
        metrics
    }
    
    fn name(&self) -> &str {
        "agent_metrics"
    }
    
    fn descriptors(&self) -> Vec<MetricDescriptor> {
        vec![
            MetricDescriptor::new(
                "agent_operation_count",
                "Total number of operations performed by the agent",
                MetricType::Counter,
            ),
            MetricDescriptor::new(
                "agent_operation_duration_ms",
                "Duration of agent operations in milliseconds",
                MetricType::Histogram,
            )
            .with_unit("milliseconds"),
            MetricDescriptor::new(
                "agent_error_count",
                "Total number of errors encountered by the agent",
                MetricType::Counter,
            ),
            MetricDescriptor::new(
                "agent_memory_usage_mb",
                "Current memory usage of the agent in megabytes",
                MetricType::Gauge,
            )
            .with_unit("megabytes"),
            MetricDescriptor::new(
                "agent_cpu_usage_percent",
                "Current CPU usage percentage of the agent",
                MetricType::Gauge,
            )
            .with_unit("percent"),
            MetricDescriptor::new(
                "agent_queue_depth",
                "Number of messages in the agent's queue",
                MetricType::Gauge,
            ),
        ]
    }
}

/// Agent-type specific metric extensions
pub trait AgentMetricExtensions {
    /// Get specialized metrics for researcher agents
    fn researcher_metrics(&self) -> ResearcherMetrics;
    
    /// Get specialized metrics for coder agents
    fn coder_metrics(&self) -> CoderMetrics;
    
    /// Get specialized metrics for coordinator agents
    fn coordinator_metrics(&self) -> CoordinatorMetrics;
}

#[derive(Debug, Clone)]
pub struct ResearcherMetrics {
    pub sources_searched: u64,
    pub relevance_scores: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct CoderMetrics {
    pub lines_generated: u64,
    pub complexity_scores: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct CoordinatorMetrics {
    pub agents_managed: u64,
    pub coordination_latency_ms: f64,
}