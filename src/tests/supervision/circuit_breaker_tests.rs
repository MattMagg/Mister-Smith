//! Tests for circuit breaker implementation

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use crate::supervision::*;

#[tokio::test]
async fn test_circuit_breaker_closed_state() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
    
    // Should be closed initially
    assert_eq!(cb.get_state().await, CircuitState::Closed);
    
    // Successful calls should work
    let result = cb.call(async { Ok::<_, std::io::Error>(42) }).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    
    // State should remain closed
    assert_eq!(cb.get_state().await, CircuitState::Closed);
}

#[tokio::test]
async fn test_circuit_breaker_opens_on_failures() {
    let mut config = CircuitBreakerConfig::default();
    config.failure_threshold = 3;
    
    let cb = CircuitBreaker::new(config);
    
    // Cause failures
    for i in 0..3 {
        let _ = cb.call(async move {
            Err::<(), _>(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("test error {}", i),
            ))
        }).await;
    }
    
    // Circuit should be open
    assert_eq!(cb.get_state().await, CircuitState::Open);
    
    // Next call should fail fast
    let result = cb.call(async { Ok::<_, std::io::Error>(()) }).await;
    assert!(matches!(result, Err(CircuitBreakerError::Open)));
}

#[tokio::test]
async fn test_circuit_breaker_half_open_transition() {
    let mut config = CircuitBreakerConfig::default();
    config.failure_threshold = 2;
    config.reset_timeout = Duration::from_millis(100);
    
    let cb = CircuitBreaker::new(config);
    
    // Open the circuit
    for _ in 0..2 {
        let _ = cb.call(async {
            Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "test"))
        }).await;
    }
    
    assert_eq!(cb.get_state().await, CircuitState::Open);
    
    // Wait for reset timeout
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Next call should transition to half-open and execute
    let _result = cb.call(async { Ok::<_, std::io::Error>(42) }).await;
    
    // Should be in half-open or closed (if succeeded)
    let state = cb.get_state().await;
    assert!(state == CircuitState::HalfOpen || state == CircuitState::Closed);
}

#[tokio::test]
async fn test_circuit_breaker_half_open_success() {
    let mut config = CircuitBreakerConfig::default();
    config.failure_threshold = 2;
    config.reset_timeout = Duration::from_millis(50);
    config.success_threshold_to_close = 2;
    
    let cb = CircuitBreaker::new(config);
    
    // Open the circuit
    for _ in 0..2 {
        let _ = cb.call(async {
            Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "test"))
        }).await;
    }
    
    // Wait and transition to half-open
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Successful calls in half-open
    for i in 0..2 {
        let result = cb.call(async move { Ok::<_, std::io::Error>(i) }).await;
        assert!(result.is_ok());
    }
    
    // Should be closed now
    assert_eq!(cb.get_state().await, CircuitState::Closed);
}

#[tokio::test]
async fn test_circuit_breaker_half_open_failure() {
    let mut config = CircuitBreakerConfig::default();
    config.failure_threshold = 2;
    config.reset_timeout = Duration::from_millis(50);
    
    let cb = CircuitBreaker::new(config);
    
    // Open the circuit
    for _ in 0..2 {
        let _ = cb.call(async {
            Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "test"))
        }).await;
    }
    
    // Wait and transition to half-open
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Fail in half-open state
    let _ = cb.call(async {
        Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "test"))
    }).await;
    
    // Should be open again
    assert_eq!(cb.get_state().await, CircuitState::Open);
}

#[tokio::test]
async fn test_circuit_breaker_metrics() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
    
    // Make some successful calls
    for i in 0..5 {
        let _ = cb.call(async move { Ok::<_, std::io::Error>(i) }).await;
    }
    
    // Make some failed calls
    for _ in 0..3 {
        let _ = cb.call(async {
            Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "test"))
        }).await;
    }
    
    let metrics = cb.get_metrics().await;
    assert_eq!(metrics.successful_calls, 5);
    assert_eq!(metrics.failed_calls, 3);
    assert_eq!(metrics.total_calls, 8);
    assert!(metrics.success_rate() > 0.6 && metrics.success_rate() < 0.7);
}

#[tokio::test]
async fn test_circuit_breaker_timeout() {
    let mut config = CircuitBreakerConfig::default();
    config.operation_timeout = Some(Duration::from_millis(50));
    
    let cb = CircuitBreaker::new(config);
    
    // Operation that takes too long
    let result = cb.call(async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok::<_, std::io::Error>(())
    }).await;
    
    assert!(matches!(result, Err(CircuitBreakerError::Timeout)));
}

#[tokio::test]
async fn test_circuit_breaker_manual_reset() {
    let mut config = CircuitBreakerConfig::default();
    config.failure_threshold = 2;
    
    let cb = CircuitBreaker::new(config);
    
    // Open the circuit
    for _ in 0..2 {
        let _ = cb.call(async {
            Err::<(), _>(std::io::Error::new(std::io::ErrorKind::Other, "test"))
        }).await;
    }
    
    assert_eq!(cb.get_state().await, CircuitState::Open);
    
    // Manual reset
    cb.reset().await;
    
    // Should be closed
    assert_eq!(cb.get_state().await, CircuitState::Closed);
    
    // Calls should work again
    let result = cb.call(async { Ok::<_, std::io::Error>(42) }).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_circuit_breaker_concurrent_calls() {
    let cb = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
    let counter = Arc::new(AtomicU32::new(0));
    
    // Spawn multiple concurrent calls
    let mut handles = vec![];
    
    for i in 0..10 {
        let cb_clone = cb.clone();
        let counter_clone = counter.clone();
        
        let handle = tokio::spawn(async move {
            let result = cb_clone.call(async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Ok::<_, std::io::Error>(i)
            }).await;
            
            assert!(result.is_ok());
        });
        
        handles.push(handle);
    }
    
    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}