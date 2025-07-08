use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::collections::HashMap;
use anyhow::Result;
use tokio::sync::RwLock;

/// Test data generators for various scenarios
pub struct TestDataGenerator {
    seed: u64,
}

impl TestDataGenerator {
    pub fn new() -> Self {
        Self { seed: 42 }
    }

    pub fn with_seed(seed: u64) -> Self {
        Self { seed }
    }

    /// Generate agent configurations
    pub fn generate_agent_configs(&mut self, count: usize) -> Vec<mistersmith::agent::AgentConfig> {
        let agent_types = vec!["research", "code", "analysis", "communication"];
        let capabilities = HashMap::from([
            ("research", vec!["search", "summarize", "analyze"]),
            ("code", vec!["write", "review", "debug", "test"]),
            ("analysis", vec!["data", "metrics", "report"]),
            ("communication", vec!["email", "chat", "document"]),
        ]);

        (0..count).map(|i| {
            let agent_type = agent_types[i % agent_types.len()];
            mistersmith::agent::AgentConfig {
                id: mistersmith::agent::AgentId::from(format!("test-agent-{}", i)),
                agent_type: agent_type.to_string(),
                capabilities: capabilities[agent_type].iter()
                    .map(|s| s.to_string())
                    .collect(),
                resource_limits: mistersmith::agent::ResourceLimits {
                    max_memory_mb: 512,
                    max_cpu_percent: 50.0,
                    max_file_handles: 100,
                },
            }
        }).collect()
    }

    /// Generate test tasks
    pub fn generate_tasks(&mut self, count: usize) -> Vec<mistersmith::task::Task> {
        let task_types = vec![
            mistersmith::task::TaskType::Research,
            mistersmith::task::TaskType::Code,
            mistersmith::task::TaskType::Analysis,
            mistersmith::task::TaskType::Communication,
        ];

        let descriptions = vec![
            "Research latest async patterns in Rust",
            "Implement retry logic for network requests",
            "Analyze performance metrics from last week",
            "Draft email summarizing project status",
        ];

        (0..count).map(|i| {
            let task_type = task_types[i % task_types.len()].clone();
            let description = descriptions[i % descriptions.len()];
            
            let mut task = mistersmith::task::Task::new(task_type, description);
            
            // Add variety to tasks
            if i % 3 == 0 {
                task = task.with_priority(mistersmith::task::TaskPriority::High);
            }
            if i % 5 == 0 {
                task = task.with_deadline(
                    chrono::Utc::now() + chrono::Duration::hours((i % 24) as i64)
                );
            }
            
            task
        }).collect()
    }

    /// Generate failure scenarios
    pub fn generate_failures(&mut self, count: usize) -> Vec<mistersmith::supervision::FailureReason> {
        use mistersmith::supervision::FailureReason;
        
        let failures = vec![
            FailureReason::ProcessCrash {
                exit_code: -11,
                message: "Segmentation fault".to_string(),
            },
            FailureReason::TaskExecutionError {
                task_id: "task-123".to_string(),
                error: "Timeout waiting for response".to_string(),
            },
            FailureReason::ResourceExhaustion {
                resource_type: "memory".to_string(),
                limit: 512,
                requested: 1024,
            },
            FailureReason::UnrecoverableError {
                error_type: "ConfigurationError".to_string(),
                message: "Invalid API credentials".to_string(),
            },
        ];

        (0..count).map(|i| failures[i % failures.len()].clone()).collect()
    }
}

/// Test environment manager
pub struct TestEnvironmentManager {
    containers: Vec<testcontainers::Container<'static, Box<dyn testcontainers::Image>>>,
    cleanup_tasks: Vec<Box<dyn FnOnce() + Send>>,
}

impl TestEnvironmentManager {
    pub fn new() -> Self {
        Self {
            containers: Vec::new(),
            cleanup_tasks: Vec::new(),
        }
    }

    pub async fn setup_full_environment(&mut self) -> Result<TestEnvironmentConfig> {
        let docker = testcontainers::clients::Cli::default();
        
        // PostgreSQL
        let postgres = testcontainers::images::postgres::Postgres::default();
        let postgres_container = docker.run(postgres);
        let database_url = format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            postgres_container.get_host_port_ipv4(5432)
        );

        // NATS
        let nats = testcontainers::images::generic::GenericImage::new("nats", "latest")
            .with_exposed_port(4222);
        let nats_container = docker.run(nats);
        let nats_url = format!(
            "nats://localhost:{}",
            nats_container.get_host_port_ipv4(4222)
        );

        // Redis (for future use)
        let redis = testcontainers::images::redis::Redis::default();
        let redis_container = docker.run(redis);
        let redis_url = format!(
            "redis://localhost:{}",
            redis_container.get_host_port_ipv4(6379)
        );

        Ok(TestEnvironmentConfig {
            database_url,
            nats_url,
            redis_url,
        })
    }

    pub fn add_cleanup_task<F>(&mut self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.cleanup_tasks.push(Box::new(task));
    }

    pub fn cleanup(self) {
        for task in self.cleanup_tasks {
            task();
        }
    }
}

pub struct TestEnvironmentConfig {
    pub database_url: String,
    pub nats_url: String,
    pub redis_url: String,
}

