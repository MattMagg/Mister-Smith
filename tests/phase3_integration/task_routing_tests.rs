use super::*;
use super::test_builders::{TestSupervisionBuilder, TestAgentPoolBuilder};
use mistersmith::supervision::{AgentPoolSupervisor, TaskRouter, NodeType};
use mistersmith::agent::{AgentId, AgentCapabilities};
use mistersmith::task::{Task, TaskType, TaskPriority, TaskStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use std::collections::HashMap;
use anyhow::Result;

#[tokio::test]
async fn test_hub_and_spoke_task_routing() -> Result<()> {
    let env = setup_test_env().await;
    
    // Build supervision tree with hub-and-spoke routing
    let supervision_tree = TestSupervisionBuilder::new()
        .with_task_router(TaskRouter::HubAndSpoke)
        .build()
        .await?;

    // Create agent pools for different spoke types
    let research_pool = TestAgentPoolBuilder::new()
        .with_capacity(3)
        .build(&env.nats_url)
        .await?;

    let code_pool = TestAgentPoolBuilder::new()
        .with_capacity(3)
        .build(&env.nats_url)
        .await?;

    let analysis_pool = TestAgentPoolBuilder::new()
        .with_capacity(2)
        .build(&env.nats_url)
        .await?;

    // Create supervisors for each spoke
    let research_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(research_pool)),
    ));

    let code_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(code_pool)),
    ));

    let analysis_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(analysis_pool)),
    ));

    // Register spokes with supervision tree
    supervision_tree.register_spoke(
        NodeId::from("research-spoke"),
        research_supervisor.clone(),
        TaskType::Research,
    ).await?;

    supervision_tree.register_spoke(
        NodeId::from("code-spoke"),
        code_supervisor.clone(),
        TaskType::Code,
    ).await?;

    supervision_tree.register_spoke(
        NodeId::from("analysis-spoke"),
        analysis_supervisor.clone(),
        TaskType::Analysis,
    ).await?;

    // Spawn specialized agents
    for i in 0..3 {
        research_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(format!("research-{}", i)),
            agent_type: "research".to_string(),
            capabilities: vec!["search".to_string(), "summarize".to_string()],
            resource_limits: Default::default(),
        }).await?;

        code_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(format!("code-{}", i)),
            agent_type: "code".to_string(),
            capabilities: vec!["write".to_string(), "review".to_string()],
            resource_limits: Default::default(),
        }).await?;
    }

    for i in 0..2 {
        analysis_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(format!("analysis-{}", i)),
            agent_type: "analysis".to_string(),
            capabilities: vec!["analyze".to_string(), "report".to_string()],
            resource_limits: Default::default(),
        }).await?;
    }

    // Submit tasks of different types
    let tasks = vec![
        Task::new(TaskType::Research, "Find information about Rust async patterns"),
        Task::new(TaskType::Code, "Implement async file reader"),
        Task::new(TaskType::Analysis, "Analyze performance metrics"),
        Task::new(TaskType::Research, "Research supervision tree patterns"),
        Task::new(TaskType::Code, "Write unit tests for agent pool"),
    ];

    let mut task_ids = Vec::new();
    for task in tasks {
        let task_id = supervision_tree.submit_task(task).await?;
        task_ids.push(task_id);
    }

    // Verify tasks are routed to correct spokes
    tokio::time::sleep(Duration::from_millis(200)).await;

    let research_tasks = research_supervisor.get_assigned_tasks().await?;
    let code_tasks = code_supervisor.get_assigned_tasks().await?;
    let analysis_tasks = analysis_supervisor.get_assigned_tasks().await?;

    assert_eq!(research_tasks.len(), 2);
    assert_eq!(code_tasks.len(), 2);
    assert_eq!(analysis_tasks.len(), 1);

    // Verify task types match spoke specialization
    for task in research_tasks {
        assert_eq!(task.task_type, TaskType::Research);
    }

    for task in code_tasks {
        assert_eq!(task.task_type, TaskType::Code);
    }

    for task in analysis_tasks {
        assert_eq!(task.task_type, TaskType::Analysis);
    }

    Ok(())
}

