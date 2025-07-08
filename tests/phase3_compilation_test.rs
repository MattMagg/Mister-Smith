//! Phase 3 Module Compilation Test
//! 
//! This test ensures all Phase 3 modules compile correctly
//! and are accessible from external code.

use mistersmith::{
    // Core modules
    agent::{Agent, AgentId, AgentState, AgentPool},
    runtime::{ProcessManager, ClaudeExecutor},
    transport::{NatsTransport, TransportError},
    
    // Phase 3 modules
    supervision::{
        SupervisionTree, SupervisionTreeConfig,
        Supervisor, SupervisorNode, NodeId, NodeType,
        SupervisionStrategy, RestartPolicy, FailureReason,
        CircuitBreaker, CircuitBreakerState,
        FailureDetector, Phi, PhiWindow,
        AgentPoolSupervisor,
        types::{ChildId, ChildRef, ChildState, SupervisionDecision}
    },
    persistence::{
        SupervisionPersistence, SupervisionDb, TaskDb,
        ConnectionPool,
    },
    metrics::{
        MetricsCollector, MetricsRegistry, PrometheusExporter,
        AgentMetrics, SupervisionMetrics,
    }
};

#[test]
fn test_phase3_modules_compile() {
    // This test just ensures all modules compile
    // and exports are accessible
    
    // Test supervision types exist
    let _strategy = SupervisionStrategy::OneForOne;
    let _node_id = NodeId::new("test");
    let _child_id = ChildId::new("child");
    
    // Test persistence types exist
    // Note: These would need actual implementations
    
    // Test metrics types exist
    // Note: These would need actual implementations
    
    println!("✅ All Phase 3 modules compile successfully!");
}

#[test]
fn test_agent_pool_supervisor_exists() {
    // Verify the core Phase 3 integration component exists
    use std::sync::Arc;
    
    let agent_pool = Arc::new(AgentPool::new(3));
    let config = mistersmith::agent::AgentConfig::default();
    
    // Create supervisor (just test it compiles)
    let _supervisor = AgentPoolSupervisor::new(
        "test-supervisor".to_string(),
        agent_pool,
        config,
        SupervisionStrategy::OneForOne,
        RestartPolicy::default(),
    );
    
    println!("✅ AgentPoolSupervisor compiles and can be instantiated!");
}