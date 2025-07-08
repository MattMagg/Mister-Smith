//! Comprehensive Performance Test Runner for MisterSmith
//!
//! This runner executes systematic performance tests with multiple agents
//! and generates detailed performance reports.
//!
//! Usage: cargo run --example performance_test_runner

use std::collections::HashMap;
use std::time::{Duration, Instant};
use anyhow::Result;
use tokio::time::sleep;
use uuid::Uuid;
use serde_json::json;

use mistersmith::collaboration::{
    DiscoveryBroadcaster, DiscoveryListener, DiscoveryType,
};

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerformanceTestConfig {
    pub nats_url: String,
    pub test_scenarios: Vec<TestScenario>,
    pub report_interval: Duration,
    pub output_file: Option<String>,
}

impl Default for PerformanceTestConfig {
    fn default() -> Self {
        Self {
            nats_url: "nats://localhost:4222".to_string(),
            test_scenarios: vec![
                TestScenario {
                    name: "Light Load".to_string(),
                    agent_count: 2,
                    discoveries_per_agent: 20,
                    test_duration: Duration::from_secs(10),
                    burst_mode: false,
                },
                TestScenario {
                    name: "Medium Load".to_string(),
                    agent_count: 5,
                    discoveries_per_agent: 50,
                    test_duration: Duration::from_secs(15),
                    burst_mode: false,
                },
                TestScenario {
                    name: "Heavy Load".to_string(),
                    agent_count: 10,
                    discoveries_per_agent: 100,
                    test_duration: Duration::from_secs(20),
                    burst_mode: false,
                },
                TestScenario {
                    name: "Burst Load".to_string(),
                    agent_count: 5,
                    discoveries_per_agent: 200,
                    test_duration: Duration::from_secs(10),
                    burst_mode: true,
                },
                TestScenario {
                    name: "Memory Stress".to_string(),
                    agent_count: 3,
                    discoveries_per_agent: 400,
                    test_duration: Duration::from_secs(25),
                    burst_mode: false,
                },
            ],
            report_interval: Duration::from_secs(1),
            output_file: Some("performance_report.json".to_string()),
        }
    }
}

/// Test scenario definition
#[derive(Debug, Clone)]
pub struct TestScenario {
    pub name: String,
    pub agent_count: usize,
    pub discoveries_per_agent: usize,
    pub test_duration: Duration,
    pub burst_mode: bool,
}

/// Performance metrics for a test run
#[derive(Debug, Clone)]
pub struct PerformanceResults {
    pub scenario_name: String,
    pub agent_count: usize,
    pub total_discoveries_sent: u64,
    pub total_discoveries_received: u64,
    pub test_duration: Duration,
    pub discoveries_per_second: f64,
    pub average_latency_ms: f64,
    pub max_latency_ms: f64,
    pub min_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub memory_usage_estimate: u64,
    pub throughput_over_time: Vec<(u64, u64)>, // (elapsed_seconds, total_discoveries)
    pub error_count: u64,
    pub success_rate: f64,
    pub resource_utilization: ResourceUtilization,
}

/// Resource utilization metrics
#[derive(Debug, Clone)]
pub struct ResourceUtilization {
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
}

/// Performance test agent
pub struct PerformanceTestAgent {
    pub id: String,
    pub role: String,
    pub broadcaster: DiscoveryBroadcaster,
    pub nats_client: async_nats::Client,
    pub discoveries_sent: std::sync::Arc<std::sync::Mutex<u64>>,
    pub discoveries_received: std::sync::Arc<std::sync::Mutex<u64>>,
    pub latency_measurements: std::sync::Arc<std::sync::Mutex<Vec<Duration>>>,
    pub error_count: std::sync::Arc<std::sync::Mutex<u64>>,
    pub start_times: std::sync::Arc<std::sync::Mutex<HashMap<String, Instant>>>,
}

