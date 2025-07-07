# Phase 3: Multi-Agent Expansion Architecture

## Executive Summary

Phase 3 transforms the MisterSmith framework from a single-agent system to a distributed multi-agent architecture supporting 15 specialized domains. This document defines the minimal, incremental path from Phase 2's foundation to a scalable multi-agent system.

**Key Principles**:
- Build on existing Phase 2 foundation (Tokio runtime, NATS message bus, supervision tree)
- Incremental expansion - each step must be independently valuable
- No central coordinator - coordination emerges from specialized agents
- Domain isolation with well-defined boundaries
- Capability-based discovery over static configuration

## Architecture Overview

### System Evolution

```
Phase 2 (Current):
┌─────────────────┐
│ Single Agent    │
│ + Core Runtime  │
│ + Message Bus   │
│ + Supervision   │
└─────────────────┘

Phase 3.0 (Foundation):
┌─────────────────────────────────────┐
│         Root Supervisor             │
├─────────────┬──────────────┬────────┤
│   Agent     │   Domain     │ Domain │
│  Registry   │ Supervisor A │ Sup. B │
│             ├──────┬───────┼────────┤
│             │Agent │Agent  │Agent   │
└─────────────┴──────┴───────┴────────┘

Phase 3.x (Full Expansion):
┌─────────────────────────────────────┐
│         Root Supervisor             │
├─────┬───────┬──────────┬───────────┤
│     │  15 Domain Supervisors   ... │
│     └───┬───┴────┬─────┴───────────┤
│         │  60+ Specialized Agents  │
└─────────┴────────┴─────────────────┘
```

## Core Architectural Components

### 1. Hierarchical Supervision Tree

The flat supervision model of Phase 2 evolves into a hierarchical structure:

```rust
pub struct SupervisionHierarchy {
    // Root supervisor remains from Phase 2
    root: RootSupervisor,
    
    // New layer: Domain supervisors
    domains: HashMap<DomainId, DomainSupervisor>,
    
    // Supervision strategies by level
    strategies: SupervisionStrategies,
}

pub struct SupervisionStrategies {
    root_strategy: OneForAll,        // System-wide restart on critical failure
    domain_strategy: RestForOne,     // Restart domain and dependents
    agent_strategy: OneForOne,       // Isolate individual agent failures
}
```

**Benefits**:
- Failure isolation at domain boundaries
- Reduced blast radius for restarts
- Clear cognitive model matching system architecture
- Domain-specific supervision strategies

### 2. Agent Identity and Addressing

Rich identity model supporting multiple instances and versions:

```rust
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct AgentId {
    domain: DomainId,
    name: String,
    instance_id: Uuid,           // Support multiple instances
    version: SemanticVersion,    // Enable rolling updates
}

pub enum AgentAddress {
    // Local in-process communication
    InProcess(tokio::sync::mpsc::Sender<Message>),
    
    // NATS-based communication
    Nats { 
        subject: String,         // e.g., "agents.data.kvcache.v1"
        queue_group: Option<String>,
    },
}
```

### 3. Agent Registry and Discovery

Minimal service locator pattern with capability-based discovery:

```rust
pub struct AgentRegistry {
    // Basic registry data
    agents: Arc<RwLock<HashMap<AgentId, AgentRecord>>>,
    
    // Capability index for smart routing
    capabilities: Arc<RwLock<HashMap<String, Vec<AgentId>>>>,
    
    // NATS connection for distributed updates
    nats: async_nats::Client,
}

pub struct AgentRecord {
    id: AgentId,
    address: AgentAddress,
    capabilities: Vec<Capability>,
    state: AgentState,
    health: HealthMetrics,
    registered_at: Instant,
}

// Discovery patterns
impl AgentRegistry {
    // Direct lookup
    pub async fn resolve(&self, id: &AgentId) -> Option<AgentAddress>;
    
    // Capability-based discovery
    pub async fn discover(&self, capability: &str) -> Vec<AgentRecord>;
    
    // Health-aware routing
    pub async fn find_healthy(&self, capability: &str) -> Option<AgentRecord>;
}
```

### 4. Cross-Domain Communication

All inter-domain communication flows through the message bus with strict contracts:

```rust
// Domain API boundaries
pub mod domain_contracts {
    pub mod data_persistence {
        #[derive(Serialize, Deserialize)]
        pub enum Request {
            Store { key: String, value: Vec<u8>, ttl: Option<Duration> },
            Retrieve { key: String },
            Query { filter: QueryFilter, limit: usize },
        }
        
        #[derive(Serialize, Deserialize)]
        pub enum Response {
            Stored { key: String, version: u64 },
            Retrieved { key: String, value: Option<Vec<u8>> },
            QueryResult { results: Vec<Record>, total: usize },
            Error { code: ErrorCode, message: String },
        }
    }
}

// Message routing patterns
pub enum RoutingPattern {
    Direct(AgentId),              // Point-to-point via registry
    Broadcast(String),            // Domain-wide: "agents.security.*"
    RequestReply(String),         // RPC-style with timeout
    LoadBalanced(String),         // Queue group for work distribution
}
```

