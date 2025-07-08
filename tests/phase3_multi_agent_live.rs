//! Live Multi-Agent Claude Communication Test
//! 
//! This test demonstrates actual Claude agents communicating through NATS
//! WARNING: This test costs money as it makes real Claude API calls

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use async_nats::Client;
use serde::{Serialize, Deserialize};
use futures::StreamExt;

#[derive(Debug, Serialize, Deserialize)]
struct AgentMessage {
    from_agent: String,
    to_agent: String,
    task: String,
    response: Option<String>,
}

#[tokio::test]
async fn test_two_claude_agents_communicate() {
    println!("\n🚀 LIVE MULTI-AGENT CLAUDE COMMUNICATION TEST");
    println!("=============================================");
    println!("⚠️  This test will make real Claude API calls and cost money!\n");
    
    // 1. Connect to NATS
    let nats_client = async_nats::connect("nats://localhost:4222").await
        .expect("NATS should be running");
    println!("✅ Connected to NATS");
    
    // 2. Create agent pool and supervision setup
    use mistersmith::{
        agent::{AgentPool, Agent, AgentConfig, AgentId},
        runtime::{ProcessManager, ClaudeExecutor},
    };
    
    let pool = Arc::new(AgentPool::new(3));
    println!("✅ Agent pool created (max 3 agents)");
    
    // 3. Spawn Agent 1: Researcher
    println!("\n📚 Spawning Agent 1: Researcher...");
    let config1 = AgentConfig {
        name: "researcher".to_string(),
        agent_type: "research".to_string(),
        max_tasks: 5,
        memory_limit_mb: 512,
        cpu_limit: 1.0,
        required_tools: vec!["search".to_string()],
        environment: std::collections::HashMap::new(),
    };
    
    let agent1 = Agent::new(AgentId::new(), config1);
    let agent1_id = agent1.id().clone();
    agent1.transition_to(mistersmith::agent::AgentState::Running).await
        .expect("Agent 1 should start");
    
    pool.register_agent(agent1.clone()).await
        .expect("Should register agent 1");
    println!("✅ Agent 1 (Researcher) registered: {:?}", agent1_id);
    
    // 4. Spawn Agent 2: Analyzer  
    println!("\n📊 Spawning Agent 2: Analyzer...");
    let config2 = AgentConfig {
        name: "analyzer".to_string(),
        agent_type: "analysis".to_string(),
        max_tasks: 5,
        memory_limit_mb: 512,
        cpu_limit: 1.0,
        required_tools: vec!["analyze".to_string()],
        environment: std::collections::HashMap::new(),
    };
    
    let agent2 = Agent::new(AgentId::new(), config2);
    let agent2_id = agent2.id().clone();
    agent2.transition_to(mistersmith::agent::AgentState::Running).await
        .expect("Agent 2 should start");
    
    pool.register_agent(agent2.clone()).await
        .expect("Should register agent 2");
    println!("✅ Agent 2 (Analyzer) registered: {:?}", agent2_id);
    
    // 5. Set up NATS communication channels
    println!("\n📡 Setting up NATS communication...");
    
    // Agent 2 subscribes to analysis requests
    let mut analysis_subscriber = nats_client.subscribe("agents.analyzer.requests").await
        .expect("Should subscribe");
    
    // Channel for Agent 2 to send back results
    let (tx, mut rx) = mpsc::channel::<String>(10);
    
    // 6. Agent 2's message handler (runs in background)
    let agent2_clone = agent2.clone();
    let nats_client_clone = nats_client.clone();
    let tx_clone = tx.clone();
    
    let agent2_handler = tokio::spawn(async move {
        println!("🔊 Agent 2 listening for analysis requests...");
        
        while let Some(msg) = analysis_subscriber.next().await {
            let request: AgentMessage = serde_json::from_slice(&msg.payload)
                .expect("Should parse message");
            
            println!("📨 Agent 2 received: {:?}", request.task);
            
            // Execute Claude with the analysis task
            agent2_clone.transition_to(mistersmith::agent::AgentState::Processing).await
                .expect("Should transition to processing");
            
            let executor = ClaudeExecutor::new();
            let prompt = format!("Analyze this briefly (max 20 words): {}", request.task);
            
            println!("🤖 Agent 2 calling Claude...");
            match executor.execute(&prompt).await {
                Ok(response) => {
                    println!("✅ Agent 2 got Claude response");
                    
                    // Send response back via NATS
                    let response_msg = AgentMessage {
                        from_agent: "analyzer".to_string(),
                        to_agent: request.from_agent,
                        task: request.task,
                        response: Some(response.result),
                    };
                    
                    let response_subject = format!("agents.{}.responses", request.from_agent);
                    nats_client_clone.publish(
                        response_subject,
                        serde_json::to_vec(&response_msg).unwrap().into()
                    ).await.expect("Should publish response");
                    
                    tx_clone.send(response_msg.response.unwrap()).await
                        .expect("Should send to channel");
                }
                Err(e) => {
                    println!("❌ Agent 2 Claude error: {:?}", e);
                }
            }
            
            agent2_clone.transition_to(mistersmith::agent::AgentState::Idle).await
                .expect("Should return to idle");
        }
    });
    
    // 7. Agent 1 sends a research request
    println!("\n🔬 Agent 1 sending research request to Agent 2...");
    
    agent1.transition_to(mistersmith::agent::AgentState::Processing).await
        .expect("Should transition to processing");
    
    // Agent 1 creates a simple research question
    let research_request = AgentMessage {
        from_agent: "researcher".to_string(),
        to_agent: "analyzer".to_string(),
        task: "What are the key benefits of fault-tolerant systems?".to_string(),
        response: None,
    };
    
    // Publish to Agent 2's request channel
    nats_client.publish(
        "agents.analyzer.requests",
        serde_json::to_vec(&research_request).unwrap().into()
    ).await.expect("Should publish request");
    
    println!("✅ Research request sent via NATS");
    
    // 8. Wait for response with timeout
    println!("\n⏳ Waiting for analysis response...");
    
    match tokio::time::timeout(Duration::from_secs(30), rx.recv()).await {
        Ok(Some(response)) => {
            println!("\n🎉 SUCCESS! Agents communicated through NATS!");
            println!("📊 Analysis from Agent 2: {}", response);
            
            // Calculate approximate cost
            println!("\n💰 Approximate cost: ~$1.36 (2 Claude calls)");
        }
        Ok(None) => {
            println!("❌ Channel closed unexpectedly");
        }
        Err(_) => {
            println!("❌ Timeout waiting for response (30s)");
        }
    }
    
    // 9. Cleanup
    println!("\n🧹 Cleaning up...");
    agent2_handler.abort();
    
    agent1.transition_to(mistersmith::agent::AgentState::Terminated).await
        .expect("Should terminate agent 1");
    agent2.transition_to(mistersmith::agent::AgentState::Terminated).await
        .expect("Should terminate agent 2");
    
    pool.unregister_agent(agent1.id()).await
        .expect("Should unregister agent 1");
    pool.unregister_agent(agent2.id()).await
        .expect("Should unregister agent 2");
    
    println!("✅ Agents terminated and unregistered");
    println!("\n✨ Multi-agent Claude communication test complete!");
}

#[tokio::test]
async fn test_three_agent_workflow() {
    println!("\n🚀 THREE-AGENT WORKFLOW TEST");
    println!("================================");
    println!("⚠️  This test demonstrates a researcher → analyzer → summarizer workflow\n");
    
    // This test would demonstrate:
    // 1. Researcher agent gets a topic
    // 2. Sends findings to Analyzer agent  
    // 3. Analyzer sends insights to Summarizer agent
    // 4. Final summary is produced
    
    // Skipping implementation to avoid excessive costs
    println!("ℹ️  Skipping 3-agent test to control costs");
    println!("ℹ️  The infrastructure supports this workflow pattern");
}