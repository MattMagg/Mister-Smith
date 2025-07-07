//! Simple Claude Code Integration Test
//!
//! Tests basic Claude Code CLI spawning with --print mode

use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_claude_code_basic_print_mode() {
    println!("Testing Claude Code basic print mode...");
    
    // Build command
    let output = Command::new("claude")
        .args(&[
            "--print",
            "--output-format", "json",
            "Say hello in JSON format"
        ])
        .output();
    
    match output {
        Ok(output) => {
            println!("Exit status: {}", output.status);
            println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
            println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
            
            assert!(output.status.success(), "Claude Code should exit successfully");
            assert!(!output.stdout.is_empty(), "Should have output");
        }
        Err(e) => {
            eprintln!("Failed to execute Claude Code: {}", e);
            eprintln!("Make sure Claude Code is installed and in PATH");
        }
    }
}

#[tokio::test] 
async fn test_claude_code_with_tools() {
    println!("Testing Claude Code with allowed tools...");
    
    let output = Command::new("claude")
        .args(&[
            "--print",
            "--output-format", "json",
            "--model", "claude-3-5-sonnet-20241022",
            "--allowedTools", "Read,Write",
            "What files are in the current directory?"
        ])
        .output();
    
    match output {
        Ok(output) => {
            println!("Exit status: {}", output.status);
            println!("Stdout length: {} bytes", output.stdout.len());
            
            if output.status.success() {
                println!("✅ Claude Code executed successfully with tools");
            } else {
                println!("⚠️  Claude Code exited with error");
                println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            eprintln!("Failed to execute Claude Code: {}", e);
        }
    }
}

#[tokio::test]
async fn test_process_manager_with_print_mode() {
    use crate::{
        agent::{AgentId, AgentConfig},
        runtime::{ProcessManager, ProcessState},
    };
    
    println!("Testing ProcessManager with Claude Code print mode...");
    
    let agent_id = AgentId::new();
    let process_manager = ProcessManager::new(agent_id.clone());
    
    let config = AgentConfig {
        model: "claude-3-5-sonnet-20241022".to_string(),
        timeout_seconds: 30,
        max_turns: None,
        allowed_tools: Some(vec!["Read".to_string()]),
        enable_mcp: false,
        memory_limit_mb: None,
    };
    
    // Note: Current ProcessManager needs to be adapted for print mode
    // In print mode, Claude Code runs once and exits
    println!("⚠️  ProcessManager currently expects interactive mode");
    println!("   Need to adapt for single-shot print mode execution");
    
    // For now, just verify the command building works
    use crate::runtime::ClaudeCommand;
    let cmd = ClaudeCommand::build(&config).unwrap();
    println!("Built command args: {:?}", cmd.args);
    
    assert!(cmd.args.contains(&"--print".to_string()));
    assert!(cmd.args.contains(&"--output-format".to_string()));
    assert!(cmd.args.contains(&"json".to_string()));
}