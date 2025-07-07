//! Performance baseline definitions and monitoring
//! 
//! Based on the performance baselines defined in the observability framework

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Performance baseline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaselines {
    pub agent_baselines: AgentBaselines,
    pub communication_baselines: CommunicationBaselines,
    pub system_baselines: SystemBaselines,
}

/// Agent performance baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBaselines {
    pub task_completion: TaskCompletionBaseline,
    pub agent_spawn: AgentSpawnBaseline,
    pub resource_usage: ResourceUsageBaseline,
}

/// Task completion performance baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletionBaseline {
    pub p95_duration_seconds: f64,
    pub p99_duration_seconds: f64,
    pub success_rate_percent: f64,
}

/// Agent spawn performance baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnBaseline {
    pub p95_duration_seconds: f64,
    pub p99_duration_seconds: f64,
    pub success_rate_percent: f64,
}

/// Resource usage baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageBaseline {
    pub memory_baseline_mb: u64,
    pub memory_max_mb: u64,
    pub cpu_baseline_percent: f64,
    pub cpu_max_percent: f64,
}

/// Communication performance baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationBaselines {
    pub message_delivery: MessageDeliveryBaseline,
    pub queue_processing: QueueProcessingBaseline,
    pub context_propagation: ContextPropagationBaseline,
}

/// Message delivery baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeliveryBaseline {
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub delivery_success_rate: f64,
}

/// Queue processing baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueProcessingBaseline {
    pub processing_time_ms: f64,
    pub max_queue_depth: u64,
}

/// Context propagation baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPropagationBaseline {
    pub overhead_ms: f64,
    pub success_rate: f64,
}

/// System-wide performance baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBaselines {
    pub throughput: ThroughputBaseline,
    pub utilization: UtilizationBaseline,
    pub error_rates: ErrorRateBaseline,
}

/// Throughput baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputBaseline {
    pub tasks_per_minute: u64,
    pub burst_capacity: u64,
}

/// Utilization baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilizationBaseline {
    pub optimal_range_percent: (f64, f64),
    pub max_sustainable_percent: f64,
}

/// Error rate baselines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRateBaseline {
    pub baseline_error_rate: f64,
    pub alert_threshold: f64,
    pub critical_threshold: f64,
}

impl Default for PerformanceBaselines {
    fn default() -> Self {
        Self {
            agent_baselines: AgentBaselines {
                task_completion: TaskCompletionBaseline {
                    p95_duration_seconds: 10.0,
                    p99_duration_seconds: 30.0,
                    success_rate_percent: 99.0,
                },
                agent_spawn: AgentSpawnBaseline {
                    p95_duration_seconds: 2.0,
                    p99_duration_seconds: 5.0,
                    success_rate_percent: 99.5,
                },
                resource_usage: ResourceUsageBaseline {
                    memory_baseline_mb: 256,
                    memory_max_mb: 512,
                    cpu_baseline_percent: 15.0,
                    cpu_max_percent: 80.0,
                },
            },
            communication_baselines: CommunicationBaselines {
                message_delivery: MessageDeliveryBaseline {
                    p95_latency_ms: 50.0,
                    p99_latency_ms: 100.0,
                    delivery_success_rate: 99.9,
                },
                queue_processing: QueueProcessingBaseline {
                    processing_time_ms: 25.0,
                    max_queue_depth: 100,
                },
                context_propagation: ContextPropagationBaseline {
                    overhead_ms: 1.0,
                    success_rate: 99.99,
                },
            },
            system_baselines: SystemBaselines {
                throughput: ThroughputBaseline {
                    tasks_per_minute: 100,
                    burst_capacity: 500,
                },
                utilization: UtilizationBaseline {
                    optimal_range_percent: (70.0, 85.0),
                    max_sustainable_percent: 90.0,
                },
                error_rates: ErrorRateBaseline {
                    baseline_error_rate: 0.01,
                    alert_threshold: 0.05,
                    critical_threshold: 0.10,
                },
            },
        }
    }
}

/// Baseline configuration and monitoring
pub struct BaselineConfig {
    baselines: PerformanceBaselines,
    violations: HashMap<String, Vec<BaselineViolation>>,
}

/// Represents a baseline violation
#[derive(Debug, Clone)]
pub struct BaselineViolation {
    pub metric_name: String,
    pub baseline_value: f64,
    pub actual_value: f64,
    pub threshold_type: ThresholdType,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, Copy)]
pub enum ThresholdType {
    Above,
    Below,
}

impl BaselineConfig {
    pub fn new() -> Self {
        Self {
            baselines: PerformanceBaselines::default(),
            violations: HashMap::new(),
        }
    }
    
    pub fn with_baselines(baselines: PerformanceBaselines) -> Self {
        Self {
            baselines,
            violations: HashMap::new(),
        }
    }
    
    /// Check if a metric violates its baseline
    pub fn check_baseline(&mut self, metric_name: &str, value: f64) -> Option<BaselineViolation> {
        // Implementation would check against specific baselines
        // This is a placeholder for the actual baseline checking logic
        None
    }
    
    /// Get all current violations
    pub fn get_violations(&self) -> &HashMap<String, Vec<BaselineViolation>> {
        &self.violations
    }
    
    /// Clear violation history
    pub fn clear_violations(&mut self) {
        self.violations.clear();
    }
}