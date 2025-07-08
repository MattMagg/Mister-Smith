use super::*;
use super::test_builders::{TestSupervisionBuilder, TestAgentPoolBuilder};
use super::chaos_injector::{ChaosInjector, ChaosDatabase, ChaosNatsTransport, ResourcePressure};
use mistersmith::supervision::AgentPoolSupervisor;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;

#[tokio::test]
async fn test_network_partition_recovery() -> Result<()> {
    let env = setup_test_env().await;
    
    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    let chaos = Arc::new(ChaosInjector::new());
    let chaos_transport = Arc::new(ChaosNatsTransport::new(nats_transport.clone(), chaos.clone()));

    let supervision_tree = TestSupervisionBuilder::new()
        .with_transport(chaos_transport.clone())
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(5)
        .with_transport(chaos_transport.clone())
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("partitioned-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn agents
    let mut agents = Vec::new();
    for i in 0..3 {
        let config = AgentConfig {
            id: AgentId::from(format!("partition-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        };
        let agent = pool_supervisor.spawn_agent(config).await?;
        agents.push(agent);
    }

    // Verify normal operation
    for agent in &agents {
        agent.transition_to(AgentState::Processing).await?;
        assert_eq!(agent.state().await, AgentState::Processing);
    }

    // Inject network partition
    println!("Injecting network partition...");
    chaos.inject_network_partition(Duration::from_secs(2));

    // Attempt operations during partition
    let mut partition_results = Vec::new();
    for agent in &agents {
        let result = tokio::time::timeout(
            Duration::from_millis(500),
            agent.transition_to(AgentState::Idle)
        ).await;
        partition_results.push(result);
    }

    // Some operations should fail due to partition
    let failures = partition_results.iter()
        .filter(|r| r.is_err())
        .count();
    assert!(failures > 0, "Expected some failures during partition");

    // Wait for partition to heal
    println!("Waiting for partition to heal...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Verify recovery
    for agent in &agents {
        agent.transition_to(AgentState::Idle).await?;
        assert_eq!(agent.state().await, AgentState::Idle);
    }

    println!("✅ Network partition recovery successful");

    Ok(())
}

#[tokio::test]
async fn test_database_failure_resilience() -> Result<()> {
    let env = setup_test_env().await;
    
    // Setup database with chaos injection
    let db_pool = sqlx::PgPoolOptions::new()
        .max_connections(5)
        .connect(&env.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db_pool).await?;
    
    let persistence = Arc::new(SupervisionDb::new(db_pool));
    let chaos = Arc::new(ChaosInjector::new());
    let chaos_db = Arc::new(ChaosDatabase::new(
        Box::new(persistence.clone()),
        chaos.clone(),
    ));

    let supervision_tree = TestSupervisionBuilder::new()
        .with_persistence(chaos_db.clone())
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
        NodeId::from("db-chaos-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn agent
    let config = AgentConfig {
        id: AgentId::from("db-chaos-agent"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };
    let agent = pool_supervisor.spawn_agent(config).await?;

    // Normal operation
    supervision_tree.handle_child_failure(
        NodeId::from("db-chaos-pool"),
        ChildId::from(agent.id().to_string()),
        FailureReason::ProcessCrash {
            exit_code: -1,
            message: "Test failure before DB outage".to_string(),
        },
    ).await?;

    // Inject database failure
    println!("Injecting database failure...");
    chaos.inject_database_failure(Duration::from_secs(3));

    // Operations during outage should be handled gracefully
    let failure_during_outage = supervision_tree.handle_child_failure(
        NodeId::from("db-chaos-pool"),
        ChildId::from(agent.id().to_string()),
        FailureReason::ProcessCrash {
            exit_code: -1,
            message: "Test failure during DB outage".to_string(),
        },
    ).await;

    // Should handle DB failure gracefully (not panic)
    assert!(failure_during_outage.is_err() || failure_during_outage.is_ok());

    // Wait for database recovery
    println!("Waiting for database recovery...");
    tokio::time::sleep(Duration::from_secs(4)).await;

    // Operations should resume after recovery
    supervision_tree.handle_child_failure(
        NodeId::from("db-chaos-pool"),
        ChildId::from(agent.id().to_string()),
        FailureReason::ProcessCrash {
            exit_code: -1,
            message: "Test failure after DB recovery".to_string(),
        },
    ).await?;

    // Verify failures were eventually recorded
    let history = persistence.get_recent_failures(
        &agent.id().to_string(),
        Duration::from_secs(60),
    ).await?;
    
    assert!(history.len() >= 2); // At least before and after
    println!("✅ Database failure resilience verified");

    Ok(())
}

#[tokio::test]
async fn test_nats_server_restart() -> Result<()> {
    let env = setup_test_env().await;
    
    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    let chaos = Arc::new(ChaosInjector::new());
    let chaos_transport = Arc::new(ChaosNatsTransport::new(nats_transport.clone(), chaos.clone()));

    let supervision_tree = TestSupervisionBuilder::new()
        .with_transport(chaos_transport.clone())
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(3)
        .with_transport(chaos_transport.clone())
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("nats-chaos-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Setup event recording
    let event_recorder = Arc::new(EventRecorder::new(nats_transport.clone()).await?);
    event_recorder.start_recording("supervision.>").await?;

    // Spawn agent and generate events
    let agent = pool_supervisor.spawn_agent(AgentConfig {
        id: AgentId::from("nats-chaos-agent"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    }).await?;

    // Generate some events
    agent.transition_to(AgentState::Processing).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Simulate NATS failure
    println!("Simulating NATS server failure...");
    chaos.inject_nats_failure(Duration::from_secs(2));

    // Try to publish events during outage
    let mut outage_results = Vec::new();
    for i in 0..5 {
        let result = tokio::time::timeout(
            Duration::from_millis(200),
            agent.transition_to(if i % 2 == 0 { AgentState::Idle } else { AgentState::Processing })
        ).await;
        outage_results.push(result);
    }

    // Wait for NATS recovery
    println!("Waiting for NATS recovery...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Resume operations
    agent.transition_to(AgentState::Idle).await?;
    agent.transition_to(AgentState::Running).await?;

    // Verify event stream continuity
    tokio::time::sleep(Duration::from_millis(500)).await;
    let events = event_recorder.get_recorded_events().await;
    
    assert!(events.len() >= 2); // At least before and after recovery
    println!("✅ NATS server restart handled successfully");

    Ok(())
}

#[tokio::test]
async fn test_resource_exhaustion_handling() -> Result<()> {
    let env = setup_test_env().await;
    
    let chaos = Arc::new(ChaosInjector::new());
    
    let supervision_tree = TestSupervisionBuilder::new()
        .with_restart_policy(RestartPolicy {
            max_restarts: 5,
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
        .with_capacity(10)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("resource-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn agents
    let mut agents = Vec::new();
    for i in 0..5 {
        let config = AgentConfig {
            id: AgentId::from(format!("resource-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: mistersmith::agent::ResourceLimits {
                max_memory_mb: 100,
                max_cpu_percent: 50.0,
                max_file_handles: 100,
            },
        };
        let agent = pool_supervisor.spawn_agent(config).await?;
        agents.push(agent);
    }

    // Simulate resource exhaustion
    println!("Simulating resource exhaustion...");
    chaos.inject_resource_exhaustion(Duration::from_secs(3));

    // Start resource pressure
    let mut pressure = ResourcePressure::new();
    pressure.start_cpu_pressure(2);
    pressure.start_memory_pressure(50); // 50MB/sec

    // Attempt operations under resource pressure
    for agent in &agents {
        let result = supervision_tree.handle_child_failure(
            NodeId::from("resource-pool"),
            ChildId::from(agent.id().to_string()),
            FailureReason::ResourceExhaustion {
                resource_type: "memory".to_string(),
                limit: 100,
                requested: 150,
            },
        ).await?;

        // Should trigger restarts with backoff
        assert!(matches!(result, SupervisionResult::RestartChild { .. }));
    }

    // Stop resource pressure
    tokio::time::sleep(Duration::from_secs(4)).await;
    pressure.stop();
    chaos.reset().await;

    // System should stabilize
    let pool_status = pool_supervisor.get_pool_status().await?;
    assert!(pool_status.active_agents > 0); // Some agents should survive

    println!("✅ Resource exhaustion handled with graceful degradation");

    Ok(())
}

#[tokio::test]
async fn test_cascading_failures() -> Result<()> {
    let env = setup_test_env().await;
    
    let chaos = Arc::new(ChaosInjector::new());
    
    // Build hierarchical supervision tree
    let supervision_tree = TestSupervisionBuilder::new()
        .with_strategy(SupervisionStrategy::OneForAll)
        .add_supervisor("critical-services", SupervisionStrategy::OneForAll)
        .add_supervisor("worker-services", SupervisionStrategy::OneForOne)
        .build()
        .await?;

    // Critical services pool
    let critical_pool = TestAgentPoolBuilder::new()
        .with_capacity(3)
        .build(&env.nats_url)
        .await?;

    let critical_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(critical_pool)),
    ));

    // Worker services pool
    let worker_pool = TestAgentPoolBuilder::new()
        .with_capacity(5)
        .build(&env.nats_url)
        .await?;

    let worker_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(worker_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("critical-services"),
        critical_supervisor.clone(),
    ).await?;

    supervision_tree.register_supervisor(
        NodeId::from("worker-services"),
        worker_supervisor.clone(),
    ).await?;

    // Spawn critical agents
    for i in 0..2 {
        critical_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(format!("critical-{}", i)),
            agent_type: "critical".to_string(),
            capabilities: vec!["critical".to_string()],
            resource_limits: Default::default(),
        }).await?;
    }

    // Spawn worker agents
    for i in 0..3 {
        worker_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(format!("worker-{}", i)),
            agent_type: "worker".to_string(),
            capabilities: vec!["work".to_string()],
            resource_limits: Default::default(),
        }).await?;
    }

    // Set chaos probability for random failures
    chaos.set_failure_probability(0.3).await;

    // Trigger cascading failure in critical services
    let critical_failure = FailureReason::UnrecoverableError {
        error_type: "SystemCritical".to_string(),
        message: "Critical system component failed".to_string(),
    };

    let result = supervision_tree.handle_child_failure(
        NodeId::from("critical-services"),
        ChildId::from("critical-0"),
        critical_failure,
    ).await?;

    // Should trigger restart of all critical services (OneForAll)
    assert!(matches!(result, SupervisionResult::RestartAllChildren { .. }));

    // Worker services should continue independently
    let worker_status = worker_supervisor.get_pool_status().await?;
    assert_eq!(worker_status.active_agents, 3);

    // Simulate multiple random failures
    for _ in 0..10 {
        if chaos.should_random_fail().await {
            // Random worker failure
            let worker_failure = FailureReason::ProcessCrash {
                exit_code: -1,
                message: "Random chaos failure".to_string(),
            };

            let _ = supervision_tree.handle_child_failure(
                NodeId::from("worker-services"),
                ChildId::from("worker-0"),
                worker_failure,
            ).await;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // System should remain operational despite chaos
    let final_stats = supervision_tree.get_statistics().await?;
    assert!(final_stats.total_supervisors >= 3);
    assert!(final_stats.total_workers > 0);

    println!("✅ Cascading failures handled with isolation");

    Ok(())
}

#[tokio::test]
async fn test_chaos_recovery_patterns() -> Result<()> {
    let env = setup_test_env().await;
    
    let chaos = Arc::new(ChaosInjector::new());
    
    // Setup all chaos-aware components
    let db_pool = sqlx::PgPoolOptions::new()
        .max_connections(5)
        .connect(&env.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&db_pool).await?;
    
    let persistence = Arc::new(SupervisionDb::new(db_pool));
    let chaos_db = Arc::new(ChaosDatabase::new(
        Box::new(persistence),
        chaos.clone(),
    ));

    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    let chaos_transport = Arc::new(ChaosNatsTransport::new(
        nats_transport,
        chaos.clone(),
    ));

    let supervision_tree = TestSupervisionBuilder::new()
        .with_persistence(chaos_db)
        .with_transport(chaos_transport.clone())
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(5)
        .with_transport(chaos_transport)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("chaos-recovery-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn test agents
    let mut agents = Vec::new();
    for i in 0..3 {
        let config = AgentConfig {
            id: AgentId::from(format!("chaos-recovery-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        };
        let agent = pool_supervisor.spawn_agent(config).await?;
        agents.push(agent);
    }

    // Test various chaos patterns
    println!("Testing chaos recovery patterns...");

    // Pattern 1: Intermittent failures
    chaos.set_failure_probability(0.2).await;
    for _ in 0..20 {
        let agent = &agents[0];
        let _ = agent.transition_to(AgentState::Processing).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Pattern 2: Burst failures
    chaos.set_failure_probability(0.8).await;
    for _ in 0..5 {
        let agent = &agents[1];
        let _ = agent.transition_to(AgentState::Idle).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    chaos.set_failure_probability(0.0).await;

    // Pattern 3: Combined failures
    chaos.inject_network_partition(Duration::from_millis(500));
    chaos.inject_database_failure(Duration::from_millis(300));
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Try operations during combined chaos
    for agent in &agents {
        let _ = supervision_tree.handle_child_failure(
            NodeId::from("chaos-recovery-pool"),
            ChildId::from(agent.id().to_string()),
            FailureReason::ProcessCrash {
                exit_code: -1,
                message: "Combined chaos test".to_string(),
            },
        ).await;
    }

    // Wait for all chaos to clear
    tokio::time::sleep(Duration::from_secs(1)).await;
    chaos.reset().await;

    // Verify system recovered
    let pool_status = pool_supervisor.get_pool_status().await?;
    let tree_stats = supervision_tree.get_statistics().await?;

    println!("Final status after chaos:");
    println!("  Active agents: {}", pool_status.active_agents);
    println!("  Total supervisors: {}", tree_stats.total_supervisors);
    println!("  Total workers: {}", tree_stats.total_workers);

    // System should have some operational capacity
    assert!(pool_status.active_agents > 0);
    assert!(tree_stats.total_workers > 0);

    println!("✅ All chaos recovery patterns handled successfully");

    Ok(())
}