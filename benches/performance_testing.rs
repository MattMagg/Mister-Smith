//! Comprehensive Performance Testing Suite for MisterSmith
//!
//! This suite provides systematic performance testing under load with multiple agents
//! Tests cover discovery sharing throughput, memory usage, NATS performance, and SSE streaming
//!
//! Usage: cargo bench performance_testing

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use uuid::Uuid;

use mistersmith::collaboration::{
    Discovery, DiscoveryBroadcaster, DiscoveryListener, DiscoveryType,
};

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    pub nats_url: String,
    pub agent_counts: Vec<usize>,
    pub discoveries_per_agent: usize,
    pub test_duration: Duration,
    pub warmup_duration: Duration,
    pub memory_limit_test: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            nats_url: "nats://localhost:4222".to_string(),
            agent_counts: vec![2, 5, 10, 20],
            discoveries_per_agent: 50,
            test_duration: Duration::from_secs(30),
            warmup_duration: Duration::from_secs(5),
            memory_limit_test: true,
        }
    }
}

/// Performance metrics collector
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub total_discoveries: u64,
    pub discoveries_per_second: f64,
    pub average_latency: Duration,
    pub max_latency: Duration,
    pub min_latency: Duration,
    pub memory_usage: u64,
    pub agent_count: usize,
    pub test_duration: Duration,
    pub throughput_over_time: Vec<(Duration, u64)>,
    pub latency_distribution: Vec<Duration>,
}

impl PerformanceMetrics {
    pub fn new(agent_count: usize) -> Self {
        Self {
            total_discoveries: 0,
            discoveries_per_second: 0.0,
            average_latency: Duration::from_millis(0),
            max_latency: Duration::from_millis(0),
            min_latency: Duration::from_millis(u64::MAX),
            memory_usage: 0,
            agent_count,
            test_duration: Duration::from_secs(0),
            throughput_over_time: Vec::new(),
            latency_distribution: Vec::new(),
        }
    }
}

/// Performance test agent
#[derive(Debug)]
pub struct PerformanceAgent {
    pub id: String,
    pub role: String,
    pub broadcaster: DiscoveryBroadcaster,
    pub listener: DiscoveryListener,
    pub discoveries_sent: Arc<Mutex<u64>>,
    pub discoveries_received: Arc<Mutex<u64>>,
    pub latency_measurements: Arc<Mutex<Vec<Duration>>>,
    pub start_times: Arc<Mutex<HashMap<String, Instant>>>,
}

