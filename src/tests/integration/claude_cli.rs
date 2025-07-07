//! Claude Code CLI Integration Tests
//!
//! Tests for Claude CLI process management, command building, and integration patterns.

use std::time::Duration;
use anyhow::Result;
use tokio::time::timeout;
use crate::{
    agent::{AgentId, AgentConfig},
    runtime::{ProcessManager, ProcessState, ClaudeCommand, OutputFormat},
};

#[tokio::test]
async fn test_claude_cli_spawn_basic() -> Result<()> {
    // Test basic Claude CLI process spawning
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: Some(5),
        allowed_tools: Some(vec!["Read".to_string(), "Write".to_string()]),
        enable_mcp: false,
        memory_limit_mb: None,
    };
    
    // Spawn process
    process_manager.spawn(&config).await?;
    
    // Verify state
    assert_eq!(process_manager.get_state().await, ProcessState::Running);
    assert!(process_manager.is_healthy().await?);
    
    // Clean up
    process_manager.stop().await?;
    assert_eq!(process_manager.get_state().await, ProcessState::Terminated);
    
    Ok(())
}

#[tokio::test]
async fn test_command_building_with_mcp() -> Result<()> {
    // Test command building with MCP configuration
    let config = AgentConfig {
        model: "claude-opus-4-20250514".to_string(),
        timeout_seconds: 60,
        max_turns: Some(10),
        allowed_tools: Some(vec![
            "mcp__filesystem__read_file".to_string(),
            "mcp__filesystem__list_directory".to_string(),
        ]),
        enable_mcp: true,
        memory_limit_mb: None,
    };
    
    let command = ClaudeCommand::build(&config)?;
    
    // Verify arguments
    assert!(command.args.contains(&"--model".to_string()));
    assert!(command.args.contains(&"claude-opus-4-20250514".to_string()));
    assert!(command.args.contains(&"--output-format".to_string()));
    assert!(command.args.contains(&"json".to_string()));
    assert!(command.args.contains(&"--max-turns".to_string()));
    assert!(command.args.contains(&"10".to_string()));
    assert!(command.args.contains(&"--mcp".to_string()));
    assert!(command.args.contains(&"--allowed-tools".to_string()));
    
    // Verify allowed tools format
    let tools_arg_index = command.args.iter().position(|arg| arg == "--allowed-tools").unwrap();
    let tools_value = &command.args[tools_arg_index + 1];
    assert_eq!(tools_value, "mcp__filesystem__read_file,mcp__filesystem__list_directory");
    
    Ok(())
}

#[tokio::test]
async fn test_graceful_shutdown() -> Result<()> {
    // Test graceful shutdown with exit command
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
    assert_eq!(process_manager.get_state().await, ProcessState::Running);
    
    // Graceful shutdown
    process_manager.stop().await?;
    
    // Verify clean termination
    assert_eq!(process_manager.get_state().await, ProcessState::Terminated);
    assert!(!process_manager.is_healthy().await?);
    
    Ok(())
}

#[tokio::test]
async fn test_force_kill_timeout() -> Result<()> {
    // Test force kill when graceful shutdown times out
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
    
    // Force kill directly
    process_manager.kill().await?;
    
    assert_eq!(process_manager.get_state().await, ProcessState::Terminated);
    
    Ok(())
}

#[tokio::test]
async fn test_input_output_handling() -> Result<()> {
    // Test sending input and reading output
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
    
    // Send a simple prompt
    process_manager.send_input("Hello, Claude!").await?;
    
    // Try to read output with timeout
    let read_result = timeout(
        Duration::from_secs(5),
        process_manager.read_output_line()
    ).await;
    
    match read_result {
        Ok(Ok(Some(line))) => {
            // Verify we got some output
            assert!(!line.is_empty());
        }
        Ok(Ok(None)) => {
            // EOF is acceptable if process ended
        }
        Ok(Err(e)) => {
            // Log error but don't fail test - Claude might not be available
            eprintln!("Output read error: {}", e);
        }
        Err(_) => {
            // Timeout is acceptable if Claude is not available
            eprintln!("Read timeout - Claude CLI might not be available");
        }
    }
    
    process_manager.stop().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_pause_resume_functionality() -> Result<()> {
    // Test pausing and resuming process
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
    
    // Pause process
    process_manager.pause().await?;
    assert_eq!(process_manager.get_state().await, ProcessState::Paused);
    assert!(process_manager.is_paused().await);
    
    // Try to send input while paused (should fail)
    let send_result = process_manager.send_input("test").await;
    assert!(send_result.is_err());
    
    // Resume process
    process_manager.resume().await?;
    assert_eq!(process_manager.get_state().await, ProcessState::Running);
    assert!(!process_manager.is_paused().await);
    
    // Should be able to send input again
    process_manager.send_input("test").await?;
    
    process_manager.stop().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_state_transitions() -> Result<()> {
    // Test all valid state transitions
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    // Initial state
    assert_eq!(process_manager.get_state().await, ProcessState::NotStarted);
    
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: None,
        allowed_tools: None,
        enable_mcp: false,
    };
    
    // NotStarted -> Starting -> Running
    process_manager.spawn(&config).await?;
    assert_eq!(process_manager.get_state().await, ProcessState::Running);
    
    // Running -> Paused
    process_manager.pause().await?;
    assert_eq!(process_manager.get_state().await, ProcessState::Paused);
    
    // Paused -> Running
    process_manager.resume().await?;
    assert_eq!(process_manager.get_state().await, ProcessState::Running);
    
    // Running -> Stopping -> Terminated
    process_manager.stop().await?;
    assert_eq!(process_manager.get_state().await, ProcessState::Terminated);
    
    Ok(())
}

#[tokio::test]
async fn test_invalid_state_transitions() -> Result<()> {
    // Test invalid state transitions
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    // Try to pause when not running
    let pause_result = process_manager.pause().await;
    assert!(pause_result.is_err());
    
    // Try to resume when not paused
    let resume_result = process_manager.resume().await;
    assert!(resume_result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_command_validation() -> Result<()> {
    // Test command validation
    let mut command = ClaudeCommand::new();
    
    // Empty command should fail validation
    assert!(command.validate().is_err());
    
    // Add required model argument
    command = command.model("claude-sonnet-4-20250514");
    assert!(command.validate().is_ok());
    
    // Test with all options
    command = command
        .output_format(OutputFormat::StreamJson)
        .max_turns(10)
        .allowed_tools(&["Read".to_string(), "Write".to_string()])
        .enable_mcp()
        .timeout(60);
    
    assert!(command.validate().is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_output_format_variations() -> Result<()> {
    // Test different output format configurations
    let formats = vec![
        (OutputFormat::Text, "text"),
        (OutputFormat::Json, "json"),
        (OutputFormat::StreamJson, "stream-json"),
    ];
    
    for (format, expected_str) in formats {
        assert_eq!(format.as_str(), expected_str);
        
        let command = ClaudeCommand::new()
            .model("claude-sonnet-4-20250514")
            .output_format(format);
        
        assert!(command.args.contains(&expected_str.to_string()));
    }
    
    Ok(())
}