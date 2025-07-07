//! Main metrics collector that aggregates all metric sources

use crate::metrics::{
    MetricCollector,
    AgentMetricsCollector,
    SystemMetricsCollector,
    types::{MetricValue, MetricDescriptor},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

/// Main metrics collector that aggregates metrics from all sources
pub struct MetricsCollector {
    collectors: Arc<RwLock<Vec<Arc<dyn MetricCollector>>>>,
    agent_collector: Arc<AgentMetricsCollector>,
    system_collector: Arc<SystemMetricsCollector>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let agent_collector = Arc::new(AgentMetricsCollector::new());
        let system_collector = Arc::new(SystemMetricsCollector::new());
        
        let mut collectors: Vec<Arc<dyn MetricCollector>> = vec![
            agent_collector.clone() as Arc<dyn MetricCollector>,
            system_collector.clone() as Arc<dyn MetricCollector>,
        ];
        
        Ok(Self {
            collectors: Arc::new(RwLock::new(collectors)),
            agent_collector,
            system_collector,
        })
    }
    
    /// Get the agent metrics collector
    pub fn agent_collector(&self) -> &Arc<AgentMetricsCollector> {
        &self.agent_collector
    }
    
    /// Get the system metrics collector
    pub fn system_collector(&self) -> &Arc<SystemMetricsCollector> {
        &self.system_collector
    }
    
    /// Register an additional metrics collector
    pub async fn register_collector(&self, collector: Arc<dyn MetricCollector>) {
        let mut collectors = self.collectors.write().await;
        collectors.push(collector);
        info!("Registered new metrics collector");
    }
    
    /// Collect metrics from all registered collectors
    pub async fn collect_all(&self) -> Result<Vec<MetricValue>, Box<dyn std::error::Error>> {
        let collectors = self.collectors.read().await;
        let mut all_metrics = Vec::new();
        
        for collector in collectors.iter() {
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                collector.collect()
            ).await {
                Ok(metrics) => {
                    all_metrics.extend(metrics);
                }
                Err(_) => {
                    error!("Timeout collecting metrics from {}", collector.name());
                }
            }
        }
        
        Ok(all_metrics)
    }
    
    /// Get all metric descriptors
    pub async fn get_descriptors(&self) -> Vec<MetricDescriptor> {
        let collectors = self.collectors.read().await;
        let mut descriptors = Vec::new();
        
        for collector in collectors.iter() {
            descriptors.extend(collector.descriptors());
        }
        
        descriptors
    }
    
    /// Get metrics for a specific collector by name
    pub async fn collect_by_name(&self, name: &str) -> Option<Vec<MetricValue>> {
        let collectors = self.collectors.read().await;
        
        for collector in collectors.iter() {
            if collector.name() == name {
                return Some(collector.collect().await);
            }
        }
        
        None
    }
}

/// Performance monitoring helpers
impl MetricsCollector {
    /// Record the start of an operation
    pub fn start_timer() -> std::time::Instant {
        std::time::Instant::now()
    }
    
    /// Calculate operation duration in milliseconds
    pub fn duration_ms(start: std::time::Instant) -> f64 {
        start.elapsed().as_secs_f64() * 1000.0
    }
    
    /// Record an operation with automatic timing
    pub async fn record_operation<F, T>(
        &self,
        agent_id: &str,
        operation_name: &str,
        f: F,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        F: std::future::Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        let start = Self::start_timer();
        let result = f.await;
        let duration = Self::duration_ms(start);
        
        // Record operation metrics
        // This would integrate with the agent metrics collector
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_metrics_collection() {
        let collector = MetricsCollector::new().await.unwrap();
        
        // Should have default collectors registered
        let metrics = collector.collect_all().await.unwrap();
        assert!(!metrics.is_empty());
        
        // Should have descriptors for all metrics
        let descriptors = collector.get_descriptors().await;
        assert!(!descriptors.is_empty());
    }
    
    #[tokio::test]
    async fn test_timer_helpers() {
        let start = MetricsCollector::start_timer();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let duration = MetricsCollector::duration_ms(start);
        
        assert!(duration >= 10.0);
        assert!(duration < 100.0); // Should not take too long
    }
}