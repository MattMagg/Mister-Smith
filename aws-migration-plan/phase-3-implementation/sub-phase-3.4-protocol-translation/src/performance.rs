// Performance optimization module for NATS to AWS protocol translation

use std::sync::Arc;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use thiserror::Error;
use metrics::{counter, gauge, histogram};
use crate::{AwsTarget, NatsMessageEnvelope, TranslationError};

#[derive(Error, Debug)]
pub enum PerformanceError {
    #[error("Batch processing error: {0}")]
    BatchError(String),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Ordering error: {0}")]
    OrderingError(String),
    
    #[error("Circuit breaker open for service: {0}")]
    CircuitBreakerOpen(String),
}

// Performance monitor
pub struct PerformanceMonitor {
    metrics_collector: Arc<MetricsCollector>,
    batch_processor: Arc<BatchProcessor>,
    ordering_preserver: Arc<OrderingPreserver>,
    latency_compensator: Arc<LatencyCompensator>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics_collector: Arc::new(MetricsCollector::new()),
            batch_processor: Arc::new(BatchProcessor::new(100, Duration::from_millis(100))),
            ordering_preserver: Arc::new(OrderingPreserver::new()),
            latency_compensator: Arc::new(LatencyCompensator::new()),
        }
    }
    
    pub async fn record_translation(
        &self,
        subject: &str,
        target: &AwsTarget,
        duration: Duration,
    ) {
        self.metrics_collector.record_translation(subject, target, duration).await;
    }
    
    pub fn batch_processor(&self) -> &Arc<BatchProcessor> {
        &self.batch_processor
    }
    
    pub fn ordering_preserver(&self) -> &Arc<OrderingPreserver> {
        &self.ordering_preserver
    }
    
    pub fn latency_compensator(&self) -> &Arc<LatencyCompensator> {
        &self.latency_compensator
    }
}

// Metrics collector
pub struct MetricsCollector {
    translation_counts: DashMap<String, u64>,
    translation_latencies: DashMap<String, Vec<Duration>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            translation_counts: DashMap::new(),
            translation_latencies: DashMap::new(),
        }
    }
    
    pub async fn record_translation(
        &self,
        subject: &str,
        target: &AwsTarget,
        duration: Duration,
    ) {
        // Update counts
        self.translation_counts
            .entry(subject.to_string())
            .and_modify(|c| *c += 1)
            .or_insert(1);
        
        // Record latency
        self.translation_latencies
            .entry(subject.to_string())
            .and_modify(|v| v.push(duration))
            .or_insert_with(|| vec![duration]);
        
        // Emit metrics
        let target_type = match target {
            AwsTarget::EventBridge(_) => "eventbridge",
            AwsTarget::Sqs(_) => "sqs",
            AwsTarget::Lambda(_) => "lambda",
            AwsTarget::DynamoDb(_) => "dynamodb",
        };
        
        counter!("protocol_translation_total", 1,
            "source" => "nats",
            "target" => target_type,
            "subject" => subject.to_string(),
        );
        
        histogram!("protocol_translation_duration_ms", duration.as_millis() as f64,
            "source" => "nats",
            "target" => target_type,
        );
    }
}

// Batch processor for efficient AWS API calls
pub struct BatchProcessor {
    batch_size: usize,
    batch_timeout: Duration,
    eventbridge_batcher: Arc<EventBridgeBatcher>,
    sqs_batcher: Arc<SqsBatcher>,
}

impl BatchProcessor {
    pub fn new(batch_size: usize, batch_timeout: Duration) -> Self {
        Self {
            batch_size,
            batch_timeout,
            eventbridge_batcher: Arc::new(EventBridgeBatcher::new(batch_size, batch_timeout)),
            sqs_batcher: Arc::new(SqsBatcher::new(batch_size, batch_timeout)),
        }
    }
    
