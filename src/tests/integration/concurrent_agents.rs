//! Concurrent Agents Integration Tests
//!
//! Tests for managing multiple Claude CLI agents concurrently.

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tokio::sync::{Semaphore, RwLock};
use tokio::time::{timeout, sleep};
use crate::{
    agent::{Agent, AgentId, AgentConfig, AgentPool},
    runtime::{ProcessManager, ProcessState},
};

#[tokio::test]
async fn test_concurrent_agent_spawning() -> Result<()> {
    // Test spawning multiple agents concurrently
    let num_agents = 5;
    let pool = Arc::new(AgentPool::new(10));
    
    let mut handles = vec![];
    
    for i in 0..num_agents {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let config = AgentConfig {
                model: "claude-sonnet-4-20250514".to_string(),
                timeout_seconds: 30,
                max_turns: Some(1),
                allowed_tools: None,
                enable_mcp: false,
            };
            
            let agent = Agent::new(config);
            let agent_id = agent.id.clone();
            agent.spawn().await?;
            let _permit = pool_clone.register(agent.clone()).await?;
            
            // Simulate some work
            sleep(Duration::from_millis(100 + i as u64 * 50)).await;
            
            // Clean up
            agent.stop().await?;
            pool_clone.unregister(&agent_id).await?;
            
            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }
    
    // Wait for all spawns to complete
    for handle in handles {
        handle.await??;
    }
    
    // Verify pool is empty
    let status = pool.get_status().await;
    assert_eq!(status.active_count, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_resource_contention() -> Result<()> {
    // Test behavior when more agents are requested than pool capacity
    let pool_size = 3;
    let num_agents = 6;
    let pool = Arc::new(AgentPool::new(pool_size));
    
    let active_count = Arc::new(RwLock::new(0));
    let max_concurrent = Arc::new(RwLock::new(0));
    
    let mut handles = vec![];
    
    for i in 0..num_agents {
        let pool_clone = pool.clone();
        let active_count_clone = active_count.clone();
        let max_concurrent_clone = max_concurrent.clone();
        
        let handle = tokio::spawn(async move {
            let config = AgentConfig {
                model: "claude-sonnet-4-20250514".to_string(),
                timeout_seconds: 30,
                max_turns: Some(1),
                allowed_tools: None,
                enable_mcp: false,
            };
            
            let agent = Agent::new(config);
            let agent_id = agent.id.clone();
            agent.spawn().await?;
            
            // This might block if pool is full
            let start = std::time::Instant::now();
            let _permit = pool_clone.register(agent.clone()).await?;
            let wait_time = start.elapsed();
            
            // Track active agents
            {
                let mut count = active_count_clone.write().await;
                *count += 1;
                let current = *count;
                
                let mut max = max_concurrent_clone.write().await;
                if current > *max {
                    *max = current;
                }
            }
            
            // Simulate work
            sleep(Duration::from_millis(200)).await;
            
            // Decrement active count
            {
                let mut count = active_count_clone.write().await;
                *count -= 1;
            }
            
            // Clean up
            agent.stop().await?;
            pool_clone.unregister(&agent_id).await?;
            
            Ok::<_, anyhow::Error>((i, wait_time))
        });
        handles.push(handle);
    }
    
    // Collect results
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await??);
    }
    
    // Verify max concurrent never exceeded pool size
    let max = *max_concurrent.read().await;
    assert!(max <= pool_size, "Max concurrent ({}) exceeded pool size ({})", max, pool_size);
    
    // Some agents should have waited
    let waited_agents = results.iter().filter(|(_, wait_time)| wait_time.as_millis() > 50).count();
    assert!(waited_agents > 0, "Expected some agents to wait for pool slots");
    
    Ok(())
}

