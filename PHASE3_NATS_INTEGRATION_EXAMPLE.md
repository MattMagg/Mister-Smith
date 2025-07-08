# Phase 3 NATS Integration Example

**Agent**: Agent-NATS  
**Date**: 2025-07-07  
**Purpose**: Practical integration example for Phase 3 advanced NATS patterns

---

## Integration with Existing Phase 2 Implementation

This document demonstrates how the advanced NATS patterns integrate with the current Phase 2 implementation, showing concrete code examples and migration strategies.

### Current Phase 2 Implementation Analysis

**Existing Structure** (`src/transport/nats.rs`):
- `NatsTransport` with basic client and router
- Two subscriptions: `agent.command`, `system.control`
- Basic `MessageRouter` with simple routing
- Automatic reconnection handling

**Enhancement Strategy**:
- Extend rather than replace existing implementation
- Maintain backward compatibility
- Add new capabilities incrementally

---

## 1. Extending Message Types

### Current Message Types (Phase 2)
```rust
// src/message/types.rs - Current implementation
pub enum AgentEvent {
    Spawned { agent_id: AgentId, config: AgentConfig },
    StateChanged { agent_id: AgentId, old_state: AgentState, new_state: AgentState },
    Terminated { agent_id: AgentId, reason: String },
    Error { agent_id: AgentId, error: String },
    // ... existing events
}
```

### Phase 3 Extension
```rust
// src/message/types.rs - Phase 3 additions
use crate::supervision::SupervisionEvent;
use crate::coordination::CoordinationMessage;

// Extend existing MessageEnvelope for supervision events
impl<T> MessageEnvelope<T> {
    /// Create supervision event envelope
    pub fn supervision_event(payload: T, severity: Option<FailureSeverity>) -> Self {
        let mut envelope = Self::new(payload);
        
        // Add supervision-specific metadata
        if let Some(severity) = severity {
            envelope.metadata.insert("severity".to_string(), 
                serde_json::to_value(severity).unwrap());
        }
        envelope.metadata.insert("event_category".to_string(), 
            serde_json::Value::String("supervision".to_string()));
            
        envelope
    }
    
    /// Create coordination message envelope
    pub fn coordination_message(payload: T, coordination_type: &str) -> Self {
        let mut envelope = Self::new(payload);
        envelope.metadata.insert("coordination_type".to_string(),
            serde_json::Value::String(coordination_type.to_string()));
        envelope
    }
}

// New message types for Phase 3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Phase3MessageType {
    // Wrap existing types for compatibility
    AgentCommand(AgentCommand),
    AgentEvent(AgentEvent),
    SystemMessage(SystemMessage),
    
    // New Phase 3 types
    SupervisionEvent(SupervisionEvent),
    CoordinationMessage(CoordinationMessage),
}
```

### Enhanced Subject Builder
```rust
// src/message/types.rs - Enhanced Subject implementation
impl Subject {
    // Existing methods remain unchanged for compatibility
    // ... existing implementations
    
    // New Phase 3 supervision subjects
    pub fn supervision_health_check(agent_id: &AgentId) -> String {
        format!("supervision.health.{}", agent_id.uuid())
    }
    
    pub fn supervision_failure(severity: &str) -> String {
        format!("supervision.failures.{}", severity)
    }
    
    pub fn supervision_recovery(agent_id: &AgentId) -> String {
        format!("supervision.recovery.{}", agent_id.uuid())
    }
    
    // Coordination subjects
    pub fn coordination_task_assignment(task_type: &str) -> String {
        format!("coordination.tasks.{}", task_type)
    }
    
    pub fn coordination_resource_request(resource_type: &str) -> String {
        format!("coordination.resources.{}", resource_type)
    }
    
    // Wildcard patterns for monitoring
    pub fn all_supervision_wildcards() -> Vec<&'static str> {
        vec![
            "supervision.>",           // All supervision events
            "supervision.health.*",    // All health events
            "supervision.failures.*",  // All failure events
            "supervision.recovery.*",  // All recovery events
        ]
    }
    
    pub fn all_coordination_wildcards() -> Vec<&'static str> {
        vec![
            "coordination.>",          // All coordination messages
            "coordination.tasks.*",    // All task coordination
            "coordination.resources.*", // All resource coordination
        ]
    }
}
```

---

