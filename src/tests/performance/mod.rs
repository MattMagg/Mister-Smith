//! Performance testing module for baseline validation

pub mod agent_performance;
pub mod message_throughput;
pub mod system_load;

use crate::metrics::{MetricsCollector, PerformanceBaselines};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;

/// Performance test configuration
pub struct PerformanceTestConfig {
    pub duration: Duration,
    pub concurrent_agents: usize,
    pub message_rate: u64,
    pub baselines: PerformanceBaselines,
}

impl Default for PerformanceTestConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(60),
            concurrent_agents: 10,
            message_rate: 1000,
            baselines: PerformanceBaselines::default(),
        }
    }
}

/// Performance test results
#[derive(Debug, Clone)]
pub struct PerformanceTestResults {
    pub test_name: String,
    pub duration: Duration,
    pub passed: bool,
    pub metrics: Vec<PerformanceMetric>,
    pub violations: Vec<BaselineViolation>,
}

#[derive(Debug, Clone)]
pub struct PerformanceMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub baseline: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct BaselineViolation {
    pub metric: String,
    pub expected: f64,
    pub actual: f64,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, Copy)]
pub enum ViolationSeverity {
    Warning,
    Error,
    Critical,
}

/// Base trait for performance tests
#[async_trait::async_trait]
pub trait PerformanceTest {
    /// Test name
    fn name(&self) -> &str;
    
    /// Run the performance test
    async fn run(
        &self,
        config: &PerformanceTestConfig,
        metrics: Arc<MetricsCollector>,
    ) -> Result<PerformanceTestResults, Box<dyn std::error::Error>>;
    
    /// Validate results against baselines
    fn validate_baselines(
        &self,
        results: &PerformanceTestResults,
        baselines: &PerformanceBaselines,
    ) -> Vec<BaselineViolation>;
}

/// Performance test runner
pub struct PerformanceTestRunner {
    tests: Vec<Box<dyn PerformanceTest>>,
    config: PerformanceTestConfig,
    metrics: Arc<MetricsCollector>,
}

impl PerformanceTestRunner {
    pub fn new(
        config: PerformanceTestConfig,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            tests: Vec::new(),
            config,
            metrics,
        }
    }
    
    /// Add a test to the runner
    pub fn add_test(&mut self, test: Box<dyn PerformanceTest>) {
        self.tests.push(test);
    }
    
    /// Run all performance tests
    pub async fn run_all(&self) -> Vec<PerformanceTestResults> {
        let mut results = Vec::new();
        
        for test in &self.tests {
            info!("Running performance test: {}", test.name());
            
            match test.run(&self.config, self.metrics.clone()).await {
                Ok(mut result) => {
                    // Validate against baselines
                    let violations = test.validate_baselines(&result, &self.config.baselines);
                    result.violations = violations;
                    result.passed = result.violations.is_empty();
                    
                    results.push(result);
                }
                Err(e) => {
                    eprintln!("Performance test {} failed: {}", test.name(), e);
                }
            }
        }
        
        results
    }
    
    /// Generate performance report
    pub fn generate_report(&self, results: &[PerformanceTestResults]) -> String {
        let mut report = String::from("# Performance Test Report\n\n");
        
        for result in results {
            report.push_str(&format!("## Test: {}\n", result.test_name));
            report.push_str(&format!("Duration: {:?}\n", result.duration));
            report.push_str(&format!("Status: {}\n\n", if result.passed { "PASSED" } else { "FAILED" }));
            
            report.push_str("### Metrics\n");
            for metric in &result.metrics {
                report.push_str(&format!(
                    "- {}: {} {} ",
                    metric.name, metric.value, metric.unit
                ));
                
                if let Some(baseline) = metric.baseline {
                    report.push_str(&format!("(baseline: {} {})", baseline, metric.unit));
                }
                report.push_str("\n");
            }
            
            if !result.violations.is_empty() {
                report.push_str("\n### Baseline Violations\n");
                for violation in &result.violations {
                    report.push_str(&format!(
                        "- {:?}: {} - expected: {}, actual: {}\n",
                        violation.severity, violation.metric, violation.expected, violation.actual
                    ));
                }
            }
            
            report.push_str("\n");
        }
        
        report
    }
}