    pub async fn process_batch(
        &self,
        messages: Vec<(NatsMessageEnvelope, AwsTarget)>,
    ) -> Result<BatchResult, PerformanceError> {
        let mut eventbridge_messages = Vec::new();
        let mut sqs_messages = HashMap::<String, Vec<_>>::new();
        let mut lambda_invocations = Vec::new();
        let mut dynamodb_operations = Vec::new();
        
        // Group messages by target type
        for (envelope, target) in messages {
            match target {
                AwsTarget::EventBridge(pattern) => {
                    eventbridge_messages.push((envelope, pattern));
                }
                AwsTarget::Sqs(msg) => {
                    sqs_messages
                        .entry(msg.queue_url.clone())
                        .or_insert_with(Vec::new)
                        .push((envelope, msg));
                }
                AwsTarget::Lambda(invocation) => {
                    lambda_invocations.push((envelope, invocation));
                }
                AwsTarget::DynamoDb(operation) => {
                    dynamodb_operations.push((envelope, operation));
                }
            }
        }
        
        let mut total_success = 0;
        let mut total_failed = 0;
        
        // Process EventBridge batches
        if !eventbridge_messages.is_empty() {
            let result = self.eventbridge_batcher.process_batch(eventbridge_messages).await?;
            total_success += result.success;
            total_failed += result.failed.len();
        }
        
        // Process SQS batches
        for (queue_url, messages) in sqs_messages {
            let result = self.sqs_batcher.process_batch(&queue_url, messages).await?;
            total_success += result.success;
            total_failed += result.failed.len();
        }
        
        // Lambda and DynamoDB are processed individually (for now)
        total_success += lambda_invocations.len();
        total_success += dynamodb_operations.len();
        
        Ok(BatchResult {
            total: total_success + total_failed,
            success: total_success,
            failed: vec![],
        })
    }
}

// EventBridge batcher
pub struct EventBridgeBatcher {
    batch_size: usize,
    batch_timeout: Duration,
    pending_batches: Arc<Mutex<Vec<EventBridgeBatch>>>,
}

#[derive(Default)]
struct EventBridgeBatch {
    entries: Vec<(NatsMessageEnvelope, crate::EventBridgePattern)>,
    created_at: Instant,
}

impl EventBridgeBatcher {
    pub fn new(batch_size: usize, batch_timeout: Duration) -> Self {
        let batcher = Self {
            batch_size,
            batch_timeout,
            pending_batches: Arc::new(Mutex::new(Vec::new())),
        };
        
        // Start background processor
        let pending_batches = batcher.pending_batches.clone();
        let batch_timeout = batcher.batch_timeout;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(batch_timeout / 2);
            loop {
                interval.tick().await;
                
                let mut batches = pending_batches.lock().await;
                let now = Instant::now();
                
                // Process timed-out batches
                batches.retain(|batch| {
                    if now.duration_since(batch.created_at) > batch_timeout {
                        // Process batch
                        // In production, would send to EventBridge
                        false
                    } else {
                        true
                    }
                });
            }
        });
        
        batcher
    }
    
    pub async fn process_batch(
        &self,
        messages: Vec<(NatsMessageEnvelope, crate::EventBridgePattern)>,
    ) -> Result<BatchResult, PerformanceError> {
        // Split into chunks of 10 (EventBridge limit)
        let mut success_count = 0;
        let mut failed_indices = Vec::new();
        
        for (chunk_idx, chunk) in messages.chunks(10).enumerate() {
            // In production, would call EventBridge PutEvents
            // For now, simulate success
            success_count += chunk.len();
        }
        
        Ok(BatchResult {
            total: messages.len(),
            success: success_count,
            failed: failed_indices,
        })
    }
}

// SQS batcher
pub struct SqsBatcher {
    batch_size: usize,
    batch_timeout: Duration,
}

impl SqsBatcher {
    pub fn new(batch_size: usize, batch_timeout: Duration) -> Self {
        Self {
            batch_size,
            batch_timeout,
        }
    }
    
    pub async fn process_batch(
        &self,
        queue_url: &str,
        messages: Vec<(NatsMessageEnvelope, crate::SqsMessage)>,
    ) -> Result<BatchResult, PerformanceError> {
        // Split into chunks of 10 (SQS limit)
        let mut success_count = 0;
        let mut failed_indices = Vec::new();
        
        for (chunk_idx, chunk) in messages.chunks(10).enumerate() {
            // In production, would call SQS SendMessageBatch
            // For now, simulate success
            success_count += chunk.len();
        }
        
        Ok(BatchResult {
            total: messages.len(),
            success: success_count,
            failed: failed_indices,
        })
    }
}