impl PerformanceTestAgent {
    pub async fn new(id: String, role: String, nats_client: async_nats::Client) -> Self {
        let broadcaster = DiscoveryBroadcaster::new(id.clone(), role.clone(), nats_client.clone());
        
        Self {
            id,
            role,
            broadcaster,
            nats_client,
            discoveries_sent: std::sync::Arc::new(std::sync::Mutex::new(0)),
            discoveries_received: std::sync::Arc::new(std::sync::Mutex::new(0)),
            latency_measurements: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            error_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
            start_times: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_listening(&self) {
        let discoveries_received = self.discoveries_received.clone();
        let latency_measurements = self.latency_measurements.clone();
        let start_times = self.start_times.clone();
        
        let listener = DiscoveryListener::new(self.id.clone(), self.role.clone(), self.nats_client.clone());
        
        tokio::spawn(async move {
            let _ = listener.listen(move |discovery| {
                // Record discovery received
                {
                    let mut count = discoveries_received.lock().unwrap();
                    *count += 1;
                }
                
                // Calculate latency if we have the start time
                if let Ok(start_times_guard) = start_times.lock() {
                    if let Some(start_time) = start_times_guard.get(&discovery.content) {
                        let latency = start_time.elapsed();
                        if let Ok(mut latencies) = latency_measurements.lock() {
                            latencies.push(latency);
                        }
                    }
                }
                
                Ok(())
            }).await;
        });
    }

    pub async fn share_discovery_with_timing(&self, discovery_type: DiscoveryType, content: &str, confidence: f32) -> Result<()> {
        // Record start time for latency measurement
        let start_time = Instant::now();
        {
            let mut start_times = self.start_times.lock().unwrap();
            start_times.insert(content.to_string(), start_time);
        }
        
        // Share discovery
        match self.broadcaster.share_discovery(discovery_type, content, confidence).await {
            Ok(_) => {
                let mut count = self.discoveries_sent.lock().unwrap();
                *count += 1;
                Ok(())
            }
            Err(e) => {
                let mut error_count = self.error_count.lock().unwrap();
                *error_count += 1;
                Err(e)
            }
        }
    }

    pub fn get_stats(&self) -> (u64, u64, Vec<Duration>, u64) {
        let sent = *self.discoveries_sent.lock().unwrap();
        let received = *self.discoveries_received.lock().unwrap();
        let latencies = self.latency_measurements.lock().unwrap().clone();
        let errors = *self.error_count.lock().unwrap();
        
        (sent, received, latencies, errors)
    }
}

/// Main performance test orchestrator
#[derive(Debug)]
pub struct PerformanceTestOrchestrator {
    pub config: PerformanceTestConfig,
    pub nats_client: async_nats::Client,
    pub results: Vec<PerformanceResults>,
}

impl PerformanceTestOrchestrator {
    pub async fn new(config: PerformanceTestConfig) -> Result<Self> {
        let nats_client = async_nats::connect(&config.nats_url).await?;
        
        Ok(Self {
            config,
            nats_client,
            results: Vec::new(),
        })
    }

    pub async fn run_all_tests(&mut self) -> Result<()> {
        println!("🚀 Starting Comprehensive Performance Testing Suite");
        println!("=================================================");
        
        for scenario in self.config.test_scenarios.clone() {
            println!("\n🧪 Running scenario: {}", scenario.name);
            println!("   - Agents: {}", scenario.agent_count);
            println!("   - Discoveries per agent: {}", scenario.discoveries_per_agent);
            println!("   - Duration: {:?}", scenario.test_duration);
            println!("   - Burst mode: {}", scenario.burst_mode);
            
            match self.run_scenario(&scenario).await {
                Ok(result) => {
                    self.results.push(result);
                    println!("   ✅ Scenario completed successfully");
                }
                Err(e) => {
                    println!("   ❌ Scenario failed: {}", e);
                }
            }
            
            // Cool down between tests
            sleep(Duration::from_secs(2)).await;
        }
        
        self.generate_final_report().await?;
        
        Ok(())
    }

    async fn run_scenario(&self, scenario: &TestScenario) -> Result<PerformanceResults> {
        // Create agents
        let mut agents = Vec::new();
        for i in 0..scenario.agent_count {
            let agent_id = format!("perf-agent-{}-{}", scenario.name.replace(" ", "-").to_lowercase(), i);
            let role = format!("TestAgent{}", i);
            
            let agent = PerformanceTestAgent::new(agent_id, role, self.nats_client.clone()).await;
            agent.start_listening().await;
            agents.push(agent);
        }
        
        // Give agents time to subscribe
        sleep(Duration::from_millis(500)).await;
        
        // Start monitoring
        let monitoring_handle = self.start_monitoring(&agents).await;
        
        // Run the test
        let test_start = Instant::now();
        
        if scenario.burst_mode {
            self.run_burst_test(&agents, scenario).await?;
        } else {
            self.run_sustained_test(&agents, scenario).await?;
        }
        
        // Wait for test completion
        sleep(scenario.test_duration).await;
        
        // Stop monitoring
        monitoring_handle.abort();
        
        // Collect results
        let test_duration = test_start.elapsed();
        let results = self.collect_results(&agents, scenario, test_duration).await;
        
        Ok(results)
    }

    async fn run_burst_test(&self, agents: &[PerformanceTestAgent], scenario: &TestScenario) -> Result<()> {
        println!("   💥 Running burst test...");
        
        let mut tasks = Vec::new();
        
        for (i, agent) in agents.iter().enumerate() {
            let broadcaster = agent.broadcaster.clone();
            let discoveries_sent = agent.discoveries_sent.clone();
            let discoveries_per_agent = scenario.discoveries_per_agent;
            let agent_id = agent.id.clone();
            
            tasks.push(tokio::spawn(async move {
                for j in 0..discoveries_per_agent {
                    let content = format!("Burst discovery {} from {} at {:?}", j, agent_id, Instant::now());
                    
                    match broadcaster.share_discovery(
                        DiscoveryType::Pattern,
                        &content,
                        0.8
                    ).await {
                        Ok(_) => {
                            let mut count = discoveries_sent.lock().unwrap();
                            *count += 1;
                        }
                        Err(_) => {
                            // Error handling could be added here
                        }
                    }
                }
            }));
        }
        
        // Wait for all burst tasks to complete
        for task in tasks {
            let _ = task.await;
        }
        
        Ok(())
    }

    async fn run_sustained_test(&self, agents: &[PerformanceTestAgent], scenario: &TestScenario) -> Result<()> {
        println!("   🔄 Running sustained test...");
        
        let mut tasks = Vec::new();
        
        for agent in agents {
            let broadcaster = agent.broadcaster.clone();
            let discoveries_sent = agent.discoveries_sent.clone();
            let discoveries_per_agent = scenario.discoveries_per_agent;
            let interval = scenario.test_duration / discoveries_per_agent as u32;
            let agent_id = agent.id.clone();
            
            tasks.push(tokio::spawn(async move {
                for i in 0..discoveries_per_agent {
                    let content = format!("Sustained discovery {} from {} at {:?}", i, agent_id, Instant::now());
                    
                    match broadcaster.share_discovery(
                        DiscoveryType::Insight,
                        &content,
                        0.75
                    ).await {
                        Ok(_) => {
                            let mut count = discoveries_sent.lock().unwrap();
                            *count += 1;
                        }
                        Err(_) => {
                            // Error handling could be added here
                        }
                    }
                    
                    sleep(interval).await;
                }
            }));
        }
        
        // Don't wait for tasks - let them run in background
        Ok(())
    }

    async fn start_monitoring(&self, agents: &[PerformanceTestAgent]) -> tokio::task::JoinHandle<()> {
        let agents_clone: Vec<_> = agents.iter().map(|a| {
            (a.discoveries_sent.clone(), a.discoveries_received.clone())
        }).collect();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let start_time = Instant::now();
            
            loop {
                interval.tick().await;
                
                let mut total_sent = 0;
                let mut total_received = 0;
                
                for (sent, received) in &agents_clone {
                    total_sent += *sent.lock().unwrap();
                    total_received += *received.lock().unwrap();
                }
                
                let elapsed = start_time.elapsed();
                let rate = total_sent as f64 / elapsed.as_secs_f64();
                
                println!("   📊 {:?}: {} sent, {} received, {:.1} discoveries/sec", 
                         elapsed, total_sent, total_received, rate);
            }
        })
    }

