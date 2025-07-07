//! Example demonstrating metrics collection and monitoring

use mistersmith::metrics::{
    init_metrics, MetricsRegistry, PerformanceBaselines, 
    AgentPerformanceMetrics, SystemPerformanceMetrics
};
use mistersmith::agent::{Agent, AgentConfig, AgentType};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    info!("Starting MisterSmith metrics example");
    
    // Initialize metrics collector
    let metrics = init_metrics().await?;
    info!("Metrics system initialized");
    
    // Create metrics registry and start exporter
    let registry = MetricsRegistry::new().await?;
    
    // Start Prometheus exporter in background
    let registry_clone = Arc::new(registry);
    tokio::spawn(async move {
        if let Err(e) = registry_clone.start_exporter(9090).await {
            eprintln!("Failed to start metrics exporter: {}", e);
        }
    });
    
    info!("Prometheus metrics available at http://localhost:9090/metrics");
    
    // Create some test agents
    let mut agents = Vec::new();
    for i in 0..3 {
        let config = AgentConfig {
            id: format!("example_agent_{}", i),
            agent_type: AgentType::Worker,
            ..Default::default()
        };
        
        match Agent::spawn(config).await {
            Ok(agent) => {
                info!("Spawned agent: {}", agent.id());
                agents.push(agent);
                
                // Update system metrics
                metrics.system_collector().increment_active_agents().await;
            }
            Err(e) => eprintln!("Failed to spawn agent: {}", e),
        }
    }
    
    // Simulate some work and collect metrics
    for i in 0..10 {
        info!("Iteration {}", i);
        
        // Simulate agent work
        for (idx, agent) in agents.iter().enumerate() {
            // Record agent metrics
            let agent_metrics = AgentPerformanceMetrics {
                operation_count: (i + 1) as u64,
                operation_duration_ms: 50.0 + (idx as f64 * 10.0),
                error_count: if i % 5 == 0 { 1 } else { 0 },
                memory_usage_mb: 100.0 + (idx as f64 * 50.0),
                cpu_usage_percent: 10.0 + (idx as f64 * 5.0),
                queue_depth: (10 - i) as u64,
            };
            
            metrics.agent_collector()
                .update_agent_metrics(
                    agent.id(),
                    "worker",
                    agent_metrics,
                )
                .await;
        }
        
        // Update system metrics
        let system_metrics = SystemPerformanceMetrics {
            active_agents: agents.len() as u64,
            total_tasks: (i + 1) as u64 * agents.len() as u64,
            completed_tasks: i as u64 * agents.len() as u64,
            failed_tasks: (i / 5) as u64,
            throughput_per_second: 10.0 + i as f64,
            average_latency_ms: 50.0 + (i as f64 * 2.0),
        };
        
        metrics.system_collector()
            .update_system_metrics(system_metrics)
            .await;
        
        // Collect and display current metrics
        match metrics.collect_all().await {
            Ok(current_metrics) => {
                info!("Collected {} metrics", current_metrics.len());
                
                // Sample some key metrics
                for metric in current_metrics.iter().take(5) {
                    info!(
                        "  {} = {} (type: {:?})",
                        metric.name, metric.value, metric.metric_type
                    );
                }
            }
            Err(e) => eprintln!("Failed to collect metrics: {}", e),
        }
        
        // Check performance baselines
        let baselines = PerformanceBaselines::default();
        if system_metrics.average_latency_ms > baselines.communication_baselines.message_delivery.p95_latency_ms {
            info!("⚠️  Latency exceeding baseline: {} > {}",
                system_metrics.average_latency_ms,
                baselines.communication_baselines.message_delivery.p95_latency_ms
            );
        }
        
        sleep(Duration::from_secs(2)).await;
    }
    
    info!("Metrics example completed");
    info!("Check http://localhost:9090/metrics for Prometheus metrics");
    
    // Keep running for a bit to allow metric inspection
    sleep(Duration::from_secs(30)).await;
    
    Ok(())
}

/// Example of custom metric collection
async fn collect_custom_metrics(metrics: &Arc<mistersmith::metrics::MetricsCollector>) {
    // Time an operation
    let start = mistersmith::metrics::MetricsCollector::start_timer();
    
    // Do some work...
    sleep(Duration::from_millis(100)).await;
    
    let duration = mistersmith::metrics::MetricsCollector::duration_ms(start);
    info!("Operation took {} ms", duration);
    
    // Record the metric
    metrics.system_collector()
        .record_task_completion(true, duration)
        .await;
}