## 2. Enhanced Transport Layer

### Backward-Compatible Transport Enhancement
```rust
// src/transport/enhanced_nats.rs - New file extending existing implementation
use crate::transport::nats::{NatsTransport, NatsConfig};
use crate::supervision::SupervisionEventProcessor;
use crate::coordination::CoordinationManager;

/// Enhanced NATS transport for Phase 3 with backward compatibility
pub struct EnhancedNatsTransport {
    // Wrap existing implementation
    base_transport: NatsTransport,
    
    // New Phase 3 components
    supervision_processor: Option<SupervisionEventProcessor>,
    coordination_manager: Option<CoordinationManager>,
    advanced_router: Option<AdvancedMessageRouter>,
    jetstream_manager: Option<JetStreamManager>,
}

impl EnhancedNatsTransport {
    /// Create enhanced transport with Phase 2 compatibility
    pub async fn new_with_phase2_compatibility(
        server_url: &str,
        config: NatsConfig,
    ) -> Result<Self, TransportError> {
        // Initialize Phase 2 transport first
        let base_transport = NatsTransport::with_config(server_url, config).await?;
        
        Ok(Self {
            base_transport,
            supervision_processor: None,
            coordination_manager: None,
            advanced_router: None,
            jetstream_manager: None,
        })
    }
    
    /// Enable Phase 3 supervision features
    pub async fn enable_supervision(&mut self) -> Result<(), TransportError> {
        if self.supervision_processor.is_none() {
            let processor = SupervisionEventProcessor::new(
                self.base_transport.client().clone()
            ).await?;
            
            // Set up supervision subscriptions
            self.setup_supervision_subscriptions().await?;
            
            self.supervision_processor = Some(processor);
        }
        Ok(())
    }
    
    /// Enable Phase 3 coordination features
    pub async fn enable_coordination(&mut self) -> Result<(), TransportError> {
        if self.coordination_manager.is_none() {
            let manager = CoordinationManager::new(
                self.base_transport.client().clone()
            ).await?;
            
            // Set up coordination subscriptions
            self.setup_coordination_subscriptions().await?;
            
            self.coordination_manager = Some(manager);
        }
        Ok(())
    }
    
    /// Enable advanced routing
    pub async fn enable_advanced_routing(&mut self, policies: RoutingPolicies) -> Result<(), TransportError> {
        if self.advanced_router.is_none() {
            let router = AdvancedMessageRouter::new(
                self.base_transport.router(),
                policies,
            ).await?;
            
            self.advanced_router = Some(router);
        }
        Ok(())
    }
    
    /// Set up supervision event subscriptions
    async fn setup_supervision_subscriptions(&self) -> Result<(), TransportError> {
        let router = self.base_transport.router();
        
        // Subscribe to all supervision events
        for pattern in Subject::all_supervision_wildcards() {
            router.subscribe(pattern, SupervisionEventHandler::new()).await?;
        }
        
        // Subscribe to specific critical failure events
        router.subscribe(
            Subject::supervision_failure("critical"),
            CriticalFailureHandler::new(),
        ).await?;
        
        Ok(())
    }
    
    /// Set up coordination subscriptions
    async fn setup_coordination_subscriptions(&self) -> Result<(), TransportError> {
        let router = self.base_transport.router();
        
        // Subscribe to coordination patterns
        for pattern in Subject::all_coordination_wildcards() {
            router.subscribe(pattern, CoordinationMessageHandler::new()).await?;
        }
        
        Ok(())
    }
    
    // Delegate Phase 2 methods to base transport for compatibility
    pub async fn setup_handlers(&self, agent_pool: Arc<AgentPool>) -> Result<(), TransportError> {
        self.base_transport.setup_handlers(agent_pool).await
    }
    
    pub fn client(&self) -> &async_nats::Client {
        self.base_transport.client()
    }
    
    pub fn router(&self) -> Arc<MessageRouter> {
        self.base_transport.router()
    }
}
```

---

## 3. Supervision Event Integration

