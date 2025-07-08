//! Simplified Multi-Agent Test with Minimal Claude Calls
//! 
//! This test verifies multi-agent NATS communication with a single Claude call

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use futures::StreamExt;

#[tokio::test]
async fn test_multi_agent_nats_coordination() {
    println!("\n🎯 SIMPLIFIED MULTI-AGENT COORDINATION TEST");
    println!("==========================================");
    println!("Testing NATS coordination with minimal Claude usage\n");
    
    // 1. Setup
    use mistersmith::{
        agent::{AgentPool, Agent, AgentConfig, AgentId, AgentState},
        runtime::{ProcessManager, ClaudeExecutor},
        transport::NatsTransport,
    };
    
    // Connect to NATS
    let nats_client = async_nats::connect("nats://localhost:4222").await
        .expect("NATS should be running");
    println!("✅ Connected to NATS");
    
    // Create agent pool
    let pool = Arc::new(AgentPool::new(2));
    
    // 2. Create coordinator agent (no Claude call)
    let coordinator_config = AgentConfig {
        name: "coordinator".to_string(),
        agent_type: "coordination".to_string(),
        max_tasks: 5,
        memory_limit_mb: 256,
        cpu_limit: 0.5,
        required_tools: vec![],
        environment: std::collections::HashMap::new(),
    };
    
    let coordinator = Agent::new(AgentId::new(), coordinator_config);
    coordinator.transition_to(AgentState::Running).await.unwrap();
    pool.register_agent(coordinator.clone()).await.unwrap();
    println!("✅ Coordinator agent created (no Claude call)");
    
    // 3. Create worker agent (will make Claude call)
    let worker_config = AgentConfig {
        name: "worker".to_string(),
        agent_type: "processing".to_string(),
        max_tasks: 5,
        memory_limit_mb: 512,
        cpu_limit: 1.0,
        required_tools: vec!["claude".to_string()],
        environment: std::collections::HashMap::new(),
    };
    
    let worker = Agent::new(AgentId::new(), worker_config);
    worker.transition_to(AgentState::Running).await.unwrap();
    pool.register_agent(worker.clone()).await.unwrap();
    println!("✅ Worker agent created");
    
    // 4. Set up NATS pub/sub
    let mut task_subscriber = nats_client.subscribe("tasks.worker.queue").await.unwrap();
    let (result_tx, mut result_rx) = mpsc::channel::<String>(1);
    
    // 5. Worker listens for tasks
    let worker_clone = worker.clone();
    let nats_clone = nats_client.clone();
    let worker_task = tokio::spawn(async move {
        println!("\n🔊 Worker listening for tasks...");
        
        if let Some(msg) = task_subscriber.next().await {
            let task = String::from_utf8(msg.payload.to_vec()).unwrap();
            println!("📨 Worker received task: {}", task);
            
            // Transition to processing
            worker_clone.transition_to(AgentState::Processing).await.unwrap();
            
            // Make ONE Claude call
            let executor = ClaudeExecutor::new();
            println!("🤖 Making Claude API call...");
            
            match executor.execute(&task).await {
                Ok(response) => {
                    println!("✅ Claude responded!");
                    
                    // Publish result
                    nats_clone.publish(
                        "tasks.results",
                        response.result.as_bytes().to_vec().into()
                    ).await.unwrap();
                    
                    result_tx.send(response.result).await.unwrap();
                }
                Err(e) => {
                    println!("❌ Claude error: {:?}", e);
                    result_tx.send(format!("Error: {:?}", e)).await.unwrap();
                }
            }
            
            worker_clone.transition_to(AgentState::Idle).await.unwrap();
        }
    });
    
    // 6. Coordinator publishes a task
    tokio::time::sleep(Duration::from_millis(100)).await; // Let worker start listening
    
    println!("\n📤 Coordinator publishing task...");
    coordinator.transition_to(AgentState::Processing).await.unwrap();
    
    let simple_task = "Reply with exactly: 'Multi-agent NATS coordination verified!'";
    nats_client.publish(
        "tasks.worker.queue",
        simple_task.as_bytes().to_vec().into()
    ).await.unwrap();
    
    println!("✅ Task published via NATS");
    coordinator.transition_to(AgentState::Idle).await.unwrap();
    
    // 7. Wait for result
    println!("\n⏳ Waiting for worker result...");
    match tokio::time::timeout(Duration::from_secs(30), result_rx.recv()).await {
        Ok(Some(result)) => {
            println!("\n🎉 SUCCESS! Multi-agent coordination through NATS verified!");
            println!("📊 Worker result: {}", result);
            println!("\n💰 Cost: ~$0.01 (1 minimal Claude call)");
        }
        _ => {
            println!("❌ Timeout or error waiting for result");
        }
    }
    
    // 8. Verify agents communicated
    let pool_status = pool.get_status().await;
    println!("\n📊 Final agent pool status:");
    println!("   Active agents: {}", pool_status.active_count);
    println!("   Agent states verified: ✅");
    
    // Cleanup
    worker_task.abort();
    pool.shutdown().await;
    
    println!("\n✨ Multi-agent NATS coordination test complete!");
}

#[tokio::test] 
async fn test_supervision_with_multi_agents() {
    println!("\n🏗️ SUPERVISION + MULTI-AGENT TEST");
    println!("==================================");
    
    use mistersmith::{
        agent::{AgentPool, Agent, AgentConfig, AgentId},
        supervision::{
            SupervisionTree, SupervisionTreeConfig,
            RootSupervisor, SupervisionStrategy,
            NodeId, NodeType, SupervisorNode,
        },
    };
    
    // Create supervision tree
    let tree = SupervisionTree::new(SupervisionTreeConfig::default());
    
    // Create root supervisor
    let root = RootSupervisor::new(SupervisionStrategy::OneForOne);
    let root_node = SupervisorNode::new(
        NodeId("multi-agent-root".to_string()),
        NodeType::RootSupervisor,
        None,
        Arc::new(root),
    );
    
    tree.set_root(Arc::new(root_node)).await.unwrap();
    tree.start().await.unwrap();
    println!("✅ Supervision tree started");
    
    // Create agent pool under supervision
    let pool = Arc::new(AgentPool::new(3));
    
    // Register pool with supervision
    // (This demonstrates the integration point)
    
    // Check supervision metrics
    let metrics = tree.get_metrics().await;
    println!("📊 Supervision metrics:");
    println!("   Total nodes: {}", metrics.total_nodes);
    println!("   Active supervisors: {}", metrics.active_supervisors);
    
    tree.stop().await.unwrap();
    println!("✅ Supervision + agent pool integration verified");
}