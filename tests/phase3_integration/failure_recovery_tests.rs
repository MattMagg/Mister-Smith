use super::*;
use super::test_builders::{TestSupervisionBuilder, TestAgentPoolBuilder, MockClaudeExecutor};
use super::chaos_injector::ChaosInjector;
use mistersmith::supervision::{
    AgentPoolSupervisor, FailureReason, BackoffStrategy,
    RestartPolicy, SupervisionResult
};
use mistersmith::agent::{AgentId, AgentState};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use anyhow::Result;

#[tokio::test]
async fn test_agent_crash_and_automatic_restart() -> Result<()> {
    let env = setup_test_env().await;
    
    // Build supervision tree with restart policy
    let supervision_tree = TestSupervisionBuilder::new()
        .with_restart_policy(RestartPolicy {
            max_restarts: 3,
            time_window: Duration::from_secs(60),
            backoff_strategy: BackoffStrategy::Exponential {
                initial: Duration::from_millis(100),
                max: Duration::from_secs(5),
                factor: 2.0,
            },
        })
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(3)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("agent-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn agent with crash-prone executor
    let crash_count = Arc::new(AtomicU32::new(0));
    let crash_count_clone = crash_count.clone();
    
    let config = AgentConfig {
        id: AgentId::from("crash-test-agent"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };

    // Use mock executor that crashes on first 2 calls
    let mock_executor = MockClaudeExecutor::new()
        .with_crash_count(2, crash_count_clone);

    pool_supervisor.spawn_agent_with_executor(
        config.clone(),
        Arc::new(mock_executor),
    ).await?;

    // Trigger agent execution that will crash
    let task_result = pool_supervisor.execute_task(
        &config.id,
        "Test task that triggers crash",
    ).await;

    assert!(task_result.is_err());
    assert_eq!(crash_count.load(Ordering::SeqCst), 1);

    // Wait for automatic restart
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify agent was restarted
    let child_id = ChildId::from(config.id.to_string());
    let child_state = pool_supervisor.get_child_state(&child_id).await?;
    assert_eq!(child_state, ChildState::Running);

    // Execute again - should crash and restart again
    let task_result2 = pool_supervisor.execute_task(
        &config.id,
        "Second task",
    ).await;
    
    assert!(task_result2.is_err());
    assert_eq!(crash_count.load(Ordering::SeqCst), 2);

    // Wait for second restart
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Third execution should succeed (no more crashes)
    let task_result3 = pool_supervisor.execute_task(
        &config.id,
        "Third task should succeed",
    ).await;

    assert!(task_result3.is_ok());
    assert_eq!(task_result3?, "Mock response from Claude");

    Ok(())
}

#[tokio::test]
async fn test_exponential_backoff_timing() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
        .with_restart_policy(RestartPolicy {
            max_restarts: 5,
            time_window: Duration::from_secs(60),
            backoff_strategy: BackoffStrategy::Exponential {
                initial: Duration::from_millis(100),
                max: Duration::from_secs(2),
                factor: 2.0,
            },
        })
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(1)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("agent-pool"),
        pool_supervisor.clone(),
    ).await?;

    let config = AgentConfig {
        id: AgentId::from("backoff-test"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };

    pool_supervisor.spawn_agent(config.clone()).await?;

    // Track restart delays
    let mut restart_delays = Vec::new();
    let failure = FailureReason::ProcessCrash {
        exit_code: -1,
        message: "Testing backoff".to_string(),
    };

    // Trigger multiple failures and measure backoff
    for i in 0..4 {
        let start = Instant::now();
        
        let result = supervision_tree.handle_child_failure(
            NodeId::from("agent-pool"),
            ChildId::from(config.id.to_string()),
            failure.clone(),
        ).await?;

        if let SupervisionResult::RestartChild { delay, .. } = result {
            restart_delays.push(delay);
            
            // Verify exponential growth
            let expected_delay = Duration::from_millis(100 * 2u64.pow(i));
            let capped_delay = expected_delay.min(Duration::from_secs(2));
            
            assert!(
                delay >= capped_delay * 9 / 10 && delay <= capped_delay * 11 / 10,
                "Delay {:?} not within 10% of expected {:?}",
                delay,
                capped_delay
            );
        }

        // Wait for restart to complete
        tokio::time::sleep(restart_delays.last().unwrap().clone()).await;
    }

    // Verify delays increased exponentially up to max
    assert_eq!(restart_delays[0], Duration::from_millis(100));
    assert_eq!(restart_delays[1], Duration::from_millis(200));
    assert_eq!(restart_delays[2], Duration::from_millis(400));
    assert_eq!(restart_delays[3], Duration::from_millis(800));

    Ok(())
}

#[tokio::test]
async fn test_max_restart_limit_enforcement() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
        .with_restart_policy(RestartPolicy {
            max_restarts: 3,
            time_window: Duration::from_secs(10),
            backoff_strategy: BackoffStrategy::Fixed {
                delay: Duration::from_millis(50),
            },
        })
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(1)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("agent-pool"),
        pool_supervisor.clone(),
    ).await?;

    let config = AgentConfig {
        id: AgentId::from("max-restart-test"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };

    pool_supervisor.spawn_agent(config.clone()).await?;

    // Trigger failures up to the limit
    let failure = FailureReason::ProcessCrash {
        exit_code: -1,
        message: "Testing max restarts".to_string(),
    };

    for i in 0..3 {
        let result = supervision_tree.handle_child_failure(
            NodeId::from("agent-pool"),
            ChildId::from(config.id.to_string()),
            failure.clone(),
        ).await?;

        assert!(
            matches!(result, SupervisionResult::RestartChild { .. }),
            "Expected restart on failure {}", i + 1
        );

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Fourth failure should exceed limit
    let final_result = supervision_tree.handle_child_failure(
        NodeId::from("agent-pool"),
        ChildId::from(config.id.to_string()),
        failure,
    ).await?;

    assert!(
        matches!(final_result, SupervisionResult::Stop { .. }),
        "Expected Stop after exceeding max restarts"
    );

    Ok(())
}

#[tokio::test]
async fn test_persistent_failure_handling() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
        .with_restart_policy(RestartPolicy {
            max_restarts: 5,
            time_window: Duration::from_secs(30),
            backoff_strategy: BackoffStrategy::Exponential {
                initial: Duration::from_millis(100),
                max: Duration::from_secs(5),
                factor: 2.0,
            },
        })
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(2)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("agent-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn agent that will persistently fail
    let config = AgentConfig {
        id: AgentId::from("persistent-failure-test"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };

    let always_fail_executor = MockClaudeExecutor::new()
        .with_persistent_failure("Persistent error");

    pool_supervisor.spawn_agent_with_executor(
        config.clone(),
        Arc::new(always_fail_executor),
    ).await?;

    // Track failure history
    let mut failure_count = 0;
    let child_id = ChildId::from(config.id.to_string());

    // Execute tasks that will fail
    for _ in 0..5 {
        let result = pool_supervisor.execute_task(
            &config.id,
            "Task that will fail",
        ).await;

        if result.is_err() {
            failure_count += 1;
            
            // Report failure to supervision
            let failure = FailureReason::TaskExecutionError {
                task_id: format!("task-{}", failure_count),
                error: result.unwrap_err().to_string(),
            };

            supervision_tree.handle_child_failure(
                NodeId::from("agent-pool"),
                child_id.clone(),
                failure,
            ).await?;

            // Wait for potential restart
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    // After persistent failures, agent should be stopped
    let final_state = pool_supervisor.get_child_state(&child_id).await?;
    assert_eq!(final_state, ChildState::Stopped);

    // Verify failure history was tracked
    let failure_history = supervision_tree.get_failure_history(&child_id).await?;
    assert!(failure_history.len() >= 5);

    Ok(())
}

#[tokio::test]
async fn test_graceful_degradation() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(5)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("agent-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn 5 agents
    let mut agent_ids = Vec::new();
    for i in 0..5 {
        let config = AgentConfig {
            id: AgentId::from(format!("degradation-test-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        };
        let agent = pool_supervisor.spawn_agent(config).await?;
        agent_ids.push(agent.id().clone());
    }

    // Simulate progressive failures
    for i in 0..3 {
        let failure = FailureReason::ResourceExhaustion {
            resource_type: "memory".to_string(),
            limit: 1024,
            requested: 2048,
        };

        supervision_tree.handle_child_failure(
            NodeId::from("agent-pool"),
            ChildId::from(agent_ids[i].to_string()),
            failure,
        ).await?;
    }

    // Verify system degrades gracefully
    let pool_status = pool_supervisor.get_pool_status().await?;
    assert_eq!(pool_status.active_agents, 2); // Only 2 agents still active

    // System should still be operational with reduced capacity
    let health = pool_supervisor.health_check().await?;
    assert!(matches!(health, HealthResult::Degraded(_)));

    // Remaining agents should still work
    for i in 3..5 {
        let result = pool_supervisor.execute_task(
            &agent_ids[i],
            "Task for remaining agents",
        ).await;
        assert!(result.is_ok());
    }

    Ok(())
}

#[tokio::test]
async fn test_recovery_after_time_window() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
        .with_restart_policy(RestartPolicy {
            max_restarts: 2,
            time_window: Duration::from_secs(2), // Short window for testing
            backoff_strategy: BackoffStrategy::Fixed {
                delay: Duration::from_millis(100),
            },
        })
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(1)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("agent-pool"),
        pool_supervisor.clone(),
    ).await?;

    let config = AgentConfig {
        id: AgentId::from("time-window-test"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };

    pool_supervisor.spawn_agent(config.clone()).await?;

    let failure = FailureReason::ProcessCrash {
        exit_code: -1,
        message: "Time window test".to_string(),
    };

    // Use up restart quota
    for _ in 0..2 {
        supervision_tree.handle_child_failure(
            NodeId::from("agent-pool"),
            ChildId::from(config.id.to_string()),
            failure.clone(),
        ).await?;
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    // Wait for time window to expire
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Failure counter should reset, allowing more restarts
    let result = supervision_tree.handle_child_failure(
        NodeId::from("agent-pool"),
        ChildId::from(config.id.to_string()),
        failure,
    ).await?;

    assert!(
        matches!(result, SupervisionResult::RestartChild { .. }),
        "Expected restart after time window reset"
    );

    Ok(())
}