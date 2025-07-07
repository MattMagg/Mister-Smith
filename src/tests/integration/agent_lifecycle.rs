//! Agent Lifecycle Integration Tests
//!
//! Tests for complete agent lifecycle management including spawning, monitoring, and termination.

use std::time::Duration;
use anyhow::Result;
use tokio::time::{timeout, sleep};
use crate::{
    agent::{Agent, AgentId, AgentConfig, AgentPool, PoolStatus, AgentState},
    runtime::{ProcessManager, ProcessState},
};

#[tokio::test]
async fn test_agent_pool_basic_operations() -> Result<()> {
    // Test basic agent pool operations
    let pool = AgentPool::new(5); // Max 5 concurrent agents
    
    // Check initial status
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 0);
    assert_eq!(status.max_capacity, 5);
    assert_eq!(status.available_slots, 5);
    
    // Create and spawn an agent
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: Some(5),
        allowed_tools: None,
        enable_mcp: false,
    };
    
    let agent = Agent::new(config);
    let agent_id = agent.id.clone();
    agent.spawn().await?;
    
    // Register with pool
    let _permit = pool.register(agent.clone()).await?;
    
    // Check updated status
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 1);
    assert_eq!(status.available_slots, 4);
    assert!(status.agents.contains(&agent_id));
    
    // Clean up
    agent.stop().await?;
    pool.unregister(&agent_id).await?;
    
    // Check final status
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 0);
    assert_eq!(status.available_slots, 5);
    
    Ok(())
}

