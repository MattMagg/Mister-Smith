use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use anyhow::Result;
use tokio::sync::RwLock;

/// Chaos injection utilities for testing failure scenarios
pub struct ChaosInjector {
    network_partition: Arc<AtomicBool>,
    database_failure: Arc<AtomicBool>,
    nats_failure: Arc<AtomicBool>,
    resource_exhaustion: Arc<AtomicBool>,
    failure_probability: Arc<RwLock<f64>>,
}

impl ChaosInjector {
    pub fn new() -> Self {
        Self {
            network_partition: Arc::new(AtomicBool::new(false)),
            database_failure: Arc::new(AtomicBool::new(false)),
            nats_failure: Arc::new(AtomicBool::new(false)),
            resource_exhaustion: Arc::new(AtomicBool::new(false)),
            failure_probability: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Enable network partition between agents
    pub fn inject_network_partition(&self, duration: Duration) {
        let partition = self.network_partition.clone();
        partition.store(true, Ordering::SeqCst);
        
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            partition.store(false, Ordering::SeqCst);
        });
    }

    /// Simulate database connection failure
    pub fn inject_database_failure(&self, duration: Duration) {
        let failure = self.database_failure.clone();
        failure.store(true, Ordering::SeqCst);
        
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            failure.store(false, Ordering::SeqCst);
        });
    }

    /// Simulate NATS messaging failure
    pub fn inject_nats_failure(&self, duration: Duration) {
        let failure = self.nats_failure.clone();
        failure.store(true, Ordering::SeqCst);
        
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            failure.store(false, Ordering::SeqCst);
        });
    }

    /// Simulate resource exhaustion
    pub fn inject_resource_exhaustion(&self, duration: Duration) {
        let exhaustion = self.resource_exhaustion.clone();
        exhaustion.store(true, Ordering::SeqCst);
        
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            exhaustion.store(false, Ordering::SeqCst);
        });
    }

    /// Set random failure probability (0.0 - 1.0)
    pub async fn set_failure_probability(&self, probability: f64) {
        *self.failure_probability.write().await = probability;
    }

    /// Check if a network operation should fail
    pub fn should_fail_network(&self) -> bool {
        self.network_partition.load(Ordering::SeqCst)
    }

    /// Check if a database operation should fail
    pub fn should_fail_database(&self) -> bool {
        self.database_failure.load(Ordering::SeqCst)
    }

    /// Check if a NATS operation should fail
    pub fn should_fail_nats(&self) -> bool {
        self.nats_failure.load(Ordering::SeqCst)
    }

    /// Check if resources are exhausted
    pub fn is_resource_exhausted(&self) -> bool {
        self.resource_exhaustion.load(Ordering::SeqCst)
    }

    /// Check if a random failure should occur
    pub async fn should_random_fail(&self) -> bool {
        let probability = *self.failure_probability.read().await;
        if probability > 0.0 {
            rand::random::<f64>() < probability
        } else {
            false
        }
    }

    /// Reset all chaos conditions
    pub async fn reset(&self) {
        self.network_partition.store(false, Ordering::SeqCst);
        self.database_failure.store(false, Ordering::SeqCst);
        self.nats_failure.store(false, Ordering::SeqCst);
        self.resource_exhaustion.store(false, Ordering::SeqCst);
        *self.failure_probability.write().await = 0.0;
    }
}

/// Extension for MockClaudeExecutor to support crash scenarios
impl super::test_builders::MockClaudeExecutor {
    pub fn with_crash_count(mut self, crash_count: u32, counter: Arc<AtomicU32>) -> Self {
        self.crash_count = Some((crash_count, counter));
        self
    }

    pub fn with_persistent_failure(mut self, message: &str) -> Self {
        self.persistent_failure = Some(message.to_string());
        self
    }
}

/// Chaos-aware database connection wrapper
pub struct ChaosDatabase {
    inner: Box<dyn mistersmith::persistence::SupervisionPersistence>,
    chaos: Arc<ChaosInjector>,
}

impl ChaosDatabase {
    pub fn new(
        inner: Box<dyn mistersmith::persistence::SupervisionPersistence>,
        chaos: Arc<ChaosInjector>,
    ) -> Self {
        Self { inner, chaos }
    }
}

#[async_trait::async_trait]
impl mistersmith::persistence::SupervisionPersistence for ChaosDatabase {
    async fn create_node(
        &self,
        node_id: &str,
        node_type: &str,
        parent_id: Option<&str>,
    ) -> Result<()> {
        if self.chaos.should_fail_database() {
            return Err(anyhow::anyhow!("Chaos: Database connection failed"));
        }
        self.inner.create_node(node_id, node_type, parent_id).await
    }