    async fn collect_results(&self, agents: &[PerformanceTestAgent], scenario: &TestScenario, test_duration: Duration) -> PerformanceResults {
        let mut total_sent = 0;
        let mut total_received = 0;
        let mut total_errors = 0;
        let mut all_latencies = Vec::new();
        
        for agent in agents {
            let (sent, received, latencies, errors) = agent.get_stats();
            total_sent += sent;
            total_received += received;
            total_errors += errors;
            all_latencies.extend(latencies);
        }
        
        // Sort latencies for percentile calculations
        all_latencies.sort();
        
        let average_latency_ms = if !all_latencies.is_empty() {
            all_latencies.iter().sum::<Duration>().as_millis() as f64 / all_latencies.len() as f64
        } else {
            0.0
        };
        
        let max_latency_ms = all_latencies.iter().max().map(|d| d.as_millis() as f64).unwrap_or(0.0);
        let min_latency_ms = all_latencies.iter().min().map(|d| d.as_millis() as f64).unwrap_or(0.0);
        
        let p95_latency_ms = if !all_latencies.is_empty() {
            let index = (all_latencies.len() as f64 * 0.95) as usize;
            all_latencies.get(index).map(|d| d.as_millis() as f64).unwrap_or(0.0)
        } else {
            0.0
        };
        
        let p99_latency_ms = if !all_latencies.is_empty() {
            let index = (all_latencies.len() as f64 * 0.99) as usize;
            all_latencies.get(index).map(|d| d.as_millis() as f64).unwrap_or(0.0)
        } else {
            0.0
        };
        
        let discoveries_per_second = total_sent as f64 / test_duration.as_secs_f64();
        let success_rate = if total_sent > 0 { 
            ((total_sent - total_errors) as f64 / total_sent as f64) * 100.0 
        } else { 
            0.0 
        };
        
        PerformanceResults {
            scenario_name: scenario.name.clone(),
            agent_count: scenario.agent_count,
            total_discoveries_sent: total_sent,
            total_discoveries_received: total_received,
            test_duration,
            discoveries_per_second,
            average_latency_ms,
            max_latency_ms,
            min_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            memory_usage_estimate: total_sent * 200, // Rough estimate
            throughput_over_time: Vec::new(), // Would need real-time collection
            error_count: total_errors,
            success_rate,
            resource_utilization: ResourceUtilization {
                cpu_percent: 0.0, // Would need system monitoring
                memory_mb: (total_sent * 200) / 1024 / 1024,
                network_bytes_sent: total_sent * 1000, // Rough estimate
                network_bytes_received: total_received * 1000,
            },
        }
    }

