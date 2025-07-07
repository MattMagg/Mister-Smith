//! Circuit breaker implementation for preventing cascading failures
//!
//! The circuit breaker pattern prevents system overload by failing fast when
//! a service is experiencing issues, allowing it time to recover.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::future::Future;
use tokio::sync::Mutex;
use tokio::time::{timeout, sleep};
use thiserror::Error;
use tracing::{info, warn, error, debug};

/// State of the circuit breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests allowed
    Closed,
    /// Circuit tripped - requests fail fast
    Open,
    /// Testing if service recovered - limited requests allowed
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    /// Current state of the circuit
    state: Arc<Mutex<CircuitBreakerState>>,
    /// Configuration
    config: CircuitBreakerConfig,
    /// Metrics
    metrics: Arc<Mutex<CircuitBreakerMetrics>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitBreakerState::new())),
            config,
            metrics: Arc::new(Mutex::new(CircuitBreakerMetrics::default())),
        }
    }

    /// Execute an operation with circuit breaker protection
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // Check circuit state
        let current_state = self.get_state().await;
        
        match current_state {
            CircuitState::Closed => {
                // Normal operation
                self.execute_with_tracking(operation).await
            }
            CircuitState::Open => {
                // Check if we should transition to half-open
                if self.should_attempt_reset().await {
                    self.transition_to_half_open().await;
                    self.execute_half_open(operation).await
                } else {
                    // Fail fast
                    self.metrics.lock().await.rejected_calls += 1;
                    Err(CircuitBreakerError::Open)
                }
            }
            CircuitState::HalfOpen => {
                // Limited calls allowed
                self.execute_half_open(operation).await
            }
        }
    }

    /// Get current circuit state
    pub async fn get_state(&self) -> CircuitState {
        self.state.lock().await.current
    }

    /// Get circuit breaker metrics
    pub async fn get_metrics(&self) -> CircuitBreakerMetrics {
        self.metrics.lock().await.clone()
    }

    /// Reset the circuit breaker
    pub async fn reset(&self) {
        info!("Manually resetting circuit breaker");
        
        let mut state = self.state.lock().await;
        state.current = CircuitState::Closed;
        state.consecutive_failures = 0;
        state.half_open_successes = 0;
        
        let mut metrics = self.metrics.lock().await;
        metrics.state_changes += 1;
    }

    /// Execute operation and track results
    async fn execute_with_tracking<F, T, E>(
        &self,
        operation: F,
    ) -> Result<T, CircuitBreakerError>
    where
        F: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let start = Instant::now();
        
        // Apply timeout if configured
        let result = if let Some(op_timeout) = self.config.operation_timeout {
            match timeout(op_timeout, operation).await {
                Ok(inner_result) => inner_result.map_err(|e| CircuitBreakerError::Operation(Box::new(e))),
                Err(_) => Err(CircuitBreakerError::Timeout),
            }
        } else {
            operation.await.map_err(|e| CircuitBreakerError::Operation(Box::new(e)))
        };
        
        let duration = start.elapsed();
        
        // Update metrics
        {
            let mut metrics = self.metrics.lock().await;
            metrics.total_calls += 1;
            
            if result.is_ok() {
                metrics.successful_calls += 1;
                metrics.total_success_duration += duration;
            } else {
                metrics.failed_calls += 1;
            }
        }
        
        // Update state based on result
        self.record_result(&result).await;
        
        result
    }

    /// Execute operation in half-open state
    async fn execute_half_open<F, T, E>(
        &self,
        operation: F,
    ) -> Result<T, CircuitBreakerError>
    where
        F: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // Check if we can make another test call
        let can_attempt = {
            let state = self.state.lock().await;
            state.half_open_attempts < self.config.half_open_max_calls
        };
        
        if !can_attempt {
            self.metrics.lock().await.rejected_calls += 1;
            return Err(CircuitBreakerError::HalfOpenLimitExceeded);
        }
        
        // Increment attempt counter
        self.state.lock().await.half_open_attempts += 1;
        
        // Execute and track
        self.execute_with_tracking(operation).await
    }

    /// Record operation result and update state
    async fn record_result<T>(&self, result: &Result<T, CircuitBreakerError>) {
        let mut state = self.state.lock().await;
        let mut metrics = self.metrics.lock().await;
        
        match (state.current, result) {
            (CircuitState::Closed, Ok(_)) => {
                // Reset failure counter on success
                state.consecutive_failures = 0;
            }
            (CircuitState::Closed, Err(_)) => {
                // Increment failure counter
                state.consecutive_failures += 1;
                state.last_failure_time = Some(Instant::now());
                
                // Check if we should open the circuit
                if state.consecutive_failures >= self.config.failure_threshold {
                    warn!(
                        "Opening circuit breaker after {} consecutive failures",
                        state.consecutive_failures
                    );
                    state.current = CircuitState::Open;
                    state.opened_at = Some(Instant::now());
                    metrics.state_changes += 1;
                }
            }
            (CircuitState::HalfOpen, Ok(_)) => {
                // Success in half-open state
                state.half_open_successes += 1;
                
                // Check if we should close the circuit
                if state.half_open_successes >= self.config.success_threshold_to_close {
                    info!("Closing circuit breaker after successful recovery");
                    state.current = CircuitState::Closed;
                    state.consecutive_failures = 0;
                    state.half_open_successes = 0;
                    state.half_open_attempts = 0;
                    metrics.state_changes += 1;
                }
            }
            (CircuitState::HalfOpen, Err(_)) => {
                // Failure in half-open state - reopen immediately
                warn!("Reopening circuit breaker after failure in half-open state");
                state.current = CircuitState::Open;
                state.opened_at = Some(Instant::now());
                state.half_open_successes = 0;
                state.half_open_attempts = 0;
                metrics.state_changes += 1;
            }
            _ => {} // Other combinations don't change state
        }
    }

    /// Check if enough time has passed to attempt reset
    async fn should_attempt_reset(&self) -> bool {
        let state = self.state.lock().await;
        
        if let Some(opened_at) = state.opened_at {
            opened_at.elapsed() >= self.config.reset_timeout
        } else {
            false
        }
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        info!("Transitioning circuit breaker to half-open state");
        
        let mut state = self.state.lock().await;
        state.current = CircuitState::HalfOpen;
        state.half_open_attempts = 0;
        state.half_open_successes = 0;
        
        self.metrics.lock().await.state_changes += 1;
    }
}

