use super::*;
use super::test_builders::{TestSupervisionBuilder, TestAgentPoolBuilder};
use mistersmith::supervision::{AgentPoolSupervisor, ChildState};
use mistersmith::agent::{AgentState, AgentId};
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;

#[tokio::test]
async fn test_spawn_multiple_agents_under_supervision() -> Result<()> {
    // Setup test environment
    let env = setup_test_env().await;
    
    // Build supervision tree
    let supervision_tree = TestSupervisionBuilder::new()
        .with_strategy(SupervisionStrategy::OneForOne)
        .build()
        .await?;

    // Build agent pool with capacity for 5 agents
    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(5)
        .build(&env.nats_url)
        .await?;

    // Create AgentPoolSupervisor to bridge pool and tree
    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    // Register pool supervisor with supervision tree
    supervision_tree.register_supervisor(
        NodeId::from("agent-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn 5 agents
    let mut agent_ids = Vec::new();
    for i in 0..5 {
        let agent_id = AgentId::from(format!("test-agent-{}", i));
        let config = AgentConfig {
            id: agent_id.clone(),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        };

        // Spawn agent through pool
        let agent = pool_supervisor.spawn_agent(config).await?;
        agent_ids.push(agent.id().clone());

        // Verify agent is registered with supervision
        let child_id = ChildId::from(agent_id.to_string());
        let child_ref = pool_supervisor.get_child(&child_id).await?;
        assert!(child_ref.is_some());
    }

    // Verify all agents are running
    let pool_status = pool_supervisor.get_pool_status().await?;
    assert_eq!(pool_status.active_agents, 5);
    assert_eq!(pool_status.total_capacity, 5);

    // Verify supervision tree knows about all agents
    let tree_stats = supervision_tree.get_statistics().await?;
    assert!(tree_stats.total_workers >= 5);

    // Verify state synchronization
    pool_supervisor.sync_states().await?;
    
    for agent_id in &agent_ids {
        let child_id = ChildId::from(agent_id.to_string());
        let child_state = pool_supervisor.get_child_state(&child_id).await?;
        assert_eq!(child_state, ChildState::Running);
    }

    Ok(())
}

#[tokio::test]
async fn test_pool_capacity_limits_respected() -> Result<()> {
    let env = setup_test_env().await;
    
    // Create pool with capacity 3
    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(3)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    // Successfully spawn 3 agents
    for i in 0..3 {
        let config = AgentConfig {
            id: AgentId::from(format!("agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        };
        pool_supervisor.spawn_agent(config).await?;
    }

    // Attempt to spawn 4th agent should fail
    let config = AgentConfig {
        id: AgentId::from("agent-overflow"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };
    
    let result = pool_supervisor.spawn_agent(config).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("capacity"));

    // Verify pool health reflects capacity limit
    let health = pool_supervisor.health_check().await?;
    match health {
        HealthResult::Unhealthy(reason) => {
            assert!(reason.contains("at capacity"));
        }
        _ => panic!("Expected unhealthy status at capacity"),
    }

    Ok(())
}

#[tokio::test]
async fn test_agent_lifecycle_under_supervision() -> Result<()> {
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

    // Spawn an agent
    let agent_id = AgentId::from("lifecycle-test-agent");
    let config = AgentConfig {
        id: agent_id.clone(),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };

    let agent = pool_supervisor.spawn_agent(config).await?;

    // Verify initial state
    let child_id = ChildId::from(agent_id.to_string());
    assert_eq!(
        pool_supervisor.get_child_state(&child_id).await?,
        ChildState::Running
    );

    // Transition agent through states
    agent.transition_to(AgentState::Processing).await?;
    pool_supervisor.sync_states().await?;
    assert_eq!(
        pool_supervisor.get_child_state(&child_id).await?,
        ChildState::Running // Processing maps to Running
    );

    // Pause agent
    agent.transition_to(AgentState::Paused).await?;
    pool_supervisor.sync_states().await?;
    assert_eq!(
        pool_supervisor.get_child_state(&child_id).await?,
        ChildState::Suspended
    );

    // Resume agent
    agent.transition_to(AgentState::Idle).await?;
    pool_supervisor.sync_states().await?;
    assert_eq!(
        pool_supervisor.get_child_state(&child_id).await?,
        ChildState::Running
    );

    // Terminate agent
    agent.terminate().await?;
    pool_supervisor.sync_states().await?;
    assert_eq!(
        pool_supervisor.get_child_state(&child_id).await?,
        ChildState::Stopped
    );

    // Verify supervision unregisters terminated agent
    let child_ref = pool_supervisor.get_child(&child_id).await?;
    assert!(child_ref.is_none());

    Ok(())
}

#[tokio::test]
async fn test_concurrent_agent_spawning() -> Result<()> {
    let env = setup_test_env().await;
    
    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(10)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    // Spawn 10 agents concurrently
    let mut spawn_tasks = Vec::new();
    
    for i in 0..10 {
        let supervisor = pool_supervisor.clone();
        let task = tokio::spawn(async move {
            let config = AgentConfig {
                id: AgentId::from(format!("concurrent-agent-{}", i)),
                agent_type: "claude-cli".to_string(),
                capabilities: vec!["test".to_string()],
                resource_limits: Default::default(),
            };
            supervisor.spawn_agent(config).await
        });
        spawn_tasks.push(task);
    }

    // Wait for all spawns to complete
    let results: Vec<Result<_>> = futures::future::join_all(spawn_tasks)
        .await
        .into_iter()
        .map(|r| r?)
        .collect();

    // Verify all spawns succeeded
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(result.is_ok());
    }

    // Verify pool status
    let status = pool_supervisor.get_pool_status().await?;
    assert_eq!(status.active_agents, 10);

    // Verify all agents are registered with supervision
    pool_supervisor.sync_states().await?;
    let children = pool_supervisor.get_all_children().await?;
    assert_eq!(children.len(), 10);

    Ok(())
}

#[tokio::test]
async fn test_agent_type_diversity() -> Result<()> {
    let env = setup_test_env().await;
    
    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(6)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    // Spawn different types of agents
    let agent_types = vec![
        ("research-agent", vec!["search", "analyze"]),
        ("code-agent", vec!["code", "review"]),
        ("comm-agent", vec!["email", "chat"]),
    ];

    for (i, (agent_type, capabilities)) in agent_types.iter().enumerate() {
        // Spawn 2 of each type
        for j in 0..2 {
            let config = AgentConfig {
                id: AgentId::from(format!("{}-{}", agent_type, j)),
                agent_type: agent_type.to_string(),
                capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
                resource_limits: Default::default(),
            };
            pool_supervisor.spawn_agent(config).await?;
        }
    }

    // Verify agent diversity
    let status = pool_supervisor.get_pool_status().await?;
    assert_eq!(status.active_agents, 6);

    // Verify agents can be queried by type
    let agents_by_type = pool_supervisor.get_agents_by_type().await?;
    assert_eq!(agents_by_type.len(), 3);
    
    for (agent_type, _) in agent_types {
        assert!(agents_by_type.contains_key(agent_type));
        assert_eq!(agents_by_type[agent_type].len(), 2);
    }

    Ok(())
}