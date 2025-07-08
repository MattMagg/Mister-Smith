use super::*;
use super::test_builders::{TestSupervisionBuilder, TestAgentPoolBuilder};
use mistersmith::supervision::AgentPoolSupervisor;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::Result;
use criterion::{black_box, Criterion};

#[tokio::test]
async fn test_large_scale_agent_spawning() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(100) // Large capacity
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("large-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Measure time to spawn 50 agents
    let start = Instant::now();
    let mut spawn_times = Vec::new();

    for i in 0..50 {
        let spawn_start = Instant::now();
        
        let config = AgentConfig {
            id: AgentId::from(format!("perf-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        };

        pool_supervisor.spawn_agent(config).await?;
        
        let spawn_time = spawn_start.elapsed();
        spawn_times.push(spawn_time);
    }

    let total_time = start.elapsed();

    // Calculate statistics
    let avg_spawn_time = spawn_times.iter()
        .map(|d| d.as_millis())
        .sum::<u128>() / spawn_times.len() as u128;

    let max_spawn_time = spawn_times.iter()
        .map(|d| d.as_millis())
        .max()
        .unwrap_or(0);

    println!("Performance: Large-scale agent spawning");
    println!("  Total agents spawned: 50");
    println!("  Total time: {:?}", total_time);
    println!("  Average spawn time: {}ms", avg_spawn_time);
    println!("  Max spawn time: {}ms", max_spawn_time);

    // Performance assertions
    assert!(avg_spawn_time < 100, "Average spawn time too high: {}ms", avg_spawn_time);
    assert!(max_spawn_time < 200, "Max spawn time too high: {}ms", max_spawn_time);
    assert!(total_time < Duration::from_secs(10), "Total spawn time too high: {:?}", total_time);

    Ok(())
}

#[tokio::test]
async fn test_supervision_overhead() -> Result<()> {
    let env = setup_test_env().await;
    
    // Test 1: Operations without supervision
    let transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    let plain_pool = AgentPool::new(10, transport.clone());
    
    let start_plain = Instant::now();
    for i in 0..10 {
        let agent = plain_pool.spawn_agent(AgentConfig {
            id: AgentId::from(format!("plain-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        }).await?;
        
        // Perform some operations
        agent.transition_to(AgentState::Processing).await?;
        agent.transition_to(AgentState::Idle).await?;
    }
    let plain_duration = start_plain.elapsed();

    // Test 2: Operations with supervision
    let supervision_tree = TestSupervisionBuilder::new()
        .build()
        .await?;

    let supervised_pool = TestAgentPoolBuilder::new()
        .with_capacity(10)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(supervised_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("supervised-pool"),
        pool_supervisor.clone(),
    ).await?;

    let start_supervised = Instant::now();
    for i in 0..10 {
        let agent = pool_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(format!("supervised-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        }).await?;
        
        // Same operations
        agent.transition_to(AgentState::Processing).await?;
        agent.transition_to(AgentState::Idle).await?;
    }
    let supervised_duration = start_supervised.elapsed();

    // Calculate overhead
    let overhead_ms = supervised_duration.as_millis() as i64 - plain_duration.as_millis() as i64;
    let overhead_percent = (overhead_ms as f64 / plain_duration.as_millis() as f64) * 100.0;

    println!("Performance: Supervision overhead");
    println!("  Without supervision: {:?}", plain_duration);
    println!("  With supervision: {:?}", supervised_duration);
    println!("  Overhead: {}ms ({:.1}%)", overhead_ms, overhead_percent);

    // Supervision should add minimal overhead
    assert!(overhead_percent < 20.0, "Supervision overhead too high: {:.1}%", overhead_percent);

    Ok(())
}

#[tokio::test]
async fn test_database_query_performance() -> Result<()> {
    let env = setup_test_env().await;
    
    // Setup database
    let db_pool = sqlx::PgPoolOptions::new()
        .max_connections(10)
        .connect(&env.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db_pool).await?;
    let persistence = Arc::new(SupervisionDb::new(db_pool));

    // Create test data
    let node_count = 100;
    for i in 0..node_count {
        persistence.create_node(
            &format!("node-{}", i),
            "worker",
            Some("root"),
        ).await?;
    }

    // Test 1: Query individual nodes
    let start_individual = Instant::now();
    for i in 0..50 {
        let _ = persistence.get_node(&format!("node-{}", i)).await?;
    }
    let individual_duration = start_individual.elapsed();
    let avg_query_time = individual_duration.as_millis() / 50;

    // Test 2: Query supervision tree
    let start_tree = Instant::now();
    let _ = persistence.get_supervision_tree().await?;
    let tree_duration = start_tree.elapsed();

    // Test 3: Query recent failures
    let start_failures = Instant::now();
    for i in 0..20 {
        let _ = persistence.get_recent_failures(
            &format!("node-{}", i),
            Duration::from_secs(3600),
        ).await?;
    }
    let failures_duration = start_failures.elapsed();

    println!("Performance: Database queries");
    println!("  Individual node queries: {}ms average", avg_query_time);
    println!("  Full tree query: {:?}", tree_duration);
    println!("  Failure history queries: {:?}", failures_duration);

    // Performance assertions
    assert!(avg_query_time < 10, "Node query too slow: {}ms", avg_query_time);
    assert!(tree_duration < Duration::from_millis(100), "Tree query too slow: {:?}", tree_duration);

    Ok(())
}

#[tokio::test]
async fn test_nats_throughput_with_supervision() -> Result<()> {
    let env = setup_test_env().await;
    
    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    
    // Setup event recorder
    let event_recorder = Arc::new(EventRecorder::new(nats_transport.clone()).await?);
    event_recorder.start_recording("supervision.perf.>").await?;

    // Generate high volume of events
    let event_count = 1000;
    let start = Instant::now();

    for i in 0..event_count {
        let event = SupervisionEvent::StateChanged {
            node_id: NodeId::from("perf-test"),
            child_id: ChildId::from(format!("child-{}", i)),
            old_state: ChildState::Running,
            new_state: ChildState::Processing,
            timestamp: chrono::Utc::now(),
        };

        nats_transport.publish(
            "supervision.perf.state",
            &serde_json::to_vec(&event)?,
        ).await?;
    }

    let publish_duration = start.elapsed();

    // Wait for all events to be recorded
    event_recorder.wait_for_events(event_count, Duration::from_secs(5)).await?;
    let receive_duration = start.elapsed();

    // Calculate throughput
    let publish_rate = event_count as f64 / publish_duration.as_secs_f64();
    let receive_rate = event_count as f64 / receive_duration.as_secs_f64();

    println!("Performance: NATS event throughput");
    println!("  Events published: {}", event_count);
    println!("  Publish time: {:?}", publish_duration);
    println!("  Publish rate: {:.0} events/sec", publish_rate);
    println!("  Receive rate: {:.0} events/sec", receive_rate);

    // Performance assertions
    assert!(publish_rate > 1000.0, "Publish rate too low: {:.0} events/sec", publish_rate);
    assert!(receive_rate > 500.0, "Receive rate too low: {:.0} events/sec", receive_rate);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_supervision_operations() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
        .build()
        .await?;

    // Create multiple pools
    let mut supervisors = Vec::new();
    for i in 0..5 {
        let pool = TestAgentPoolBuilder::new()
            .with_capacity(20)
            .build(&env.nats_url)
            .await?;

        let supervisor = Arc::new(AgentPoolSupervisor::new(
            Arc::new(tokio::sync::RwLock::new(pool)),
        ));

        supervision_tree.register_supervisor(
            NodeId::from(format!("pool-{}", i)),
            supervisor.clone(),
        ).await?;

        supervisors.push(supervisor);
    }

    // Spawn concurrent operations
    let start = Instant::now();
    let mut tasks = Vec::new();

    for (pool_idx, supervisor) in supervisors.iter().enumerate() {
        for agent_idx in 0..10 {
            let supervisor = supervisor.clone();
            let task = tokio::spawn(async move {
                let config = AgentConfig {
                    id: AgentId::from(format!("concurrent-{}-{}", pool_idx, agent_idx)),
                    agent_type: "claude-cli".to_string(),
                    capabilities: vec!["test".to_string()],
                    resource_limits: Default::default(),
                };

                let agent = supervisor.spawn_agent(config).await?;
                
                // Perform operations
                for _ in 0..5 {
                    agent.transition_to(AgentState::Processing).await?;
                    tokio::time::sleep(Duration::from_micros(100)).await;
                    agent.transition_to(AgentState::Idle).await?;
                }

                Ok::<(), anyhow::Error>(())
            });
            tasks.push(task);
        }
    }

    // Wait for all operations
    let results: Vec<Result<()>> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r?)
        .collect();

    let duration = start.elapsed();

    // Verify all succeeded
    for result in results {
        assert!(result.is_ok());
    }

    let total_operations = 5 * 10 * 5; // 5 pools * 10 agents * 5 transitions
    let ops_per_second = total_operations as f64 / duration.as_secs_f64();

    println!("Performance: Concurrent supervision operations");
    println!("  Total operations: {}", total_operations);
    println!("  Duration: {:?}", duration);
    println!("  Operations/sec: {:.0}", ops_per_second);

    // Should handle high concurrency efficiently
    assert!(ops_per_second > 1000.0, "Concurrent ops too slow: {:.0} ops/sec", ops_per_second);

    Ok(())
}

#[tokio::test]
async fn test_memory_usage_under_load() -> Result<()> {
    use sysinfo::{System, SystemExt, ProcessExt};
    
    let env = setup_test_env().await;
    
    // Get initial memory usage
    let mut system = System::new_all();
    system.refresh_all();
    let pid = sysinfo::get_current_pid().unwrap();
    let initial_memory = system.process(pid).unwrap().memory();

    // Create large supervision structure
    let supervision_tree = TestSupervisionBuilder::new()
        .build()
        .await?;

    let mut supervisors = Vec::new();
    
    // Create 10 pools with 50 agents each
    for pool_idx in 0..10 {
        let pool = TestAgentPoolBuilder::new()
            .with_capacity(50)
            .build(&env.nats_url)
            .await?;

        let supervisor = Arc::new(AgentPoolSupervisor::new(
            Arc::new(tokio::sync::RwLock::new(pool)),
        ));

        supervision_tree.register_supervisor(
            NodeId::from(format!("memory-pool-{}", pool_idx)),
            supervisor.clone(),
        ).await?;

        // Spawn agents
        for agent_idx in 0..50 {
            supervisor.spawn_agent(AgentConfig {
                id: AgentId::from(format!("mem-agent-{}-{}", pool_idx, agent_idx)),
                agent_type: "claude-cli".to_string(),
                capabilities: vec!["test".to_string()],
                resource_limits: Default::default(),
            }).await?;
        }

        supervisors.push(supervisor);
    }

    // Refresh memory stats
    system.refresh_all();
    let loaded_memory = system.process(pid).unwrap().memory();
    let memory_increase = loaded_memory - initial_memory;
    let memory_per_agent = memory_increase / 500; // 500 total agents

    println!("Performance: Memory usage");
    println!("  Initial memory: {} KB", initial_memory);
    println!("  Loaded memory: {} KB", loaded_memory);
    println!("  Memory increase: {} KB", memory_increase);
    println!("  Memory per agent: {} KB", memory_per_agent);

    // Memory usage should be reasonable
    assert!(memory_per_agent < 100, "Memory per agent too high: {} KB", memory_per_agent);

    Ok(())
}

#[tokio::test]
async fn test_restart_performance() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
        .with_restart_policy(RestartPolicy {
            max_restarts: 10,
            time_window: Duration::from_secs(60),
            backoff_strategy: BackoffStrategy::Fixed {
                delay: Duration::from_millis(10), // Fast restarts for testing
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
        NodeId::from("restart-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn agents
    let mut agent_ids = Vec::new();
    for i in 0..5 {
        let config = AgentConfig {
            id: AgentId::from(format!("restart-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        };
        let agent = pool_supervisor.spawn_agent(config).await?;
        agent_ids.push(agent.id().clone());
    }

    // Measure restart performance
    let start = Instant::now();
    let mut restart_times = Vec::new();

    for agent_id in &agent_ids {
        for _ in 0..3 {
            let restart_start = Instant::now();
            
            // Simulate failure
            supervision_tree.handle_child_failure(
                NodeId::from("restart-pool"),
                ChildId::from(agent_id.to_string()),
                FailureReason::ProcessCrash {
                    exit_code: -1,
                    message: "Performance test crash".to_string(),
                },
            ).await?;

            // Wait for restart
            tokio::time::sleep(Duration::from_millis(20)).await;
            
            let restart_time = restart_start.elapsed();
            restart_times.push(restart_time);
        }
    }

    let total_duration = start.elapsed();
    let avg_restart_time = restart_times.iter()
        .map(|d| d.as_millis())
        .sum::<u128>() / restart_times.len() as u128;

    println!("Performance: Agent restart");
    println!("  Total restarts: {}", restart_times.len());
    println!("  Total time: {:?}", total_duration);
    println!("  Average restart time: {}ms", avg_restart_time);

    // Restart should be fast
    assert!(avg_restart_time < 50, "Restart time too high: {}ms", avg_restart_time);

    Ok(())
}