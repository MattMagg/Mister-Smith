//! Failure detection system
//!
//! This module implements adaptive failure detection using the Phi Accrual algorithm,
//! which provides better accuracy and fewer false positives than traditional timeout-based
//! failure detectors.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{info, warn, error, debug};
use anyhow::Result;

use crate::supervision::types::{NodeId, FailureReason};

/// Event representing a detected failure
#[derive(Debug, Clone)]
pub struct FailureEvent {
    pub node_id: NodeId,
    pub reason: FailureReason,
    pub timestamp: Instant,
}

/// Health information for a monitored node
#[derive(Debug, Clone)]
struct NodeHealth {
    last_heartbeat: Instant,
    heartbeat_intervals: Vec<Duration>,
    phi_threshold: f64,
    is_healthy: bool,
}

impl NodeHealth {
    fn new(phi_threshold: f64) -> Self {
        Self {
            last_heartbeat: Instant::now(),
            heartbeat_intervals: Vec::with_capacity(100),
            phi_threshold,
            is_healthy: true,
        }
    }

    /// Record a heartbeat and update interval history
    fn record_heartbeat(&mut self) {
        let now = Instant::now();
        let interval = now.duration_since(self.last_heartbeat);
        
        self.heartbeat_intervals.push(interval);
        
        // Keep history bounded
        if self.heartbeat_intervals.len() > 100 {
            self.heartbeat_intervals.remove(0);
        }
        
        self.last_heartbeat = now;
        self.is_healthy = true;
    }

    /// Calculate phi value for current time
    fn calculate_phi(&self, now: Instant) -> f64 {
        if self.heartbeat_intervals.is_empty() {
            return 0.0;
        }

        let time_since_last = now.duration_since(self.last_heartbeat);
        let time_ms = time_since_last.as_millis() as f64;

        // Calculate mean and variance of heartbeat intervals
        let mean = self.calculate_mean();
        let variance = self.calculate_variance(mean);
        let std_dev = variance.sqrt();

        // Phi = -log10(1 - CDF(time))
        // Using normal distribution CDF approximation
        if std_dev > 0.0 {
            let z_score = (time_ms - mean) / std_dev;
            let cdf = normal_cdf(z_score);
            
            if cdf >= 0.9999 {
                // Avoid log(0)
                16.0 // Very high phi value
            } else {
                -((1.0 - cdf).log10())
            }
        } else {
            // No variance, use simple threshold
            if time_ms > mean * 2.0 {
                10.0
            } else {
                0.0
            }
        }
    }

    fn calculate_mean(&self) -> f64 {
        if self.heartbeat_intervals.is_empty() {
            return 1000.0; // Default 1 second
        }

        let sum: f64 = self.heartbeat_intervals
            .iter()
            .map(|d| d.as_millis() as f64)
            .sum();
        
        sum / self.heartbeat_intervals.len() as f64
    }

    fn calculate_variance(&self, mean: f64) -> f64 {
        if self.heartbeat_intervals.len() < 2 {
            return 0.0;
        }

        let sum_squared_diff: f64 = self.heartbeat_intervals
            .iter()
            .map(|d| {
                let diff = d.as_millis() as f64 - mean;
                diff * diff
            })
            .sum();
        
        sum_squared_diff / (self.heartbeat_intervals.len() - 1) as f64
    }
}

/// Failure detector using Phi Accrual algorithm
pub struct FailureDetector {
    /// Heartbeat check interval
    heartbeat_interval: Duration,
    /// Default phi threshold for failure detection
    default_phi_threshold: f64,
    /// Monitored nodes health information
    monitored_nodes: Arc<RwLock<HashMap<NodeId, NodeHealth>>>,
    /// Whether the detector is running
    is_running: Arc<RwLock<bool>>,
    /// Channel for failure events (in real impl would be mpsc)
    failure_callback: Arc<RwLock<Option<Box<dyn Fn(FailureEvent) + Send + Sync>>>>,
}