// Message ordering preserver
pub struct OrderingPreserver {
    sequence_tracker: Arc<RwLock<HashMap<String, SequenceInfo>>>,
    pending_messages: Arc<RwLock<HashMap<String, BTreeMap<u64, PendingMessage>>>>,
}

#[derive(Debug, Clone)]
struct SequenceInfo {
    last_processed: u64,
    highest_seen: u64,
}

#[derive(Debug, Clone)]
struct PendingMessage {
    envelope: NatsMessageEnvelope,
    target: AwsTarget,
    received_at: chrono::DateTime<chrono::Utc>,
}

impl OrderingPreserver {
    pub fn new() -> Self {
        Self {
            sequence_tracker: Arc::new(RwLock::new(HashMap::new())),
            pending_messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn process_ordered_message<F, Fut>(
        &self,
        partition_key: &str,
        sequence: u64,
        envelope: NatsMessageEnvelope,
        target: AwsTarget,
        processor: F,
    ) -> Result<(), PerformanceError>
    where
        F: Fn(NatsMessageEnvelope, AwsTarget) -> Fut,
        Fut: std::future::Future<Output = Result<(), TranslationError>>,
    {
        let mut tracker = self.sequence_tracker.write().await;
        let seq_info = tracker.entry(partition_key.to_string())
            .or_insert(SequenceInfo {
                last_processed: 0,
                highest_seen: 0,
            });
        
        seq_info.highest_seen = seq_info.highest_seen.max(sequence);
        
        if sequence == seq_info.last_processed + 1 {
            // Process immediately
            processor(envelope, target).await
                .map_err(|e| PerformanceError::OrderingError(e.to_string()))?;
            seq_info.last_processed = sequence;
            
            // Process any pending messages
            let mut pending = self.pending_messages.write().await;
            if let Some(partition_pending) = pending.get_mut(partition_key) {
                while let Some((next_seq, _)) = partition_pending.iter().next() {
                    if *next_seq == seq_info.last_processed + 1 {
                        let (_, msg) = partition_pending.pop_first().unwrap();
                        processor(msg.envelope, msg.target).await
                            .map_err(|e| PerformanceError::OrderingError(e.to_string()))?;
                        seq_info.last_processed = *next_seq;
                    } else {
                        break;
                    }
                }
            }
        } else if sequence > seq_info.last_processed + 1 {
            // Store for later processing
            let mut pending = self.pending_messages.write().await;
            pending.entry(partition_key.to_string())
                .or_insert_with(BTreeMap::new)
                .insert(sequence, PendingMessage {
                    envelope,
                    target,
                    received_at: chrono::Utc::now(),
                });
        }
        
        Ok(())
    }
}

// Latency compensator with predictive caching
pub struct LatencyCompensator {
    predictive_cache: Arc<PredictiveCache>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl LatencyCompensator {
    pub fn new() -> Self {
        Self {
            predictive_cache: Arc::new(PredictiveCache::new(Duration::from_secs(300))),
            circuit_breaker: Arc::new(CircuitBreaker::new()),
        }
    }
    
    pub async fn compensate_request<T, F, Fut>(
        &self,
        operation: &str,
        cache_key: &str,
        request: F,
    ) -> Result<T, PerformanceError>
    where
        T: Clone + Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, TranslationError>>,
    {
        // Check predictive cache
        if let Some(cached) = self.predictive_cache.get::<T>(cache_key).await {
            counter!("latency_compensator_cache_hits", 1,
                "operation" => operation.to_string(),
            );
            return Ok(cached);
        }
        
        // Check circuit breaker
        if !self.circuit_breaker.is_available(operation).await {
            return Err(PerformanceError::CircuitBreakerOpen(operation.to_string()));
        }
        
        // Execute with timing
        let start = Instant::now();
        match tokio::time::timeout(Duration::from_secs(30), request()).await {
            Ok(Ok(value)) => {
                let latency = start.elapsed();
                histogram!("operation_latency_ms", latency.as_millis() as f64,
                    "operation" => operation.to_string(),
                    "status" => "success",
                );
                
                // Update circuit breaker
                self.circuit_breaker.record_success(operation).await;
                
                // Cache if latency is high
                if latency > Duration::from_millis(100) {
                    self.predictive_cache.put(cache_key, value.clone()).await;
                }
                
                Ok(value)
            }
            Ok(Err(e)) => {
                self.circuit_breaker.record_failure(operation).await;
                Err(PerformanceError::BatchError(e.to_string()))
            }
            Err(_) => {
                self.circuit_breaker.record_failure(operation).await;
                Err(PerformanceError::BatchError("Request timeout".to_string()))
            }
        }
    }
}

// Predictive cache implementation
pub struct PredictiveCache {
    cache: Arc<DashMap<String, (Box<dyn std::any::Any + Send + Sync>, Instant)>>,
    ttl: Duration,
}

impl PredictiveCache {
    pub fn new(ttl: Duration) -> Self {
        let cache = Arc::new(DashMap::new());
        
        // Start background cleanup
        let cache_clone = cache.clone();
        let ttl_clone = ttl;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(ttl_clone / 2);
            loop {
                interval.tick().await;
                
                let now = Instant::now();
                cache_clone.retain(|_, (_, timestamp)| {
                    now.duration_since(*timestamp) < ttl_clone
                });
            }
        });
        
        Self { cache, ttl }
    }
    
    pub async fn get<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        if let Some((value, timestamp)) = self.cache.get(key) {
            if timestamp.elapsed() < self.ttl {
                return value.downcast_ref::<T>().cloned();
            }
        }
        None
    }
    
