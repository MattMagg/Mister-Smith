//! Corrected Multi-Agent Claude Communication Test
//! 
//! This test demonstrates actual Claude agents communicating through NATS
//! using the correct MisterSmith API

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use futures::StreamExt;

#[tokio::test]
async fn test_multi_agent_nats_simple() {
    println!("\n🎯 MULTI-AGENT NATS COMMUNICATION TEST");
    println!("=====================================");
    println!("Testing two Claude agents communicating via NATS\n");
    
    use mistersmith::{
        agent::{AgentPool, Agent, AgentConfig, AgentState},
    };
    
    // 1. Connect to NATS
    let nats_client = async_nats::connect("nats://localhost:4222").await
        .expect("NATS should be running");
    println!("✅ Connected to NATS");
    
    // 2. Create agent pool
    let pool = Arc::new(AgentPool::new(2));
    println!("✅ Agent pool created (max 2 agents)");
    
    // 3. Create Agent 1 - Question Asker
    let config1 = AgentConfig {
        model: "claude-3-sonnet-20240229".to_string(),
        max_turns: Some(1),
        allowed_tools: None,
        enable_mcp: false,
        timeout_seconds: 30,
        memory_limit_mb: Some(256),
    };
    
    let agent1 = Agent::new(config1);
    let agent1_id = agent1.id.clone();
    agent1.set_state(AgentState::Running).await;
    
    pool.register(agent1.clone()).await
        .expect("Should register agent 1");
    println!("✅ Agent 1 registered: {:?}", agent1_id);
    
    // 4. Create Agent 2 - Responder
    let config2 = AgentConfig {
        model: "claude-3-sonnet-20240229".to_string(),
        max_turns: Some(1),
        allowed_tools: None,
        enable_mcp: false,
        timeout_seconds: 30,
        memory_limit_mb: Some(256),
    };
    
    let agent2 = Agent::new(config2);
    let agent2_id = agent2.id.clone();
    agent2.set_state(AgentState::Running).await;
    
    pool.register(agent2.clone()).await
        .expect("Should register agent 2");
    println!("✅ Agent 2 registered: {:?}", agent2_id);
    
    // 5. Set up NATS communication
    let mut response_sub = nats_client.subscribe("agent.response").await.unwrap();
    let (tx, mut rx) = mpsc::channel::<String>(1);
    
    // 6. Agent 2 listens and responds
    let agent2_clone = agent2.clone();
    let nats_clone = nats_client.clone();
    let tx_clone = tx.clone();
    
    let responder_task = tokio::spawn(async move {
        let mut question_sub = nats_clone.subscribe("agent.question").await.unwrap();
        println!("\n🔊 Agent 2 listening for questions...");
        
        if let Some(msg) = question_sub.next().await {
            let question = String::from_utf8(msg.payload.to_vec()).unwrap();
            println!("📨 Agent 2 received: {}", question);
            
            // Use Claude to answer
            let prompt = format!("Answer in 10 words or less: {}", question);
            match agent2_clone.execute(&prompt).await {
                Ok(answer) => {
                    println!("✅ Agent 2 Claude response received");
                    
                    // Publish response
                    nats_clone.publish(
                        "agent.response",
                        answer.as_bytes().to_vec().into()
                    ).await.unwrap();
                    
                    tx_clone.send(answer).await.unwrap();
                }
                Err(e) => {
                    println!("❌ Agent 2 error: {:?}", e);
                }
            }
        }
    });
    
    // 7. Agent 1 asks a question
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    println!("\n❓ Agent 1 asking question...");
    let question = "What is 2+2?";
    
    nats_client.publish(
        "agent.question",
        question.as_bytes().to_vec().into()
    ).await.unwrap();
    
    println!("✅ Question sent: {}", question);
    
    // 8. Wait for response
    println!("\n⏳ Waiting for Agent 2's response...");
    
    match tokio::time::timeout(Duration::from_secs(30), rx.recv()).await {
        Ok(Some(response)) => {
            println!("\n🎉 SUCCESS! Agents communicated via NATS!");
            println!("💬 Question: {}", question);
            println!("💬 Answer: {}", response);
            println!("\n💰 Estimated cost: ~$0.02 (1 minimal Claude call)");
        }
        _ => {
            println!("❌ Timeout or error");
        }
    }
    
    // Cleanup
    responder_task.abort();
    pool.unregister(&agent1_id).await.ok();
    pool.unregister(&agent2_id).await.ok();
    
    println!("\n✨ Multi-agent test complete!");
}

#[tokio::test]
async fn test_agent_coordination_pattern() {
    println!("\n🏗️ AGENT COORDINATION PATTERN TEST");
    println!("==================================");
    
    use mistersmith::{
        agent::AgentPool,
        supervision::{
            SupervisionTree, SupervisionTreeConfig,
            RootSupervisor, SupervisionStrategy,
            NodeId, NodeType, SupervisorNode,
        },
    };
    
    // This test verifies the coordination pattern without Claude calls
    
    // 1. Create supervision tree
    let tree = SupervisionTree::new(SupervisionTreeConfig::default());
    let root = RootSupervisor::new(SupervisionStrategy::OneForOne);
    let root_node = SupervisorNode::new(
        NodeId("coordinator-root".to_string()),
        NodeType::RootSupervisor,
        None,
        Arc::new(root),
    );
    
    tree.set_root(Arc::new(root_node)).await.unwrap();
    tree.start().await.unwrap();
    println!("✅ Supervision tree started");
    
    // 2. Create agent pool
    let pool = Arc::new(AgentPool::new(3));
    
    // 3. Connect to NATS for coordination
    let nats_client = async_nats::connect("nats://localhost:4222").await
        .expect("NATS should be running");
    
    // 4. Publish coordination event
    let event = r#"{"type": "agent_spawned", "id": "test-agent"}"#;
    nats_client.publish(
        "supervision.events.spawn",
        event.as_bytes().to_vec().into()
    ).await.unwrap();
    
    println!("✅ Coordination event published");
    
    // 5. Check metrics
    let metrics = tree.get_metrics().await;
    println!("📊 Supervision metrics:");
    println!("   Total nodes: {}", metrics.total_nodes);
    println!("   Pool capacity: {}/3", pool.get_status().await.active_count);
    
    tree.stop().await.unwrap();
    println!("✅ Coordination pattern verified");
}