### 5. Agent Lifecycle State Machine

Well-defined states for consistent behavior:

```rust
#[derive(Debug, Clone)]
pub enum AgentState {
    Unregistered,
    Registering { started_at: Instant },
    Registered { registry_id: Uuid },
    Active { 
        since: Instant,
        message_count: u64,
    },
    Draining { 
        reason: String,
        deadline: Instant,
    },
    Suspended { reason: String },
    Terminated { reason: String },
}

pub trait AgentLifecycle {
    async fn transition(&mut self, to: AgentState) -> Result<()>;
    async fn can_transition(&self, to: &AgentState) -> bool;
    async fn on_state_change(&mut self, from: AgentState, to: AgentState);
}
```

## Domain Implementation Order

### Phase 3.0: Foundation Layer (Week 1)

**Goal**: Enable multiple agents to coexist and communicate.

1. **Supervision Tree Evolution** (1 day)
   - Refactor RootSupervisor to support DomainSupervisor children
   - Implement supervision strategy selection

2. **Agent Registry** (2 days)
   - Basic in-memory registry with NATS persistence
   - Registration/deregistration flows
   - Health tracking via heartbeats

3. **Discovery Patterns** (1 day)
   - NATS subject hierarchy: `agents.{domain}.{name}.{action}`
   - Request-reply for synchronous discovery
   - Wildcard subscriptions for events

4. **First Agent Pair** (3 days)
   - Implement two agents in Agent Lifecycle domain
   - Verify communication patterns
   - Establish testing framework

### Phase 3.1: Core Domains (Week 2-3)

**Goal**: Implement essential system functionality.

**Domains** (in dependency order):
1. **Agent Lifecycle** (Registry, Lifecycle Manager)
2. **Message Schema** (Schema Registry, Validator)
3. **Security** (Auth, RBAC Policy)
4. **Configuration Management** (Config Watcher, Reload Coordinator)

### Phase 3.2: Data and Transport (Week 4-5)

**Goal**: Enable persistent state and external communication.

**Domains**:
1. **Data Persistence** (Schema Evolution, KV Cache)
2. **Transport Layer** (NATS Subject, Protocol Bridge)
3. **Observability** (Metrics, Tracing)

### Phase 3.3: Application Domains (Week 6+)

**Goal**: Business logic and advanced features.

**Domains**:
1. **Task Orchestration**
2. **Network Protocol**
3. **Storage Optimization**
4. **Backup and Recovery**
5. **Deployment and Scaling**
6. **Integration Testing**
7. **Neural/AI Operations**

## Minimal Implementation Path

### Step 1: Extend Supervision (Day 1)

```rust
// Minimal change to Phase 2
impl RootSupervisor {
    pub async fn add_domain(&mut self, domain: DomainId) -> Result<()> {
        let supervisor = DomainSupervisor::new(domain, self.config.clone());
        self.domains.insert(domain, supervisor);
        Ok(())
    }
}
```

### Step 2: Bootstrap Registry (Day 2-3)

```rust
// First specialized agent
pub struct AgentRegistryAgent {
    base: BaseAgent,  // Reuse Phase 2 agent base
    registry: HashMap<AgentId, AgentRecord>,
}

impl Agent for AgentRegistryAgent {
    async fn handle_message(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::Register(agent_id, address) => {
                self.registry.insert(agent_id, AgentRecord::new(address));
                Ok(())
            }
            Message::Resolve(agent_id, reply_to) => {
                let address = self.registry.get(&agent_id).map(|r| r.address.clone());
                reply_to.send(Message::ResolveResponse(address)).await?;
                Ok(())
            }
            _ => self.base.handle_message(msg).await,
        }
    }
}
```

### Step 3: Enable Discovery (Day 4)

```rust
// NATS-based discovery
impl NatsDiscovery {
    pub async fn setup_discovery(&self) -> Result<()> {
        // Subscribe to discovery requests
        self.nats.subscribe("agents.discovery.*").await?;
        
        // Publish agent availability
        self.nats.publish(
            "agents.registry.available",
            serde_json::to_vec(&self.agent_id)?,
        ).await?;
        
        Ok(())
    }
}
```

### Step 4: Second Agent (Day 5-7)

```rust
// Lifecycle Manager as second agent
pub struct LifecycleManagerAgent {
    base: BaseAgent,
    registry: Arc<AgentRegistry>,
    agents: HashMap<AgentId, AgentState>,
}

// Now we have two agents communicating!
```

## State Consistency Model

### Event Sourcing for Distributed State

