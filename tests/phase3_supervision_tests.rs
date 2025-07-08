//! Phase 3 Supervision Tree Tests
//! 
//! Tests supervision tree operations without full agent integration

use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_01_supervision_tree_root() {
    use mistersmith::supervision::*;
    
    // Create tree
    let config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(config);
    
    // Create a root supervisor
    let root_supervisor = RootSupervisor::new(SupervisionStrategy::OneForOne);
    let root_node = SupervisorNode::new(
        NodeId("root".to_string()),
        NodeType::RootSupervisor,
        None,
        Arc::new(root_supervisor),
    );
    
    // Set root
    tree.set_root(Arc::new(root_node)).await.expect("Should set root");
    
    println!("✅ Test 01: Supervision tree accepts root supervisor");
}

#[tokio::test]
async fn test_02_supervision_tree_lifecycle() {
    use mistersmith::supervision::*;
    
    let config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(config);
    
    // Start tree (should work even without root)
    tree.start().await.expect("Should start");
    
    // Stop tree
    tree.stop().await.expect("Should stop");
    
    println!("✅ Test 02: Supervision tree lifecycle works");
}

#[tokio::test]
async fn test_03_restart_policy_configuration() {
    use mistersmith::supervision::*;
    use std::time::Duration;
    
    // Test different restart policies
    let policy1 = RestartPolicy::exponential_backoff()
        .with_max_restarts(5)
        .with_time_window(Duration::from_secs(60));
    
    let policy2 = RestartPolicy::max_restarts(3)
        .with_min_restart_delay(Duration::from_millis(100));
    
    let policy3 = RestartPolicy::always_restart()
        .with_backoff_factor(2.0);
    
    println!("✅ Test 03: Restart policies can be configured");
}

#[tokio::test]
async fn test_04_circuit_breaker_operations() {
    use mistersmith::supervision::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
    
    let config = CircuitBreakerConfig::default();
    let breaker = CircuitBreaker::new(config);
    
    // Initial state should be closed
    assert!(matches!(breaker.get_state().await, CircuitState::Closed));
    
    // Record success
    breaker.record_success().await;
    assert!(matches!(breaker.get_state().await, CircuitState::Closed));
    
    // Allow operation
    let allowed = breaker.call_allowed().await;
    assert!(allowed);
    
    println!("✅ Test 04: Circuit breaker basic operations work");
}

#[tokio::test]
async fn test_05_failure_detector_monitoring() {
    use mistersmith::supervision::{FailureDetector, FailureDetectorConfig, NodeId};
    use std::time::Duration;
    
    let config = FailureDetectorConfig::default();
    let detector = FailureDetector::new(config);
    
    let node_id = NodeId("test-node".to_string());
    
    // Record some heartbeats
    for _ in 0..5 {
        detector.record_heartbeat(&node_id).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Check if node is healthy
    let is_failed = detector.is_node_failed(&node_id).await;
    assert!(!is_failed, "Node should not be failed after heartbeats");
    
    println!("✅ Test 05: Failure detector monitoring works");
}

#[tokio::test]
async fn test_06_supervision_metrics() {
    use mistersmith::supervision::*;
    
    let config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(config);
    
    // Get metrics
    let metrics = tree.metrics().await;
    
    // Check basic metrics
    assert_eq!(metrics.total_nodes(), 0, "Should have no nodes initially");
    assert_eq!(metrics.active_nodes(), 0, "Should have no active nodes");
    
    println!("✅ Test 06: Supervision metrics accessible");
}

#[tokio::test]
async fn test_07_supervision_node_hierarchy() {
    use mistersmith::supervision::*;
    
    // Create parent node
    let parent_supervisor = RootSupervisor::new(SupervisionStrategy::OneForAll);
    let parent_node = SupervisorNode::new(
        NodeId("parent".to_string()),
        NodeType::RootSupervisor,
        None,
        Arc::new(parent_supervisor),
    );
    
    // Create child node
    let child_supervisor = AgentSupervisor::new(
        "worker".to_string(),
        SupervisionStrategy::OneForOne,
    );
    let child_node = SupervisorNode::new(
        NodeId("child".to_string()),
        NodeType::AgentSupervisor("worker".to_string()),
        Some(NodeId("parent".to_string())),
        Arc::new(child_supervisor),
    );
    
    // Verify hierarchy
    assert_eq!(parent_node.id(), &NodeId("parent".to_string()));
    assert_eq!(child_node.parent_id(), Some(&NodeId("parent".to_string())));
    
    println!("✅ Test 07: Supervision node hierarchy can be created");
}