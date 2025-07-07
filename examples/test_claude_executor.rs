//! Test Claude Executor Example
//!
//! Simple example to test the Claude Code single-shot execution pattern.

use anyhow::Result;
use mistersmith::runtime::{ClaudeExecutor, ClaudeTransport, PrintModeTransport};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Testing Claude Code Executor");

    // Test 1: Basic execution
    println!("\n=== Test 1: Basic Execution ===");
    let executor = ClaudeExecutor::new(30);
    
    match executor.execute("Say 'Hello from MisterSmith!' and nothing else").await {
        Ok(response) => {
            println!("✅ Success!");
            println!("Session ID: {}", response.session_id);
            println!("Result: {:?}", response.result);
            println!("Cost: ${:.6}", response.total_cost_usd);
            if let Some(usage) = response.usage {
                println!("Tokens - Input: {}, Output: {}", 
                    usage.input_tokens, usage.output_tokens);
            }
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            eprintln!("Make sure Claude Code CLI is installed and available");
        }
    }

    // Test 2: Transport abstraction
    println!("\n=== Test 2: Transport Abstraction ===");
    let transport = PrintModeTransport::new(30);
    
    match transport.execute("What is 2+2? Answer with just the number.").await {
        Ok(response) => {
            println!("✅ Transport Success!");
            println!("Result: {:?}", response.result);
        }
        Err(e) => {
            eprintln!("❌ Transport Error: {}", e);
        }
    }

    // Test 3: Error handling with short timeout
    println!("\n=== Test 3: Error Handling ===");
    let quick_executor = ClaudeExecutor::new(2); // 2 second timeout
    
    match quick_executor.execute("Count to 100 slowly").await {
        Ok(_) => {
            println!("⚠️  Unexpected success with short timeout");
        }
        Err(e) => {
            println!("✅ Error handling works as expected: {}", e);
        }
    }

    println!("\n=== All tests completed ===");
    
    Ok(())
}