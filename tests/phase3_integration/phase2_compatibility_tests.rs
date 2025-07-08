use super::*;
use mistersmith::agent::{Agent, AgentId, AgentPool, AgentState};
use mistersmith::runtime::{ProcessManager, ClaudeExecutor};
use mistersmith::transport::nats::NatsTransport;
use mistersmith::message::{MessageRouter, MessageHandler};
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;

#[tokio::test]
async fn test_phase2_single_agent_mode() -> Result<()> {
    // Run without any supervision tree - pure Phase 2 mode
    let transport = Arc::new(NatsTransport::new("nats://localhost:4222").await?);
    let agent_pool = AgentPool::new(3, transport.clone());

    // Spawn single agent as in Phase 2
    let config = AgentConfig {
        id: AgentId::from("phase2-agent"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["general".to_string()],
        resource_limits: Default::default(),
    };

    let agent = agent_pool.spawn_agent(config).await?;

    // Verify agent lifecycle works
    assert_eq!(agent.state().await, AgentState::Running);

    agent.transition_to(AgentState::Processing).await?;
    assert_eq!(agent.state().await, AgentState::Processing);

    agent.transition_to(AgentState::Idle).await?;
    assert_eq!(agent.state().await, AgentState::Idle);

    agent.terminate().await?;
    assert_eq!(agent.state().await, AgentState::Terminated);

    // Verify pool tracking
    let status = agent_pool.get_pool_status().await;
    assert_eq!(status.active_agents, 0); // Agent terminated
    assert_eq!(status.total_capacity, 3);

    Ok(())
}

#[tokio::test]
async fn test_phase2_claude_integration() -> Result<()> {
    // Test Claude CLI integration without supervision
    let transport = Arc::new(NatsTransport::new("nats://localhost:4222").await?);
    let agent_pool = AgentPool::new(1, transport.clone());

    let config = AgentConfig {
        id: AgentId::from("claude-test-agent"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };

    // Use mock executor for testing (same as Phase 2)
    let mock_executor = Arc::new(MockClaudeExecutor::new());
    let agent = agent_pool.spawn_agent_with_executor(
        config,
        mock_executor.clone(),
    ).await?;

    // Execute task through agent
    let result = agent.execute_task("Test prompt for Phase 2").await?;
    assert_eq!(result, "Mock response from Claude");

    // Verify process management still works
    let process_manager = ProcessManager::new();
    let process_id = process_manager.track_process(
        "claude-cli",
        agent.id().to_string(),
    ).await?;

    assert!(process_manager.is_process_running(&process_id).await?);

    Ok(())
}

#[tokio::test]
async fn test_phase2_nats_messaging() -> Result<()> {
    // Test NATS messaging without supervision events
    let transport = Arc::new(NatsTransport::new("nats://localhost:4222").await?);

    // Setup message router as in Phase 2
    let mut router = MessageRouter::new();
    
    // Register handlers
    let handler_called = Arc::new(AtomicBool::new(false));
    let handler_flag = handler_called.clone();
    
    router.register_handler(
        "agent.task",
        Box::new(move |msg| {
            handler_flag.store(true, Ordering::SeqCst);
            Ok(())
        }),
    );

    // Subscribe to messages
    let mut subscriber = transport.subscribe("agent.>").await?;

    // Publish message
    let test_message = serde_json::json!({
        "type": "task",
        "agent_id": "test-agent",
        "payload": "test task"
    });

    transport.publish(
        "agent.task",
        &serde_json::to_vec(&test_message)?,
    ).await?;

    // Verify message received
    let received = tokio::time::timeout(
        Duration::from_secs(1),
        subscriber.next()
    ).await?;

    assert!(received.is_some());
    let msg_data = String::from_utf8(received.unwrap())?;
    assert!(msg_data.contains("test task"));

    Ok(())
}

#[tokio::test]
async fn test_phase2_agent_pool_limits() -> Result<()> {
    // Test agent pool capacity limits from Phase 2
    let transport = Arc::new(NatsTransport::new("nats://localhost:4222").await?);
    let agent_pool = AgentPool::new(2, transport.clone());

    // Spawn agents up to limit
    let agent1 = agent_pool.spawn_agent(AgentConfig {
        id: AgentId::from("pool-agent-1"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    }).await?;

    let agent2 = agent_pool.spawn_agent(AgentConfig {
        id: AgentId::from("pool-agent-2"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    }).await?;

    // Try to spawn beyond limit
    let result = agent_pool.spawn_agent(AgentConfig {
        id: AgentId::from("pool-agent-3"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    }).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("capacity"));

    // Terminate one agent
    agent1.terminate().await?;

    // Now spawning should work
    let agent3 = agent_pool.spawn_agent(AgentConfig {
        id: AgentId::from("pool-agent-3"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    }).await?;

    assert_eq!(agent3.state().await, AgentState::Running);

    Ok(())
}

#[tokio::test]
async fn test_phase2_state_transitions() -> Result<()> {
    // Verify all Phase 2 state transitions still work
    let transport = Arc::new(NatsTransport::new("nats://localhost:4222").await?);
    let agent_pool = AgentPool::new(1, transport.clone());

    let agent = agent_pool.spawn_agent(AgentConfig {
        id: AgentId::from("state-test-agent"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    }).await?;

    // Test all valid state transitions from Phase 2
    let transitions = vec![
        (AgentState::Created, AgentState::Starting),
        (AgentState::Starting, AgentState::Running),
        (AgentState::Running, AgentState::Processing),
        (AgentState::Processing, AgentState::Idle),
        (AgentState::Idle, AgentState::Processing),
        (AgentState::Processing, AgentState::Running),
        (AgentState::Running, AgentState::Paused),
        (AgentState::Paused, AgentState::Resuming),
        (AgentState::Resuming, AgentState::Running),
        (AgentState::Running, AgentState::Terminating),
        (AgentState::Terminating, AgentState::Terminated),
    ];

    // Start from Created state
    agent.force_state(AgentState::Created).await?;

    for (from_state, to_state) in transitions {
        // Ensure we're in the expected state
        if agent.state().await != from_state {
            agent.force_state(from_state).await?;
        }

        // Perform transition
        let result = agent.transition_to(to_state).await;
        assert!(
            result.is_ok(),
            "Failed to transition from {:?} to {:?}",
            from_state,
            to_state
        );
        assert_eq!(agent.state().await, to_state);
    }

    Ok(())
}

#[tokio::test]
async fn test_phase2_verification_suite() -> Result<()> {
    // Run the original Phase 2 verification tests
    let transport = Arc::new(NatsTransport::new("nats://localhost:4222").await?);
    let agent_pool = AgentPool::new(3, transport.clone());

    // Test 1: Agent creation and lifecycle
    println!("Phase 2 Test 1: Agent creation and lifecycle");
    let agent = agent_pool.spawn_agent(AgentConfig {
        id: AgentId::from("verification-agent-1"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    }).await?;
    
    assert_eq!(agent.state().await, AgentState::Running);
    agent.terminate().await?;
    assert_eq!(agent.state().await, AgentState::Terminated);
    println!("✅ Test 1 passed");

    // Test 2: Agent pool management
    println!("Phase 2 Test 2: Agent pool management");
    let status = agent_pool.get_pool_status().await;
    assert_eq!(status.active_agents, 0);
    assert_eq!(status.total_capacity, 3);
    println!("✅ Test 2 passed");

    // Test 3: State transitions
    println!("Phase 2 Test 3: State transitions");
    let agent2 = agent_pool.spawn_agent(AgentConfig {
        id: AgentId::from("verification-agent-2"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    }).await?;

    agent2.transition_to(AgentState::Processing).await?;
    assert_eq!(agent2.state().await, AgentState::Processing);
    agent2.transition_to(AgentState::Idle).await?;
    assert_eq!(agent2.state().await, AgentState::Idle);
    println!("✅ Test 3 passed");

    // Test 4: Claude execution (with mock)
    println!("Phase 2 Test 4: Claude execution");
    let mock_executor = Arc::new(MockClaudeExecutor::new());
    let agent3 = agent_pool.spawn_agent_with_executor(
        AgentConfig {
            id: AgentId::from("verification-agent-3"),
            agent_type: "claude-cli".to_string(),
            capabilities: vec!["test".to_string()],
            resource_limits: Default::default(),
        },
        mock_executor,
    ).await?;

    let result = agent3.execute_task("Phase 2 verification prompt").await?;
    assert_eq!(result, "Mock response from Claude");
    println!("✅ Test 4 passed");

    // Test 5: NATS communication
    println!("Phase 2 Test 5: NATS communication");
    let test_subject = "phase2.verification.test";
    let test_message = b"Phase 2 NATS test message";
    
    let mut subscriber = transport.subscribe(test_subject).await?;
    transport.publish(test_subject, test_message).await?;
    
    let received = tokio::time::timeout(
        Duration::from_secs(1),
        subscriber.next()
    ).await?;
    
    assert!(received.is_some());
    assert_eq!(received.unwrap(), test_message);
    println!("✅ Test 5 passed");

    // Test 6: System health
    println!("Phase 2 Test 6: System health");
    let health = agent_pool.health_check().await;
    assert!(matches!(health, HealthStatus::Healthy));
    println!("✅ Test 6 passed");

    println!("\n✅ All Phase 2 verification tests passed!");

    Ok(())
}

#[tokio::test]
async fn test_phase2_api_compatibility() -> Result<()> {
    // Ensure all Phase 2 APIs still work
    let transport = Arc::new(NatsTransport::new("nats://localhost:4222").await?);
    
    // Test AgentPool API
    let pool = AgentPool::new(5, transport.clone());
    assert_eq!(pool.capacity(), 5);
    assert_eq!(pool.available_slots().await, 5);

    // Test Agent API
    let agent = pool.spawn_agent(AgentConfig {
        id: AgentId::from("api-test-agent"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    }).await?;

    // All Phase 2 methods should work
    assert_eq!(agent.id(), &AgentId::from("api-test-agent"));
    assert_eq!(agent.agent_type(), "claude-cli");
    assert!(agent.has_capability("test"));
    assert_eq!(agent.state().await, AgentState::Running);

    // Test ProcessManager API
    let process_manager = ProcessManager::new();
    let pid = process_manager.track_process(
        "test-process",
        "test-id".to_string(),
    ).await?;
    assert!(process_manager.is_process_running(&pid).await?);

    // Test MessageRouter API
    let mut router = MessageRouter::new();
    router.register_handler(
        "test.topic",
        Box::new(|_| Ok(())),
    );
    assert!(router.has_handler("test.topic"));

    Ok(())
}

#[tokio::test]
async fn test_phase3_doesnt_break_phase2() -> Result<()> {
    // Verify Phase 3 additions don't interfere with Phase 2
    let transport = Arc::new(NatsTransport::new("nats://localhost:4222").await?);
    
    // Create Phase 2 setup
    let agent_pool = AgentPool::new(3, transport.clone());
    
    // Spawn agent without any supervision
    let agent = agent_pool.spawn_agent(AgentConfig {
        id: AgentId::from("no-supervision-agent"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    }).await?;

    // Verify no supervision events are published
    let mut event_subscriber = transport.subscribe("supervision.>").await?;
    
    // Perform operations
    agent.transition_to(AgentState::Processing).await?;
    agent.transition_to(AgentState::Idle).await?;
    agent.terminate().await?;

    // Should not receive any supervision events
    let event = tokio::time::timeout(
        Duration::from_millis(100),
        event_subscriber.next()
    ).await;
    
    assert!(event.is_err()); // Timeout expected - no events

    // Pool should still track agents correctly
    let status = agent_pool.get_pool_status().await;
    assert_eq!(status.active_agents, 0);

    Ok(())
}