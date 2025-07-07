# Process & Message System Module Structure

## Complete Module Organization for Phase 2 Implementation

### src/runtime/ - Process Management Module

```
src/runtime/
├── mod.rs                  // Module exports and public API
├── process_manager.rs      // Core process management implementation
├── supervisor.rs           // Process supervision and restart logic
├── health_monitor.rs       // Health checking and monitoring
├── config.rs              // Process configuration structures
├── error.rs               // Runtime-specific error types
└── tests/
    ├── mod.rs
    ├── process_manager_test.rs
    └── supervisor_test.rs
```

#### Module Details

**mod.rs**
```rust
pub mod process_manager;
pub mod supervisor;
pub mod health_monitor;
pub mod config;
pub mod error;

pub use process_manager::{ProcessManager, ProcessHandle, ProcessId};
pub use supervisor::{Supervisor, RestartPolicy};
pub use health_monitor::{HealthMonitor, HealthStatus};
pub use config::{ProcessConfig, ResourceLimits};
pub use error::{ProcessError, ProcessResult};
```

**Key Types**
- `ProcessManager`: Main entry point for process lifecycle management
- `ProcessHandle`: Represents a running process with its metadata
- `Supervisor`: Implements supervision tree patterns
- `HealthMonitor`: Periodic health checking service
- `ProcessConfig`: Configuration for spawning processes

### src/message/ - Message System Module

```
src/message/
├── mod.rs                  // Module exports and public API
├── envelope.rs            // Base message envelope and traits
├── router.rs              // Message routing implementation
├── dispatcher.rs          // Message dispatch and handling
├── schemas/               // Message schema definitions
│   ├── mod.rs
│   ├── agent.rs          // Agent-related message schemas
│   ├── task.rs           // Task-related message schemas
│   └── system.rs         // System operation schemas
├── serialization.rs       // Message serialization/deserialization
├── validation.rs          // Schema validation
├── error.rs              // Message-specific error types
└── tests/
    ├── mod.rs
    ├── router_test.rs
    └── serialization_test.rs
```

#### Module Details

**mod.rs**
```rust
pub mod envelope;
pub mod router;
pub mod dispatcher;
pub mod schemas;
pub mod serialization;
pub mod validation;
pub mod error;

pub use envelope::{BaseMessageEnvelope, Message, MessageType};
pub use router::{MessageRouter, RoutingTable};
pub use dispatcher::{MessageDispatcher, MessageHandler};
pub use serialization::{MessageSerializer, SerializationFormat};
pub use error::{MessageError, MessageResult};
```

**Key Types**
- `BaseMessageEnvelope`: Common fields for all messages
- `MessageRouter`: Routes messages to appropriate destinations
- `MessageDispatcher`: Processes incoming messages
- `MessageSerializer`: Handles JSON/MessagePack serialization

### src/transport/ - Transport Abstraction Module

```
src/transport/
├── mod.rs                  // Module exports and Transport trait
├── traits.rs              // Core transport traits
├── nats_transport.rs      // NATS transport implementation
├── connection_pool.rs     // Generic connection pooling
├── health_check.rs        // Transport health monitoring
├── circuit_breaker.rs     // Circuit breaker pattern
├── config.rs             // Transport configuration
├── error.rs              // Transport-specific errors
└── tests/
    ├── mod.rs
    ├── nats_transport_test.rs
    └── connection_pool_test.rs
```

#### Module Details

**mod.rs**
```rust
pub mod traits;
pub mod nats_transport;
pub mod connection_pool;
pub mod health_check;
pub mod circuit_breaker;
pub mod config;
pub mod error;

pub use traits::{Transport, MessageHandler};
pub use nats_transport::NatsTransport;
pub use connection_pool::{ConnectionPool, PoolConfig};
pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use error::{TransportError, TransportResult};
```

**Key Types**
- `Transport`: Core trait for all transport implementations
- `NatsTransport`: NATS-specific implementation
- `ConnectionPool`: Manages connection lifecycle
- `CircuitBreaker`: Fault tolerance pattern

### Integration Points

