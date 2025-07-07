//! Agent performance tests

use super::{
    PerformanceTest, PerformanceTestConfig, PerformanceTestResults,
    PerformanceMetric, BaselineViolation, ViolationSeverity,
};
use crate::agent::{Agent, AgentConfig, AgentType};
use crate::metrics::{MetricsCollector, PerformanceBaselines};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::info;

/// Agent spawn performance test
pub struct AgentSpawnTest;

#[async_trait::async_trait]
impl PerformanceTest for AgentSpawnTest {
    fn name(&self) -> &str {
        "Agent Spawn Performance"
    }
    
    async fn run(
        &self,
        config: &PerformanceTestConfig,
        metrics: Arc<MetricsCollector>,
    ) -> Result<PerformanceTestResults, Box<dyn std::error::Error>> {
        let mut spawn_times = Vec::new();
        let mut success_count = 0;
        let mut total_count = 0;
        
        let test_start = Instant::now();
        
        // Spawn agents repeatedly
        for i in 0..config.concurrent_agents {
            let spawn_start = Instant::now();
            
            let agent_config = AgentConfig {
                id: format!("perf_test_agent_{}", i),
                agent_type: AgentType::Worker,
                ..Default::default()
            };
            
            match Agent::spawn(agent_config).await {
                Ok(_agent) => {
                    success_count += 1;
                    let spawn_duration = spawn_start.elapsed();
                    spawn_times.push(spawn_duration.as_secs_f64());
                    
                    // Record spawn metrics
                    metrics.system_collector().increment_active_agents().await;
                }
                Err(e) => {
                    eprintln!("Failed to spawn agent: {}", e);
                }
            }
            
            total_count += 1;
            
            // Small delay between spawns
            sleep(Duration::from_millis(100)).await;
        }
        
        let test_duration = test_start.elapsed();
        
        // Calculate percentiles
        spawn_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_index = (spawn_times.len() as f64 * 0.95) as usize;
        let p99_index = (spawn_times.len() as f64 * 0.99) as usize;
        
        let p95_duration = spawn_times.get(p95_index).copied().unwrap_or(0.0);
        let p99_duration = spawn_times.get(p99_index).copied().unwrap_or(0.0);
        let success_rate = (success_count as f64 / total_count as f64) * 100.0;
        
        let metrics_vec = vec![
            PerformanceMetric {
                name: "agent_spawn_p95".to_string(),
                value: p95_duration,
                unit: "seconds".to_string(),
                baseline: Some(2.0), // From baselines
            },
            PerformanceMetric {
                name: "agent_spawn_p99".to_string(),
                value: p99_duration,
                unit: "seconds".to_string(),
                baseline: Some(5.0), // From baselines
            },
            PerformanceMetric {
                name: "agent_spawn_success_rate".to_string(),
                value: success_rate,
                unit: "percent".to_string(),
                baseline: Some(99.5), // From baselines
            },
        ];
        
        Ok(PerformanceTestResults {
            test_name: self.name().to_string(),
            duration: test_duration,
            passed: true, // Will be set based on baseline validation
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
                "agent_spawn_p95" => {
                    if metric.value > baselines.agent_baselines.agent_spawn.p95_duration_seconds {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.agent_baselines.agent_spawn.p95_duration_seconds,
                            actual: metric.value,
                            severity: ViolationSeverity::Warning,
                        });
                    }
                }
                "agent_spawn_p99" => {
                    if metric.value > baselines.agent_baselines.agent_spawn.p99_duration_seconds {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.agent_baselines.agent_spawn.p99_duration_seconds,
                            actual: metric.value,
                            severity: ViolationSeverity::Error,
                        });
                    }
                }
                "agent_spawn_success_rate" => {
                    if metric.value < baselines.agent_baselines.agent_spawn.success_rate_percent {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.agent_baselines.agent_spawn.success_rate_percent,
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

/// Agent task completion performance test
pub struct AgentTaskCompletionTest;

#[async_trait::async_trait]
impl PerformanceTest for AgentTaskCompletionTest {
    fn name(&self) -> &str {
        "Agent Task Completion Performance"
    }
    
    async fn run(
        &self,
        config: &PerformanceTestConfig,
        metrics: Arc<MetricsCollector>,
    ) -> Result<PerformanceTestResults, Box<dyn std::error::Error>> {
        let mut completion_times = Vec::new();
        let mut success_count = 0;
        let test_start = Instant::now();
        
        // Create test agents
        let mut agents = Vec::new();
        for i in 0..5 {
            let agent_config = AgentConfig {
                id: format!("task_test_agent_{}", i),
                agent_type: AgentType::Worker,
                ..Default::default()
            };
            
            let agent = Agent::spawn(agent_config).await?;
            agents.push(agent);
        }
        
        // Run tasks
        let tasks_per_agent = 20;
        for agent in &agents {
            for _ in 0..tasks_per_agent {
                let task_start = Instant::now();
                
                // Simulate task execution
                sleep(Duration::from_millis(100)).await;
                
                let duration = task_start.elapsed().as_secs_f64();
                completion_times.push(duration);
                success_count += 1;
                
                // Record task completion
                metrics.system_collector()
                    .record_task_completion(true, duration * 1000.0)
                    .await;
            }
        }
        
        let test_duration = test_start.elapsed();
        
        // Calculate percentiles
        completion_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_index = (completion_times.len() as f64 * 0.95) as usize;
        let p99_index = (completion_times.len() as f64 * 0.99) as usize;
        
        let p95_duration = completion_times.get(p95_index).copied().unwrap_or(0.0);
        let p99_duration = completion_times.get(p99_index).copied().unwrap_or(0.0);
        let success_rate = (success_count as f64 / (agents.len() * tasks_per_agent) as f64) * 100.0;
        
        let metrics_vec = vec![
            PerformanceMetric {
                name: "task_completion_p95".to_string(),
                value: p95_duration,
                unit: "seconds".to_string(),
                baseline: Some(10.0),
            },
            PerformanceMetric {
                name: "task_completion_p99".to_string(),
                value: p99_duration,
                unit: "seconds".to_string(),
                baseline: Some(30.0),
            },
            PerformanceMetric {
                name: "task_success_rate".to_string(),
                value: success_rate,
                unit: "percent".to_string(),
                baseline: Some(99.0),
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
                "task_completion_p95" => {
                    if metric.value > baselines.agent_baselines.task_completion.p95_duration_seconds {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.agent_baselines.task_completion.p95_duration_seconds,
                            actual: metric.value,
                            severity: ViolationSeverity::Warning,
                        });
                    }
                }
                "task_completion_p99" => {
                    if metric.value > baselines.agent_baselines.task_completion.p99_duration_seconds {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.agent_baselines.task_completion.p99_duration_seconds,
                            actual: metric.value,
                            severity: ViolationSeverity::Error,
                        });
                    }
                }
                "task_success_rate" => {
                    if metric.value < baselines.agent_baselines.task_completion.success_rate_percent {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.agent_baselines.task_completion.success_rate_percent,
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