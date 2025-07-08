use super::*;
use super::test_builders::{TestSupervisionBuilder, TestAgentPoolBuilder};
use super::chaos_injector::ChaosInjector;
use mistersmith::persistence::{SupervisionPersistence, SupervisionDb, ConnectionManager};
use mistersmith::supervision::{AgentPoolSupervisor, NodeId, ChildId, NodeType};
use mistersmith::agent::AgentId;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn test_supervision_state_persistence() -> Result<()> {
    let env = setup_test_env().await;
    
    // Create database connection
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env.database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await?;

    let persistence = Arc::new(SupervisionDb::new(db_pool.clone()));

    // Build supervision tree with persistence
    let supervision_tree = TestSupervisionBuilder::new()
        .with_persistence(persistence.clone())
        .build()
        .await?;

    // Create hierarchical structure
    let root_id = NodeId::from("persistent-root");
    let group1_id = NodeId::from("group-1");
    let group2_id = NodeId::from("group-2");

    // Add supervisors
    supervision_tree.add_supervisor(
        root_id.clone(),
        group1_id.clone(),
        SupervisionStrategy::OneForOne,
        Default::default(),
    ).await?;

    supervision_tree.add_supervisor(
        root_id.clone(),
        group2_id.clone(),
        SupervisionStrategy::OneForAll,
        Default::default(),
    ).await?;

    // Add workers
    for i in 0..3 {
        supervision_tree.add_child(
            group1_id.clone(),
            ChildId::from(format!("worker-1-{}", i)),
            NodeType::Worker,
        ).await?;
    }

    for i in 0..2 {
        supervision_tree.add_child(
            group2_id.clone(),
            ChildId::from(format!("worker-2-{}", i)),
            NodeType::Worker,
        ).await?;
    }

    // Verify tree was persisted
    let stored_tree = persistence.get_supervision_tree().await?;
    
    // Check structure in JSON
    assert!(stored_tree["nodes"].as_array().unwrap().len() >= 8); // root + 2 groups + 5 workers
    
    // Simulate system restart - create new tree from persistence
    let new_tree = SupervisionTree::from_persistence(persistence.clone()).await?;
    
    // Verify structure was restored
    let stats = new_tree.get_statistics().await?;
    assert_eq!(stats.total_supervisors, 3);
    assert_eq!(stats.total_workers, 5);

    // Verify supervision strategies were preserved
    let group1_strategy = new_tree.get_supervisor_strategy(&group1_id).await?;
    assert_eq!(group1_strategy, SupervisionStrategy::OneForOne);

    let group2_strategy = new_tree.get_supervisor_strategy(&group2_id).await?;
    assert_eq!(group2_strategy, SupervisionStrategy::OneForAll);

    Ok(())
}

#[tokio::test]
async fn test_failure_history_tracking() -> Result<()> {
    let env = setup_test_env().await;
    
    // Setup database
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db_pool).await?;
    let persistence = Arc::new(SupervisionDb::new(db_pool));

    let supervision_tree = TestSupervisionBuilder::new()
        .with_persistence(persistence.clone())
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
    let agent_id = AgentId::from("failure-tracked-agent");
    let config = AgentConfig {
        id: agent_id.clone(),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };

    pool_supervisor.spawn_agent(config).await?;

    // Simulate multiple failures
    let child_id = ChildId::from(agent_id.to_string());
    let failures = vec![
        FailureReason::ProcessCrash {
            exit_code: -11,
            message: "Segmentation fault".to_string(),
        },
        FailureReason::TaskExecutionError {
            task_id: "task-123".to_string(),
            error: "Out of memory".to_string(),
        },
        FailureReason::ResourceExhaustion {
            resource_type: "cpu".to_string(),
            limit: 100,
            requested: 150,
        },
    ];

    // Record failures
    for (i, failure) in failures.iter().enumerate() {
        supervision_tree.handle_child_failure(
            NodeId::from("agent-pool"),
            child_id.clone(),
            failure.clone(),
        ).await?;

        // Add small delay between failures
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Query failure history
    let history = persistence.get_recent_failures(
        &child_id.to_string(),
        Duration::from_secs(60),
    ).await?;

    // Verify all failures were recorded
    assert_eq!(history.len(), 3);
    
    // Verify failure details
    assert!(history[0].message.contains("Segmentation fault"));
    assert!(history[1].message.contains("Out of memory"));
    assert!(history[2].message.contains("cpu"));

    // Verify timestamps are ordered
    for i in 1..history.len() {
        assert!(history[i].occurred_at >= history[i-1].occurred_at);
    }

    Ok(())
}

