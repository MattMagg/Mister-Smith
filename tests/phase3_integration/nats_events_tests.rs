use super::*;
use super::test_builders::{TestSupervisionBuilder, TestAgentPoolBuilder};
use super::event_recorder::EventRecorder;
use mistersmith::supervision::{AgentPoolSupervisor, SupervisionEvent, NodeId, ChildId};
use mistersmith::transport::nats::NatsTransport;
use mistersmith::agent::{AgentId, AgentState};
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use futures::StreamExt;

#[tokio::test]
async fn test_supervision_event_publishing() -> Result<()> {
    let env = setup_test_env().await;
    
    // Create NATS transport and event recorder
    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    let event_recorder = Arc::new(EventRecorder::new(nats_transport.clone()).await?);

    // Build supervision tree with NATS events enabled
    let supervision_tree = TestSupervisionBuilder::new()
        .with_event_publisher(nats_transport.clone())
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

    // Start recording events
    event_recorder.start_recording("supervision.>").await?;

    // Spawn an agent - should generate events
    let config = AgentConfig {
        id: AgentId::from("event-test-agent"),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };

    let agent = pool_supervisor.spawn_agent(config).await?;

    // Trigger state changes
    agent.transition_to(AgentState::Processing).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    agent.transition_to(AgentState::Idle).await?;

    // Wait for events to propagate
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify events were published
    let events = event_recorder.get_recorded_events().await;
    
    // Should have at least: node_added, state_changed (multiple)
    assert!(events.len() >= 3, "Expected at least 3 events, got {}", events.len());

    // Verify event types
    let event_types: Vec<String> = events.iter()
        .map(|e| e.event_type.clone())
        .collect();

    assert!(event_types.contains(&"node_added".to_string()));
    assert!(event_types.contains(&"state_changed".to_string()));

    // Verify event ordering
    let timestamps: Vec<_> = events.iter()
        .map(|e| e.timestamp)
        .collect();
    
    for i in 1..timestamps.len() {
        assert!(timestamps[i] >= timestamps[i-1], "Events out of order");
    }

    Ok(())
}

#[tokio::test]
async fn test_multi_subscriber_event_handling() -> Result<()> {
    let env = setup_test_env().await;
    
    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    
    // Create multiple subscribers for the same events
    let mut subscriber1 = nats_transport.subscribe("supervision.failures").await?;
    let mut subscriber2 = nats_transport.subscribe("supervision.failures").await?;
    let mut subscriber3 = nats_transport.subscribe("supervision.failures").await?;

    let supervision_tree = TestSupervisionBuilder::new()
        .with_event_publisher(nats_transport.clone())
        .build()
        .await?;

    // Publish a failure event
    let failure_event = SupervisionEvent::NodeFailure {
        node_id: NodeId::from("test-node"),
        child_id: ChildId::from("test-child"),
        reason: FailureReason::ProcessCrash {
            exit_code: -1,
            message: "Test crash".to_string(),
        },
        timestamp: chrono::Utc::now(),
    };

    supervision_tree.publish_event(failure_event).await?;

    // Verify all subscribers receive the event
    let timeout = Duration::from_secs(1);
    
    let event1 = tokio::time::timeout(timeout, subscriber1.next()).await?;
    let event2 = tokio::time::timeout(timeout, subscriber2.next()).await?;
    let event3 = tokio::time::timeout(timeout, subscriber3.next()).await?;

    assert!(event1.is_some());
    assert!(event2.is_some());
    assert!(event3.is_some());

    // Verify all received the same event
    let msg1 = String::from_utf8(event1.unwrap())?;
    let msg2 = String::from_utf8(event2.unwrap())?;
    let msg3 = String::from_utf8(event3.unwrap())?;

    assert_eq!(msg1, msg2);
    assert_eq!(msg2, msg3);
    assert!(msg1.contains("ProcessCrash"));

    Ok(())
}

