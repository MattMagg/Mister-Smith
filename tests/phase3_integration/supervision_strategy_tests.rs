use super::*;
use super::test_builders::{TestSupervisionBuilder, TestAgentPoolBuilder, MockClaudeExecutor};
use mistersmith::supervision::{
    SupervisionStrategy, AgentPoolSupervisor, FailureReason,
    SupervisionResult, NodeId, ChildId
};
use mistersmith::agent::{AgentId, AgentState};
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;

#[tokio::test]
async fn test_one_for_one_strategy() -> Result<()> {
    let env = setup_test_env().await;
    
    // Build supervision tree with OneForOne strategy
    let supervision_tree = TestSupervisionBuilder::new()
        .with_strategy(SupervisionStrategy::OneForOne)
        .build()
        .await?;

    // Create agent pool with multiple agents
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

    // Spawn 3 agents
    let mut agents = Vec::new();
    for i in 0..3 {
        let config = AgentConfig {
            id: AgentId::from(format!("one-for-one-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        };
        let agent = pool_supervisor.spawn_agent(config).await?;
        agents.push(agent);
    }

    // Verify all agents are running
    pool_supervisor.sync_states().await?;
    for agent in &agents {
        let child_id = ChildId::from(agent.id().to_string());
        assert_eq!(
            pool_supervisor.get_child_state(&child_id).await?,
            ChildState::Running
        );
    }

    // Simulate failure of agent 1
    let failed_agent = &agents[1];
    failed_agent.transition_to(AgentState::Error).await?;
    
    // Report failure to supervision tree
    let failure = FailureReason::ProcessCrash {
        exit_code: -1,
        message: "Simulated agent crash".to_string(),
    };
    
    let result = supervision_tree.handle_child_failure(
        NodeId::from("agent-pool"),
        ChildId::from(failed_agent.id().to_string()),
        failure,
    ).await?;

    // Verify only the failed agent is restarted
    match result {
        SupervisionResult::RestartChild { child_id, delay } => {
            assert_eq!(child_id.to_string(), failed_agent.id().to_string());
            assert!(delay.as_millis() >= 100); // Initial backoff
        }
        _ => panic!("Expected RestartChild result"),
    }

    // Verify other agents are still running
    assert_eq!(agents[0].state().await, AgentState::Running);
    assert_eq!(agents[2].state().await, AgentState::Running);

    Ok(())
}

#[tokio::test]
async fn test_one_for_all_strategy() -> Result<()> {
    let env = setup_test_env().await;
    
    // Build supervision tree with OneForAll strategy
    let supervision_tree = TestSupervisionBuilder::new()
        .with_strategy(SupervisionStrategy::OneForAll)
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

    // Spawn 3 agents
    let mut agent_ids = Vec::new();
    for i in 0..3 {
        let config = AgentConfig {
            id: AgentId::from(format!("one-for-all-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        };
        let agent = pool_supervisor.spawn_agent(config).await?;
        agent_ids.push(agent.id().clone());
    }

    // Simulate failure of one agent
    let failed_id = &agent_ids[1];
    let failure = FailureReason::ProcessCrash {
        exit_code: -1,
        message: "Simulated crash triggering all restart".to_string(),
    };

    let result = supervision_tree.handle_child_failure(
        NodeId::from("agent-pool"),
        ChildId::from(failed_id.to_string()),
        failure,
    ).await?;

    // Verify all children are restarted
    match result {
        SupervisionResult::RestartAllChildren { delay } => {
            assert!(delay.as_millis() >= 100);
        }
        _ => panic!("Expected RestartAllChildren result"),
    }

    Ok(())
}

#[tokio::test]
async fn test_rest_for_one_strategy() -> Result<()> {
    let env = setup_test_env().await;
    
    // Build hierarchical supervision tree
    let supervision_tree = TestSupervisionBuilder::new()
        .with_strategy(SupervisionStrategy::RestForOne)
        .add_supervisor("group-1", SupervisionStrategy::OneForOne)
        .add_supervisor("group-2", SupervisionStrategy::OneForOne)
        .build()
        .await?;

    // Create agents in ordered groups
    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(6)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    // Register groups
    supervision_tree.register_supervisor(
        NodeId::from("group-1"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn agents in order
    let mut group1_agents = Vec::new();
    let mut group2_agents = Vec::new();

    // Group 1 agents (earlier in start order)
    for i in 0..3 {
        let config = AgentConfig {
            id: AgentId::from(format!("group1-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["group1".to_string()],
            resource_limits: Default::default(),
        };
        let agent = pool_supervisor.spawn_agent(config).await?;
        group1_agents.push(agent.id().clone());
    }

    // Group 2 agents (later in start order)
    for i in 0..3 {
        let config = AgentConfig {
            id: AgentId::from(format!("group2-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["group2".to_string()],
            resource_limits: Default::default(),
        };
        let agent = pool_supervisor.spawn_agent(config).await?;
        group2_agents.push(agent.id().clone());
    }

    // Simulate failure in group 1 (should restart group 1 and 2)
    let failure = FailureReason::ProcessCrash {
        exit_code: -1,
        message: "RestForOne test failure".to_string(),
    };

    let result = supervision_tree.handle_child_failure(
        NodeId::from("root"),
        ChildId::from(group1_agents[0].to_string()),
        failure,
    ).await?;

    // Verify rest-for-one behavior
    match result {
        SupervisionResult::RestartRest { failed_child, delay } => {
            assert_eq!(failed_child.to_string(), group1_agents[0].to_string());
            assert!(delay.as_millis() >= 100);
        }
        _ => panic!("Expected RestartRest result"),
    }

    Ok(())
}

#[tokio::test]
async fn test_escalation_strategy() -> Result<()> {
    let env = setup_test_env().await;
    
    // Build tree with escalation strategy
    let supervision_tree = TestSupervisionBuilder::new()
        .with_strategy(SupervisionStrategy::Escalate)
        .add_supervisor("critical-agents", SupervisionStrategy::OneForOne)
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
        NodeId::from("critical-agents"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn a critical agent
    let config = AgentConfig {
        id: AgentId::from("critical-agent-1"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["critical".to_string()],
        resource_limits: Default::default(),
    };
    let agent = pool_supervisor.spawn_agent(config).await?;

    // Simulate unrecoverable failure
    let failure = FailureReason::UnrecoverableError {
        error_type: "SystemFailure".to_string(),
        message: "Critical system component failed".to_string(),
    };

    let result = supervision_tree.handle_child_failure(
        NodeId::from("critical-agents"),
        ChildId::from(agent.id().to_string()),
        failure,
    ).await?;

    // Verify escalation
    match result {
        SupervisionResult::Escalate { to_parent } => {
            assert_eq!(to_parent.to_string(), "root");
        }
        _ => panic!("Expected Escalate result"),
    }

    Ok(())
}

#[tokio::test]
async fn test_restart_policy_enforcement() -> Result<()> {
    let env = setup_test_env().await;
    
    // Build tree with strict restart policy
    let supervision_tree = TestSupervisionBuilder::new()
        .with_strategy(SupervisionStrategy::OneForOne)
        .with_restart_policy(RestartPolicy {
            max_restarts: 2,
            time_window: Duration::from_secs(10),
            backoff_strategy: BackoffStrategy::Exponential {
                initial: Duration::from_millis(100),
                max: Duration::from_secs(1),
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

    // Spawn an agent
    let config = AgentConfig {
        id: AgentId::from("restart-policy-test"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };
    let agent = pool_supervisor.spawn_agent(config).await?;
    let agent_id = agent.id().clone();

    // Simulate multiple failures
    let failure = FailureReason::ProcessCrash {
        exit_code: -1,
        message: "Repeated failure".to_string(),
    };

    // First failure - should restart
    let result1 = supervision_tree.handle_child_failure(
        NodeId::from("agent-pool"),
        ChildId::from(agent_id.to_string()),
        failure.clone(),
    ).await?;
    
    assert!(matches!(result1, SupervisionResult::RestartChild { .. }));

    // Second failure - should restart with increased backoff
    let result2 = supervision_tree.handle_child_failure(
        NodeId::from("agent-pool"),
        ChildId::from(agent_id.to_string()),
        failure.clone(),
    ).await?;

    match result2 {
        SupervisionResult::RestartChild { delay, .. } => {
            assert!(delay.as_millis() >= 200); // Exponential backoff
        }
        _ => panic!("Expected RestartChild"),
    }

    // Third failure - should exceed max restarts
    let result3 = supervision_tree.handle_child_failure(
        NodeId::from("agent-pool"),
        ChildId::from(agent_id.to_string()),
        failure,
    ).await?;

    assert!(matches!(result3, SupervisionResult::Stop { .. }));

    Ok(())
}

#[tokio::test]
async fn test_custom_supervision_strategies() -> Result<()> {
    let env = setup_test_env().await;
    
    // Test custom strategy combinations
    let supervision_tree = TestSupervisionBuilder::new()
        .with_strategy(SupervisionStrategy::OneForOne)
        .add_supervisor("high-priority", SupervisionStrategy::OneForAll)
        .add_supervisor("low-priority", SupervisionStrategy::OneForOne)
        .build()
        .await?;

    // Create separate pools for different priority groups
    let high_priority_pool = TestAgentPoolBuilder::new()
        .with_capacity(2)
        .build(&env.nats_url)
        .await?;

    let low_priority_pool = TestAgentPoolBuilder::new()
        .with_capacity(3)
        .build(&env.nats_url)
        .await?;

    let high_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(high_priority_pool)),
    ));

    let low_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(low_priority_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("high-priority"),
        high_supervisor.clone(),
    ).await?;

    supervision_tree.register_supervisor(
        NodeId::from("low-priority"),
        low_supervisor.clone(),
    ).await?;

    // Spawn agents in both groups
    for i in 0..2 {
        let config = AgentConfig {
            id: AgentId::from(format!("high-priority-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["critical".to_string()],
            resource_limits: Default::default(),
        };
        high_supervisor.spawn_agent(config).await?;
    }

    for i in 0..3 {
        let config = AgentConfig {
            id: AgentId::from(format!("low-priority-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["background".to_string()],
            resource_limits: Default::default(),
        };
        low_supervisor.spawn_agent(config).await?;
    }

    // Verify tree structure
    let stats = supervision_tree.get_statistics().await?;
    assert!(stats.total_supervisors >= 3); // root + 2 groups
    assert!(stats.total_workers >= 5); // 2 high + 3 low

    Ok(())
}