/// Test metrics collector
pub struct TestMetricsCollector {
    metrics: Arc<RwLock<TestMetrics>>,
}

#[derive(Default)]
pub struct TestMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub operation_durations: Vec<std::time::Duration>,
    pub error_types: HashMap<String, u32>,
}

impl TestMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(TestMetrics::default())),
        }
    }

    pub async fn record_operation<F, T>(&self, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start = std::time::Instant::now();
        let result = operation.await;
        let duration = start.elapsed();

        let mut metrics = self.metrics.write().await;
        metrics.total_operations += 1;
        metrics.operation_durations.push(duration);

        match &result {
            Ok(_) => metrics.successful_operations += 1,
            Err(e) => {
                metrics.failed_operations += 1;
                let error_type = e.to_string().split_whitespace().next()
                    .unwrap_or("Unknown")
                    .to_string();
                *metrics.error_types.entry(error_type).or_insert(0) += 1;
            }
        }

        result
    }

    pub async fn get_summary(&self) -> TestMetricsSummary {
        let metrics = self.metrics.read().await;
        
        let avg_duration = if metrics.operation_durations.is_empty() {
            std::time::Duration::ZERO
        } else {
            let total: std::time::Duration = metrics.operation_durations.iter().sum();
            total / metrics.operation_durations.len() as u32
        };

        let success_rate = if metrics.total_operations > 0 {
            metrics.successful_operations as f64 / metrics.total_operations as f64
        } else {
            0.0
        };

        TestMetricsSummary {
            total_operations: metrics.total_operations,
            success_rate,
            average_duration: avg_duration,
            error_distribution: metrics.error_types.clone(),
        }
    }
}

pub struct TestMetricsSummary {
    pub total_operations: u64,
    pub success_rate: f64,
    pub average_duration: std::time::Duration,
    pub error_distribution: HashMap<String, u32>,
}

/// Mock implementations for testing
pub mod mocks {
    use super::*;

    /// Enhanced mock Claude executor with configurable behavior
    pub struct ConfigurableMockExecutor {
        responses: Vec<String>,
        response_index: AtomicU32,
        failure_pattern: FailurePattern,
        delay: std::time::Duration,
    }

    pub enum FailurePattern {
        Never,
        Always,
        FirstN(u32),
        EveryNth(u32),
        Random(f64),
    }

    impl ConfigurableMockExecutor {
        pub fn new() -> Self {
            Self {
                responses: vec!["Mock response".to_string()],
                response_index: AtomicU32::new(0),
                failure_pattern: FailurePattern::Never,
                delay: std::time::Duration::ZERO,
            }
        }

        pub fn with_responses(mut self, responses: Vec<String>) -> Self {
            self.responses = responses;
            self
        }

        pub fn with_failure_pattern(mut self, pattern: FailurePattern) -> Self {
            self.failure_pattern = pattern;
            self
        }

        pub fn with_delay(mut self, delay: std::time::Duration) -> Self {
            self.delay = delay;
            self
        }

        fn should_fail(&self, call_count: u32) -> bool {
            match &self.failure_pattern {
                FailurePattern::Never => false,
                FailurePattern::Always => true,
                FailurePattern::FirstN(n) => call_count < *n,
                FailurePattern::EveryNth(n) => call_count % n == 0,
                FailurePattern::Random(probability) => rand::random::<f64>() < *probability,
            }
        }
    }

    #[async_trait::async_trait]
    impl mistersmith::runtime::claude_executor::ClaudeExecutor for ConfigurableMockExecutor {
        async fn execute(&self, _prompt: &str) -> Result<String> {
            let call_count = self.response_index.fetch_add(1, Ordering::SeqCst);
            
            if self.delay > std::time::Duration::ZERO {
                tokio::time::sleep(self.delay).await;
            }

            if self.should_fail(call_count) {
                return Err(anyhow::anyhow!("Mock executor failure"));
            }

            let response_idx = (call_count as usize) % self.responses.len();
            Ok(self.responses[response_idx].clone())
        }
    }
}

/// Test assertion helpers
pub mod assertions {
    use super::*;

    /// Assert that an operation completes within a timeout
    pub async fn assert_completes_within<F, T>(
        timeout: std::time::Duration,
        future: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = T>,
    {
        tokio::time::timeout(timeout, future)
            .await
            .map_err(|_| anyhow::anyhow!("Operation timed out after {:?}", timeout))
    }

    /// Assert that a condition becomes true within a timeout
    pub async fn assert_eventually_true<F>(
        condition: F,
        timeout: std::time::Duration,
        check_interval: std::time::Duration,
    ) -> Result<()>
    where
        F: Fn() -> bool,
    {
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            if condition() {
                return Ok(());
            }
            tokio::time::sleep(check_interval).await;
        }
        
        Err(anyhow::anyhow!("Condition not met within {:?}", timeout))
    }

    /// Assert that all items in a collection satisfy a predicate
    pub fn assert_all<T, F>(items: &[T], predicate: F) -> Result<()>
    where
        F: Fn(&T) -> bool,
    {
        for (i, item) in items.iter().enumerate() {
            if !predicate(item) {
                return Err(anyhow::anyhow!("Item at index {} failed predicate", i));
            }
        }
        Ok(())
    }
}