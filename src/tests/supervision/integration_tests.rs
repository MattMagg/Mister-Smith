//! Integration tests for the complete supervision system

use std::sync::Arc;
use std::time::Duration;
use crate::supervision::*;

#[tokio::test]
async fn test_complete_supervision_tree_setup() {
    // Create supervision tree
    let tree_config = SupervisionTreeConfig::default();
    let tree = Arc::new(SupervisionTree::new(tree_config));
    
    // Create root supervisor with OneForAll strategy
    let root_supervisor = Arc::new(RootSupervisor::new(SupervisionStrategy::OneForAll));
    let root_node = Arc::new(SupervisorNode::new(
        NodeId("root".to_string()),
        NodeType::RootSupervisor,
        None,
        root_supervisor,
    ));
    
    // Set root
    tree.set_root(root_node.clone()).await.unwrap();
    
    // Create agent supervisors
    let research_supervisor = Arc::new(AgentSupervisor::new(
        "research",
        SupervisionStrategy::OneForOne,
        RestartPolicy::exponential_backoff(),
    ));
    
    let research_node = Arc::new(SupervisorNode::new(
        NodeId("research-supervisor".to_string()),
        NodeType::AgentSupervisor("research".to_string()),
        Some(NodeId("root".to_string())),
        research_supervisor,
    ));
    
    let code_supervisor = Arc::new(AgentSupervisor::new(
        "code",
        SupervisionStrategy::OneForOne,
        RestartPolicy::max_restarts(5),
    ));
    
    let code_node = Arc::new(SupervisorNode::new(
        NodeId("code-supervisor".to_string()),
        NodeType::AgentSupervisor("code".to_string()),
        Some(NodeId("root".to_string())),
        code_supervisor,
    ));
    
    // Register nodes
    tree.register_node(research_node.clone()).await.unwrap();
    tree.register_node(code_node.clone()).await.unwrap();
    
    // Add as children to root
    root_node.add_child(ChildRef {
        id: ChildId("research-supervisor".to_string()),
        node_type: NodeType::AgentSupervisor("research".to_string()),
        state: Arc::new(tokio::sync::RwLock::new(ChildState::Running)),
    }).await.unwrap();
    
    root_node.add_child(ChildRef {
        id: ChildId("code-supervisor".to_string()),
        node_type: NodeType::AgentSupervisor("code".to_string()),
        state: Arc::new(tokio::sync::RwLock::new(ChildState::Running)),
    }).await.unwrap();
    
    // Verify structure
    assert_eq!(tree.all_nodes().await.len(), 3); // root + 2 supervisors
    assert_eq!(root_node.get_children().await.len(), 2);
}

#[tokio::test]
async fn test_failure_propagation() {
    let tree = Arc::new(SupervisionTree::new(SupervisionTreeConfig::default()));
    
    // Create hierarchy with escalation
    let root = Arc::new(RootSupervisor::new(SupervisionStrategy::OneForAll));
    let root_node = Arc::new(SupervisorNode::new(
        NodeId("root".to_string()),
        NodeType::RootSupervisor,
        None,
        root,
    ));
    
    tree.set_root(root_node.clone()).await.unwrap();
    
    // Child with escalate strategy
    let child_supervisor = Arc::new(TestEscalatingSupervisor);
    let child_node = Arc::new(SupervisorNode::new(
        NodeId("child".to_string()),
        NodeType::AgentSupervisor("test".to_string()),
        Some(NodeId("root".to_string())),
        child_supervisor,
    ));
    
    tree.register_node(child_node.clone()).await.unwrap();
    
    // Test failure handling
    let event = FailureEvent {
        node_id: NodeId("child".to_string()),
        reason: FailureReason::ResourceExhaustion("out of memory".to_string()),
        timestamp: std::time::Instant::now(),
    };
    
    // This should escalate to root
    tree.handle_node_failure(event).await.unwrap();
}

