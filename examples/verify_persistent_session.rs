//! Verify that we actually spawn persistent Claude sessions

use mistersmith::InteractiveClaudeSession;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use std::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n🔍 VERIFYING PERSISTENT CLAUDE SESSION SPAWNING");
    println!("==============================================\n");
    
    // Check current Claude processes
    println!("📊 Claude processes BEFORE spawning:");
    show_claude_processes();
    
    // Start a session
    println!("\n🚀 Starting InteractiveClaudeSession...");
    let mut session = match InteractiveClaudeSession::start().await {
        Ok(s) => {
            println!("✅ Session started!");
            s
        }
        Err(e) => {
            println!("❌ Failed to start: {}", e);
            return Ok(());
        }
    };
    
    // Give it a moment to fully start
    sleep(Duration::from_secs(2)).await;
    
    // Check processes again
    println!("\n📊 Claude processes AFTER spawning:");
    show_claude_processes();
    
    // Send a test message
    println!("\n💬 Sending test message...");
    match session.send_message("What is 2+2? Reply with just the number.").await {
        Ok(response) => println!("📝 Response: {}", response.trim()),
        Err(e) => println!("❌ Send failed: {}", e),
    }
    
    // Keep it alive for a bit to verify persistence
    println!("\n⏳ Keeping session alive for 5 seconds...");
    sleep(Duration::from_secs(5)).await;
    
    println!("\n📊 Claude processes STILL RUNNING:");
    show_claude_processes();
    
    // Shutdown
    println!("\n🛑 Shutting down session...");
    session.shutdown().await?;
    
    sleep(Duration::from_secs(1)).await;
    
    println!("\n📊 Claude processes AFTER shutdown:");
    show_claude_processes();
    
    Ok(())
}

fn show_claude_processes() {
    let output = Command::new("ps")
        .args(&["aux"])
        .output()
        .expect("Failed to run ps");
    
    let ps_output = String::from_utf8_lossy(&output.stdout);
    let claude_processes: Vec<&str> = ps_output
        .lines()
        .filter(|line| line.contains("claude") && !line.contains("grep"))
        .collect();
    
    if claude_processes.is_empty() {
        println!("   No Claude processes found");
    } else {
        for process in claude_processes {
            // Extract PID and command
            let parts: Vec<&str> = process.split_whitespace().collect();
            if parts.len() > 10 {
                println!("   PID: {} | CMD: {}... ", parts[1], parts[10]);
            }
        }
    }
}