### Event Processing Integration
```rust
// src/supervision/processor.rs - Integration with existing agent management
use crate::agent::{AgentPool, AgentId, AgentState};
use crate::message::{MessageEnvelope, Subject};

pub struct SupervisionEventProcessor {
    client: async_nats::Client,
    agent_pool: Arc<AgentPool>,
    pattern_detector: PatternDetector,
    alert_manager: AlertManager,
}

impl SupervisionEventProcessor {
    pub fn new(client: async_nats::Client, agent_pool: Arc<AgentPool>) -> Self {
        Self {
            client,
            agent_pool,
            pattern_detector: PatternDetector::new(),
            alert_manager: AlertManager::new(),
        }
    }
    
    /// Integrate with existing agent lifecycle events
    pub async fn handle_agent_state_change(
        &self,
        agent_id: &AgentId,
        old_state: &AgentState,
        new_state: &AgentState,
    ) -> Result<(), SupervisionError> {
        // Create supervision event from state change
        let supervision_event = match (old_state, new_state) {
            (AgentState::Running, AgentState::Terminated) => {
                Some(SupervisionEvent::Failure {
                    agent_id: *agent_id,
                    failure_type: FailureType::ProcessCrash,
                    severity: FailureSeverity::High,
                    context: FailureContext::StateTransition {
                        from: old_state.clone(),
                        to: new_state.clone(),
                    },
                    detection_time: SystemTime::now(),
                })
            }
            (_, AgentState::Running) => {
                Some(SupervisionEvent::Health {
                    agent_id: *agent_id,
                    status: HealthStatus::Healthy,
                    metrics: self.collect_agent_metrics(agent_id).await?,
                    timestamp: SystemTime::now(),
                })
            }
            _ => None,
        };
        
        // Publish supervision event if relevant
        if let Some(event) = supervision_event {
            self.publish_supervision_event(&event).await?;
            
            // Check for patterns
            if let Some(pattern) = self.pattern_detector.analyze_event(&event).await? {
                self.handle_detected_pattern(pattern).await?;
            }
        }
        
        Ok(())
    }
    
    /// Publish supervision event to NATS
    async fn publish_supervision_event(&self, event: &SupervisionEvent) -> Result<(), SupervisionError> {
        let subject = match event {
            SupervisionEvent::Health { agent_id, .. } => {
                Subject::supervision_health_check(agent_id)
            }
            SupervisionEvent::Failure { severity, .. } => {
                Subject::supervision_failure(&format!("{:?}", severity).to_lowercase())
            }
            SupervisionEvent::Recovery { agent_id, .. } => {
                Subject::supervision_recovery(agent_id)
            }
            _ => Subject::supervision_events().to_string(),
        };
        
        let envelope = MessageEnvelope::supervision_event(
            event.clone(),
            event.get_severity(),
        );
        
        let payload = serde_json::to_vec(&envelope)?;
        self.client.publish(subject, payload.into()).await?;
        
        Ok(())
    }
    
    /// Collect agent metrics for health events
    async fn collect_agent_metrics(&self, agent_id: &AgentId) -> Result<HealthMetrics, SupervisionError> {
        // Integration with existing agent pool to get metrics
        if let Some(agent) = self.agent_pool.get_agent(agent_id).await {
            // Collect metrics from the existing agent implementation
            Ok(HealthMetrics {
                cpu_usage: 0.0,  // Would integrate with actual process metrics
                memory_usage: 0.0,
                response_time_ms: 100,
                error_rate: 0.0,
                throughput: 1.0,
            })
        } else {
            Err(SupervisionError::AgentNotFound(*agent_id))
        }
    }
}
```

### Integration with Existing Agent Pool
```rust
// src/agent/pool.rs - Extend existing AgentPool with supervision hooks
use crate::supervision::SupervisionEventProcessor;

impl AgentPool {
    // Existing methods remain unchanged...
    
    /// Add supervision integration to existing spawn method
    pub async fn spawn_agent_with_supervision(
        &self,
        config: AgentConfig,
        supervision_processor: Option<&SupervisionEventProcessor>,
    ) -> Result<AgentId, AgentPoolError> {
        // Use existing spawn logic
        let agent_id = self.spawn_agent(config.clone()).await?;
        
        // Add supervision integration
        if let Some(processor) = supervision_processor {
            // Register agent for health monitoring
            processor.register_agent_for_monitoring(&agent_id).await?;
            
            // Publish agent spawned event
            processor.handle_agent_state_change(
                &agent_id,
                &AgentState::Created,
                &AgentState::Running,
            ).await?;
        }
        
        Ok(agent_id)
    }
    
    /// Enhanced stop method with supervision
    pub async fn stop_agent_with_supervision(
        &self,
        agent_id: &AgentId,
        supervision_processor: Option<&SupervisionEventProcessor>,
    ) -> Result<(), AgentPoolError> {
        // Get current state for supervision
        let current_state = self.get_agent_state(agent_id).await?;
        
        // Use existing stop logic
        self.stop_agent(agent_id).await?;
        
        // Add supervision integration
        if let Some(processor) = supervision_processor {
            processor.handle_agent_state_change(
                agent_id,
                &current_state,
                &AgentState::Terminated,
            ).await?;
        }
        
        Ok(())
    }
}
```

