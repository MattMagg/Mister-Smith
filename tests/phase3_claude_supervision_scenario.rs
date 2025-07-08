//! Claude CLI Agent Supervision Scenario Test
//! 
//! Demonstrates a complete supervision setup for managing Claude Code CLI agents

use std::sync::Arc;
use std::collections::HashMap;

#[tokio::test]
async fn test_claude_agent_supervision_scenario() {
    use mistersmith::supervision::*;
    
    println!("\n🎭 CLAUDE AGENT SUPERVISION SCENARIO");
    println!("=====================================\n");
    
    // 1. Create supervision tree
    println!("1️⃣ Creating supervision tree for MisterSmith...");
    let config = SupervisionTreeConfig::default();
    let tree = SupervisionTree::new(config);
    
    // 2. Create root supervisor with OneForAll strategy
    // (if one critical component fails, restart everything)
    println!("2️⃣ Creating root supervisor with OneForAll strategy...");
    let root_supervisor = RootSupervisor::new(SupervisionStrategy::OneForAll);
    let root_node = SupervisorNode::new(
        NodeId("mistersmith-root".to_string()),
        NodeType::RootSupervisor,
        None,
        Arc::new(root_supervisor),
    );
    
    // 3. Set root (this also registers it internally)
    tree.set_root(Arc::new(root_node)).await
        .expect("Should set root");
    println!("✅ Root supervisor configured");
    
    // 4. Create different supervisors for different agent types
    println!("\n3️⃣ Creating specialized supervisors for different Claude agent types...");
    
    // Research agents - can fail independently
    let research_supervisor = AgentSupervisor::new(
        "research-agents",
        SupervisionStrategy::OneForOne,
        RestartPolicy::exponential_backoff(),
    );
    let research_node = SupervisorNode::new(
        NodeId("research-supervisor".to_string()),
        NodeType::AgentSupervisor("research-agents".to_string()),
        Some(NodeId("mistersmith-root".to_string())),
        Arc::new(research_supervisor),
    );
    tree.register_node(Arc::new(research_node)).await
        .expect("Should register research supervisor");
    println!("✅ Research agent supervisor registered (OneForOne strategy)");
    
    // Code generation agents - restart all if one fails
    let codegen_supervisor = AgentSupervisor::new(
        "codegen-agents",
        SupervisionStrategy::OneForAll,
        RestartPolicy::max_restarts(5),
    );
    let codegen_node = SupervisorNode::new(
        NodeId("codegen-supervisor".to_string()),
        NodeType::AgentSupervisor("codegen-agents".to_string()),
        Some(NodeId("mistersmith-root".to_string())),
        Arc::new(codegen_supervisor),
    );
    tree.register_node(Arc::new(codegen_node)).await
        .expect("Should register codegen supervisor");
    println!("✅ Code generation agent supervisor registered (OneForAll strategy)");
    
    // Analysis agents - restart failed and subsequent
    let analysis_supervisor = AgentSupervisor::new(
        "analysis-agents",
        SupervisionStrategy::RestForOne,
        RestartPolicy::always_restart(),
    );
    let analysis_node = SupervisorNode::new(
        NodeId("analysis-supervisor".to_string()),
        NodeType::AgentSupervisor("analysis-agents".to_string()),
        Some(NodeId("mistersmith-root".to_string())),
        Arc::new(analysis_supervisor),
    );
    tree.register_node(Arc::new(analysis_node)).await
        .expect("Should register analysis supervisor");
    println!("✅ Analysis agent supervisor registered (RestForOne strategy)");
    
    // 5. Configure restart policies for different scenarios
    println!("\n4️⃣ Configuring restart policies for different agent types...");
    let mut policies = HashMap::new();
    
    // Research agents: exponential backoff, max 10 restarts
    policies.insert(
        NodeType::AgentSupervisor("research-agents".to_string()),
        RestartPolicy::exponential_backoff()
    );
    
    // Critical services: limited restarts
    policies.insert(
        NodeType::Service("nats-handler".to_string()),
        RestartPolicy::max_restarts(3)
    );
    
    // Workers: always restart
    policies.insert(
        NodeType::Worker("claude-worker".to_string()),
        RestartPolicy::always_restart()
    );
    
    tree.set_restart_policies(policies).await;
    println!("✅ Restart policies configured");
    
    // 6. Start the supervision tree
    println!("\n5️⃣ Starting supervision tree...");
    tree.start().await.expect("Should start tree");
    println!("✅ Supervision tree started");
    
    // 7. Check metrics
    println!("\n6️⃣ Checking supervision metrics...");
    let metrics = tree.get_metrics().await;
    println!("📊 Supervision Tree Status:");
    println!("   Total nodes: {}", metrics.total_nodes);
    println!("   Active supervisors: {}", metrics.active_supervisors);
    println!("   Active workers: {}", metrics.active_workers);
    println!("   Total restarts: {}", metrics.total_restarts);
    println!("   Total failures: {}", metrics.total_failures);
    
    // 8. List all registered nodes
    println!("\n7️⃣ Listing all supervision nodes...");
    let all_nodes = tree.all_nodes().await;
    println!("📋 Registered nodes ({} total):", all_nodes.len());
    for node_id in &all_nodes {
        println!("   - {:?}", node_id);
    }
    
    // 9. Simulate checking specific nodes
    println!("\n8️⃣ Verifying specific supervisors...");
    let research_id = NodeId("research-supervisor".to_string());
    if let Some(_node) = tree.get_node(&research_id).await {
        println!("✅ Research supervisor is active");
    }
    
    let codegen_id = NodeId("codegen-supervisor".to_string());
    if let Some(_node) = tree.get_node(&codegen_id).await {
        println!("✅ Code generation supervisor is active");
    }
    
    // 10. Clean shutdown
    println!("\n9️⃣ Shutting down supervision tree...");
    tree.stop().await.expect("Should stop tree");
    println!("✅ Supervision tree stopped cleanly");
    
    println!("\n✨ Claude agent supervision scenario completed successfully!");
    println!("\nThis demonstrates how MisterSmith would manage different types");
    println!("of Claude CLI agents with appropriate supervision strategies:");
    println!("- Research agents: Independent failures (OneForOne)");
    println!("- Code generation: Coordinated restart (OneForAll)");
    println!("- Analysis pipeline: Sequential dependencies (RestForOne)");
}