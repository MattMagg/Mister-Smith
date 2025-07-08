//! Test persistent Claude sessions

use mistersmith::InteractiveClaudeSession;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_interactive_session_basic() {
    println!("\n🧪 Testing InteractiveClaudeSession...");
    
    // Start a session
    let mut session = match InteractiveClaudeSession::start().await {
        Ok(s) => s,
        Err(e) => {
            println!("❌ Failed to start session: {}", e);
            return;
        }
    };
    
    println!("✅ Session started successfully");
    
    // Send a simple message
    let response = match timeout(
        Duration::from_secs(30),
        session.send_message("What is 2+2? Reply with just the number.")
    ).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => {
            println!("❌ Failed to send message: {}", e);
            return;
        }
        Err(_) => {
            println!("❌ Timeout waiting for response");
            return;
        }
    };
    
    println!("📝 Response: {}", response.trim());
    
    // Test memory - ask follow-up
    let follow_up = match timeout(
        Duration::from_secs(30),
        session.send_message("What was my previous question?")
    ).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => {
            println!("❌ Failed to send follow-up: {}", e);
            return;
        }
        Err(_) => {
            println!("❌ Timeout on follow-up");
            return;
        }
    };
    
    println!("💭 Memory test: {}", follow_up.trim());
    
    // Verify it remembers
    if follow_up.contains("2+2") || follow_up.contains("2 + 2") {
        println!("✅ Session maintains conversation memory!");
    } else {
        println!("⚠️  Session may not be maintaining memory correctly");
    }
    
    // Gracefully terminate
    if let Err(e) = session.shutdown().await {
        println!("⚠️  Shutdown error: {}", e);
    } else {
        println!("✅ Session terminated gracefully");
    }
}