    async fn update_node_status(&self, node_id: &str, status: &str) -> Result<()> {
        if self.chaos.should_fail_database() {
            return Err(anyhow::anyhow!("Chaos: Database connection failed"));
        }
        self.inner.update_node_status(node_id, status).await
    }

    async fn record_failure(
        &self,
        node_id: &str,
        failure_type: &str,
        message: &str,
    ) -> Result<()> {
        if self.chaos.should_fail_database() {
            return Err(anyhow::anyhow!("Chaos: Database connection failed"));
        }
        self.inner.record_failure(node_id, failure_type, message).await
    }

    async fn get_recent_failures(
        &self,
        node_id: &str,
        window: Duration,
    ) -> Result<Vec<mistersmith::persistence::FailureRecord>> {
        if self.chaos.should_fail_database() {
            return Err(anyhow::anyhow!("Chaos: Database connection failed"));
        }
        self.inner.get_recent_failures(node_id, window).await
    }

    async fn get_supervision_tree(&self) -> Result<serde_json::Value> {
        if self.chaos.should_fail_database() {
            return Err(anyhow::anyhow!("Chaos: Database connection failed"));
        }
        self.inner.get_supervision_tree().await
    }
}

/// Chaos-aware NATS transport wrapper
pub struct ChaosNatsTransport {
    inner: Arc<mistersmith::transport::nats::NatsTransport>,
    chaos: Arc<ChaosInjector>,
}

impl ChaosNatsTransport {
    pub fn new(
        inner: Arc<mistersmith::transport::nats::NatsTransport>,
        chaos: Arc<ChaosInjector>,
    ) -> Self {
        Self { inner, chaos }
    }
}

#[async_trait::async_trait]
impl mistersmith::transport::Transport for ChaosNatsTransport {
    async fn publish(&self, subject: &str, message: &[u8]) -> Result<()> {
        if self.chaos.should_fail_nats() {
            return Err(anyhow::anyhow!("Chaos: NATS connection failed"));
        }
        if self.chaos.should_fail_network() {
            return Err(anyhow::anyhow!("Chaos: Network partition"));
        }
        self.inner.publish(subject, message).await
    }

    async fn subscribe(&self, subject: &str) -> Result<Box<dyn futures::Stream<Item = Vec<u8>>>> {
        if self.chaos.should_fail_nats() {
            return Err(anyhow::anyhow!("Chaos: NATS connection failed"));
        }
        if self.chaos.should_fail_network() {
            return Err(anyhow::anyhow!("Chaos: Network partition"));
        }
        self.inner.subscribe(subject).await
    }

    async fn request(&self, subject: &str, message: &[u8], timeout: Duration) -> Result<Vec<u8>> {
        if self.chaos.should_fail_nats() {
            return Err(anyhow::anyhow!("Chaos: NATS connection failed"));
        }
        if self.chaos.should_fail_network() {
            return Err(anyhow::anyhow!("Chaos: Network partition"));
        }
        self.inner.request(subject, message, timeout).await
    }
}

/// Utility to simulate CPU/memory pressure
pub struct ResourcePressure {
    threads: Vec<std::thread::JoinHandle<()>>,
    stop_signal: Arc<AtomicBool>,
}

impl ResourcePressure {
    pub fn new() -> Self {
        Self {
            threads: Vec::new(),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start consuming CPU resources
    pub fn start_cpu_pressure(&mut self, num_threads: usize) {
        for _ in 0..num_threads {
            let stop = self.stop_signal.clone();
            let handle = std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    // Busy loop to consume CPU
                    for _ in 0..1000000 {
                        std::hint::black_box(42);
                    }
                }
            });
            self.threads.push(handle);
        }
    }

    /// Start consuming memory
    pub fn start_memory_pressure(&mut self, mb_per_second: usize) {
        let stop = self.stop_signal.clone();
        let handle = std::thread::spawn(move || {
            let mut allocations = Vec::new();
            while !stop.load(Ordering::Relaxed) {
                // Allocate memory
                let allocation = vec![0u8; mb_per_second * 1024 * 1024];
                allocations.push(allocation);
                std::thread::sleep(Duration::from_secs(1));
            }
        });
        self.threads.push(handle);
    }

    /// Stop resource pressure
    pub fn stop(mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}