#[tokio::test]
async fn test_task_assignment_persistence() -> Result<()> {
    let env = setup_test_env().await;
    
    // Setup database
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db_pool).await?;
    let persistence = Arc::new(SupervisionDb::new(db_pool));

    // Create task definitions
    let task_types = vec![
        ("research", vec!["search", "analyze"]),
        ("code", vec!["write", "review"]),
        ("communication", vec!["email", "chat"]),
    ];

    for (task_type, capabilities) in &task_types {
        persistence.create_task_definition(
            task_type,
            &serde_json::json!({
                "capabilities": capabilities,
                "timeout": 300,
                "retries": 3,
            }),
        ).await?;
    }

    // Setup supervision with agents
    let supervision_tree = TestSupervisionBuilder::new()
        .with_persistence(persistence.clone())
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(6)
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("agent-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn specialized agents
    let mut agent_nodes = Vec::new();
    for (i, (task_type, capabilities)) in task_types.iter().enumerate() {
        for j in 0..2 {
            let agent_id = AgentId::from(format!("{}-agent-{}", task_type, j));
            let config = AgentConfig {
                id: agent_id.clone(),
                agent_type: task_type.to_string(),
                capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
                resource_limits: Default::default(),
            };
            
            pool_supervisor.spawn_agent(config).await?;
            
            // Register agent's node for task assignment
            let node_id = format!("node-{}-{}", task_type, j);
            persistence.create_node(&node_id, "worker", Some("agent-pool")).await?;
            agent_nodes.push((node_id, task_type.to_string()));
        }
    }

    // Create tasks
    let mut task_ids = Vec::new();
    for i in 0..10 {
        let task_type = &task_types[i % 3].0;
        let task_id = persistence.create_task(
            task_type,
            &serde_json::json!({
                "description": format!("Test {} task {}", task_type, i),
                "priority": i % 3 + 1,
            }),
            i as i32 % 3 + 1,
            Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        ).await?;
        task_ids.push((task_id, task_type.to_string()));
    }

    // Auto-assign tasks
    for (task_id, expected_type) in &task_ids {
        let assigned_node = persistence.auto_assign_task(task_id).await?;
        
        // Verify task was assigned to correct type of agent
        let node_type = agent_nodes.iter()
            .find(|(node_id, _)| node_id == &assigned_node)
            .map(|(_, node_type)| node_type.clone())
            .unwrap();
        
        assert_eq!(&node_type, expected_type);
    }

    // Verify task distribution
    for (node_id, _) in &agent_nodes {
        let node_tasks = persistence.get_tasks_by_node(node_id).await?;
        assert!(node_tasks.len() > 0, "Node {} has no tasks", node_id);
        assert!(node_tasks.len() <= 3, "Node {} overloaded with {} tasks", node_id, node_tasks.len());
    }

    // Update task statuses
    for (task_id, _) in &task_ids[0..5] {
        persistence.update_task_status(task_id, "completed").await?;
    }

    // Verify pending tasks
    let pending_tasks = persistence.get_pending_tasks().await?;
    assert_eq!(pending_tasks.len(), 5);

    Ok(())
}

