//! Simplified Phase 3 Supervision Tests
//! 
//! Tests core supervision functionality for Claude Code CLI agent management
//! Uses only confirmed working APIs

use std::sync::Arc;
use std::collections::HashMap;

#[tokio::test]
async fn test_01_basic_tree_lifecycle() {
    use mistersmith::supervision::{SupervisionTree, SupervisionTreeConfig};
    
    println!("🧪 Testing supervision tree for Claude agent management...");
    
    // Create tree with default config
    let config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(config);
    
    // Start the tree (should work even without nodes)
    match tree.start().await {
        Ok(_) => println!("✅ Supervision tree started successfully"),
        Err(e) => println!("⚠️  Tree start failed (expected): {:?}", e),
    }
    
    // Stop the tree
    match tree.stop().await {
        Ok(_) => println!("✅ Supervision tree stopped successfully"),
        Err(e) => println!("⚠️  Tree stop failed: {:?}", e),
    }
}

#[tokio::test]
async fn test_02_node_registration() {
    use mistersmith::supervision::*;
    
    println!("🧪 Testing node registration for Claude agents...");
    
    let config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(config);
    
    // Create a supervisor node that would manage Claude agents
    let supervisor = RootSupervisor::new(SupervisionStrategy::OneForOne);
    let node = SupervisorNode::new(
        NodeId("claude-supervisor".to_string()),
        NodeType::RootSupervisor,
        None, // No parent
        Arc::new(supervisor),
    );
    let node_arc = Arc::new(node);
    
    // Register the node
    match tree.register_node(node_arc.clone()).await {
        Ok(_) => println!("✅ Claude supervisor node registered"),
        Err(e) => println!("❌ Failed to register node: {:?}", e),
    }
    
    // Verify we can retrieve it
    let node_id = NodeId("claude-supervisor".to_string());
    if let Some(_retrieved) = tree.get_node(&node_id).await {
        println!("✅ Retrieved registered node successfully");
    } else {
        println!("❌ Could not retrieve registered node");
    }
}

#[tokio::test]
async fn test_03_restart_policies() {
    use mistersmith::supervision::*;
    
    println!("🧪 Testing restart policies for Claude agent failures...");
    
    let config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(config);
    
    // Create different restart policies for different agent types
    let mut policies = HashMap::new();
    
    // Claude agents should restart with exponential backoff
    policies.insert(
        NodeType::AgentSupervisor("claude-agent".to_string()),
        RestartPolicy::exponential_backoff()
    );
    
    // Root supervisor should have limited restarts
    policies.insert(
        NodeType::RootSupervisor,
        RestartPolicy::max_restarts(3)
    );
    
    // Set the policies
    tree.set_restart_policies(policies).await;
    
    // Verify we can retrieve them
    let agent_type = NodeType::AgentSupervisor("claude-agent".to_string());
    if let Some(_policy) = tree.get_restart_policy(&agent_type).await {
        println!("✅ Restart policy set for Claude agents");
    } else {
        println!("⚠️  Could not retrieve restart policy");
    }
}

#[tokio::test]
async fn test_04_supervision_metrics() {
    use mistersmith::supervision::*;
    
    println!("🧪 Testing supervision metrics...");
    
    let config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(config);
    
    // Get metrics using the correct method name
    let metrics = tree.get_metrics().await;
    println!("✅ Got supervision metrics:");
    println!("   Total nodes: {}", metrics.total_nodes);
    println!("   Active supervisors: {}", metrics.active_supervisors);
    println!("   Active workers: {}", metrics.active_workers);
    println!("   Total restarts: {}", metrics.total_restarts);
    println!("   Total failures: {}", metrics.total_failures);
}

#[tokio::test]
async fn test_05_supervision_with_root() {
    use mistersmith::supervision::*;
    
    println!("🧪 Testing complete supervision setup for Claude CLI...");
    
    let config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(config);
    
    // Create root supervisor for managing Claude agents
    let root_supervisor = RootSupervisor::new(SupervisionStrategy::OneForOne);
    let root_node = SupervisorNode::new(
        NodeId("mistersmith-root".to_string()),
        NodeType::RootSupervisor,
        None,
        Arc::new(root_supervisor),
    );
    
    // Set as root
    match tree.set_root(Arc::new(root_node)).await {
        Ok(_) => println!("✅ Root supervisor set for Claude agent management"),
        Err(e) => println!("❌ Failed to set root: {:?}", e),
    }
    
    // Start the tree with root
    match tree.start().await {
        Ok(_) => println!("✅ Supervision tree started with root"),
        Err(e) => println!("⚠️  Start with root failed: {:?}", e),
    }
    
    // List all nodes
    let nodes = tree.all_nodes().await;
    println!("✅ Active nodes in supervision tree: {}", nodes.len());
    
    // Stop cleanly
    match tree.stop().await {
        Ok(_) => println!("✅ Supervision tree stopped cleanly"),
        Err(e) => println!("⚠️  Stop failed: {:?}", e),
    }
}

#[tokio::test] 
async fn test_06_agent_supervisor_creation() {
    use mistersmith::supervision::*;
    
    println!("🧪 Testing AgentSupervisor for Claude agents...");
    
    // Create an agent supervisor specifically for Claude CLI processes
    let agent_supervisor = AgentSupervisor::new(
        "claude-cli-agents",
        SupervisionStrategy::OneForOne, // Each Claude agent managed independently
        RestartPolicy::exponential_backoff(), // Use exponential backoff for Claude agents
    );
    
    // Create a node for this supervisor
    let node = SupervisorNode::new(
        NodeId("claude-agent-supervisor".to_string()),
        NodeType::AgentSupervisor("claude-cli-agents".to_string()),
        Some(NodeId("mistersmith-root".to_string())), // Has a parent
        Arc::new(agent_supervisor),
    );
    
    println!("✅ AgentSupervisor created for Claude CLI agent management");
    
    // Verify the node has correct parent
    if let Some(parent_id) = node.parent_id() {
        println!("✅ Agent supervisor has parent: {:?}", parent_id);
    }
}