#[tokio::test]
async fn test_load_balancing() -> Result<()> {
    // Test load distribution across multiple agents
    let num_agents = 4;
    let tasks_per_agent = 5;
    let pool = Arc::new(AgentPool::new(num_agents));
    
    // Create agents
    let mut agents = vec![];
    let mut _permits = vec![];
    
    for _ in 0..num_agents {
        let config = AgentConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            timeout_seconds: 60,
            max_turns: Some(tasks_per_agent as u32),
            allowed_tools: None,
            enable_mcp: false,
        };
        
        let agent = Agent::new(config);
        agent.spawn().await?;
        let permit = pool.register(agent.clone()).await?;
        agents.push((agent.id.clone(), agent.clone(), Arc::new(RwLock::new(0))));
        _permits.push(permit);
    }
    
    // Distribute tasks across agents
    let task_semaphore = Arc::new(Semaphore::new(num_agents));
    let mut task_handles = vec![];
    
    for task_id in 0..num_agents * tasks_per_agent {
        let agents_clone = agents.clone();
        let task_sem_clone = task_semaphore.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = task_sem_clone.acquire().await?;
            
            // Simple round-robin distribution
            let agent_idx = task_id % agents_clone.len();
            let (_, agent, task_count) = &agents_clone[agent_idx];
            
            // Send task to agent
            agent.process_manager.send_input(&format!("Task {}", task_id)).await?;
            
            // Increment task count
            let mut count = task_count.write().await;
            *count += 1;
            
            // Simulate task processing
            sleep(Duration::from_millis(50)).await;
            
            Ok::<_, anyhow::Error>(())
        });
        task_handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in task_handles {
        handle.await??;
    }
    
    // Verify task distribution
    for (i, (_, _, task_count)) in agents.iter().enumerate() {
        let count = *task_count.read().await;
        assert_eq!(count, tasks_per_agent, "Agent {} processed {} tasks, expected {}", 
                   i, count, tasks_per_agent);
    }
    
    // Clean up agents
    for (agent_id, agent, _) in agents {
        agent.stop().await?;
        pool.unregister(&agent_id).await?;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_cascading_failures() -> Result<()> {
    // Test handling of cascading failures across multiple agents
    let num_agents = 3;
    let pool = Arc::new(AgentPool::new(num_agents * 2));
    
    // Create interconnected agents
    let agents = Arc::new(RwLock::new(vec![]));
    let mut _permits = vec![];
    
    for i in 0..num_agents {
        let config = AgentConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            timeout_seconds: 30,
            max_turns: None,
            allowed_tools: None,
            enable_mcp: false,
        };
        
        let agent = Agent::new(config);
        agent.spawn().await?;
        let permit = pool.register(agent.clone()).await?;
        
        let mut agents_guard = agents.write().await;
        agents_guard.push((agent.id.clone(), agent, i));
        _permits.push(permit);
    }
    
    // Simulate cascading failure
    let agents_clone = agents.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        
        // Kill first agent
        let agents_guard = agents_clone.read().await;
        if let Some((_, agent, _)) = agents_guard.first() {
            let _ = agent.process_manager.kill().await;
        }
    });
    
    // Monitor agent health
    let mut failure_detected = false;
    for _ in 0..10 {
        sleep(Duration::from_millis(100)).await;
        
        let agents_guard = agents.read().await;
        for (_, agent, idx) in agents_guard.iter() {
            if !agent.process_manager.is_healthy().await.unwrap_or(false) {
                println!("Agent {} failed", idx);
                failure_detected = true;
            }
        }
        
        if failure_detected {
            break;
        }
    }
    
    assert!(failure_detected, "Expected to detect agent failure");
    
    // Clean up remaining agents
    let agents_guard = agents.read().await;
    for (agent_id, agent, _) in agents_guard.iter() {
        let _ = agent.stop().await;
        let _ = pool.unregister(agent_id).await;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_state_transitions() -> Result<()> {
    // Test concurrent state transitions on multiple agents
    let num_agents = 3;
    let mut agents = vec![];
    
    // Create agents
    for _ in 0..num_agents {
        let config = AgentConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            timeout_seconds: 30,
            max_turns: None,
            allowed_tools: None,
            enable_mcp: false,
        };
        
        let agent = Agent::new(config);
        agent.spawn().await?;
        agents.push(agent);
    }
    
    // Perform concurrent state transitions
    let mut handles = vec![];
    
    // Pause all agents concurrently
    for agent in &agents {
        let process_manager = agent.process_manager.clone();
        let handle = tokio::spawn(async move {
            process_manager.pause().await
        });
        handles.push(handle);
    }
    
    for handle in handles.drain(..) {
        handle.await??;
    }
    
    // Verify all are paused
    for agent in &agents {
        assert_eq!(agent.process_manager.get_state().await, ProcessState::Paused);
    }
    
    // Resume all agents concurrently
    for agent in &agents {
        let process_manager = agent.process_manager.clone();
        let handle = tokio::spawn(async move {
            process_manager.resume().await
        });
        handles.push(handle);
    }
    
    for handle in handles.drain(..) {
        handle.await??;
    }
    
    // Verify all are running
    for agent in &agents {
        assert_eq!(agent.process_manager.get_state().await, ProcessState::Running);
    }
    
    // Stop all agents concurrently
    for agent in &agents {
        let agent_clone = agent.clone();
        let handle = tokio::spawn(async move {
            agent_clone.stop().await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await??;
    }
    
    // Verify all are terminated
    for agent in &agents {
        assert_eq!(agent.process_manager.get_state().await, ProcessState::Terminated);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_agent_performance_monitoring() -> Result<()> {
    // Test monitoring performance across multiple agents
    let num_agents = 3;
    let pool = Arc::new(AgentPool::new(num_agents));
    
    #[derive(Default)]
    struct AgentMetrics {
        messages_sent: u64,
        messages_received: u64,
        total_latency_ms: u64,
    }
    
    let metrics = Arc::new(RwLock::new(vec![AgentMetrics::default(); num_agents]));
    let mut _permits = vec![];
    
    let mut agent_handles = vec![];
    
    for i in 0..num_agents {
        let pool_clone = pool.clone();
        let metrics_clone = metrics.clone();
        
        let handle = tokio::spawn(async move {
            let config = AgentConfig {
                model: "claude-sonnet-4-20250514".to_string(),
                timeout_seconds: 60,
                max_turns: Some(10),
                allowed_tools: None,
                enable_mcp: false,
            };
            
            let agent = Agent::new(config);
            let agent_id = agent.id.clone();
            agent.spawn().await?;
            let permit = pool_clone.register(agent.clone()).await?;
            
            // Simulate work and measure performance
            for j in 0..5 {
                let start = std::time::Instant::now();
                
                agent.process_manager.send_input(&format!("Agent {} message {}", i, j)).await?;
                
                // Simulate response time
                sleep(Duration::from_millis(20 + (i as u64 * 10))).await;
                
                let latency = start.elapsed().as_millis() as u64;
                
                // Update metrics
                let mut metrics_guard = metrics_clone.write().await;
                metrics_guard[i].messages_sent += 1;
                metrics_guard[i].messages_received += 1;
                metrics_guard[i].total_latency_ms += latency;
            }
            
            // Clean up
            agent.stop().await?;
            pool_clone.unregister(&agent_id).await?;
            
            Ok::<_, anyhow::Error>(permit)
        });
        agent_handles.push(handle);
    }
    
    // Wait for all agents to complete
    for handle in agent_handles {
        _permits.push(handle.await??);
    }
    
    // Analyze metrics
    let metrics_guard = metrics.read().await;
    for (i, agent_metrics) in metrics_guard.iter().enumerate() {
        assert_eq!(agent_metrics.messages_sent, 5);
        assert_eq!(agent_metrics.messages_received, 5);
        
        let avg_latency = agent_metrics.total_latency_ms / agent_metrics.messages_sent;
        println!("Agent {} average latency: {}ms", i, avg_latency);
        
        // Verify reasonable latency
        assert!(avg_latency < 200, "Agent {} latency too high: {}ms", i, avg_latency);
    }
    
    Ok(())
}