#[tokio::test]
async fn test_load_balancing_across_agents() -> Result<()> {
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
        NodeId::from("balanced-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn 5 identical agents
    let mut agent_task_counts = HashMap::new();
    for i in 0..5 {
        let agent_id = AgentId::from(format!("balanced-agent-{}", i));
        agent_task_counts.insert(agent_id.clone(), AtomicU32::new(0));
        
        pool_supervisor.spawn_agent(AgentConfig {
            id: agent_id,
            agent_type: "worker".to_string(),
            capabilities: vec!["general".to_string()],
            resource_limits: Default::default(),
        }).await?;
    }

    // Submit 100 tasks
    for i in 0..100 {
        let task = Task::new(
            TaskType::General,
            &format!("Balanced task {}", i),
        );
        
        let assigned_agent = pool_supervisor.route_task(task).await?;
        
        // Increment counter for assigned agent
        if let Some(counter) = agent_task_counts.get(&assigned_agent) {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    // Verify balanced distribution
    let counts: Vec<u32> = agent_task_counts.values()
        .map(|c| c.load(Ordering::SeqCst))
        .collect();

    let avg_count = 100.0 / 5.0; // 20 tasks per agent on average
    
    for count in counts {
        let deviation = (count as f64 - avg_count).abs() / avg_count;
        assert!(
            deviation < 0.2, // Allow 20% deviation
            "Unbalanced distribution: {} tasks (expected ~{})",
            count,
            avg_count
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_capability_based_task_routing() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
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
        NodeId::from("capability-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn agents with different capabilities
    let agent_capabilities = vec![
        ("python-expert", vec!["python", "debug", "test"]),
        ("rust-expert", vec!["rust", "async", "performance"]),
        ("frontend-expert", vec!["javascript", "react", "css"]),
        ("data-expert", vec!["sql", "analytics", "visualization"]),
        ("devops-expert", vec!["kubernetes", "docker", "ci-cd"]),
        ("generalist", vec!["python", "javascript", "sql"]),
    ];

    for (agent_type, capabilities) in &agent_capabilities {
        pool_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(agent_type.to_string()),
            agent_type: agent_type.to_string(),
            capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
            resource_limits: Default::default(),
        }).await?;
    }

    // Submit tasks requiring specific capabilities
    let tasks_with_requirements = vec![
        (Task::new(TaskType::Code, "Fix Python async bug"), vec!["python"]),
        (Task::new(TaskType::Code, "Optimize Rust performance"), vec!["rust", "performance"]),
        (Task::new(TaskType::Code, "Build React component"), vec!["javascript", "react"]),
        (Task::new(TaskType::Analysis, "Query database performance"), vec!["sql"]),
        (Task::new(TaskType::Operations, "Setup K8s deployment"), vec!["kubernetes"]),
    ];

    for (task, required_capabilities) in tasks_with_requirements {
        let assigned_agent = pool_supervisor.route_task_with_requirements(
            task,
            &required_capabilities,
        ).await?;

        // Verify assigned agent has required capabilities
        let agent_caps = pool_supervisor.get_agent_capabilities(&assigned_agent).await?;
        
        for required in required_capabilities {
            assert!(
                agent_caps.contains(&required.to_string()),
                "Agent {} missing required capability: {}",
                assigned_agent,
                required
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_priority_based_task_scheduling() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
        .build()
        .await?;

    let agent_pool = TestAgentPoolBuilder::new()
        .with_capacity(2) // Limited capacity to test queueing
        .build(&env.nats_url)
        .await?;

    let pool_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(agent_pool)),
    ));

    supervision_tree.register_supervisor(
        NodeId::from("priority-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn 2 agents
    for i in 0..2 {
        pool_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(format!("priority-agent-{}", i)),
            agent_type: "worker".to_string(),
            capabilities: vec!["general".to_string()],
            resource_limits: Default::default(),
        }).await?;
    }

    // Submit tasks with different priorities
    let tasks = vec![
        Task::with_priority(TaskType::General, "Low priority task 1", TaskPriority::Low),
        Task::with_priority(TaskType::General, "High priority task 1", TaskPriority::High),
        Task::with_priority(TaskType::General, "Critical task", TaskPriority::Critical),
        Task::with_priority(TaskType::General, "Normal task", TaskPriority::Normal),
        Task::with_priority(TaskType::General, "High priority task 2", TaskPriority::High),
        Task::with_priority(TaskType::General, "Low priority task 2", TaskPriority::Low),
    ];

    let mut task_ids = Vec::new();
    for task in tasks {
        let task_id = pool_supervisor.submit_task(task).await?;
        task_ids.push(task_id);
    }

    // Track execution order
    let execution_order = Arc::new(RwLock::new(Vec::new()));
    let order_clone = execution_order.clone();

    // Set up task execution tracking
    pool_supervisor.set_task_completion_callback(move |task_id| {
        let order = order_clone.clone();
        tokio::spawn(async move {
            order.write().await.push(task_id);
        });
    }).await;

    // Wait for all tasks to be processed
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify priority order
    let completed_order = execution_order.read().await;
    assert_eq!(completed_order.len(), 6);

    // Critical task should be first
    assert_eq!(completed_order[0], task_ids[2]);

    // High priority tasks should come before normal/low
    let high_priority_indices: Vec<usize> = completed_order.iter()
        .position(|id| id == &task_ids[1])
        .into_iter()
        .chain(completed_order.iter().position(|id| id == &task_ids[4]))
        .collect();

    let low_priority_indices: Vec<usize> = completed_order.iter()
        .position(|id| id == &task_ids[0])
        .into_iter()
        .chain(completed_order.iter().position(|id| id == &task_ids[5]))
        .collect();

    // All high priority tasks should come before low priority
    for high_idx in &high_priority_indices {
        for low_idx in &low_priority_indices {
            assert!(high_idx < low_idx);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_deadline_aware_scheduling() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
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
        NodeId::from("deadline-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn agents
    for i in 0..3 {
        pool_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(format!("deadline-agent-{}", i)),
            agent_type: "worker".to_string(),
            capabilities: vec!["general".to_string()],
            resource_limits: Default::default(),
        }).await?;
    }

    // Submit tasks with different deadlines
    let now = chrono::Utc::now();
    let tasks = vec![
        Task::with_deadline(
            TaskType::General,
            "Task with 1 hour deadline",
            now + chrono::Duration::hours(1),
        ),
        Task::with_deadline(
            TaskType::General,
            "Task with 10 minute deadline",
            now + chrono::Duration::minutes(10),
        ),
        Task::with_deadline(
            TaskType::General,
            "Task with 30 minute deadline",
            now + chrono::Duration::minutes(30),
        ),
        Task::with_deadline(
            TaskType::General,
            "Task with 5 minute deadline",
            now + chrono::Duration::minutes(5),
        ),
    ];

    let mut task_deadlines = HashMap::new();
    for task in tasks {
        let deadline = task.deadline.clone();
        let task_id = pool_supervisor.submit_task(task).await?;
        task_deadlines.insert(task_id, deadline);
    }

    // Get scheduled order
    let scheduled_tasks = pool_supervisor.get_scheduled_tasks().await?;

    // Verify tasks are scheduled by deadline (earliest first)
    let mut previous_deadline = None;
    for task in scheduled_tasks {
        if let Some(deadline) = task_deadlines.get(&task.id) {
            if let Some(prev) = previous_deadline {
                assert!(
                    deadline >= &prev,
                    "Tasks not scheduled by deadline order"
                );
            }
            previous_deadline = Some(deadline.clone());
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_task_retry_on_failure() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
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
        NodeId::from("retry-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Spawn agents with failure simulation
    let failure_count = Arc::new(AtomicU32::new(0));
    
    for i in 0..2 {
        let failure_counter = failure_count.clone();
        let mock_executor = MockClaudeExecutor::new()
            .with_controlled_failures(2, failure_counter); // Fail first 2 attempts
        
        pool_supervisor.spawn_agent_with_executor(
            AgentConfig {
                id: AgentId::from(format!("retry-agent-{}", i)),
                agent_type: "worker".to_string(),
                capabilities: vec!["general".to_string()],
                resource_limits: Default::default(),
            },
            Arc::new(mock_executor),
        ).await?;
    }

    // Submit task with retry policy
    let task = Task::with_retry_policy(
        TaskType::General,
        "Task that needs retries",
        3, // max retries
        Duration::from_millis(100), // retry delay
    );

    let task_id = pool_supervisor.submit_task(task).await?;

    // Wait for task completion
    let mut task_status = TaskStatus::Pending;
    let start = std::time::Instant::now();
    
    while task_status != TaskStatus::Completed && start.elapsed() < Duration::from_secs(5) {
        task_status = pool_supervisor.get_task_status(&task_id).await?;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Verify task eventually succeeded
    assert_eq!(task_status, TaskStatus::Completed);

    // Verify it took multiple attempts
    let attempts = failure_count.load(Ordering::SeqCst);
    assert_eq!(attempts, 2); // Failed twice, succeeded on third

    Ok(())
}

#[tokio::test]
async fn test_cross_spoke_task_delegation() -> Result<()> {
    let env = setup_test_env().await;
    
    let supervision_tree = TestSupervisionBuilder::new()
        .with_task_router(TaskRouter::HubAndSpoke)
        .enable_cross_spoke_delegation()
        .build()
        .await?;

    // Create specialized spokes
    let primary_pool = TestAgentPoolBuilder::new()
        .with_capacity(2)
        .build(&env.nats_url)
        .await?;

    let secondary_pool = TestAgentPoolBuilder::new()
        .with_capacity(3)
        .build(&env.nats_url)
        .await?;

    let primary_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(primary_pool)),
    ));

    let secondary_supervisor = Arc::new(AgentPoolSupervisor::new(
        Arc::new(tokio::sync::RwLock::new(secondary_pool)),
    ));

    supervision_tree.register_spoke(
        NodeId::from("primary-spoke"),
        primary_supervisor.clone(),
        TaskType::Primary,
    ).await?;

    supervision_tree.register_spoke(
        NodeId::from("secondary-spoke"), 
        secondary_supervisor.clone(),
        TaskType::Secondary,
    ).await?;

    // Fill primary spoke to capacity
    for i in 0..2 {
        primary_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(format!("primary-{}", i)),
            agent_type: "primary".to_string(),
            capabilities: vec!["primary-work".to_string()],
            resource_limits: Default::default(),
        }).await?;
    }

    // Add available capacity to secondary
    for i in 0..3 {
        secondary_supervisor.spawn_agent(AgentConfig {
            id: AgentId::from(format!("secondary-{}", i)),
            agent_type: "secondary".to_string(),
            capabilities: vec!["secondary-work".to_string(), "overflow".to_string()],
            resource_limits: Default::default(),
        }).await?;
    }

    // Submit more tasks than primary can handle
    let mut task_assignments = HashMap::new();
    
    for i in 0..5 {
        let task = Task::new(
            TaskType::Primary,
            &format!("Task {} (should overflow to secondary)", i),
        );
        
        let (task_id, assigned_spoke) = supervision_tree.submit_task_with_tracking(task).await?;
        task_assignments.insert(task_id, assigned_spoke);
    }

    // Verify overflow happened
    let primary_tasks = task_assignments.values()
        .filter(|spoke| *spoke == "primary-spoke")
        .count();
    
    let secondary_tasks = task_assignments.values()
        .filter(|spoke| *spoke == "secondary-spoke")
        .count();

    assert_eq!(primary_tasks, 2); // Primary at capacity
    assert_eq!(secondary_tasks, 3); // Overflow to secondary

    Ok(())
}