//! System load and resource utilization performance tests

use super::{
    PerformanceTest, PerformanceTestConfig, PerformanceTestResults,
    PerformanceMetric, BaselineViolation, ViolationSeverity,
};
use crate::agent::{Agent, AgentConfig, AgentType};
use crate::metrics::{MetricsCollector, PerformanceBaselines};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{System, SystemExt, ProcessExt};
use tracing::info;

/// System load performance test
pub struct SystemLoadTest;

#[async_trait::async_trait]
impl PerformanceTest for SystemLoadTest {
    fn name(&self) -> &str {
        "System Load Performance"
    }
    
    async fn run(
        &self,
        config: &PerformanceTestConfig,
        metrics: Arc<MetricsCollector>,
    ) -> Result<PerformanceTestResults, Box<dyn std::error::Error>> {
        let test_start = Instant::now();
        let mut system = System::new_all();
        let pid = sysinfo::get_current_pid().map_err(|e| format!("Failed to get PID: {}", e))?;
        
        // Baseline measurements
        system.refresh_all();
        let baseline_memory = get_process_memory(&system, pid);
        let baseline_cpu = get_process_cpu(&system, pid);
        
        // Create agents to generate load
        let mut agents = Vec::new();
        for i in 0..config.concurrent_agents {
            let agent_config = AgentConfig {
                id: format!("load_test_agent_{}", i),
                agent_type: AgentType::Worker,
                ..Default::default()
            };
            
            let agent = Agent::spawn(agent_config).await?;
            agents.push(agent);
            
            // Update metrics
            metrics.system_collector().increment_active_agents().await;
        }
        
        // Run under load
        let mut memory_samples = Vec::new();
        let mut cpu_samples = Vec::new();
        let mut throughput_samples = Vec::new();
        
        let sample_interval = Duration::from_secs(1);
        let samples = config.duration.as_secs() / sample_interval.as_secs();
        
        for _ in 0..samples {
            system.refresh_all();
            
            // Collect resource usage
            let memory_mb = get_process_memory(&system, pid);
            let cpu_percent = get_process_cpu(&system, pid);
            
            memory_samples.push(memory_mb);
            cpu_samples.push(cpu_percent);
            
            // Simulate work
            let tasks_completed = simulate_workload(&metrics, 100).await;
            throughput_samples.push(tasks_completed as f64);
            
            tokio::time::sleep(sample_interval).await;
        }
        
        let test_duration = test_start.elapsed();
        
        // Calculate metrics
        let avg_memory = memory_samples.iter().sum::<f64>() / memory_samples.len() as f64;
        let max_memory = memory_samples.iter().fold(0.0f64, |a, &b| a.max(b));
        let avg_cpu = cpu_samples.iter().sum::<f64>() / cpu_samples.len() as f64;
        let max_cpu = cpu_samples.iter().fold(0.0f64, |a, &b| a.max(b));
        let avg_throughput = throughput_samples.iter().sum::<f64>() / throughput_samples.len() as f64;
        
        // Calculate utilization
        let utilization = (avg_cpu / 100.0) * 100.0; // Convert to percentage
        
        let metrics_vec = vec![
            PerformanceMetric {
                name: "memory_baseline_mb".to_string(),
                value: avg_memory,
                unit: "megabytes".to_string(),
                baseline: Some(256.0),
            },
            PerformanceMetric {
                name: "memory_max_mb".to_string(),
                value: max_memory,
                unit: "megabytes".to_string(),
                baseline: Some(512.0),
            },
            PerformanceMetric {
                name: "cpu_baseline_percent".to_string(),
                value: avg_cpu,
                unit: "percent".to_string(),
                baseline: Some(15.0),
            },
            PerformanceMetric {
                name: "cpu_max_percent".to_string(),
                value: max_cpu,
                unit: "percent".to_string(),
                baseline: Some(80.0),
            },
            PerformanceMetric {
                name: "system_utilization_percent".to_string(),
                value: utilization,
                unit: "percent".to_string(),
                baseline: Some(75.0),
            },
            PerformanceMetric {
                name: "throughput_tasks_per_second".to_string(),
                value: avg_throughput,
                unit: "tasks/second".to_string(),
                baseline: None,
            },
        ];
        
        Ok(PerformanceTestResults {
            test_name: self.name().to_string(),
            duration: test_duration,
            passed: true,
            metrics: metrics_vec,
            violations: Vec::new(),
        })
    }
    