/// Internal state of the circuit breaker
#[derive(Debug)]
struct CircuitBreakerState {
    current: CircuitState,
    consecutive_failures: u32,
    last_failure_time: Option<Instant>,
    opened_at: Option<Instant>,
    half_open_attempts: u32,
    half_open_successes: u32,
}

impl CircuitBreakerState {
    fn new() -> Self {
        Self {
            current: CircuitState::Closed,
            consecutive_failures: 0,
            last_failure_time: None,
            opened_at: None,
            half_open_attempts: 0,
            half_open_successes: 0,
        }
    }
}

/// Configuration for circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Time to wait before attempting to close circuit
    pub reset_timeout: Duration,
    /// Maximum calls allowed in half-open state
    pub half_open_max_calls: u32,
    /// Successes required in half-open to close circuit
    pub success_threshold_to_close: u32,
    /// Optional timeout for operations
    pub operation_timeout: Option<Duration>,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            half_open_max_calls: 3,
            success_threshold_to_close: 2,
            operation_timeout: Some(Duration::from_secs(10)),
        }
    }
}

/// Metrics for circuit breaker monitoring
#[derive(Debug, Clone, Default)]
pub struct CircuitBreakerMetrics {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub rejected_calls: u64,
    pub state_changes: u64,
    pub total_success_duration: Duration,
}

impl CircuitBreakerMetrics {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 {
            0.0
        } else {
            self.successful_calls as f64 / self.total_calls as f64
        }
    }

    /// Calculate average success duration
    pub fn avg_success_duration(&self) -> Duration {
        if self.successful_calls == 0 {
            Duration::ZERO
        } else {
            self.total_success_duration / self.successful_calls as u32
        }
    }
}

/// Errors that can occur with circuit breaker
#[derive(Debug, Error)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker is open")]
    Open,
    
    #[error("Half-open call limit exceeded")]
    HalfOpenLimitExceeded,
    
    #[error("Operation timeout")]
    Timeout,
    
    #[error("Operation failed: {0}")]
    Operation(Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    #[tokio::test]
    async fn test_circuit_breaker_closed_success() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        let result = cb.call(async { Ok::<_, std::io::Error>(42) }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let mut config = CircuitBreakerConfig::default();
        config.failure_threshold = 3;
        let cb = CircuitBreaker::new(config);
        
        // Cause failures
        for _ in 0..3 {
            let _ = cb.call(async {
                Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "test"))
            }).await;
        }
        
        assert_eq!(cb.get_state().await, CircuitState::Open);
        
        // Next call should fail fast
        let result = cb.call(async { Ok::<_, std::io::Error>(()) }).await;
        assert!(matches!(result, Err(CircuitBreakerError::Open)));
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_half_open_recovery() {
        let mut config = CircuitBreakerConfig::default();
        config.failure_threshold = 2;
        config.reset_timeout = Duration::from_millis(100);
        config.success_threshold_to_close = 1;
        
        let cb = CircuitBreaker::new(config);
        
        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(async {
                Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "test"))
            }).await;
        }
        
        assert_eq!(cb.get_state().await, CircuitState::Open);
        
        // Wait for reset timeout
        sleep(Duration::from_millis(150)).await;
        
        // Should transition to half-open and succeed
        let result = cb.call(async { Ok::<_, std::io::Error>(42) }).await;
        assert!(result.is_ok());
        
        // Should be closed now
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }
}