---

## 4. Practical Phase 3 Usage Example

### Complete Integration Example
```rust
// src/main.rs - Phase 3 enhanced main function
use crate::transport::EnhancedNatsTransport;
use crate::supervision::SupervisionEventProcessor;
use crate::coordination::CoordinationManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize enhanced NATS transport with Phase 2 compatibility
    let mut transport = EnhancedNatsTransport::new_with_phase2_compatibility(
        "localhost:4222",
        NatsConfig::default(),
    ).await?;
    
    // Initialize agent pool (existing Phase 2)
    let agent_pool = Arc::new(AgentPool::new(10)); // max 10 agents
    
    // Set up Phase 2 handlers (existing functionality)
    transport.setup_handlers(agent_pool.clone()).await?;
    
    // Enable Phase 3 features incrementally
    if std::env::var("ENABLE_SUPERVISION").unwrap_or_default() == "true" {
        transport.enable_supervision().await?;
        println!("✅ Phase 3 supervision features enabled");
    }
    
    if std::env::var("ENABLE_COORDINATION").unwrap_or_default() == "true" {
        transport.enable_coordination().await?;
        println!("✅ Phase 3 coordination features enabled");
    }
    
    if std::env::var("ENABLE_ADVANCED_ROUTING").unwrap_or_default() == "true" {
        let routing_policies = RoutingPolicies::default();
        transport.enable_advanced_routing(routing_policies).await?;
        println!("✅ Phase 3 advanced routing enabled");
    }
    
    // Phase 3 usage example - supervision events
    if let Some(supervision) = transport.supervision_processor() {
        // Spawn agent with supervision
        let agent_config = AgentConfig {
            agent_type: "claude".to_string(),
            max_memory_mb: 1024,
            timeout: Duration::from_secs(300),
            ..Default::default()
        };
        
        let agent_id = agent_pool.spawn_agent_with_supervision(
            agent_config,
            Some(supervision),
        ).await?;
        
        println!("🚀 Agent {} spawned with supervision", agent_id.uuid());
        
        // Example supervision event monitoring
        tokio::spawn(async move {
            supervision.start_monitoring().await.unwrap();
        });
    }
    
    // Phase 3 usage example - coordination
    if let Some(coordination) = transport.coordination_manager() {
        // Example task coordination
        let task = CoordinationMessage::TaskCoordination {
            task_id: Uuid::new_v4(),
            coordination_type: TaskCoordinationType::Broadcast,
            participants: vec![], // Will be filled by coordinator
            requirements: TaskRequirements {
                required_capabilities: vec!["code_analysis".to_string()],
                estimated_duration: Duration::from_secs(60),
                priority: TaskPriority::Normal,
            },
        };
        
        coordination.distribute_task(task).await?;
        println!("📋 Task distributed through coordination system");
    }
    
    // Run the system
    println!("🎯 MisterSmith Phase 3 system running...");
    
    // Wait for shutdown signal
    let shutdown_signal = Arc::new(AtomicBool::new(false));
    
    // Handle Ctrl+C
    let shutdown_clone = shutdown_signal.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        shutdown_clone.store(true, Ordering::Relaxed);
    });
    
    // Main event loop
    while !shutdown_signal.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    println!("🛑 Shutting down MisterSmith Phase 3 system...");
    transport.shutdown().await?;
    
    Ok(())
}
```

