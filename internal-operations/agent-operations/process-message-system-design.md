# Process Management & Message System Design

## Overview

This document outlines the minimal design for MisterSmith's process management (`src/runtime/`) and message system (`src/message/` & `src/transport/`) components. The design focuses on Phase 2 requirements: minimal working implementation with Claude CLI process lifecycle management.

## Process Management Design (`src/runtime/`)

### Module Structure

```
src/runtime/
├── mod.rs              // Module exports and re-exports
├── process_manager.rs  // Main process management logic
├── supervisor.rs       // Process supervision and restart policies
├── health_monitor.rs   // Health checking and monitoring
└── error.rs           // Runtime-specific errors
```

### Core Components

#### ProcessManager
- Spawns and manages Claude CLI processes using `tokio::process::Command`
- Maintains registry of running processes
- Handles graceful shutdown and resource cleanup
- Integrates with message system for command/status communication

```rust
pub struct ProcessManager {
    processes: Arc<RwLock<HashMap<ProcessId, ProcessHandle>>>,
    supervisor: Arc<Supervisor>,
    config: ProcessConfig,
}

pub struct ProcessHandle {
    id: ProcessId,
    child: tokio::process::Child,
    status: ProcessStatus,
    spawn_time: Instant,
}
```

#### Supervisor
- Monitors process health and handles restarts
- Implements restart policies (Never, Always, OnFailure, ExponentialBackoff)
- Manages restart limits and windows
- Escalates failures when limits exceeded

```rust
pub struct Supervisor {
    policies: HashMap<ProcessId, RestartPolicy>,
    restart_counts: Arc<Mutex<HashMap<ProcessId, RestartTracker>>>,
    health_monitor: Arc<HealthMonitor>,
}

pub enum RestartPolicy {
    Never,
    Always,
    OnFailure,
    ExponentialBackoff {
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
    },
}
```

#### HealthMonitor
- Periodic health checks via HTTP endpoints or process status
- Heartbeat mechanism for process liveness
- Reports health status to supervisor
- Integrates with message system for alerts

```rust
pub struct HealthMonitor {
    check_interval: Duration,
    health_checks: HashMap<ProcessId, HealthChecker>,
    status_sender: mpsc::Sender<HealthStatus>,
}
```

### Process Lifecycle

```
┌─────────────┐     ┌──────┐     ┌──────┐     ┌────────┐
│Initializing │ --> │ Idle │ --> │ Busy │ --> │ Paused │
└─────────────┘     └──────┘     └──────┘     └────────┘
       │                │            │              │
       v                v            v              v
   ┌───────┐        ┌───────┐    ┌───────┐    ┌─────────┐
   │ Error │        │ Error │    │ Error │    │Stopping │
   └───────┘        └───────┘    └───────┘    └─────────┘
       │                │            │              │
       └────────────────┴────────────┴──────────────┘
                            │
                            v
                      ┌────────────┐
                      │ Terminated │
                      └────────────┘
```

## Message System Design (`src/message/`)

### Module Structure

```
src/message/
├── mod.rs             // Module exports
├── envelope.rs        // Base message envelope definition
├── router.rs          // Message routing logic
├── schemas.rs         // Message schema definitions
├── dispatcher.rs      // Message dispatch mechanisms
└── error.rs          // Message-specific errors
```

### Core Message Types

#### BaseMessageEnvelope
All messages inherit from this base structure:

```rust
pub struct BaseMessageEnvelope {
    pub message_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub schema_version: String,
    pub message_type: MessageType,
    pub correlation_id: Option<Uuid>,
    pub trace_id: Option<Uuid>,
    pub source_agent_id: Option<String>,
    pub target_agent_id: Option<String>,
    pub priority: u8,  // 1=highest, 10=lowest
    pub timeout_ms: Option<u32>,
    pub metadata: HashMap<String, String>,
}
```

#### Message Types

```rust
pub enum MessageType {
    // Agent Communication
    AgentCommand(AgentCommand),
    AgentStatus(AgentStatus),
    AgentRegistration(AgentRegistration),
    
    // Task Management
    TaskAssignment(TaskAssignment),
    TaskResult(TaskResult),
    TaskProgress(TaskProgress),
    
    // System Operations
    SystemAlert(SystemAlert),
    HealthCheck(HealthCheck),
}
```

### Message Router

The router handles message dispatch based on destination and type:

```rust
pub struct MessageRouter {
    transports: HashMap<String, Box<dyn Transport>>,
    routing_table: Arc<RwLock<RoutingTable>>,
    fallback_transport: Option<Box<dyn Transport>>,
}

impl MessageRouter {
    pub async fn route(&self, message: Message) -> Result<Response, RoutingError> {
        let destination = self.extract_destination(&message)?;
        let transport = self.lookup_transport(&destination)?;
        
        match transport.send(message).await {
            Ok(response) => Ok(response),
            Err(e) => self.handle_routing_failure(e).await,
        }
    }
}
```

### Message Dispatcher

Handles message processing pipeline:

```rust
pub struct MessageDispatcher {
    serializer: MessageSerializer,
    validator: SchemaValidator,
    priority_queue: PriorityQueue<Message>,
    handlers: HashMap<MessageType, MessageHandler>,
}
```

## Transport Abstraction Design (`src/transport/`)

### Module Structure

```
src/transport/
├── mod.rs               // Module exports and Transport trait
├── nats_transport.rs    // NATS implementation
├── connection_pool.rs   // Connection pooling logic
├── health_check.rs      // Transport health monitoring
└── error.rs            // Transport-specific errors
```