#[tokio::test]
async fn test_event_ordering_guarantees() -> Result<()> {
    let env = setup_test_env().await;
    
    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    let event_recorder = Arc::new(EventRecorder::new(nats_transport.clone()).await?);

    let supervision_tree = TestSupervisionBuilder::new()
        .with_event_publisher(nats_transport.clone())
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

    // Start recording
    event_recorder.start_recording("supervision.>").await?;

    // Generate ordered events rapidly
    let agent_id = AgentId::from("ordering-test");
    let config = AgentConfig {
        id: agent_id.clone(),
        agent_type: "claude-cli".to_string(),
        capabilities: vec!["test".to_string()],
        resource_limits: Default::default(),
    };

    let agent = pool_supervisor.spawn_agent(config).await?;

    // Rapid state transitions
    let states = vec![
        AgentState::Created,
        AgentState::Starting,
        AgentState::Running,
        AgentState::Processing,
        AgentState::Idle,
        AgentState::Paused,
        AgentState::Resuming,
        AgentState::Running,
        AgentState::Terminating,
        AgentState::Terminated,
    ];

    for state in states {
        agent.transition_to(state).await?;
        // Minimal delay to stress test ordering
        tokio::time::sleep(Duration::from_micros(100)).await;
    }

    // Wait for all events
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify event ordering
    let events = event_recorder.get_recorded_events().await;
    let state_events: Vec<_> = events.iter()
        .filter(|e| e.event_type == "state_changed")
        .collect();

    // Should have events for each state transition
    assert!(state_events.len() >= 9, "Expected at least 9 state events, got {}", state_events.len());

    // Verify timestamps are ordered
    for i in 1..state_events.len() {
        assert!(
            state_events[i].timestamp >= state_events[i-1].timestamp,
            "State events out of order at index {}", i
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_supervision_event_filtering() -> Result<()> {
    let env = setup_test_env().await;
    
    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    
    // Subscribe to specific event types
    let mut failure_sub = nats_transport.subscribe("supervision.failures.*").await?;
    let mut restart_sub = nats_transport.subscribe("supervision.restarts.*").await?;
    let mut state_sub = nats_transport.subscribe("supervision.states.*").await?;

    let supervision_tree = TestSupervisionBuilder::new()
        .with_event_publisher(nats_transport.clone())
        .build()
        .await?;

    // Publish different event types
    let events = vec![
        ("supervision.failures.crash", SupervisionEvent::NodeFailure {
            node_id: NodeId::from("node1"),
            child_id: ChildId::from("child1"),
            reason: FailureReason::ProcessCrash {
                exit_code: -1,
                message: "Crash".to_string(),
            },
            timestamp: chrono::Utc::now(),
        }),
        ("supervision.restarts.initiated", SupervisionEvent::RestartInitiated {
            node_id: NodeId::from("node1"),
            child_id: ChildId::from("child1"),
            delay: Duration::from_millis(100),
            attempt: 1,
            timestamp: chrono::Utc::now(),
        }),
        ("supervision.states.changed", SupervisionEvent::StateChanged {
            node_id: NodeId::from("node1"),
            child_id: ChildId::from("child1"),
            old_state: ChildState::Running,
            new_state: ChildState::Restarting,
            timestamp: chrono::Utc::now(),
        }),
    ];

    for (subject, event) in events {
        nats_transport.publish(
            subject,
            &serde_json::to_vec(&event)?,
        ).await?;
    }

    // Verify subscribers only receive their filtered events
    let timeout = Duration::from_millis(500);

    // Failure subscriber should get failure event
    let failure_event = tokio::time::timeout(timeout, failure_sub.next()).await?;
    assert!(failure_event.is_some());
    assert!(String::from_utf8(failure_event.unwrap())?.contains("Crash"));

    // Restart subscriber should get restart event
    let restart_event = tokio::time::timeout(timeout, restart_sub.next()).await?;
    assert!(restart_event.is_some());
    assert!(String::from_utf8(restart_event.unwrap())?.contains("RestartInitiated"));

    // State subscriber should get state event
    let state_event = tokio::time::timeout(timeout, state_sub.next()).await?;
    assert!(state_event.is_some());
    assert!(String::from_utf8(state_event.unwrap())?.contains("StateChanged"));

    // Verify no cross-contamination (shouldn't receive more events)
    let no_more_failure = tokio::time::timeout(
        Duration::from_millis(100),
        failure_sub.next()
    ).await;
    assert!(no_more_failure.is_err()); // Timeout expected

    Ok(())
}

#[tokio::test]
async fn test_event_persistence_and_replay() -> Result<()> {
    let env = setup_test_env().await;
    
    // Setup database for event persistence
    let db_pool = sqlx::PgPoolOptions::new()
        .max_connections(5)
        .connect(&env.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db_pool).await?;
    let persistence = Arc::new(SupervisionDb::new(db_pool));

    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    
    let supervision_tree = TestSupervisionBuilder::new()
        .with_event_publisher(nats_transport.clone())
        .with_event_persistence(persistence.clone())
        .build()
        .await?;

    // Generate events
    let events_to_persist = vec![
        SupervisionEvent::NodeAdded {
            parent_id: NodeId::from("root"),
            child_id: ChildId::from("child1"),
            node_type: NodeType::Worker,
            timestamp: chrono::Utc::now(),
        },
        SupervisionEvent::StateChanged {
            node_id: NodeId::from("root"),
            child_id: ChildId::from("child1"),
            old_state: ChildState::Created,
            new_state: ChildState::Running,
            timestamp: chrono::Utc::now(),
        },
        SupervisionEvent::NodeFailure {
            node_id: NodeId::from("root"),
            child_id: ChildId::from("child1"),
            reason: FailureReason::ProcessCrash {
                exit_code: -1,
                message: "Test crash".to_string(),
            },
            timestamp: chrono::Utc::now(),
        },
    ];

    // Publish and persist events
    for event in &events_to_persist {
        supervision_tree.publish_event(event.clone()).await?;
    }

    // Wait for persistence
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Query persisted events
    let persisted_events = persistence.get_events_by_node(
        "child1",
        chrono::Utc::now() - chrono::Duration::seconds(60),
        chrono::Utc::now(),
    ).await?;

    assert_eq!(persisted_events.len(), 3);

    // Setup replay subscriber
    let mut replay_sub = nats_transport.subscribe("supervision.replay.>").await?;

    // Replay events
    for event in persisted_events {
        nats_transport.publish(
            &format!("supervision.replay.{}", event.event_type),
            &event.payload,
        ).await?;
    }

    // Verify replay
    let mut replayed_count = 0;
    while let Ok(Some(msg)) = tokio::time::timeout(
        Duration::from_millis(100),
        replay_sub.next()
    ).await {
        replayed_count += 1;
        let event_str = String::from_utf8(msg)?;
        
        // Verify it's one of our original events
        assert!(
            event_str.contains("NodeAdded") ||
            event_str.contains("StateChanged") ||
            event_str.contains("NodeFailure")
        );
    }

    assert_eq!(replayed_count, 3);

    Ok(())
}

#[tokio::test]
async fn test_event_aggregation_and_metrics() -> Result<()> {
    let env = setup_test_env().await;
    
    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    let event_recorder = Arc::new(EventRecorder::new(nats_transport.clone()).await?);

    let supervision_tree = TestSupervisionBuilder::new()
        .with_event_publisher(nats_transport.clone())
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
        NodeId::from("agent-pool"),
        pool_supervisor.clone(),
    ).await?;

    // Start recording all events
    event_recorder.start_recording("supervision.>").await?;

    // Generate activity from multiple agents
    let mut agents = Vec::new();
    for i in 0..5 {
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
        
        // Simulate some failures
        if i % 2 == 0 {
            supervision_tree.handle_child_failure(
                NodeId::from("agent-pool"),
                ChildId::from(agent.id().to_string()),
                FailureReason::TaskExecutionError {
                    task_id: format!("task-{}", i),
                    error: "Simulated error".to_string(),
                },
            ).await?;
        }
        
        agent.transition_to(AgentState::Idle).await?;
    }

    // Wait for events
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Aggregate events
    let events = event_recorder.get_recorded_events().await;
    let aggregated_metrics = event_recorder.aggregate_metrics(&events);

    // Verify metrics
    assert!(aggregated_metrics.total_events >= 15); // Multiple events per agent
    assert!(aggregated_metrics.events_by_type.contains_key("state_changed"));
    assert!(aggregated_metrics.events_by_type.contains_key("node_failure"));
    assert_eq!(aggregated_metrics.unique_nodes, 5);
    assert!(aggregated_metrics.failure_rate > 0.0);
    assert!(aggregated_metrics.events_per_second > 0.0);

    Ok(())
}

#[tokio::test]
async fn test_event_backpressure_handling() -> Result<()> {
    let env = setup_test_env().await;
    
    let nats_transport = Arc::new(NatsTransport::new(&env.nats_url).await?);
    
    // Create slow subscriber
    let mut slow_subscriber = nats_transport.subscribe("supervision.flood").await?;

    // Flood with events
    let flood_count = 1000;
    for i in 0..flood_count {
        let event = SupervisionEvent::StateChanged {
            node_id: NodeId::from("flood-test"),
            child_id: ChildId::from(format!("child-{}", i)),
            old_state: ChildState::Running,
            new_state: ChildState::Processing,
            timestamp: chrono::Utc::now(),
        };

        nats_transport.publish(
            "supervision.flood",
            &serde_json::to_vec(&event)?,
        ).await?;
    }

    // Slow consumer
    let mut received = 0;
    let start = std::time::Instant::now();
    
    while received < flood_count && start.elapsed() < Duration::from_secs(5) {
        if let Ok(Some(_)) = tokio::time::timeout(
            Duration::from_millis(10),
            slow_subscriber.next()
        ).await {
            received += 1;
            
            // Simulate slow processing
            if received % 100 == 0 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }

    // Should receive all events despite slow consumption
    assert_eq!(received, flood_count);

    Ok(())
}