### Configuration for Phase 3
```toml
# Cargo.toml - Phase 3 dependencies
[dependencies]
# Existing Phase 2 dependencies remain unchanged
tokio = { version = "1.38", features = ["full"] }
async-nats = { version = "0.34", features = ["jetstream"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
tracing = "0.1"
anyhow = "1.0"
thiserror = "1.0"

# New Phase 3 dependencies
tokio-stream = "0.1"     # For event stream processing
dashmap = "5.0"          # For concurrent routing tables
lru = "0.12"             # For routing decision caching
moving_average = "4.0"   # For load balancing metrics
```

```rust
// Configuration structure for Phase 3 features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase3Config {
    // Phase 2 compatibility
    pub phase2_config: NatsConfig,
    
    // Phase 3 supervision features
    pub supervision: SupervisionConfig,
    
    // Phase 3 coordination features
    pub coordination: CoordinationConfig,
    
    // Advanced routing configuration
    pub routing: AdvancedRoutingConfig,
    
    // JetStream configuration
    pub jetstream: Option<JetStreamConfig>,
}

impl Default for Phase3Config {
    fn default() -> Self {
        Self {
            phase2_config: NatsConfig::default(),
            supervision: SupervisionConfig {
                enabled: false,
                health_check_interval: Duration::from_secs(30),
                failure_detection_threshold: 3,
                pattern_detection_enabled: true,
            },
            coordination: CoordinationConfig {
                enabled: false,
                task_distribution_strategy: TaskDistributionStrategy::RoundRobin,
                resource_sharing_enabled: true,
            },
            routing: AdvancedRoutingConfig {
                enabled: false,
                load_balancing_algorithm: BalancingAlgorithm::WeightedRoundRobin,
                circuit_breaker_enabled: true,
                routing_decision_cache_size: 1000,
            },
            jetstream: None,
        }
    }
}
```

---

## Migration Strategy

### Phase 3.0: Backward Compatibility (Week 1)
```bash
# Ensure Phase 2 continues to work
cargo test --features phase2
cargo run --features phase2
```

### Phase 3.1: Basic Supervision (Week 2)
```bash
# Enable supervision features
ENABLE_SUPERVISION=true cargo run
```

### Phase 3.2: Coordination Features (Week 3)
```bash
# Enable coordination features
ENABLE_SUPERVISION=true ENABLE_COORDINATION=true cargo run
```

### Phase 3.3: Advanced Routing (Week 4)
```bash
# Enable all Phase 3 features
ENABLE_SUPERVISION=true ENABLE_COORDINATION=true ENABLE_ADVANCED_ROUTING=true cargo run
```

### Phase 3.4: Production Deployment (Week 5)
```yaml
# Kubernetes deployment with Phase 3 features
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mistersmith-phase3
spec:
  template:
    spec:
      containers:
      - name: mistersmith
        env:
        - name: ENABLE_SUPERVISION
          value: "true"
        - name: ENABLE_COORDINATION
          value: "true"
        - name: ENABLE_ADVANCED_ROUTING
          value: "true"
        - name: NATS_URL
          value: "nats://nats-cluster:4222"
```

---

## Validation and Testing

### Integration Tests
```rust
#[cfg(test)]
mod phase3_integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_supervision_event_flow() {
        let transport = setup_test_transport().await;
        
        // Test supervision event publishing and handling
        let event = SupervisionEvent::Health {
            agent_id: AgentId::new(),
            status: HealthStatus::Healthy,
            metrics: HealthMetrics::default(),
            timestamp: SystemTime::now(),
        };
        
        transport.publish_supervision_event(&event).await.unwrap();
        
        // Verify event was processed
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(transport.supervision_processor().unwrap().event_was_processed(&event).await);
    }
    
    #[tokio::test]
    async fn test_coordination_task_distribution() {
        let transport = setup_test_transport().await;
        
        // Test task coordination
        let task = CoordinationMessage::TaskCoordination {
            task_id: Uuid::new_v4(),
            coordination_type: TaskCoordinationType::Broadcast,
            participants: vec![],
            requirements: TaskRequirements::default(),
        };
        
        transport.coordination_manager().unwrap().distribute_task(task).await.unwrap();
        
        // Verify task was distributed
        // Add verification logic
    }
}
```

This integration example demonstrates how Phase 3 advanced NATS patterns can be seamlessly integrated with the existing Phase 2 implementation while maintaining backward compatibility and enabling incremental feature adoption.

---

*Integration design ensures smooth migration path from Phase 2 to Phase 3 with minimal disruption to existing functionality.*