```rust
#[derive(Serialize, Deserialize)]
pub struct StateEvent {
    agent_id: AgentId,
    sequence: u64,
    timestamp: SystemTime,
    event_type: StateEventType,
    data: serde_json::Value,
}

pub trait EventSourced {
    async fn apply_event(&mut self, event: StateEvent) -> Result<()>;
    async fn get_events_since(&self, sequence: u64) -> Vec<StateEvent>;
    async fn snapshot(&self) -> StateSnapshot;
}
```

### Eventual Consistency

- Agents operate on local state copies
- State changes published to NATS JetStream
- Periodic reconciliation via anti-entropy
- Conflict resolution via vector clocks

## Critical Design Decisions

### 1. No Central Coordinator

Coordination emerges from specialized agents:
- **RootSupervisor**: Process coordination
- **AgentRegistry**: Discovery coordination  
- **LifecycleManager**: Lifecycle coordination
- **SAGA Coordinator**: Transaction coordination

### 2. Message Bus Architecture

Leverage NATS for:
- Subject-based routing with wildcards
- Request-reply for RPC patterns
- JetStream for persistent messaging
- Queue groups for load balancing

### 3. Failure Handling

- Circuit breakers at domain boundaries
- Bulkheads via separate queue groups
- Timeout budgets for cross-domain calls
- Graceful degradation patterns

## Testing Strategy

### Multi-Agent Test Framework

```rust
#[tokio::test]
async fn test_multi_agent_communication() {
    let mut test_runtime = MultiAgentTestRuntime::new();
    
    // Start registry
    let registry = test_runtime.spawn_agent(AgentRegistryAgent::new()).await?;
    
    // Start test agents
    let agent_a = test_runtime.spawn_agent(TestAgent::new("A")).await?;
    let agent_b = test_runtime.spawn_agent(TestAgent::new("B")).await?;
    
    // Verify discovery
    let discovered = registry.discover("test.capability").await?;
    assert_eq!(discovered.len(), 2);
    
    // Verify communication
    agent_a.send_to(agent_b.id(), TestMessage::Ping).await?;
    assert_eq!(agent_b.received_count(), 1);
}
```

## Performance Considerations

### Resource Limits

```yaml
# Per-agent resource constraints
agent_limits:
  max_memory: 512MB
  max_cpu_shares: 200  # 2 cores
  max_message_queue: 10000
  max_concurrent_tasks: 100

# Domain-level constraints  
domain_limits:
  max_agents: 10
  max_total_memory: 4GB
  message_rate_limit: 100k/sec
```

### Optimization Strategies

1. **Message Batching**: Aggregate small messages
2. **Connection Pooling**: Reuse NATS connections
3. **Lazy Agent Loading**: Start agents on-demand
4. **Adaptive Scaling**: Add/remove instances based on load

## Security Considerations

### Domain Isolation

- Separate NATS accounts per domain
- mTLS between domain boundaries
- Message encryption for sensitive data
- Audit logging for cross-domain calls

### Agent Authentication

```rust
pub struct AgentCredentials {
    agent_id: AgentId,
    jwt_token: String,
    permissions: Vec<Permission>,
    expires_at: SystemTime,
}
```

## Migration Path from Phase 2

1. **Code Changes Required**:
   - Modify supervisor to support multiple children
   - Add AgentId to existing agent
   - Implement registration on startup

2. **Configuration Changes**:
   ```yaml
   # Phase 2
   agent:
     type: single
     
   # Phase 3
   agents:
     registry:
       enabled: true
     domains:
       - agent_lifecycle
   ```

3. **Backward Compatibility**:
   - Phase 2 agent continues to work
   - Gradual migration to multi-agent
   - Feature flags for new capabilities

## Future Considerations

### Phase 4 and Beyond

1. **Distributed Deployment**: Agents across multiple nodes
2. **Federation**: Multiple MisterSmith clusters
3. **Dynamic Domains**: Runtime domain addition
4. **Agent Mobility**: Moving agents between nodes
5. **Advanced Patterns**: CQRS, Event Sourcing, Saga

### Scalability Path

- Horizontal scaling via agent instances
- Vertical scaling via resource allocation
- Geographic distribution via NATS clusters
- Sharding strategies for large domains

## Conclusion

Phase 3's multi-agent architecture provides a minimal, incremental path from single-agent to distributed multi-agent system. By leveraging the existing Phase 2 foundation and adding only essential components (hierarchical supervision, agent registry, discovery patterns), we enable gradual expansion across 15 specialized domains while maintaining system coherence and operational simplicity.

The architecture prioritizes:
- Incremental value delivery
- Failure isolation
- Loose coupling
- Operational observability
- Clear migration path

Next steps: Implement Phase 3.0 foundation layer, validate with first agent pair, then proceed with domain rollout based on business priorities.