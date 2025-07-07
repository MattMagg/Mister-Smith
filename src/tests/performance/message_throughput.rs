//! Message throughput performance tests

use super::{
    PerformanceTest, PerformanceTestConfig, PerformanceTestResults,
    PerformanceMetric, BaselineViolation, ViolationSeverity,
};
use crate::message::{Message, MessageType, MessageRouter};
use crate::metrics::{MetricsCollector, PerformanceBaselines};
use crate::transport::TransportLayer;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::info;

/// Message throughput performance test
pub struct MessageThroughputTest;

#[async_trait::async_trait]
impl PerformanceTest for MessageThroughputTest {
    fn name(&self) -> &str {
        "Message Throughput Performance"
    }
    
    async fn run(
        &self,
        config: &PerformanceTestConfig,
        metrics: Arc<MetricsCollector>,
    ) -> Result<PerformanceTestResults, Box<dyn std::error::Error>> {
        let test_start = Instant::now();
        let mut latencies = Vec::new();
        
        // Create message router
        let router = MessageRouter::new();
        
        // Metrics tracking
        let messages_sent = Arc::new(AtomicU64::new(0));
        let messages_received = Arc::new(AtomicU64::new(0));
        let messages_failed = Arc::new(AtomicU64::new(0));
        
        // Create receiver
        let (tx, mut rx) = mpsc::channel(1000);
        let received_clone = messages_received.clone();
        
        // Spawn receiver task
        tokio::spawn(async move {
            while let Some(_msg) = rx.recv().await {
                received_clone.fetch_add(1, Ordering::Relaxed);
            }
        });
        
        // Send messages at specified rate
        let message_interval = Duration::from_micros(1_000_000 / config.message_rate);
        let test_messages = config.message_rate * config.duration.as_secs();
        
        for i in 0..test_messages {
            let send_start = Instant::now();
            
            let message = Message {
                id: format!("perf_test_msg_{}", i),
                message_type: MessageType::Task,
                source: "perf_test".to_string(),
                target: Some("test_agent".to_string()),
                payload: serde_json::json!({
                    "test": true,
                    "index": i,
                }),
                timestamp: std::time::SystemTime::now(),
                correlation_id: None,
            };
            
            // Send message
            match tx.try_send(message) {
                Ok(_) => {
                    messages_sent.fetch_add(1, Ordering::Relaxed);
                    let latency = send_start.elapsed();
                    latencies.push(latency.as_micros() as f64 / 1000.0); // Convert to ms
                }
                Err(_) => {
                    messages_failed.fetch_add(1, Ordering::Relaxed);
                }
            }
            
            // Rate limiting
            tokio::time::sleep(message_interval).await;
        }
        
        // Wait for processing to complete
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        let test_duration = test_start.elapsed();
        
        // Calculate metrics
        let total_sent = messages_sent.load(Ordering::Relaxed);
        let total_received = messages_received.load(Ordering::Relaxed);
        let total_failed = messages_failed.load(Ordering::Relaxed);
        
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_index = (latencies.len() as f64 * 0.95) as usize;
        let p99_index = (latencies.len() as f64 * 0.99) as usize;
        
        let p95_latency = latencies.get(p95_index).copied().unwrap_or(0.0);
        let p99_latency = latencies.get(p99_index).copied().unwrap_or(0.0);
        let delivery_rate = (total_received as f64 / total_sent as f64) * 100.0;
        let throughput = total_sent as f64 / test_duration.as_secs_f64();
        
        let metrics_vec = vec![
            PerformanceMetric {
                name: "message_delivery_p95_latency_ms".to_string(),
                value: p95_latency,
                unit: "milliseconds".to_string(),
                baseline: Some(50.0),
            },
            PerformanceMetric {
                name: "message_delivery_p99_latency_ms".to_string(),
                value: p99_latency,
                unit: "milliseconds".to_string(),
                baseline: Some(100.0),
            },
            PerformanceMetric {
                name: "message_delivery_success_rate".to_string(),
                value: delivery_rate,
                unit: "percent".to_string(),
                baseline: Some(99.9),
            },
            PerformanceMetric {
                name: "message_throughput_per_second".to_string(),
                value: throughput,
                unit: "messages/second".to_string(),
                baseline: None,
            },
            PerformanceMetric {
                name: "messages_failed".to_string(),
                value: total_failed as f64,
                unit: "count".to_string(),
                baseline: None,
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
                "message_delivery_p95_latency_ms" => {
                    if metric.value > baselines.communication_baselines.message_delivery.p95_latency_ms {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.communication_baselines.message_delivery.p95_latency_ms,
                            actual: metric.value,
                            severity: ViolationSeverity::Warning,
                        });
                    }
                }
                "message_delivery_p99_latency_ms" => {
                    if metric.value > baselines.communication_baselines.message_delivery.p99_latency_ms {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.communication_baselines.message_delivery.p99_latency_ms,
                            actual: metric.value,
                            severity: ViolationSeverity::Error,
                        });
                    }
                }
                "message_delivery_success_rate" => {
                    if metric.value < baselines.communication_baselines.message_delivery.delivery_success_rate {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.communication_baselines.message_delivery.delivery_success_rate,
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

/// Queue processing performance test
pub struct QueueProcessingTest;

#[async_trait::async_trait]
impl PerformanceTest for QueueProcessingTest {
    fn name(&self) -> &str {
        "Queue Processing Performance"
    }
    
    async fn run(
        &self,
        config: &PerformanceTestConfig,
        metrics: Arc<MetricsCollector>,
    ) -> Result<PerformanceTestResults, Box<dyn std::error::Error>> {
        let test_start = Instant::now();
        let mut processing_times = Vec::new();
        let mut queue_depths = Vec::new();
        
        // Create a processing queue
        let (tx, mut rx) = mpsc::channel(100); // Max queue depth from baselines
        
        // Spawn processor
        let processing_times_clone = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let processor_times = processing_times_clone.clone();
        
        tokio::spawn(async move {
            while let Some(_msg) = rx.recv().await {
                let process_start = Instant::now();
                
                // Simulate message processing
                tokio::time::sleep(Duration::from_millis(20)).await;
                
                let duration = process_start.elapsed().as_millis() as f64;
                processor_times.lock().await.push(duration);
            }
        });
        
        // Send messages and track queue depth
        for i in 0..1000 {
            let message = format!("test_message_{}", i);
            
            // Record queue depth before sending
            queue_depths.push(tx.capacity() as f64);
            
            tx.send(message).await?;
            
            // Small delay between messages
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        // Wait for processing to complete
        drop(tx);
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        let test_duration = test_start.elapsed();
        
        // Get processing times
        processing_times = processing_times_clone.lock().await.clone();
        
        // Calculate metrics
        let avg_processing_time = processing_times.iter().sum::<f64>() / processing_times.len() as f64;
        let max_queue_depth = queue_depths.iter().fold(0.0f64, |a, &b| a.max(b));
        
        let metrics_vec = vec![
            PerformanceMetric {
                name: "queue_processing_time_ms".to_string(),
                value: avg_processing_time,
                unit: "milliseconds".to_string(),
                baseline: Some(25.0),
            },
            PerformanceMetric {
                name: "max_queue_depth".to_string(),
                value: max_queue_depth,
                unit: "messages".to_string(),
                baseline: Some(100.0),
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
                "queue_processing_time_ms" => {
                    if metric.value > baselines.communication_baselines.queue_processing.processing_time_ms {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.communication_baselines.queue_processing.processing_time_ms,
                            actual: metric.value,
                            severity: ViolationSeverity::Warning,
                        });
                    }
                }
                "max_queue_depth" => {
                    if metric.value > baselines.communication_baselines.queue_processing.max_queue_depth as f64 {
                        violations.push(BaselineViolation {
                            metric: metric.name.clone(),
                            expected: baselines.communication_baselines.queue_processing.max_queue_depth as f64,
                            actual: metric.value,
                            severity: ViolationSeverity::Error,
                        });
                    }
                }
                _ => {}
            }
        }
        
        violations
    }
}