#[tokio::test]
async fn test_database_recovery_after_outage() -> Result<()> {
    let env = setup_test_env().await;
    
    // Setup database with chaos injection
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db_pool).await?;
    
    let persistence = Arc::new(SupervisionDb::new(db_pool.clone()));
    let chaos = Arc::new(ChaosInjector::new());
    let chaos_db = Arc::new(super::chaos_injector::ChaosDatabase::new(
        Box::new(persistence.clone()),
        chaos.clone(),
    ));

    let supervision_tree = TestSupervisionBuilder::new()
        .with_persistence(chaos_db.clone())
        .build()
        .await?;

    // Create initial state
    let node_id = NodeId::from("db-recovery-test");
    supervision_tree.add_child(
        NodeId::from("root"),
        ChildId::from(node_id.to_string()),
        NodeType::Worker,
    ).await?;

    // Inject database failure
    chaos.inject_database_failure(Duration::from_secs(2));

    // Attempt operations during outage
    let failure_result = supervision_tree.handle_child_failure(
        NodeId::from("root"),
        ChildId::from(node_id.to_string()),
        FailureReason::ProcessCrash {
            exit_code: -1,
            message: "Test during outage".to_string(),
        },
    ).await;

    // Should fail due to database outage
    assert!(failure_result.is_err());

    // Wait for database to recover
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Retry operation after recovery
    let recovery_result = supervision_tree.handle_child_failure(
        NodeId::from("root"),
        ChildId::from(node_id.to_string()),
        FailureReason::ProcessCrash {
            exit_code: -1,
            message: "Test after recovery".to_string(),
        },
    ).await;

    // Should succeed after recovery
    assert!(recovery_result.is_ok());

    // Verify failure was recorded
    let history = persistence.get_recent_failures(
        &node_id.to_string(),
        Duration::from_secs(60),
    ).await?;
    
    assert!(history.len() >= 1);
    assert!(history.last().unwrap().message.contains("after recovery"));

    Ok(())
}

#[tokio::test]
async fn test_supervision_metrics_persistence() -> Result<()> {
    let env = setup_test_env().await;
    
    // Setup database
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db_pool).await?;
    let persistence = Arc::new(SupervisionDb::new(db_pool));

    let supervision_tree = TestSupervisionBuilder::new()
        .with_persistence(persistence.clone())
        .build()
        .await?;

    // Create agents and simulate activity
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

    // Spawn agents and create activity
    let mut agents = Vec::new();
    for i in 0..3 {
        let config = AgentConfig {
            id: AgentId::from(format!("metrics-agent-{}", i)),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        };
        let agent = pool_supervisor.spawn_agent(config).await?;
        agents.push(agent);
    }

    // Simulate various activities
    for (i, agent) in agents.iter().enumerate() {
        // State transitions
        agent.transition_to(AgentState::Processing).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        agent.transition_to(AgentState::Idle).await?;
        
        // Record some failures
        if i % 2 == 0 {
            persistence.record_failure(
                &agent.id().to_string(),
                "test_failure",
                "Simulated failure for metrics",
            ).await?;
        }
    }

    // Query aggregated metrics
    let metrics = persistence.get_supervision_metrics(
        Duration::from_secs(60),
    ).await?;

    // Verify metrics
    assert!(metrics.total_nodes >= 3);
    assert!(metrics.total_failures >= 1);
    assert!(metrics.average_restart_time.is_some());
    assert!(metrics.success_rate >= 0.0 && metrics.success_rate <= 1.0);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_persistence_operations() -> Result<()> {
    let env = setup_test_env().await;
    
    // Setup database
    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&env.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db_pool).await?;
    let persistence = Arc::new(SupervisionDb::new(db_pool));

    // Spawn multiple concurrent operations
    let mut tasks = Vec::new();
    
    for i in 0..10 {
        let persistence = persistence.clone();
        let task = tokio::spawn(async move {
            // Create node
            let node_id = format!("concurrent-node-{}", i);
            persistence.create_node(&node_id, "worker", Some("root")).await?;
            
            // Update status multiple times
            for j in 0..5 {
                let status = if j % 2 == 0 { "running" } else { "idle" };
                persistence.update_node_status(&node_id, status).await?;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            
            // Record some failures
            persistence.record_failure(
                &node_id,
                "concurrent_test",
                &format!("Concurrent test failure {}", i),
            ).await?;
            
            Ok::<(), anyhow::Error>(())
        });
        tasks.push(task);
    }

    // Wait for all operations to complete
    let results: Vec<Result<()>> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r?)
        .collect();

    // Verify all succeeded
    for result in results {
        assert!(result.is_ok());
    }

    // Verify final state
    let tree = persistence.get_supervision_tree().await?;
    let nodes = tree["nodes"].as_array().unwrap();
    assert!(nodes.len() >= 10);

    Ok(())
}