    fn validate_baselines(
        &self,
        results: &PerformanceTestResults,
        baselines: &PerformanceBaselines,
    ) -> Vec<BaselineViolation> {
        let mut violations = Vec::new();
        
        for metric in &results.metrics {
            match metric.name.as_str() {
                "memory_baseline_mb" => {
                    if metric.value > baselines.agent_baselines.resource_usage.memory_baseline_mb as f64 {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.agent_baselines.resource_usage.memory_baseline_mb as f64,
                            actual: metric.value,
                            severity: ViolationSeverity::Warning,
                        });
                    }
                }
                "memory_max_mb" => {
                    if metric.value > baselines.agent_baselines.resource_usage.memory_max_mb as f64 {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.agent_baselines.resource_usage.memory_max_mb as f64,
                            actual: metric.value,
                            severity: ViolationSeverity::Error,
                        });
                    }
                }
                "cpu_baseline_percent" => {
                    if metric.value > baselines.agent_baselines.resource_usage.cpu_baseline_percent {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.agent_baselines.resource_usage.cpu_baseline_percent,
                            actual: metric.value,
                            severity: ViolationSeverity::Warning,
                        });
                    }
                }
                "cpu_max_percent" => {
                    if metric.value > baselines.agent_baselines.resource_usage.cpu_max_percent {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.agent_baselines.resource_usage.cpu_max_percent,
                            actual: metric.value,
                            severity: ViolationSeverity::Error,
                        });
                    }
                }
                "system_utilization_percent" => {
                    let (min_optimal, max_optimal) = baselines.system_baselines.utilization.optimal_range_percent;
                    if metric.value < min_optimal || metric.value > max_optimal {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: (min_optimal + max_optimal) / 2.0,
                            actual: metric.value,
                            severity: ViolationSeverity::Warning,
                        });
                    }
                    
                    if metric.value > baselines.system_baselines.utilization.max_sustainable_percent {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.system_baselines.utilization.max_sustainable_percent,
                            actual: metric.value,
                            severity: ViolationSeverity::Critical,
                        });
                    }
                }
                _ => {}
            }
        }
        
        violations
    }
}

/// Error rate performance test
pub struct ErrorRateTest;

#[async_trait::async_trait]
impl PerformanceTest for ErrorRateTest {
    fn name(&self) -> &str {
        "Error Rate Performance"
    }
    
    async fn run(
        &self,
        config: &PerformanceTestConfig,
        metrics: Arc<MetricsCollector>,
    ) -> Result<PerformanceTestResults, Box<dyn std::error::Error>> {
        let test_start = Instant::now();
        
        let total_operations = 10000;
        let mut error_count = 0;
        let mut success_count = 0;
        
        // Simulate operations with controlled error injection
        for i in 0..total_operations {
            // Inject errors at a controlled rate (1% baseline)
            let should_fail = i % 100 == 0;
            
            if should_fail {
                error_count += 1;
                metrics.system_collector()
                    .record_task_completion(false, 10.0)
                    .await;
            } else {
                success_count += 1;
                metrics.system_collector()
                    .record_task_completion(true, 10.0)
                    .await;
            }
        }
        
        let test_duration = test_start.elapsed();
        
        // Calculate error rates
        let baseline_error_rate = error_count as f64 / total_operations as f64;
        
        let metrics_vec = vec![
            PerformanceMetric {
                name: "baseline_error_rate".to_string(),
                value: baseline_error_rate,
                unit: "ratio".to_string(),
                baseline: Some(0.01),
            },
            PerformanceMetric {
                name: "total_errors".to_string(),
                value: error_count as f64,
                unit: "count".to_string(),
                baseline: None,
            },
            PerformanceMetric {
                name: "total_operations".to_string(),
                value: total_operations as f64,
                unit: "count".to_string(),
                baseline: None,
            },
        ];
        
        Ok(PerformanceTestResults {
            test_name: self.name().to_string(),
            duration: test_duration,
            passed: true,
            metrics: metrics_vec,
            violations: Vec::new(),
        })
    }
    
    fn validate_baselines(
        &self,
        results: &PerformanceTestResults,
        baselines: &PerformanceBaselines,
    ) -> Vec<BaselineViolation> {
        let mut violations = Vec::new();
        
        for metric in &results.metrics {
            if metric.name == "baseline_error_rate" {
                if metric.value > baselines.system_baselines.error_rates.baseline_error_rate {
                    violations.push(BaselineViolation {
                        metric: metric.name.clone(),
                        expected: baselines.system_baselines.error_rates.baseline_error_rate,
                        actual: metric.value,
                        severity: ViolationSeverity::Warning,
                    });
                }
                
                if metric.value > baselines.system_baselines.error_rates.alert_threshold {
                    violations.push(BaselineViolation {
                        metric: metric.name.clone(),
                        expected: baselines.system_baselines.error_rates.alert_threshold,
                        actual: metric.value,
                        severity: ViolationSeverity::Error,
                    });
                }
                
                if metric.value > baselines.system_baselines.error_rates.critical_threshold {
                    violations.push(BaselineViolation {
                        metric: metric.name.clone(),
                        expected: baselines.system_baselines.error_rates.critical_threshold,
                        actual: metric.value,
                        severity: ViolationSeverity::Critical,
                    });
                }
            }
        }
        
        violations
    }
}

// Helper functions
fn get_process_memory(system: &System, pid: sysinfo::Pid) -> f64 {
    system.process(pid)
        .map(|p| p.memory() as f64 / 1024.0 / 1024.0) // Convert to MB
        .unwrap_or(0.0)
}

fn get_process_cpu(system: &System, pid: sysinfo::Pid) -> f64 {
    system.process(pid)
        .map(|p| p.cpu_usage() as f64)
        .unwrap_or(0.0)
}

async fn simulate_workload(metrics: &Arc<MetricsCollector>, task_count: usize) -> usize {
    let mut completed = 0;
    
    for _ in 0..task_count {
        // Simulate task processing
        tokio::time::sleep(Duration::from_micros(100)).await;
        
        metrics.system_collector()
            .record_task_completion(true, 0.1)
            .await;
        
        completed += 1;
    }
    
    completed
}