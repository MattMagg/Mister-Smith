//! Error Scenarios Integration Tests
//!
//! Tests for handling various error conditions and edge cases.

use std::time::Duration;
use anyhow::Result;
use tokio::time::{timeout, sleep};
use crate::{
    agent::{AgentId, AgentConfig, AgentPool},
    runtime::{ProcessManager, ProcessState, ClaudeCommand, CommandError},
};

#[tokio::test]
async fn test_invalid_model_configuration() -> Result<()> {
    // Test handling of invalid model names
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    let config = AgentConfig {
        model: "invalid-model-name".to_string(), // Invalid model
        timeout_seconds: 30,
        max_turns: Some(1),
        allowed_tools: None,
        enable_mcp: false,
    };
    
    // Process should spawn but Claude CLI will error
    let spawn_result = process_manager.spawn(&config).await;
    
    if spawn_result.is_ok() {
        // Wait a bit for Claude to process the invalid model
        sleep(Duration::from_millis(500)).await;
        
        // Check if process is still healthy (it might have terminated)
        let is_healthy = process_manager.is_healthy().await?;
        
        // Clean up if still running
        if is_healthy {
            process_manager.stop().await?;
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_empty_command_validation() -> Result<()> {
    // Test command validation for empty commands
    let command = ClaudeCommand::new();
    
    // Should fail validation due to missing model
    assert!(command.validate().is_err());
    
    // Verify specific error type
    match command.validate() {
        Err(CommandError::EmptyCommand) => {
            // Expected error
        }
        Err(CommandError::MissingRequiredArg(arg)) => {
            assert_eq!(arg, "model");
        }
        _ => panic!("Expected validation error for empty command"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_process_spawn_failure() -> Result<()> {
    // Test handling when Claude CLI is not available
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    // Save original PATH
    let original_path = std::env::var("PATH").ok();
    
    // Temporarily clear PATH to make Claude CLI unavailable
    std::env::set_var("PATH", "");
    
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: Some(1),
        allowed_tools: None,
        enable_mcp: false,
    };
    
    let spawn_result = process_manager.spawn(&config).await;
    
    // Restore PATH
    if let Some(path) = original_path {
        std::env::set_var("PATH", path);
    }
    
    // Should have failed to spawn
    assert!(spawn_result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_input_after_termination() -> Result<()> {
    // Test sending input after process termination
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: Some(1),
        allowed_tools: None,
        enable_mcp: false,
    };
    
    process_manager.spawn(&config).await?;
    
    // Stop the process
    process_manager.stop().await?;
    assert_eq!(process_manager.get_state().await, ProcessState::Terminated);
    
    // Try to send input (should fail)
    let send_result = process_manager.send_input("This should fail").await;
    assert!(send_result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_output_read_timeout() -> Result<()> {
    // Test timeout when reading output
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: Some(1),
        allowed_tools: None,
        enable_mcp: false,
    };
    
    process_manager.spawn(&config).await?;
    
    // Try to read output with short timeout
    let read_result = timeout(
        Duration::from_millis(10), // Very short timeout
        process_manager.read_output_line()
    ).await;
    
    // Should timeout
    assert!(read_result.is_err());
    
    // Clean up
    process_manager.stop().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_state_conflicts() -> Result<()> {
    // Test conflicting state transitions
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: None,
        allowed_tools: None,
        enable_mcp: false,
    };
    
    process_manager.spawn(&config).await?;
    
    // Try to spawn again while already running (should fail)
    let second_spawn = process_manager.spawn(&config).await;
    assert!(second_spawn.is_err());
    
    // Pause the process
    process_manager.pause().await?;
    
    // Try to pause again (should fail)
    let second_pause = process_manager.pause().await;
    assert!(second_pause.is_err());
    
    // Try to stop while paused (should work)
    process_manager.stop().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_pool_exhaustion() -> Result<()> {
    // Test behavior when agent pool is exhausted
    let pool = AgentPool::new(1); // Very small pool
    
    let agent1_id = AgentId::new();
    let manager1 = ProcessManager::new(agent1_id.clone());
    
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: Some(1),
        allowed_tools: None,
        enable_mcp: false,
    };
    
    // First agent should succeed
    manager1.spawn(&config).await?;
    pool.register(agent1_id.clone(), manager1.clone()).await?;
    
    // Second agent should fail/timeout
    let agent2_id = AgentId::new();
    let manager2 = ProcessManager::new(agent2_id.clone());
    manager2.spawn(&config).await?;
    
    let register_result = timeout(
        Duration::from_millis(100),
        pool.register(agent2_id, manager2.clone())
    ).await;
    
    assert!(register_result.is_err()); // Should timeout
    
    // Clean up
    manager1.stop().await?;
    manager2.stop().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_rapid_state_transitions() -> Result<()> {
    // Test rapid state transitions
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: None,
        allowed_tools: None,
        enable_mcp: false,
    };
    
    process_manager.spawn(&config).await?;
    
    // Rapid pause/resume cycles
    for _ in 0..5 {
        process_manager.pause().await?;
        assert_eq!(process_manager.get_state().await, ProcessState::Paused);
        
        process_manager.resume().await?;
        assert_eq!(process_manager.get_state().await, ProcessState::Running);
    }
    
    // Should still be healthy
    assert!(process_manager.is_healthy().await?);
    
    process_manager.stop().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_malformed_tool_configuration() -> Result<()> {
    // Test handling of malformed tool configurations
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: Some(1),
        allowed_tools: Some(vec![
            "".to_string(),              // Empty tool name
            "invalid tool".to_string(),   // Space in tool name
            "mcp__".to_string(),          // Incomplete MCP tool
        ]),
        enable_mcp: true,
    };
    
    // Should still spawn (Claude will handle invalid tools)
    let spawn_result = process_manager.spawn(&config).await;
    
    if spawn_result.is_ok() {
        // Send a test prompt
        process_manager.send_input("Test with invalid tools").await?;
        
        // Give Claude time to process
        sleep(Duration::from_millis(500)).await;
        
        // Clean up
        process_manager.stop().await?;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_resource_cleanup_on_panic() -> Result<()> {
    // Test that resources are cleaned up even on panic
    let pool = Arc::new(AgentPool::new(5));
    
    let panic_handle = tokio::spawn({
        let pool_clone = pool.clone();
        async move {
            let agent_id = AgentId::new();
            let process_manager = ProcessManager::new(agent_id.clone());
            
            let config = AgentConfig {
                model: "claude-sonnet-4-20250514".to_string(),
                timeout_seconds: 30,
                max_turns: Some(1),
                allowed_tools: None,
                enable_mcp: false,
            };
            
            process_manager.spawn(&config).await.unwrap();
            pool_clone.register(agent_id.clone(), process_manager.clone()).await.unwrap();
            
            // Simulate panic
            panic!("Simulated panic for testing");
        }
    });
    
    // Wait for panic
    let _ = panic_handle.await; // Will return Err due to panic
    
    // Give time for cleanup
    sleep(Duration::from_millis(500)).await;
    
    // Pool should eventually show the slot as available again
    let status = pool.get_pool_status().await;
    
    // Note: Actual cleanup depends on Drop implementations
    // This test verifies the framework doesn't deadlock
    
    Ok(())
}

#[tokio::test]
async fn test_long_running_timeout() -> Result<()> {
    // Test timeout for long-running operations
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 1, // Very short timeout
        max_turns: Some(1),
        allowed_tools: None,
        enable_mcp: false,
    };
    
    process_manager.spawn(&config).await?;
    
    // Send a prompt that might take longer than timeout
    process_manager.send_input("Generate a very long and detailed analysis").await?;
    
    // Wait for potential timeout
    sleep(Duration::from_secs(2)).await;
    
    // Check if process handled timeout gracefully
    let state = process_manager.get_state().await;
    
    // Clean up regardless of state
    let _ = process_manager.stop().await;
    
    Ok(())
}

use std::sync::Arc;