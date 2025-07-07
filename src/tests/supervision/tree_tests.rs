//! Tests for supervision tree structure and operations

use std::sync::Arc;
use crate::supervision::*;

#[tokio::test]
async fn test_supervision_tree_initialization() {
    let config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(config);
    
    // Initially empty
    assert_eq!(tree.all_nodes().await.len(), 0);
    
    // Cannot start without root
    assert!(tree.start().await.is_err());
}

#[tokio::test]
async fn test_set_root_supervisor() {
    let tree = SupervisionTree::new(SupervisionTreeConfig::default());
    
    // Create root supervisor
    let root_supervisor = Arc::new(RootSupervisor::new(SupervisionStrategy::OneForAll));
    let root_node = Arc::new(SupervisorNode::new(
        NodeId("root".to_string()),
        NodeType::RootSupervisor,
        None,
        root_supervisor,
    ));
    
    // Set root
    tree.set_root(root_node).await.unwrap();
    
    // Should have one node
    assert_eq!(tree.all_nodes().await.len(), 1);
}

#[tokio::test]
async fn test_node_registration() {
    let tree = SupervisionTree::new(SupervisionTreeConfig::default());
    
    // Create and register multiple nodes
    for i in 0..5 {
        let supervisor = Arc::new(AgentSupervisor::new(
            &format!("agent-{}", i),
            SupervisionStrategy::OneForOne,
            RestartPolicy::max_restarts(3),
        ));
        
        let node = Arc::new(SupervisorNode::new(
            NodeId(format!("node-{}", i)),
            NodeType::AgentSupervisor(format!("agent-{}", i)),
            Some(NodeId("root".to_string())),
            supervisor,
        ));
        
        tree.register_node(node).await.unwrap();
    }
    
    assert_eq!(tree.all_nodes().await.len(), 5);
}

#[tokio::test]
async fn test_node_unregistration() {
    let tree = SupervisionTree::new(SupervisionTreeConfig::default());
    
    // Register a node
    let supervisor = Arc::new(AgentSupervisor::new(
        "test",
        SupervisionStrategy::OneForOne,
        RestartPolicy::max_restarts(3),
    ));
    
    let node = Arc::new(SupervisorNode::new(
        NodeId("test-node".to_string()),
        NodeType::AgentSupervisor("test".to_string()),
        None,
        supervisor,
    ));
    
    tree.register_node(node).await.unwrap();
    assert_eq!(tree.all_nodes().await.len(), 1);
    
    // Unregister
    tree.unregister_node(&NodeId("test-node".to_string())).await.unwrap();
    assert_eq!(tree.all_nodes().await.len(), 0);
}

#[tokio::test]
async fn test_restart_policy_management() {
    let tree = SupervisionTree::new(SupervisionTreeConfig::default());
    
    // Set policies for different node types
    let mut policies = std::collections::HashMap::new();
    policies.insert(
        NodeType::AgentSupervisor("research".to_string()),
        RestartPolicy::exponential_backoff(),
    );
    policies.insert(
        NodeType::Worker("coder".to_string()),
        RestartPolicy::max_restarts(5),
    );
    
    tree.set_restart_policies(policies).await;
    
    // Verify policies
    let policy = tree.get_restart_policy(&NodeType::AgentSupervisor("research".to_string())).await;
    assert!(policy.is_some());
}

#[tokio::test]
async fn test_task_routing() {
    let tree = SupervisionTree::new(SupervisionTreeConfig::default());
    
    // Set up root with routing
    let root = Arc::new(RootSupervisor::new(SupervisionStrategy::OneForAll));
    let root_node = Arc::new(SupervisorNode::new(
        NodeId("root".to_string()),
        NodeType::RootSupervisor,
        None,
        root,
    ));
    
    tree.set_root(root_node).await.unwrap();
    
    // Test routing different task types
    let research_task = Task {
        id: "task-1".to_string(),
        task_type: TaskType::Research,
        payload: serde_json::Value::Null,
        priority: 5,
    };
    
    let code_task = Task {
        id: "task-2".to_string(),
        task_type: TaskType::Code,
        payload: serde_json::Value::Null,
        priority: 8,
    };
    
    // Should not fail even without actual children
    tree.route_task(research_task).await.unwrap();
    tree.route_task(code_task).await.unwrap();
}

#[tokio::test]
async fn test_supervision_tree_metrics() {
    let tree = SupervisionTree::new(SupervisionTreeConfig::default());
    
    // Initial metrics
    let metrics = tree.get_metrics().await;
    assert_eq!(metrics.total_nodes, 0);
    assert_eq!(metrics.total_restarts, 0);
    assert_eq!(metrics.total_failures, 0);
}

#[tokio::test]
async fn test_hierarchical_structure() {
    let tree = SupervisionTree::new(SupervisionTreeConfig::default());
    
    // Create root
    let root = Arc::new(RootSupervisor::new(SupervisionStrategy::OneForAll));
    let root_node = Arc::new(SupervisorNode::new(
        NodeId("root".to_string()),
        NodeType::RootSupervisor,
        None,
        root,
    ));
    
    tree.set_root(root_node.clone()).await.unwrap();
    
    // Create child supervisors
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
    
    tree.register_node(research_node).await.unwrap();
    
    // Add to root's children
    root_node.add_child(ChildRef {
        id: ChildId("research-supervisor".to_string()),
        node_type: NodeType::AgentSupervisor("research".to_string()),
        state: Arc::new(tokio::sync::RwLock::new(ChildState::Running)),
    }).await.unwrap();
    
    // Verify structure
    assert_eq!(tree.all_nodes().await.len(), 2);
    assert_eq!(root_node.get_children().await.len(), 1);
}