### Core Transport Trait

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    async fn connect(&mut self, config: ConnectionConfig) -> Result<(), TransportError>;
    async fn disconnect(&mut self) -> Result<(), TransportError>;
    async fn send(&self, message: Message) -> Result<Response, TransportError>;
    async fn subscribe(&self, topic: &str, handler: MessageHandler) -> Result<(), TransportError>;
    async fn health_check(&self) -> HealthStatus;
}
```

### NATS Transport Implementation

```rust
pub struct NatsTransport {
    client: Option<async_nats::Client>,
    config: NatsConfig,
    subscriptions: HashMap<String, async_nats::Subscriber>,
    health_checker: Arc<NatsHealthChecker>,
}

impl NatsTransport {
    // Subject naming convention: "mister-smith.{agent_type}.{agent_id}.{message_type}"
    fn build_subject(&self, message: &Message) -> String {
        format!(
            "mister-smith.{}.{}.{}",
            message.target_agent_type.unwrap_or("broadcast"),
            message.target_agent_id.unwrap_or("*"),
            message.message_type.as_str()
        )
    }
}
```

### Connection Pool

```rust
pub struct ConnectionPool<T: Transport> {
    connections: Vec<Arc<Mutex<T>>>,
    config: PoolConfig,
    health_monitor: Arc<HealthMonitor>,
    circuit_breakers: HashMap<usize, CircuitBreaker>,
}

pub struct PoolConfig {
    max_connections: usize,
    min_connections: usize,
    acquire_timeout: Duration,
    idle_timeout: Duration,
    health_check_interval: Duration,
}
```

## Error Handling Strategy

### Error Categories

1. **Process Errors**
   - `ProcessSpawnError`: Failed to start process
   - `ProcessCrashError`: Process terminated unexpectedly
   - `ResourceLimitError`: Exceeded memory/CPU limits
   - `SupervisionError`: Restart limits exceeded

2. **Message Errors**
   - `SerializationError`: Failed to serialize/deserialize
   - `RoutingError`: No route to destination
   - `ValidationError`: Schema validation failed
   - `TimeoutError`: Message processing timeout

3. **Transport Errors**
   - `ConnectionError`: Failed to connect
   - `DisconnectedError`: Lost connection
   - `BackpressureError`: Queue full
   - `ProtocolError`: Protocol-specific errors

### Error Propagation

```rust
pub type ProcessResult<T> = Result<T, ProcessError>;
pub type MessageResult<T> = Result<T, MessageError>;
pub type TransportResult<T> = Result<T, TransportError>;

// Unified error type for cross-module errors
pub enum SystemError {
    Process(ProcessError),
    Message(MessageError),
    Transport(TransportError),
}
```

## Integration Patterns

### Process ↔ Message Integration

```rust
// Process receives commands via messages
impl ProcessManager {
    pub async fn handle_command(&self, cmd: AgentCommand) -> ProcessResult<()> {
        match cmd.command_type {
            CommandType::Execute => self.execute_task(cmd.task_id).await,
            CommandType::Stop => self.stop_process(cmd.target_id).await,
            CommandType::Pause => self.pause_process(cmd.target_id).await,
            // ...
        }
    }
}

// Process sends status updates
impl ProcessHandle {
    pub async fn send_status_update(&self, router: &MessageRouter) -> MessageResult<()> {
        let status = AgentStatus {
            agent_id: self.id.clone(),
            status: self.status.clone(),
            resource_usage: self.get_resource_usage(),
            // ...
        };
        
        router.route(Message::AgentStatus(status)).await?;
        Ok(())
    }
}
```

### Message ↔ Transport Integration

```rust
// Message system uses transport abstraction
impl MessageRouter {
    pub async fn send_via_transport(
        &self, 
        message: Message, 
        transport: &dyn Transport
    ) -> TransportResult<Response> {
        // Serialize message
        let serialized = self.serializer.serialize(&message)?;
        
        // Send via transport
        let response = transport.send(serialized).await?;
        
        // Deserialize response
        self.serializer.deserialize(response)
    }
}
```

## Minimal Implementation Priorities

### Phase 2 Focus (Current)
1. Basic process spawning with tokio::process
2. Simple supervisor with restart-on-failure
3. NATS transport for message passing
4. Basic message routing by agent ID
5. Essential error handling

### Phase 3 Enhancements (Future)
1. Advanced supervision patterns
2. Resource limit enforcement
3. Circuit breaker implementation
4. Message persistence with JetStream
5. Multi-transport failover
6. Distributed tracing integration

## Configuration

### Process Configuration
```rust
pub struct ProcessConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: PathBuf,
    pub restart_policy: RestartPolicy,
    pub resource_limits: ResourceLimits,
}
```

### Message Configuration
```rust
pub struct MessageConfig {
    pub default_timeout_ms: u32,
    pub max_message_size: usize,
    pub retry_attempts: u32,
    pub retry_delay_ms: u32,
}
```

### Transport Configuration
```rust
pub struct TransportConfig {
    pub nats_url: String,
    pub connection_timeout: Duration,
    pub reconnect_delay: Duration,
    pub max_reconnect_attempts: u32,
}
```

## Testing Strategy

### Unit Tests
- Process lifecycle transitions
- Message serialization/deserialization
- Routing logic
- Error handling paths

### Integration Tests
- Process spawn and supervision
- End-to-end message flow
- Transport connection handling
- Health check mechanisms

### Performance Tests
- Message throughput
- Process spawn latency
- Connection pool efficiency
- Error recovery time

## Summary

This design provides a minimal but extensible foundation for MisterSmith's process management and message system. It leverages:

- **Tokio** for async runtime and process management
- **NATS** for reliable message transport
- **Supervision patterns** for fault tolerance
- **Clear module boundaries** for maintainability

The design prioritizes simplicity for Phase 2 while laying groundwork for Phase 3 enhancements.