#[tokio::test]
async fn test_agent_pool_capacity_limits() -> Result<()> {
    // Test agent pool capacity enforcement
    let pool = AgentPool::new(2); // Max 2 concurrent agents
    
    let mut agents = vec![];
    let mut _permits = vec![];
    
    // Spawn agents up to capacity
    for _ in 0..2 {
        let config = AgentConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            timeout_seconds: 30,
            max_turns: Some(1),
            allowed_tools: None,
            enable_mcp: false,
        };
        
        let agent = Agent::new(config);
        agent.spawn().await?;
        let permit = pool.register(agent.clone()).await?;
        
        agents.push(agent);
        _permits.push(permit);
    }
    
    // Verify pool is at capacity
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 2);
    assert_eq!(status.available_slots, 0);
    
    // Try to register another agent (should block or fail)
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: Some(1),
        allowed_tools: None,
        enable_mcp: false,
    };
    
    let extra_agent = Agent::new(config);
    extra_agent.spawn().await?;
    
    // This should timeout or fail due to capacity
    let register_result = timeout(
        Duration::from_millis(100),
        pool.register(extra_agent.clone())
    ).await;
    
    assert!(register_result.is_err()); // Should timeout
    
    // Clean up one agent
    agents[0].stop().await?;
    pool.unregister(&agents[0].id).await?;
    drop(_permits.remove(0)); // Release permit
    
    // Now registration should succeed
    let _permit = pool.register(extra_agent.clone()).await?;
    
    // Clean up remaining agents
    for agent in &agents[1..] {
        agent.stop().await?;
    }
    extra_agent.stop().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_agent_crash_recovery() -> Result<()> {
    // Test recovery from unexpected agent termination
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: None,
        allowed_tools: None,
        enable_mcp: false,
    };
    
    let agent = Agent::new(config.clone());
    agent.spawn().await?;
    
    // Verify agent is running
    assert_eq!(agent.get_state().await, AgentState::Running);
    assert!(agent.process_manager.is_healthy().await?);
    
    // Simulate crash by force killing
    agent.process_manager.kill().await?;
    
    // Check agent state detection
    sleep(Duration::from_millis(100)).await;
    assert!(!agent.process_manager.is_healthy().await?);
    
    // Should be able to respawn
    agent.spawn().await?;
    assert_eq!(agent.get_state().await, AgentState::Running);
    assert!(agent.process_manager.is_healthy().await?);
    
    // Clean up
    agent.stop().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_agent_lifecycle_with_work() -> Result<()> {
    // Test full agent lifecycle with actual work simulation
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 60,
        max_turns: Some(3),
        allowed_tools: Some(vec!["Read".to_string()]),
        enable_mcp: false,
    };
    
    let agent = Agent::new(config);
    
    // Spawn agent
    agent.spawn().await?;
    assert_eq!(agent.get_state().await, AgentState::Running);
    
    // Simulate work phases
    for i in 0..3 {
        // Send work
        let prompt = format!("Task {}: Analyze this code", i);
        agent.process_manager.send_input(&prompt).await?;
        
        // Simulate processing time
        sleep(Duration::from_millis(100)).await;
        
        // Check health
        assert!(agent.process_manager.is_healthy().await?);
    }
    
    // Pause during work
    agent.process_manager.pause().await?;
    assert_eq!(agent.process_manager.get_state().await, ProcessState::Paused);
    
    // Simulate pause duration
    sleep(Duration::from_millis(200)).await;
    
    // Resume work
    agent.process_manager.resume().await?;
    assert_eq!(agent.process_manager.get_state().await, ProcessState::Running);
    
    // Complete remaining work
    agent.process_manager.send_input("Final task: Generate summary").await?;
    
    // Graceful shutdown
    agent.stop().await?;
    assert_eq!(agent.get_state().await, AgentState::Terminated);
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_agent_coordination() -> Result<()> {
    // Test coordination between multiple agents
    let pool = AgentPool::new(3);
    
    let mut agents = vec![];
    let mut _permits = vec![];
    
    // Spawn 3 agents with different configurations
    let models = vec![
        "claude-sonnet-4-20250514",
        "claude-opus-4-20250514",
        "claude-sonnet-4-20250514",
    ];
    
    for (i, model) in models.iter().enumerate() {
        let config = AgentConfig {
            model: model.to_string(),
            timeout_seconds: 30,
            max_turns: Some(2),
            allowed_tools: Some(vec!["Read".to_string()]),
            enable_mcp: i == 1, // Enable MCP for one agent
        };
        
        let agent = Agent::new(config);
        agent.spawn().await?;
        let permit = pool.register(agent.clone()).await?;
        
        agents.push(agent);
        _permits.push(permit);
    }
    
    // Verify all agents are registered
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 3);
    
    // Send work to all agents concurrently
    let mut handles = vec![];
    for (i, agent) in agents.iter().enumerate() {
        let process_manager = agent.process_manager.clone();
        let handle = tokio::spawn(async move {
            process_manager.send_input(&format!("Agent {} task", i)).await
        });
        handles.push(handle);
    }
    
    // Wait for all sends to complete
    for handle in handles {
        handle.await??;
    }
    
    // Verify all agents are still healthy
    for agent in &agents {
        assert!(agent.process_manager.is_healthy().await?);
    }
    
    // Clean up all agents
    for agent in &agents {
        agent.stop().await?;
        pool.unregister(&agent.id).await?;
    }
    
    // Verify pool is empty
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_agent_restart_on_failure() -> Result<()> {
    // Test automatic restart behavior on failure
    let config = AgentConfig {
        model: "claude-sonnet-4-20250514".to_string(),
        timeout_seconds: 30,
        max_turns: None,
        allowed_tools: None,
        enable_mcp: false,
    };
    
    let agent = Agent::new(config.clone());
    
    // Track restart attempts
    let mut restart_count = 0;
    const MAX_RESTARTS: i32 = 3;
    
    while restart_count < MAX_RESTARTS {
        agent.spawn().await?;
        assert!(agent.process_manager.is_healthy().await?);
        
        // Simulate work
        agent.process_manager.send_input("Process some data").await?;
        sleep(Duration::from_millis(100)).await;
        
        // Simulate failure
        agent.process_manager.kill().await?;
        assert!(!agent.process_manager.is_healthy().await?);
        
        restart_count += 1;
        
        // Simulate restart delay
        sleep(Duration::from_millis(200)).await;
    }
    
    assert_eq!(restart_count, MAX_RESTARTS);
    
    Ok(())
}