    pub async fn put<T: Clone + Send + Sync + 'static>(&self, key: &str, value: T) {
        self.cache.insert(key.to_string(), (Box::new(value), Instant::now()));
    }
}

// Circuit breaker implementation
pub struct CircuitBreaker {
    states: Arc<DashMap<String, CircuitState>>,
    failure_threshold: u32,
    recovery_timeout: Duration,
}

#[derive(Debug, Clone)]
struct CircuitState {
    failures: u32,
    last_failure: Option<Instant>,
    state: BreakerState,
}

#[derive(Debug, Clone, PartialEq)]
enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            states: Arc::new(DashMap::new()),
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
        }
    }
    
    pub async fn is_available(&self, service: &str) -> bool {
        let mut entry = self.states.entry(service.to_string())
            .or_insert(CircuitState {
                failures: 0,
                last_failure: None,
                state: BreakerState::Closed,
            });
        
        match entry.state {
            BreakerState::Closed => true,
            BreakerState::Open => {
                if let Some(last_failure) = entry.last_failure {
                    if last_failure.elapsed() > self.recovery_timeout {
                        entry.state = BreakerState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            BreakerState::HalfOpen => true,
        }
    }
    
    pub async fn record_success(&self, service: &str) {
        if let Some(mut entry) = self.states.get_mut(service) {
            if entry.state == BreakerState::HalfOpen {
                entry.state = BreakerState::Closed;
                entry.failures = 0;
            }
        }
    }
    
    pub async fn record_failure(&self, service: &str) {
        let mut entry = self.states.entry(service.to_string())
            .or_insert(CircuitState {
                failures: 0,
                last_failure: None,
                state: BreakerState::Closed,
            });
        
        entry.failures += 1;
        entry.last_failure = Some(Instant::now());
        
        if entry.failures >= self.failure_threshold {
            entry.state = BreakerState::Open;
        }
    }
}

// Batch result
#[derive(Debug, Clone)]
pub struct BatchResult {
    pub total: usize,
    pub success: usize,
    pub failed: Vec<usize>,
}

// Performance configuration
#[derive(Debug, Clone, Deserialize)]
pub struct PerformanceConfig {
    pub batch_size: usize,
    pub batch_timeout_ms: u64,
    pub cache_ttl_secs: u64,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_recovery_secs: u64,
}