#[tokio::test]
async fn test_circuit_breaker_integration() {
    // Create a service protected by circuit breaker
    let cb = Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_millis(100),
        half_open_max_calls: 2,
        success_threshold_to_close: 2,
        operation_timeout: Some(Duration::from_secs(1)),
    }));
    
    // Simulate external service
    let failure_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let service = MockExternalService {
        failure_count: failure_count.clone(),
        fail_until: 5,
    };
    
    // Make calls through circuit breaker
    for i in 0..10 {
        let service_clone = service.clone();
        let result = cb.call(async move {
            service_clone.call(i).await
        }).await;
        
        match result {
            Ok(_) => println!("Call {} succeeded", i),
            Err(CircuitBreakerError::Open) => println!("Call {} rejected (circuit open)", i),
            Err(e) => println!("Call {} failed: {:?}", i, e),
        }
        
        // Small delay between calls
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    // Circuit should have opened and then recovered
    let metrics = cb.get_metrics().await;
    assert!(metrics.state_changes > 0);
}

#[tokio::test]
async fn test_dynamic_supervisor() {
    let factory = || ChildSpec {
        id: format!("worker-{}", uuid::Uuid::new_v4()),
        node_type: NodeType::Worker("dynamic".to_string()),
        start_timeout: Duration::from_secs(5),
    };
    
    let supervisor = Arc::new(DynamicSupervisor::new(
        "dynamic-pool",
        SupervisionStrategy::OneForOne,
        factory,
    ));
    
    let node = SupervisorNode::new(
        NodeId("dynamic".to_string()),
        NodeType::AgentSupervisor("dynamic".to_string()),
        None,
        supervisor,
    );
    
    // Route tasks - should spawn children as needed
    for i in 0..5 {
        let task = Task {
            id: format!("task-{}", i),
            task_type: TaskType::Custom("dynamic".to_string()),
            payload: serde_json::json!({ "work": i }),
            priority: 5,
        };
        
        let child_id = node.route_task(task).await;
        assert!(child_id.is_some());
    }
}

#[tokio::test]
async fn test_restart_policies_with_metrics() {
    let tree = SupervisionTree::new(SupervisionTreeConfig::default());
    
    // Set different policies for different node types
    let mut policies = std::collections::HashMap::new();
    
    policies.insert(
        NodeType::Worker("critical".to_string()),
        RestartPolicy::always_restart(),
    );
    
    policies.insert(
        NodeType::Worker("normal".to_string()),
        RestartPolicy::max_restarts(3),
    );
    
    policies.insert(
        NodeType::Worker("experimental".to_string()),
        RestartPolicy {
            max_restarts: 1,
            time_window: Duration::from_secs(60),
            backoff_strategy: BackoffStrategy::Fixed,
            min_restart_delay: Duration::from_secs(5),
            max_restart_delay: Duration::from_secs(5),
        },
    );
    
    tree.set_restart_policies(policies).await;
    
    // Verify policies are applied
    let critical_policy = tree.get_restart_policy(&NodeType::Worker("critical".to_string())).await;
    assert!(critical_policy.is_some());
    assert_eq!(critical_policy.unwrap().max_restarts, u32::MAX);
}

// Helper implementations for testing

#[derive(Debug)]
struct TestEscalatingSupervisor;

#[async_trait::async_trait]
impl Supervisor for TestEscalatingSupervisor {
    type Child = ChildRef;

    async fn supervise(&self, _children: Vec<Self::Child>) -> SupervisionResult {
        SupervisionResult::Running
    }

    fn supervision_strategy(&self) -> SupervisionStrategy {
        SupervisionStrategy::Escalate
    }

    fn restart_policy(&self) -> RestartPolicy {
        RestartPolicy::max_restarts(0) // Don't restart
    }

    fn escalation_policy(&self) -> EscalationPolicy {
        let mut policy = EscalationPolicy::default();
        policy.escalate_on_error_types.push("ResourceExhaustion".to_string());
        policy
    }

    async fn route_task(&self, _task: Task) -> Option<ChildId> {
        None
    }

    async fn handle_child_failure(
        &self,
        _child_id: ChildId,
        error: FailureReason,
    ) -> SupervisionDecision {
        SupervisionDecision::Escalate(error.to_string())
    }
}

#[derive(Clone)]
struct MockExternalService {
    failure_count: Arc<std::sync::atomic::AtomicU32>,
    fail_until: u32,
}

impl MockExternalService {
    async fn call(&self, _request: u32) -> Result<String, std::io::Error> {
        let count = self.failure_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        if count < self.fail_until {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Service temporarily unavailable",
            ))
        } else {
            Ok("Success".to_string())
        }
    }
}