    async fn generate_final_report(&self) -> Result<()> {
        println!("\n🎉 Performance Testing Complete!");
        println!("=================================");
        
        for result in &self.results {
            println!("\n📊 Results for {}", result.scenario_name);
            println!("   Agents: {}", result.agent_count);
            println!("   Duration: {:?}", result.test_duration);
            println!("   Discoveries sent: {}", result.total_discoveries_sent);
            println!("   Discoveries received: {}", result.total_discoveries_received);
            println!("   Discoveries/sec: {:.2}", result.discoveries_per_second);
            println!("   Success rate: {:.1}%", result.success_rate);
            println!("   Average latency: {:.2}ms", result.average_latency_ms);
            println!("   95th percentile: {:.2}ms", result.p95_latency_ms);
            println!("   99th percentile: {:.2}ms", result.p99_latency_ms);
            println!("   Max latency: {:.2}ms", result.max_latency_ms);
            println!("   Estimated memory: {} MB", result.resource_utilization.memory_mb);
            println!("   Errors: {}", result.error_count);
        }
        
        // Generate JSON report
        if let Some(output_file) = &self.config.output_file {
            let json_report = json!({
                "test_run_id": Uuid::new_v4(),
                "timestamp": chrono::Utc::now(),
                "system_info": {
                    "nats_url": self.config.nats_url,
                    "platform": std::env::consts::OS,
                    "architecture": std::env::consts::ARCH,
                },
                "results": self.results.iter().map(|r| json!({
                    "scenario_name": r.scenario_name,
                    "agent_count": r.agent_count,
                    "total_discoveries_sent": r.total_discoveries_sent,
                    "total_discoveries_received": r.total_discoveries_received,
                    "test_duration_secs": r.test_duration.as_secs(),
                    "discoveries_per_second": r.discoveries_per_second,
                    "average_latency_ms": r.average_latency_ms,
                    "p95_latency_ms": r.p95_latency_ms,
                    "p99_latency_ms": r.p99_latency_ms,
                    "max_latency_ms": r.max_latency_ms,
                    "success_rate": r.success_rate,
                    "error_count": r.error_count,
                    "memory_usage_mb": r.resource_utilization.memory_mb,
                })).collect::<Vec<_>>(),
                "summary": {
                    "total_scenarios": self.results.len(),
                    "total_agents_tested": self.results.iter().map(|r| r.agent_count).sum::<usize>(),
                    "total_discoveries": self.results.iter().map(|r| r.total_discoveries_sent).sum::<u64>(),
                    "peak_throughput": self.results.iter().map(|r| r.discoveries_per_second).fold(0.0, f64::max),
                    "best_latency": self.results.iter().map(|r| r.average_latency_ms).fold(f64::INFINITY, f64::min),
                }
            });
            
            std::fs::write(output_file, serde_json::to_string_pretty(&json_report)?)?;
            println!("\n📝 Detailed report saved to: {}", output_file);
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("🔧 MisterSmith Performance Testing Suite");
    println!("========================================");
    
    // Check NATS availability
    match async_nats::connect("nats://localhost:4222").await {
        Ok(_) => println!("✅ NATS server available"),
        Err(e) => {
            println!("❌ NATS server not available: {}", e);
            println!("💡 Please start NATS server: nats-server");
            return Ok(());
        }
    }
    
    // Run performance tests
    let config = PerformanceTestConfig::default();
    let mut orchestrator = PerformanceTestOrchestrator::new(config).await?;
    
    orchestrator.run_all_tests().await?;
    
    println!("\n🎯 Performance testing complete!");
    println!("Check performance_report.json for detailed results.");
    
    Ok(())
}