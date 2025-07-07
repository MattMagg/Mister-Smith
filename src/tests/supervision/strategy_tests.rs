//! Tests for supervision strategies

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::supervision::*;

/// Mock supervisor for testing strategies
#[derive(Debug)]
struct MockSupervisor {
    strategy: SupervisionStrategy,
    restart_policy: RestartPolicy,
    restart_counter: Arc<AtomicU32>,
}

#[async_trait::async_trait]
impl Supervisor for MockSupervisor {
    type Child = ChildRef;

    async fn supervise(&self, _children: Vec<Self::Child>) -> SupervisionResult {
        SupervisionResult::Running
    }

    fn supervision_strategy(&self) -> SupervisionStrategy {
        self.strategy
    }

    fn restart_policy(&self) -> RestartPolicy {
        self.restart_policy.clone()
    }

    fn escalation_policy(&self) -> EscalationPolicy {
        EscalationPolicy::default()
    }

    async fn route_task(&self, _task: Task) -> Option<ChildId> {
        None
    }

    async fn handle_child_failure(
        &self,
        _child_id: ChildId,
        _error: FailureReason,
    ) -> SupervisionDecision {
        self.restart_counter.fetch_add(1, Ordering::SeqCst);
        SupervisionDecision::Handled
    }
}

#[tokio::test]
async fn test_one_for_one_strategy() {
    let supervisor = Arc::new(MockSupervisor {
        strategy: SupervisionStrategy::OneForOne,
        restart_policy: RestartPolicy::max_restarts(3),
        restart_counter: Arc::new(AtomicU32::new(0)),
    });

    let node = SupervisorNode::new(
        NodeId("test".to_string()),
        NodeType::AgentSupervisor("test".to_string()),
        None,
        supervisor.clone(),
    );

    // Add children
    for i in 0..3 {
        node.add_child(ChildRef {
            id: ChildId(format!("child-{}", i)),
            node_type: NodeType::Worker("test".to_string()),
            state: Arc::new(tokio::sync::RwLock::new(ChildState::Running)),
        }).await.unwrap();
    }

    // Simulate failure of one child
    let decision = node.handle_child_failure(
        ChildId("child-1".to_string()),
        FailureReason::Crash("test crash".to_string()),
    ).await.unwrap();

    assert!(matches!(decision, SupervisionDecision::Handled));
    assert_eq!(supervisor.restart_counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_restart_policy_enforcement() {
    let policy = RestartPolicy::max_restarts(3);
    
    // Test within limit
    let history = vec![
        FailureRecord {
            timestamp: std::time::Instant::now(),
            reason: FailureReason::Crash("test".to_string()),
        },
        FailureRecord {
            timestamp: std::time::Instant::now(),
            reason: FailureReason::Crash("test".to_string()),
        },
    ];
    
    assert!(policy.should_restart(&history));
    
    // Test at limit
    let mut history = history;
    history.push(FailureRecord {
        timestamp: std::time::Instant::now(),
        reason: FailureReason::Crash("test".to_string()),
    });
    
    assert!(!policy.should_restart(&history));
}

#[tokio::test]
async fn test_exponential_backoff() {
    let policy = RestartPolicy::exponential_backoff();
    
    // Test delay calculation
    assert_eq!(policy.calculate_delay(0), policy.min_restart_delay);
    
    let delay1 = policy.calculate_delay(1);
    let delay2 = policy.calculate_delay(2);
    let delay3 = policy.calculate_delay(3);
    
    // Each delay should be exponentially larger
    assert!(delay2 > delay1);
    assert!(delay3 > delay2);
    
    // But capped at max
    let delay_max = policy.calculate_delay(100);
    assert_eq!(delay_max, policy.max_restart_delay);
}

#[tokio::test]
async fn test_escalation_strategy() {
    let supervisor = Arc::new(MockSupervisor {
        strategy: SupervisionStrategy::Escalate,
        restart_policy: RestartPolicy::max_restarts(1),
        restart_counter: Arc::new(AtomicU32::new(0)),
    });

    let node = SupervisorNode::new(
        NodeId("test".to_string()),
        NodeType::AgentSupervisor("test".to_string()),
        Some(NodeId("parent".to_string())), // Has parent
        supervisor,
    );

    // With escalate strategy, failures should not be handled locally
    // In a full implementation, this would propagate to parent
}

#[tokio::test]
async fn test_restart_policy_time_window() {
    let mut policy = RestartPolicy::max_restarts(3);
    policy.time_window = std::time::Duration::from_millis(100);
    
    // Create old failures outside time window
    let old_failure = FailureRecord {
        timestamp: std::time::Instant::now() - std::time::Duration::from_secs(1),
        reason: FailureReason::Crash("old".to_string()),
    };
    
    let recent_failure = FailureRecord {
        timestamp: std::time::Instant::now(),
        reason: FailureReason::Crash("recent".to_string()),
    };
    
    let history = vec![old_failure, recent_failure];
    
    // Should allow restart as only one failure is within window
    assert!(policy.should_restart(&history));
}

#[tokio::test]
async fn test_always_restart_policy() {
    let policy = RestartPolicy::always_restart();
    
    // Create many failures
    let mut history = Vec::new();
    for i in 0..100 {
        history.push(FailureRecord {
            timestamp: std::time::Instant::now(),
            reason: FailureReason::Crash(format!("failure-{}", i)),
        });
    }
    
    // Should still allow restart
    assert!(policy.should_restart(&history));
}

#[tokio::test]
async fn test_custom_escalation_policy() {
    let mut policy = EscalationPolicy::default();
    policy.escalate_on_error_types = vec![
        "ResourceExhaustion".to_string(),
        "SystemFailure".to_string(),
    ];
    
    // Test that policy contains expected error types
    assert!(policy.escalate_on_error_types.contains(&"ResourceExhaustion".to_string()));
    assert!(!policy.escalate_on_error_types.contains(&"Crash".to_string()));
}