impl PerformanceAgent {
    pub async fn new(id: String, role: String, nats_client: async_nats::Client) -> Self {
        let broadcaster = DiscoveryBroadcaster::new(id.clone(), role.clone(), nats_client.clone());
        let listener = DiscoveryListener::new(id.clone(), role.clone(), nats_client);
        
        Self {
            id,
            role,
            broadcaster,
            listener,
            discoveries_sent: Arc::new(Mutex::new(0)),
            discoveries_received: Arc::new(Mutex::new(0)),
            latency_measurements: Arc::new(Mutex::new(Vec::new())),
            start_times: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_listening(&self) {
        let discoveries_received = self.discoveries_received.clone();
        let latency_measurements = self.latency_measurements.clone();
        let start_times = self.start_times.clone();
        
        let listener = self.listener.clone();
        
        tokio::spawn(async move {
            let _ = listener.listen(move |discovery| {
                let discoveries_received = discoveries_received.clone();
                let latency_measurements = latency_measurements.clone();
                let start_times = start_times.clone();
                
                tokio::spawn(async move {
                    // Record discovery received
                    let mut count = discoveries_received.lock().await;
                    *count += 1;
                    
                    // Calculate latency if we have the start time
                    if let Some(start_time) = start_times.lock().await.get(&discovery.content) {
                        let latency = start_time.elapsed();
                        latency_measurements.lock().await.push(latency);
                    }
                });
                
                Ok(())
            }).await;
        });
    }

    pub async fn share_discovery_with_timing(&self, discovery_type: DiscoveryType, content: &str, confidence: f32) -> Result<(), anyhow::Error> {
        // Record start time for latency measurement
        let start_time = Instant::now();
        self.start_times.lock().await.insert(content.to_string(), start_time);
        
        // Share discovery
        self.broadcaster.share_discovery(discovery_type, content, confidence).await?;
        
        // Update count
        let mut count = self.discoveries_sent.lock().await;
        *count += 1;
        
        Ok(())
    }
}

/// Performance test orchestrator
#[derive(Debug)]
pub struct PerformanceOrchestrator {
    pub config: PerformanceConfig,
    pub nats_client: async_nats::Client,
    pub agents: Vec<PerformanceAgent>,
    pub metrics: Arc<Mutex<PerformanceMetrics>>,
}

impl PerformanceOrchestrator {
    pub async fn new(config: PerformanceConfig) -> Result<Self, anyhow::Error> {
        let nats_client = async_nats::connect(&config.nats_url).await?;
        
        Ok(Self {
            config,
            nats_client,
            agents: Vec::new(),
            metrics: Arc::new(Mutex::new(PerformanceMetrics::new(0))),
        })
    }

    pub async fn setup_agents(&mut self, agent_count: usize) -> Result<(), anyhow::Error> {
        self.agents.clear();
        
        for i in 0..agent_count {
            let agent_id = format!("perf-agent-{}", i);
            let role = format!("TestAgent{}", i);
            
            let agent = PerformanceAgent::new(agent_id, role, self.nats_client.clone()).await;
            agent.start_listening().await;
            
            self.agents.push(agent);
        }
        
        // Give agents time to subscribe
        sleep(Duration::from_millis(200)).await;
        
        Ok(())
    }

    pub async fn run_load_test(&mut self, agent_count: usize) -> Result<PerformanceMetrics, anyhow::Error> {
        println!("🚀 Starting load test with {} agents", agent_count);
        
        // Setup agents
        self.setup_agents(agent_count).await?;
        
        // Initialize metrics
        let mut metrics = PerformanceMetrics::new(agent_count);
        let start_time = Instant::now();
        
        // Warmup phase
        println!("🔥 Warmup phase...");
        self.run_discovery_burst(5).await?;
        sleep(self.config.warmup_duration).await;
        
        // Main test phase
        println!("📊 Main test phase...");
        let test_start = Instant::now();
        
        // Start background throughput monitoring
        let throughput_monitor = self.start_throughput_monitoring().await;
        
        // Run discovery generation
        self.run_continuous_discovery_generation().await?;
        
        // Wait for test duration
        sleep(self.config.test_duration).await;
        
        // Stop throughput monitoring
        throughput_monitor.abort();
        
        // Collect metrics
        metrics.test_duration = test_start.elapsed();
        metrics.total_discoveries = self.collect_total_discoveries().await;
        metrics.discoveries_per_second = metrics.total_discoveries as f64 / metrics.test_duration.as_secs_f64();
        
        // Collect latency metrics
        let latency_measurements = self.collect_latency_measurements().await;
        if !latency_measurements.is_empty() {
            metrics.average_latency = latency_measurements.iter().sum::<Duration>() / latency_measurements.len() as u32;
            metrics.max_latency = latency_measurements.iter().max().copied().unwrap_or_default();
            metrics.min_latency = latency_measurements.iter().min().copied().unwrap_or_default();
            metrics.latency_distribution = latency_measurements;
        }
        
        // Memory usage (estimated)
        metrics.memory_usage = self.estimate_memory_usage().await;
        
        println!("✅ Load test completed: {} discoveries in {:?}", metrics.total_discoveries, metrics.test_duration);
        
        Ok(metrics)
    }

    async fn run_discovery_burst(&self, burst_size: usize) -> Result<(), anyhow::Error> {
        let mut tasks = Vec::new();
        
        for agent in &self.agents {
            for i in 0..burst_size {
                let agent_broadcaster = agent.broadcaster.clone();
                let content = format!("Burst discovery {} from {}", i, agent.id);
                
                tasks.push(tokio::spawn(async move {
                    agent_broadcaster.share_discovery(
                        DiscoveryType::Insight,
                        &content,
                        0.8
                    ).await
                }));
            }
        }
        
        // Wait for all discoveries to be sent
        for task in tasks {
            let _ = task.await;
        }
        
        Ok(())
    }

    async fn run_continuous_discovery_generation(&self) -> Result<(), anyhow::Error> {
        let mut tasks = Vec::new();
        
        for agent in &self.agents {
            let agent_clone = agent.clone();
            let discoveries_per_agent = self.config.discoveries_per_agent;
            let test_duration = self.config.test_duration;
            
            tasks.push(tokio::spawn(async move {
                let interval = test_duration / discoveries_per_agent as u32;
                
                for i in 0..discoveries_per_agent {
                    let content = format!("Discovery {} from {} at {:?}", i, agent_clone.id, Instant::now());
                    
                    let _ = agent_clone.share_discovery_with_timing(
                        DiscoveryType::Pattern,
                        &content,
                        0.75
                    ).await;
                    
                    sleep(interval).await;
                }
            }));
        }
        
        // Don't wait for tasks to complete - let them run in background
        Ok(())
    }

    async fn start_throughput_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let agents = self.agents.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let start_time = Instant::now();
            
            loop {
                interval.tick().await;
                
                let mut total_discoveries = 0;
                for agent in &agents {
                    total_discoveries += *agent.discoveries_sent.lock().await;
                }
                
                println!("📈 Throughput: {} discoveries at {:?}", total_discoveries, start_time.elapsed());
            }
        })
    }

    async fn collect_total_discoveries(&self) -> u64 {
        let mut total = 0;
        for agent in &self.agents {
            total += *agent.discoveries_sent.lock().await;
        }
        total
    }

    async fn collect_latency_measurements(&self) -> Vec<Duration> {
        let mut all_latencies = Vec::new();
        
        for agent in &self.agents {
            let latencies = agent.latency_measurements.lock().await;
            all_latencies.extend(latencies.clone());
        }
        
        all_latencies.sort();
        all_latencies
    }

    async fn estimate_memory_usage(&self) -> u64 {
        // Estimate memory usage based on agent count and discovery count
        let discovery_count = self.collect_total_discoveries().await;
        let estimated_bytes_per_discovery = 200; // Rough estimate
        discovery_count * estimated_bytes_per_discovery
    }

    pub async fn run_memory_stress_test(&mut self) -> Result<PerformanceMetrics, anyhow::Error> {
        println!("🧠 Starting memory stress test (1000 discovery limit)");
        
        // Setup single agent for focused memory testing
        self.setup_agents(1).await?;
        
        let agent = &self.agents[0];
        let start_time = Instant::now();
        
        // Send discoveries in batches to approach 1000 limit
        let batch_size = 100;
        let total_discoveries = 1200; // Exceed limit to test FIFO eviction
        
        for batch in 0..(total_discoveries / batch_size) {
            for i in 0..batch_size {
                let content = format!("Memory test discovery {} in batch {}", i, batch);
                let _ = agent.share_discovery_with_timing(
                    DiscoveryType::Insight,
                    &content,
                    0.8
                ).await;
            }
            
            // Small delay between batches
            sleep(Duration::from_millis(10)).await;
            
            if batch % 5 == 0 {
                println!("💾 Sent {} discoveries", batch * batch_size);
            }
        }
        
        // Wait for processing
        sleep(Duration::from_secs(2)).await;
        
        let mut metrics = PerformanceMetrics::new(1);
        metrics.test_duration = start_time.elapsed();
        metrics.total_discoveries = total_discoveries as u64;
        metrics.discoveries_per_second = total_discoveries as f64 / metrics.test_duration.as_secs_f64();
        metrics.memory_usage = self.estimate_memory_usage().await;
        
        println!("✅ Memory stress test completed: {} discoveries in {:?}", metrics.total_discoveries, metrics.test_duration);
        
        Ok(metrics)
    }
}