impl FailureDetector {
    /// Create a new failure detector
    pub fn new(config: FailureDetectorConfig) -> Self {
        Self {
            heartbeat_interval: config.heartbeat_interval,
            default_phi_threshold: config.phi_threshold,
            monitored_nodes: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            failure_callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the failure detector
    pub async fn start(&self) -> Result<()> {
        info!("Starting failure detector");
        
        *self.is_running.write().await = true;
        
        // Spawn monitoring task
        let detector = self.clone();
        tokio::spawn(async move {
            detector.monitoring_loop().await;
        });
        
        Ok(())
    }

    /// Stop the failure detector
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping failure detector");
        
        *self.is_running.write().await = false;
        
        Ok(())
    }

    /// Start monitoring a node
    pub async fn monitor_node(&self, node_id: NodeId) -> Result<()> {
        debug!("Starting to monitor node: {}", node_id);
        
        let mut nodes = self.monitored_nodes.write().await;
        nodes.insert(node_id, NodeHealth::new(self.default_phi_threshold));
        
        Ok(())
    }

    /// Stop monitoring a node
    pub async fn stop_monitoring(&self, node_id: NodeId) -> Result<()> {
        debug!("Stopping monitoring for node: {}", node_id);
        
        self.monitored_nodes.write().await.remove(&node_id);
        
        Ok(())
    }

    /// Record a heartbeat from a node
    pub async fn record_heartbeat(&self, node_id: &NodeId) -> Result<()> {
        let mut nodes = self.monitored_nodes.write().await;
        
        if let Some(health) = nodes.get_mut(node_id) {
            health.record_heartbeat();
            debug!("Recorded heartbeat for node: {}", node_id);
        } else {
            warn!("Received heartbeat from unmonitored node: {}", node_id);
        }
        
        Ok(())
    }

    /// Set failure callback
    pub async fn set_failure_callback<F>(&self, callback: F)
    where
        F: Fn(FailureEvent) + Send + Sync + 'static,
    {
        *self.failure_callback.write().await = Some(Box::new(callback));
    }

    /// Main monitoring loop
    async fn monitoring_loop(&self) {
        let mut interval = interval(self.heartbeat_interval);
        
        while *self.is_running.read().await {
            interval.tick().await;
            
            let now = Instant::now();
            let mut failed_nodes = Vec::new();
            
            // Check all monitored nodes
            {
                let mut nodes = self.monitored_nodes.write().await;
                
                for (node_id, health) in nodes.iter_mut() {
                    let phi = health.calculate_phi(now);
                    
                    debug!("Node {} phi value: {:.2}", node_id, phi);
                    
                    if phi > health.phi_threshold && health.is_healthy {
                        // Node has failed
                        error!("Node {} failed detection (phi: {:.2} > {:.2})", 
                            node_id, phi, health.phi_threshold);
                        
                        health.is_healthy = false;
                        failed_nodes.push(FailureEvent {
                            node_id: node_id.clone(),
                            reason: FailureReason::HeartbeatTimeout,
                            timestamp: now,
                        });
                    }
                }
            }
            
            // Report failures
            for event in failed_nodes {
                self.report_failure(event).await;
            }
        }
        
        info!("Failure detector monitoring loop stopped");
    }

    /// Report a detected failure
    async fn report_failure(&self, event: FailureEvent) {
        if let Some(callback) = self.failure_callback.read().await.as_ref() {
            callback(event);
        }
    }
}

// Clone implementation for FailureDetector
impl Clone for FailureDetector {
    fn clone(&self) -> Self {
        Self {
            heartbeat_interval: self.heartbeat_interval,
            default_phi_threshold: self.default_phi_threshold,
            monitored_nodes: self.monitored_nodes.clone(),
            is_running: self.is_running.clone(),
            failure_callback: self.failure_callback.clone(),
        }
    }
}

/// Configuration for failure detector
#[derive(Debug, Clone)]
pub struct FailureDetectorConfig {
    /// Interval for checking node health
    pub heartbeat_interval: Duration,
    /// Phi threshold for failure detection (typical: 8-16)
    pub phi_threshold: f64,
}

impl Default for FailureDetectorConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(1),
            phi_threshold: 10.0, // Reasonable default for most networks
        }
    }
}

/// Normal cumulative distribution function approximation
fn normal_cdf(z: f64) -> f64 {
    // Using Abramowitz and Stegun approximation
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;
    
    let sign = if z < 0.0 { -1.0 } else { 1.0 };
    let z = z.abs();
    
    let t = 1.0 / (1.0 + p * z);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    
    let cdf = 1.0 - (a1 * t + a2 * t2 + a3 * t3 + a4 * t4 + a5 * t5) * (-z * z / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();
    
    0.5 + sign * (cdf - 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normal_cdf() {
        // Test some known values
        assert!((normal_cdf(0.0) - 0.5).abs() < 0.001);
        assert!((normal_cdf(1.0) - 0.8413).abs() < 0.001);
        assert!((normal_cdf(-1.0) - 0.1587).abs() < 0.001);
        assert!((normal_cdf(2.0) - 0.9772).abs() < 0.001);
    }
    
    #[tokio::test]
    async fn test_failure_detector_creation() {
        let config = FailureDetectorConfig::default();
        let detector = FailureDetector::new(config);
        
        assert!(!*detector.is_running.read().await);
    }
    
    #[tokio::test]
    async fn test_node_monitoring() {
        let config = FailureDetectorConfig::default();
        let detector = FailureDetector::new(config);
        
        let node_id = NodeId("test-node".to_string());
        detector.monitor_node(node_id.clone()).await.unwrap();
        
        let nodes = detector.monitored_nodes.read().await;
        assert!(nodes.contains_key(&node_id));
    }
}