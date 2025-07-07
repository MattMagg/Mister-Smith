//! System-wide metrics collection

use crate::metrics::types::{MetricValue, MetricType, MetricDescriptor, SystemPerformanceMetrics, BackplaneMetrics};
use crate::metrics::MetricCollector;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Collector for system-wide metrics
pub struct SystemMetricsCollector {
    metrics: Arc<RwLock<SystemPerformanceMetrics>>,
    backplane_metrics: Arc<RwLock<BackplaneMetrics>>,
}

impl SystemMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(SystemPerformanceMetrics {
                active_agents: 0,
                total_tasks: 0,
                completed_tasks: 0,
                failed_tasks: 0,
                throughput_per_second: 0.0,
                average_latency_ms: 0.0,
            })),
            backplane_metrics: Arc::new(RwLock::new(BackplaneMetrics {
                consumer_lag_ms: 0.0,
                pending_message_count: 0,
                ack_rate_per_second: 0.0,
                reconnection_count: 0,
                publish_rate_per_topic: HashMap::new(),
                message_size_p95: 0.0,
                throughput_mb_per_second: 0.0,
            })),
        }
    }
    
    /// Update system metrics
    pub async fn update_system_metrics(&self, metrics: SystemPerformanceMetrics) {
        let mut current = self.metrics.write().await;
        *current = metrics;
    }
    
    /// Update backplane metrics
    pub async fn update_backplane_metrics(&self, metrics: BackplaneMetrics) {
        let mut current = self.backplane_metrics.write().await;
        *current = metrics;
    }
    
    /// Increment active agent count
    pub async fn increment_active_agents(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.active_agents += 1;
    }
    
    /// Decrement active agent count
    pub async fn decrement_active_agents(&self) {
        let mut metrics = self.metrics.write().await;
        if metrics.active_agents > 0 {
            metrics.active_agents -= 1;
        }
    }
    
    /// Record task completion
    pub async fn record_task_completion(&self, success: bool, latency_ms: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.total_tasks += 1;
        
        if success {
            metrics.completed_tasks += 1;
        } else {
            metrics.failed_tasks += 1;
        }
        
        // Simple moving average for latency
        let total_completed = metrics.completed_tasks as f64;
        if total_completed > 0.0 {
            metrics.average_latency_ms = 
                (metrics.average_latency_ms * (total_completed - 1.0) + latency_ms) / total_completed;
        }
    }
}

#[async_trait::async_trait]
impl MetricCollector for SystemMetricsCollector {
    async fn collect(&self) -> Vec<MetricValue> {
        let mut metrics = Vec::new();
        
        // System metrics
        {
            let system = self.metrics.read().await;
            
            metrics.push(
                MetricValue::new("system_active_agents", MetricType::Gauge, system.active_agents as f64)
            );
            
            metrics.push(
                MetricValue::new("system_total_tasks", MetricType::Counter, system.total_tasks as f64)
            );
            
            metrics.push(
                MetricValue::new("system_completed_tasks", MetricType::Counter, system.completed_tasks as f64)
            );
            
            metrics.push(
                MetricValue::new("system_failed_tasks", MetricType::Counter, system.failed_tasks as f64)
            );
            
            metrics.push(
                MetricValue::new("system_throughput_per_second", MetricType::Gauge, system.throughput_per_second)
            );
            
            metrics.push(
                MetricValue::new("system_average_latency_ms", MetricType::Gauge, system.average_latency_ms)
            );
        }
        
        // Backplane metrics
        {
            let backplane = self.backplane_metrics.read().await;
            
            metrics.push(
                MetricValue::new("backplane_consumer_lag_ms", MetricType::Gauge, backplane.consumer_lag_ms)
            );
            
            metrics.push(
                MetricValue::new("backplane_pending_messages", MetricType::Gauge, backplane.pending_message_count as f64)
            );
            
            metrics.push(
                MetricValue::new("backplane_ack_rate_per_second", MetricType::Gauge, backplane.ack_rate_per_second)
            );
            
            metrics.push(
                MetricValue::new("backplane_reconnection_count", MetricType::Counter, backplane.reconnection_count as f64)
            );
            
            metrics.push(
                MetricValue::new("backplane_message_size_p95", MetricType::Gauge, backplane.message_size_p95)
            );
            
            metrics.push(
                MetricValue::new("backplane_throughput_mb_per_second", MetricType::Gauge, backplane.throughput_mb_per_second)
            );
            
            // Per-topic publish rates
            for (topic, rate) in &backplane.publish_rate_per_topic {
                metrics.push(
                    MetricValue::new("backplane_publish_rate", MetricType::Gauge, *rate)
                        .with_label("topic", topic)
                );
            }
        }
        
        metrics
    }
    
    fn name(&self) -> &str {
        "system_metrics"
    }
    
    fn descriptors(&self) -> Vec<MetricDescriptor> {
        vec![
            // System metrics
            MetricDescriptor::new(
                "system_active_agents",
                "Number of currently active agents",
                MetricType::Gauge,
            ),
            MetricDescriptor::new(
                "system_total_tasks",
                "Total number of tasks processed",
                MetricType::Counter,
            ),
            MetricDescriptor::new(
                "system_completed_tasks",
                "Number of successfully completed tasks",
                MetricType::Counter,
            ),
            MetricDescriptor::new(
                "system_failed_tasks",
                "Number of failed tasks",
                MetricType::Counter,
            ),
            MetricDescriptor::new(
                "system_throughput_per_second",
                "Tasks processed per second",
                MetricType::Gauge,
            ),
            MetricDescriptor::new(
                "system_average_latency_ms",
                "Average task completion latency",
                MetricType::Gauge,
            )
            .with_unit("milliseconds"),
            
            // Backplane metrics
            MetricDescriptor::new(
                "backplane_consumer_lag_ms",
                "Consumer lag in milliseconds",
                MetricType::Gauge,
            )
            .with_unit("milliseconds"),
            MetricDescriptor::new(
                "backplane_pending_messages",
                "Number of pending messages",
                MetricType::Gauge,
            ),
            MetricDescriptor::new(
                "backplane_ack_rate_per_second",
                "Message acknowledgment rate",
                MetricType::Gauge,
            ),
            MetricDescriptor::new(
                "backplane_reconnection_count",
                "Number of connection reconnections",
                MetricType::Counter,
            ),
            MetricDescriptor::new(
                "backplane_message_size_p95",
                "95th percentile message size",
                MetricType::Gauge,
            )
            .with_unit("bytes"),
            MetricDescriptor::new(
                "backplane_throughput_mb_per_second",
                "Backplane throughput in MB/s",
                MetricType::Gauge,
            )
            .with_unit("megabytes_per_second"),
            MetricDescriptor::new(
                "backplane_publish_rate",
                "Message publish rate per topic",
                MetricType::Gauge,
            ),
        ]
    }
}