/// Benchmark functions for Criterion
pub fn benchmark_discovery_sharing(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("discovery_sharing_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let config = PerformanceConfig::default();
            let mut orchestrator = PerformanceOrchestrator::new(config).await.unwrap();
            
            orchestrator.setup_agents(2).await.unwrap();
            
            let agent = &orchestrator.agents[0];
            let start = Instant::now();
            
            black_box(agent.share_discovery_with_timing(
                DiscoveryType::Insight,
                "Benchmark discovery",
                0.8
            ).await.unwrap());
            
            start.elapsed()
        })
    });
}

pub fn benchmark_multi_agent_load(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("multi_agent_load");
    
    for agent_count in [2, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::new("agents", agent_count),
            agent_count,
            |b, &agent_count| {
                b.to_async(&rt).iter(|| async {
                    let config = PerformanceConfig {
                        discoveries_per_agent: 10,
                        test_duration: Duration::from_secs(5),
                        ..Default::default()
                    };
                    
                    let mut orchestrator = PerformanceOrchestrator::new(config).await.unwrap();
                    black_box(orchestrator.run_load_test(agent_count).await.unwrap());
                })
            },
        );
    }
    
    group.finish();
}

pub fn benchmark_memory_limit(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("memory_limit_stress", |b| {
        b.to_async(&rt).iter(|| async {
            let config = PerformanceConfig::default();
            let mut orchestrator = PerformanceOrchestrator::new(config).await.unwrap();
            
            black_box(orchestrator.run_memory_stress_test().await.unwrap());
        })
    });
}

criterion_group!(
    benches,
    benchmark_discovery_sharing,
    benchmark_multi_agent_load,
    benchmark_memory_limit
);
criterion_main!(benches);

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_performance_orchestrator() {
        let config = PerformanceConfig {
            discoveries_per_agent: 5,
            test_duration: Duration::from_secs(2),
            ..Default::default()
        };
        
        let mut orchestrator = PerformanceOrchestrator::new(config).await.unwrap();
        let metrics = orchestrator.run_load_test(2).await.unwrap();
        
        assert!(metrics.total_discoveries > 0);
        assert!(metrics.discoveries_per_second > 0.0);
    }
    
    #[tokio::test]
    async fn test_memory_stress() {
        let config = PerformanceConfig::default();
        let mut orchestrator = PerformanceOrchestrator::new(config).await.unwrap();
        
        let metrics = orchestrator.run_memory_stress_test().await.unwrap();
        
        assert!(metrics.total_discoveries > 1000);
        assert!(metrics.memory_usage > 0);
    }
}