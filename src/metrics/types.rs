//! Core metric types and structures

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};

/// Type of metric being recorded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    /// Monotonically increasing counter
    Counter,
    /// Point-in-time value
    Gauge,
    /// Statistical distribution of values
    Histogram,
    /// Statistical summary
    Summary,
}

/// A single metric value with metadata
#[derive(Debug, Clone)]
pub struct MetricValue {
    pub name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: SystemTime,
}

impl MetricValue {
    pub fn new(name: impl Into<String>, metric_type: MetricType, value: f64) -> Self {
        Self {
            name: name.into(),
            metric_type,
            value,
            labels: HashMap::new(),
            timestamp: SystemTime::now(),
        }
    }
    
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
    
    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels.extend(labels);
        self
    }
}

/// Descriptor for a metric
#[derive(Debug, Clone)]
pub struct MetricDescriptor {
    pub name: String,
    pub help: String,
    pub metric_type: MetricType,
    pub unit: Option<String>,
}

impl MetricDescriptor {
    pub fn new(
        name: impl Into<String>, 
        help: impl Into<String>, 
        metric_type: MetricType
    ) -> Self {
        Self {
            name: name.into(),
            help: help.into(),
            metric_type,
            unit: None,
        }
    }
    
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }
}

/// Histogram buckets configuration
#[derive(Debug, Clone)]
pub struct HistogramBuckets {
    pub buckets: Vec<f64>,
}

impl Default for HistogramBuckets {
    fn default() -> Self {
        Self {
            // Default buckets from the documentation
            buckets: vec![
                0.0, 5.0, 10.0, 25.0, 50.0, 75.0, 100.0, 
                250.0, 500.0, 750.0, 1000.0, 2500.0, 5000.0, 
                7500.0, 10000.0
            ],
        }
    }
}

/// Agent performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerformanceMetrics {
    pub operation_count: u64,
    pub operation_duration_ms: f64,
    pub error_count: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub queue_depth: u64,
}

/// System-wide performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPerformanceMetrics {
    pub active_agents: u64,
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub throughput_per_second: f64,
    pub average_latency_ms: f64,
}

/// Message backplane metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackplaneMetrics {
    pub consumer_lag_ms: f64,
    pub pending_message_count: u64,
    pub ack_rate_per_second: f64,
    pub reconnection_count: u64,
    pub publish_rate_per_topic: HashMap<String, f64>,
    pub message_size_p95: f64,
    pub throughput_mb_per_second: f64,
}