#### src/integration/ - Cross-Module Integration

```
src/integration/
├── mod.rs
├── process_message.rs     // Process ↔ Message integration
├── message_transport.rs   // Message ↔ Transport integration
└── supervision.rs        // Unified supervision integration
```

### Shared Types and Utilities

#### src/common/ - Shared Components

```
src/common/
├── mod.rs
├── types.rs              // Common type definitions
├── metrics.rs            // Metrics collection
├── logging.rs            // Structured logging
└── utils.rs             // Utility functions
```

## Implementation Order

### Phase 2 - Minimal Implementation

1. **Week 1: Core Types & Traits**
   - Define Transport trait
   - Define base message types
   - Define process management interfaces

2. **Week 2: Process Management**
   - Implement ProcessManager
   - Basic supervisor with restart
   - Simple health monitoring

3. **Week 3: Message System**
   - Implement message envelope
   - Basic router
   - Simple dispatcher

4. **Week 4: Transport & Integration**
   - NATS transport implementation
   - Connect process and message systems
   - End-to-end testing

## File Templates

### Example: src/runtime/process_manager.rs

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::process::{Child, Command};
use uuid::Uuid;

use crate::runtime::{
    config::ProcessConfig,
    error::{ProcessError, ProcessResult},
    supervisor::Supervisor,
};

pub type ProcessId = String;

pub struct ProcessManager {
    processes: Arc<RwLock<HashMap<ProcessId, ProcessHandle>>>,
    supervisor: Arc<Supervisor>,
    config: ProcessConfig,
}

pub struct ProcessHandle {
    pub id: ProcessId,
    pub child: Child,
    pub status: ProcessStatus,
    pub spawn_time: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum ProcessStatus {
    Initializing,
    Idle,
    Busy,
    Paused,
    Error(String),
    Stopping,
    Terminated,
}

impl ProcessManager {
    pub fn new(supervisor: Arc<Supervisor>, config: ProcessConfig) -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            supervisor,
            config,
        }
    }

    pub async fn spawn_process(&self, id: ProcessId, command: &str, args: Vec<String>) -> ProcessResult<()> {
        // Implementation
        todo!()
    }

    pub async fn stop_process(&self, id: &ProcessId) -> ProcessResult<()> {
        // Implementation
        todo!()
    }

    pub async fn get_status(&self, id: &ProcessId) -> ProcessResult<ProcessStatus> {
        // Implementation
        todo!()
    }
}
```

### Example: src/transport/traits.rs

```rust
use async_trait::async_trait;
use crate::message::Message;
use crate::transport::error::TransportResult;

#[async_trait]
pub trait Transport: Send + Sync {
    async fn connect(&mut self, config: ConnectionConfig) -> TransportResult<()>;
    async fn disconnect(&mut self) -> TransportResult<()>;
    async fn send(&self, message: Message) -> TransportResult<Response>;
    async fn subscribe(&self, topic: &str, handler: MessageHandler) -> TransportResult<()>;
    async fn health_check(&self) -> HealthStatus;
}

pub type MessageHandler = Box<dyn Fn(Message) -> BoxFuture<'static, ()> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub url: String,
    pub timeout: Duration,
    pub retry_attempts: u32,
}

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}
```

## Cargo.toml Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1.38", features = ["full"] }
async-trait = "0.1"

# Message transport
async-nats = "0.34"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rmp-serde = "1.1"  # MessagePack

# Utilities
uuid = { version = "1.4", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
thiserror = "1.0"

# Testing
tokio-test = "0.4"
mockall = "0.11"
```

## Development Guidelines

1. **Error Handling**: Use `thiserror` for error definitions
2. **Async**: All I/O operations must be async
3. **Logging**: Use `tracing` for structured logging
4. **Testing**: Mock external dependencies with `mockall`
5. **Documentation**: Document all public APIs with examples

## Testing Structure

Each module should have:
- Unit tests in `tests/` subdirectory
- Integration tests in `tests/integration/`
- Doctests for usage examples
- Property-based tests for serialization

This module structure provides clear boundaries, minimal dependencies, and room for growth in Phase 3.