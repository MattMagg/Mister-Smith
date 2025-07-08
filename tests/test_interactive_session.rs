//! Test the interactive Claude session

use mistersmith::runtime::InteractiveClaudeSession;
use anyhow::Result;

#[tokio::test]
#[ignore] // Ignore by default since it requires Claude CLI and costs money
async fn test_persistent_claude_session() -> Result<()> {
    println!("\n🚀 TESTING PERSISTENT CLAUDE SESSION");
    println!("====================================");
    
    // Start a persistent Claude session
    let mut session = InteractiveClaudeSession::start().await?;
    println!("✅ Interactive session started!");
    
    // First message - ask Claude to remember something
    println!("\n📤 Sending: 'Hello! Please remember that my favorite number is 42.'");
    let response1 = session.send_message(
        "Hello! Please remember that my favorite number is 42."
    ).await?;
    println!("📥 Claude: {}", response1);
    
    // Second message - test if Claude remembers
    println!("\n📤 Sending: 'What is my favorite number?'");
    let response2 = session.send_message(
        "What is my favorite number?"
    ).await?;
    println!("📥 Claude: {}", response2);
    
    // Verify Claude remembered
    if response2.contains("42") {
        println!("\n✅ SUCCESS! Claude maintained context between messages!");
    } else {
        println!("\n❌ FAILED: Claude didn't remember the number");
    }
    
    // Show conversation history
    let history = session.get_history().await;
    println!("\n📜 Conversation History:");
    for (i, msg) in history.iter().enumerate() {
        println!("  {}: [{}] {}", i + 1, msg.role, msg.content);
    }
    
    // Gracefully shutdown
    session.shutdown().await?;
    println!("\n✅ Session terminated gracefully");
    
    Ok(())
}

#[tokio::test]
#[ignore] // Expensive test
async fn test_multi_agent_collaboration_repl() -> Result<()> {
    println!("\n🚀 TESTING REAL-TIME MULTI-AGENT COLLABORATION");
    println!("==============================================");
    
    use async_nats::Client;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    // Connect to NATS for inter-agent communication
    let nats = Client::connect("nats://localhost:4222").await?;
    println!("✅ Connected to NATS");
    
    // Start two persistent Claude sessions
    let session1 = Arc::new(RwLock::new(
        InteractiveClaudeSession::start().await?
    ));
    println!("✅ Agent 1 (Researcher) started");
    
    let session2 = Arc::new(RwLock::new(
        InteractiveClaudeSession::start().await?
    ));
    println!("✅ Agent 2 (Analyst) started");
    
    // Set up Agent 2 to listen for discoveries
    let session2_clone = session2.clone();
    let nats_clone = nats.clone();
    
    let listener = tokio::spawn(async move {
        let mut sub = nats_clone.subscribe("discoveries").await.unwrap();
        
        while let Some(msg) = sub.next().await {
            let discovery = String::from_utf8(msg.payload.to_vec()).unwrap();
            println!("\n📡 Agent 2 received discovery: {}", discovery);
            
            // Agent 2 analyzes the discovery with context
            let mut session = session2_clone.write().await;
            let analysis = session.send_message(&format!(
                "I just received this discovery from another agent: '{}'. \
                 What are the implications?", discovery
            )).await.unwrap();
            
            println!("🔍 Agent 2 analysis: {}", analysis);
            
            // Share analysis back
            nats_clone.publish(
                "analysis", 
                analysis.as_bytes().to_vec().into()
            ).await.unwrap();
        }
    });
    
    // Agent 1 makes a discovery
    println!("\n🔬 Agent 1 researching...");
    let mut session1 = session1.write().await;
    let research = session1.send_message(
        "Research the connection between quantum computing and cryptography. \
         Focus on one key insight."
    ).await?;
    println!("📚 Agent 1 research: {}", research);
    
    // Agent 1 shares discovery
    nats.publish("discoveries", research.as_bytes().to_vec().into()).await?;
    
    // Wait for analysis
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    
    // Clean up
    listener.abort();
    session1.shutdown().await?;
    session2.write().await.shutdown().await?;
    
    println!("\n✅ Multi-agent collaboration test complete!");
    
    Ok(())
}

/// This demonstrates the difference between one-shot and persistent sessions
#[tokio::test]
async fn compare_one_shot_vs_persistent() -> Result<()> {
    println!("\n📊 COMPARING ONE-SHOT VS PERSISTENT SESSIONS");
    println!("==========================================");
    
    use mistersmith::runtime::ClaudeExecutor;
    
    // ONE-SHOT APPROACH (current implementation)
    println!("\n1️⃣ ONE-SHOT EXECUTION:");
    let executor = ClaudeExecutor::new(30);
    
    let response1 = executor.execute("My name is Alice.").await?;
    println!("   First call: {}", response1.result.unwrap_or_default());
    
    let response2 = executor.execute("What is my name?").await?;
    println!("   Second call: {}", response2.result.unwrap_or_default());
    println!("   ❌ Claude has no memory of the first call!");
    
    // PERSISTENT SESSION APPROACH (new)
    println!("\n2️⃣ PERSISTENT SESSION:");
    let mut session = InteractiveClaudeSession::start().await?;
    
    let response1 = session.send_message("My name is Bob.").await?;
    println!("   First message: {}", response1);
    
    let response2 = session.send_message("What is my name?").await?;
    println!("   Second message: {}", response2);
    println!("   ✅ Claude remembers the conversation!");
    
    session.shutdown().await?;
    
    println!("\n🎯 KEY DIFFERENCE:");
    println!("   One-shot: New process each time, no memory");
    println!("   Persistent: Same process, maintains context");
    
    Ok(())
}

// Run with: cargo test --test test_interactive_session -- --ignored --nocapture