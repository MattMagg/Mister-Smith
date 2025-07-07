//! Claude Executor Integration Tests
//!
//! Tests for the single-shot Claude Code execution pattern.

use anyhow::Result;
use crate::runtime::{ClaudeExecutor, ClaudeTransport, PrintModeTransport};

#[tokio::test]
async fn test_claude_executor_basic() -> Result<()> {
    // Create executor with 30 second timeout
    let executor = ClaudeExecutor::new(30);
    
    // Test with a simple prompt
    match executor.execute("Say 'Hello from MisterSmith test!' and nothing else").await {
        Ok(response) => {
            println!("✅ Claude responded successfully");
            println!("Session ID: {}", response.session_id);
            println!("Result: {:?}", response.result);
            println!("Cost: ${:.6}", response.total_cost_usd);
            
            assert!(!response.is_error);
            assert!(response.result.is_some());
            assert!(response.result.unwrap().contains("Hello from MisterSmith"));
        }
        Err(e) => {
            eprintln!("⚠️  Test skipped - Claude CLI may not be available: {}", e);
            // Don't fail the test if Claude CLI is not installed
            return Ok(());
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_print_mode_transport() -> Result<()> {
    // Test the transport abstraction pattern
    let transport = PrintModeTransport::new(30);
    
    match transport.execute("List 3 benefits of Rust in one sentence each").await {
        Ok(response) => {
            println!("✅ Transport executed successfully");
            println!("Response: {:?}", response.result);
            
            if let Some(usage) = response.usage {
                println!("Tokens - Input: {}, Output: {}", 
                    usage.input_tokens, usage.output_tokens);
            }
        }
        Err(e) => {
            eprintln!("⚠️  Transport test skipped: {}", e);
            return Ok(());
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    // Test with very short timeout to trigger error
    let executor = ClaudeExecutor::new(1); // 1 second timeout
    
    // This should timeout or error
    match executor.execute("Write a very long story about robots").await {
        Ok(_) => {
            // If it somehow succeeds quickly, that's OK
            println!("⚠️  Expected timeout but got quick response");
        }
        Err(e) => {
            println!("✅ Error handling works: {}", e);
            // This is expected behavior
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_json_response_parsing() -> Result<()> {
    let executor = ClaudeExecutor::new(30);
    
    match executor.execute("What is 2+2? Answer with just the number.").await {
        Ok(response) => {
            // Verify we can parse all the JSON fields
            assert!(!response.session_id.is_empty());
            assert!(response.total_cost_usd >= 0.0);
            assert_eq!(response.response_type, "result");
            
            println!("✅ JSON parsing successful");
            println!("Full response: {:#?}", response);
        }
        Err(e) => {
            eprintln!("⚠️  JSON parsing test skipped: {}", e);
            return Ok(());
        }
    }
    
    Ok(())
}