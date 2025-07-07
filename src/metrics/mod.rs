//! Performance metrics collection and monitoring
//! 
//! This module provides comprehensive performance monitoring for the MisterSmith framework,
//! implementing the observability patterns defined in the framework specifications.

pub mod agent_metrics;
pub mod baselines;
pub mod collector;
pub mod exporter;
pub mod system_metrics;
pub mod types;

pub use agent_metrics::AgentMetricsCollector;
pub use baselines::{PerformanceBaselines, BaselineConfig};
pub use collector::MetricsCollector;
pub use exporter::PrometheusExporter;
pub use system_metrics::SystemMetricsCollector;
pub use types::{MetricType, MetricValue, MetricDescriptor};

use std::sync::Arc;
use tokio::sync::RwLock;

/// Initialize the metrics subsystem
pub async fn init_metrics() -> Result<Arc<MetricsCollector>, Box<dyn std::error::Error>> {
    let collector = MetricsCollector::new().await?;
    Ok(Arc::new(collector))
}

/// Global metrics registry for the application
pub struct MetricsRegistry {
    collectors: Arc<RwLock<Vec<Box<dyn MetricCollector>>>>,
    exporter: Arc<PrometheusExporter>,
}

impl MetricsRegistry {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let exporter = Arc::new(PrometheusExporter::new()?);
        
        Ok(Self {
            collectors: Arc::new(RwLock::new(Vec::new())),
            exporter,
        })
    }
    
    pub async fn register_collector(&self, collector: Box<dyn MetricCollector>) {
        let mut collectors = self.collectors.write().await;
        collectors.push(collector);
    }
    
    pub async fn start_exporter(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.exporter.clone().start(port).await
    }
}

/// Trait for metric collectors
#[async_trait::async_trait]
pub trait MetricCollector: Send + Sync {
    /// Collect current metrics
    async fn collect(&self) -> Vec<MetricValue>;
    
    /// Get collector name
    fn name(&self) -> &str;
    
    /// Get metric descriptors
    fn descriptors(&self) -> Vec<MetricDescriptor>;
}