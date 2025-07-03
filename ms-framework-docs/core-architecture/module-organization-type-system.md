# Module Organization & Type Definition Specification
## Complete src/ Directory Structure and Type System Architecture

**Agent 4 Deliverable**: Module Organization & Type Definition Specialist

### Overview

This document provides complete module hierarchy, type system specifications, and dependency injection patterns for the Mister Smith AI Agent Framework. It enables autonomous developers to understand exact project structure, type relationships, and implementation patterns.

---

## 1. Complete src/ Directory Structure

### 1.1 Root Module Hierarchy

```
src/
├── lib.rs                    # Root library with re-exports
├── runtime/                  # Tokio runtime management
│   ├── mod.rs
│   ├── manager.rs           # RuntimeManager implementation
│   ├── config.rs            # RuntimeConfig definitions
│   └── lifecycle.rs         # Startup/shutdown orchestration
├── async_patterns/          # Async execution patterns
│   ├── mod.rs
│   ├── task_executor.rs     # TaskExecutor and TaskHandle
│   ├── stream_processor.rs  # StreamProcessor with backpressure
│   └── circuit_breaker.rs   # CircuitBreaker implementation
├── actor/                   # Actor system implementation
│   ├── mod.rs
│   ├── system.rs           # ActorSystem orchestration
│   ├── actor_ref.rs        # ActorRef and message routing
│   ├── mailbox.rs          # Mailbox implementation
│   └── message.rs          # ActorMessage type definitions
├── supervision/             # Supervision tree and fault tolerance
│   ├── mod.rs
│   ├── tree.rs             # SupervisionTree architecture
│   ├── supervisor.rs       # Supervisor trait and node implementation
│   ├── failure_detector.rs # FailureDetector with phi accrual
│   └── strategies.rs       # SupervisionStrategy enumeration
├── events/                  # Event-driven architecture
│   ├── mod.rs
│   ├── bus.rs              # EventBus implementation
│   ├── handler.rs          # EventHandler trait definitions
│   ├── store.rs            # EventStore for persistence
│   └── serialization.rs   # Event serialization utilities
├── tools/                   # Tool system and agent integration
│   ├── mod.rs
│   ├── bus.rs              # ToolBus registry and execution
│   ├── tool.rs             # Tool trait definition
│   ├── agent_tool.rs       # Agent-as-Tool pattern implementation
│   └── permissions.rs      # Tool access control
├── transport/               # Communication layer
│   ├── mod.rs
│   ├── message_bridge.rs   # MessageBridge for routing
│   ├── routing.rs          # Routing strategy implementations
│   └── serialization.rs   # Message serialization
├── config/                  # Configuration management
│   ├── mod.rs
│   ├── manager.rs          # ConfigurationManager
│   ├── watchers.rs         # Configuration change watchers
│   └── types.rs            # Configuration type definitions
├── resources/               # Resource management
│   ├── mod.rs
│   ├── manager.rs          # ResourceManager orchestration
│   ├── pools.rs            # ConnectionPool implementations
│   ├── memory.rs           # MemoryManager utilities
│   └── file_handles.rs     # FileHandlePool management
├── monitoring/              # Health checks and metrics
│   ├── mod.rs
│   ├── health.rs           # HealthCheckManager
│   ├── metrics.rs          # MetricsCollector and registry
│   └── diagnostics.rs     # System diagnostic utilities
├── agents/                  # Agent implementation framework
│   ├── mod.rs
│   ├── spawner.rs          # RoleSpawner and spawn control
│   ├── roles.rs            # AgentRole definitions
│   ├── team.rs             # Team coordination patterns
│   └── context.rs          # Agent context management
├── security/                # Security and authentication
│   ├── mod.rs
│   ├── auth.rs             # Authentication services
│   ├── permissions.rs      # Permission system
│   └── encryption.rs       # Encryption utilities
└── errors/                  # Error handling foundation
    ├── mod.rs
    ├── system_error.rs      # SystemError hierarchy
    ├── recovery.rs          # Recovery strategy implementations
    └── result.rs            # Custom Result type definitions
```

### 1.2 Root lib.rs with Strategic Re-exports

```rust
// src/lib.rs - Main library entry point with comprehensive re-exports
#![doc = include_str!("../README.md")]
#![deny(missing_docs, unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

pub mod runtime;
pub mod async_patterns;
pub mod actor;
pub mod supervision;
pub mod events;
pub mod tools;
pub mod transport;
pub mod config;
pub mod resources;
pub mod monitoring;
pub mod agents;
pub mod security;
pub mod errors;

// Core system orchestrator (defined separately)
mod system;
pub use system::SystemCore;

/// Core runtime and execution framework re-exports
pub use crate::{
    runtime::{RuntimeManager, RuntimeConfig, RuntimeError},
    async_patterns::{
        TaskExecutor, TaskHandle, StreamProcessor, CircuitBreaker,
        TaskPriority, RetryPolicy, BackpressureConfig
    },
};

/// Actor system and supervision re-exports
pub use crate::{
    actor::{ActorSystem, ActorRef, Actor, ActorMessage, ActorError},
    supervision::{
        SupervisionTree, Supervisor, SupervisionStrategy, RestartPolicy,
        SupervisionResult, SupervisionError
    },
};

/// Event system re-exports
pub use crate::{
    events::{
        EventBus, EventHandler, Event, EventType, EventResult,
        EventError, SystemEvent
    },
};

/// Tool system re-exports  
pub use crate::{
    tools::{
        ToolBus, Tool, AgentTool, ToolSchema, ToolCapabilities,
        ToolError, ToolId
    },
};

/// Communication and configuration re-exports
pub use crate::{
    transport::{MessageBridge, RoutingStrategy, TransportError},
    config::{ConfigurationManager, Configuration, ConfigError},
};

/// Resource management re-exports
pub use crate::{
    resources::{
        ResourceManager, Resource, ConnectionPool, PooledResource,
        ResourceError, ResourceConfig
    },
};

/// Monitoring and health re-exports
pub use crate::{
    monitoring::{
        HealthCheckManager, HealthCheck, HealthResult, MetricsCollector,
        MonitoringError
    },
};

/// Agent framework re-exports
pub use crate::{
    agents::{
        RoleSpawner, AgentRole, Agent, AgentContext, Team,
        AgentError, AgentConfig
    },
};

/// Security re-exports
pub use crate::{
    security::{
        AuthService, PermissionSystem, SecurityConfig,
        SecurityError
    },
};

/// Error handling re-exports
pub use crate::{
    errors::{
        SystemError, SystemResult, RecoveryStrategy, ErrorSeverity
    },
};

/// Common external dependency re-exports for convenience
pub use tokio;
pub use serde::{Deserialize, Serialize};
pub use async_trait::async_trait;
pub use uuid::Uuid;

/// Type aliases for common patterns
pub type AgentId = Uuid;
pub type NodeId = Uuid;  
pub type ComponentId = Uuid;
pub type TaskId = Uuid;
pub type SubscriptionId = Uuid;
```

---

## 2. Core Trait Hierarchy and Type System

### 2.1 Foundational Traits with Generic Constraints

```rust
/// Core async task abstraction with complete type bounds
#[async_trait]
pub trait AsyncTask: Send + Sync + 'static {
    type Output: Send + 'static;
    type Error: Send + std::error::Error + 'static;
    
    async fn execute(&self) -> Result<Self::Output, Self::Error>;
    fn priority(&self) -> TaskPriority;
    fn timeout(&self) -> Duration;
    fn retry_policy(&self) -> RetryPolicy;
    fn task_id(&self) -> TaskId;
}

/// Stream processing abstraction with error handling
#[async_trait]
pub trait Processor<T>: Send + Sync + 'static
where T: Send + 'static 
{
    type Output: Send + 'static;
    type Error: Send + std::error::Error + 'static;
    
    async fn process(&self, item: T) -> Result<Self::Output, Self::Error>;
    fn can_process(&self, item: &T) -> bool;
    fn processor_id(&self) -> ProcessorId;
}

/// Core actor behavior with message type safety
#[async_trait]
pub trait Actor: Send + 'static {
    type Message: Send + 'static;
    type State: Send + 'static;
    type Error: Send + std::error::Error + 'static;
    
    async fn handle_message(
        &mut self, 
        message: Self::Message, 
        state: &mut Self::State
    ) -> Result<ActorResult, Self::Error>;
    
    fn pre_start(&mut self) -> Result<(), Self::Error>;
    fn post_stop(&mut self) -> Result<(), Self::Error>;
    fn actor_id(&self) -> ActorId;
}

/// Supervision hierarchy management
#[async_trait] 
pub trait Supervisor: Send + Sync + 'static {
    type Child: Send + 'static;
    type Error: Send + std::error::Error + 'static;
    
    async fn supervise(&self, children: Vec<Self::Child>) -> Result<SupervisionResult, Self::Error>;
    fn supervision_strategy(&self) -> SupervisionStrategy;
    fn restart_policy(&self) -> RestartPolicy;
    fn escalation_policy(&self) -> EscalationPolicy;
    fn supervisor_id(&self) -> NodeId;
}

/// Universal tool interface with schema validation
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    async fn execute(&self, params: Value) -> Result<Value, ToolError>;
    fn schema(&self) -> ToolSchema;
    fn capabilities(&self) -> ToolCapabilities;
    fn tool_id(&self) -> ToolId;
    fn version(&self) -> semver::Version;
}

/// Agent interface extending Tool with context
#[async_trait]
pub trait Agent: Tool + Send + Sync + 'static {
    type Context: AgentContext + Send + Sync;
    type Error: Send + std::error::Error + 'static;
    
    async fn process(&self, message: Message) -> Result<Value, Self::Error>;
    fn role(&self) -> AgentRole;
    fn context(&self) -> &Self::Context;
    async fn initialize(&mut self, context: Self::Context) -> Result<(), Self::Error>;
    fn dependencies() -> Vec<TypeId>;
    
    /// Factory method for dependency injection
    async fn create_with_dependencies(
        config: AgentConfig,
        registry: &ServiceRegistry
    ) -> Result<Self, Self::Error>
    where Self: Sized;
}
```

### 2.2 Resource Management Traits

```rust
/// Generic resource abstraction with lifecycle management
#[async_trait]
pub trait Resource: Send + Sync + 'static {
    type Config: Send + Sync + Clone + 'static;
    type Error: Send + std::error::Error + 'static;
    
    async fn acquire(config: Self::Config) -> Result<Self, Self::Error>
    where Self: Sized;
    async fn release(self) -> Result<(), Self::Error>;
    fn is_healthy(&self) -> bool;
    async fn health_check(&self) -> Result<HealthStatus, Self::Error>;
    fn resource_id(&self) -> ResourceId;
}

/// Health monitoring abstraction
#[async_trait]
pub trait HealthCheck: Send + Sync + 'static {
    async fn check_health(&self) -> HealthResult;
    fn component_id(&self) -> ComponentId;
    fn timeout(&self) -> Duration;
    fn check_interval(&self) -> Duration;
}

/// Configuration management with validation
pub trait Configuration: Send + Sync + Clone + Serialize + DeserializeOwned + 'static {
    fn validate(&self) -> Result<(), ConfigError>;
    fn merge(&mut self, other: Self) -> Result<(), ConfigError>;
    fn key() -> ConfigurationKey;
    fn version(&self) -> ConfigVersion;
}

/// Event handling with type safety
#[async_trait]
pub trait EventHandler: Send + Sync + 'static {
    type Event: Event + Send + Sync + 'static;
    
    async fn handle_event(&self, event: Self::Event) -> EventResult;
    fn event_types(&self) -> Vec<EventType>;
    fn handler_id(&self) -> HandlerId;
    fn priority(&self) -> HandlerPriority;
}
```

---

## 3. Concrete Type Definitions with Generic Constraints

### 3.1 Core System Types

```rust
/// Central system orchestrator with dependency injection
pub struct SystemCore<C, R, E> 
where 
    C: Configuration + Clone + 'static,
    R: Runtime + Send + Sync + 'static,
    E: EventHandler + Clone + 'static,
{
    runtime_manager: RuntimeManager<R>,
    actor_system: ActorSystem<E::Event>,
    supervision_tree: SupervisionTree<E::Event>,
    event_bus: EventBus<E>,
    metrics_registry: MetricsRegistry,
    configuration_manager: ConfigurationManager<C>,
    service_registry: ServiceRegistry,
}

impl<C, R, E> SystemCore<C, R, E>
where
    C: Configuration + Clone + 'static,
    R: Runtime + Send + Sync + 'static,
    E: EventHandler + Clone + 'static,
{
    pub async fn new(
        runtime_manager: RuntimeManager<R>,
        actor_system: ActorSystem<E::Event>,
        supervision_tree: SupervisionTree<E::Event>,
        event_bus: EventBus<E>,
        configuration_manager: ConfigurationManager<C>,
    ) -> Result<Self, SystemError> {
        let metrics_registry = MetricsRegistry::new();
        let service_registry = ServiceRegistry::new();
        
        let core = Self {
            runtime_manager,
            actor_system,
            supervision_tree,
            event_bus,
            metrics_registry,
            configuration_manager,
            service_registry,
        };
        
        core.wire_components().await?;
        Ok(core)
    }
    
    async fn wire_components(&self) -> Result<(), SystemError> {
        // Wire supervision tree to actor system
        self.supervision_tree.set_actor_system(self.actor_system.clone()).await?;
        
        // Wire event bus to all components
        self.actor_system.set_event_bus(self.event_bus.clone()).await?;
        self.supervision_tree.set_event_bus(self.event_bus.clone()).await?;
        
        // Wire metrics to all components
        self.actor_system.set_metrics(self.metrics_registry.clone()).await?;
        self.supervision_tree.set_metrics(self.metrics_registry.clone()).await?;
        
        Ok(())
    }
}

/// Tokio runtime wrapper with lifecycle management
pub struct RuntimeManager<R: Runtime> {
    runtime: Arc<R>,
    shutdown_signal: Arc<AtomicBool>,
    health_monitor: HealthMonitor,
    metrics_collector: MetricsCollector,
    config: RuntimeConfig,
}
```

### 3.2 Task Execution with Type Safety

```rust
/// Task executor with bounded concurrency and type constraints
pub struct TaskExecutor<T, E> 
where 
    T: AsyncTask + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    task_queue: Arc<Mutex<VecDeque<Box<dyn AsyncTask<Output = T::Output, Error = E>>>>>,
    worker_pool: Vec<JoinHandle<()>>,
    semaphore: Arc<Semaphore>,
    metrics: TaskMetrics,
    config: TaskExecutorConfig,
}

impl<T, E> TaskExecutor<T, E>
where 
    T: AsyncTask + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    pub async fn submit(&self, task: T) -> TaskHandle<T::Output, E> {
        let permit = self.semaphore.acquire().await.expect("Semaphore closed");
        let handle = TaskHandle::new(task.task_id());
        
        let spawned_task = tokio::spawn(async move {
            let result = timeout(task.timeout(), task.execute()).await;
            match result {
                Ok(Ok(output)) => handle.complete(output),
                Ok(Err(e)) => handle.fail_with_retry(e, task.retry_policy()),
                Err(_timeout) => handle.fail_with_retry(
                    E::from("Task timeout"), 
                    task.retry_policy()
                ),
            }
            drop(permit);
        });
        
        self.task_queue.lock().await.push_back(Box::new(task));
        handle
    }
}

/// Task handle with future integration and proper waker management
pub struct TaskHandle<T, E> {
    inner: Arc<Mutex<TaskState<T, E>>>,
    task_id: TaskId,
}

impl<T, E> Future for TaskHandle<T, E> 
where 
    T: Send + 'static,
    E: Send + 'static,
{
    type Output = Result<T, E>;
    
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.inner.lock().unwrap();
        match &*state {
            TaskState::Pending => {
                state.set_waker(cx.waker().clone());
                Poll::Pending
            },
            TaskState::Completed(result) => Poll::Ready(result.clone()),
            TaskState::Failed(error) => Poll::Ready(Err(error.clone())),
        }
    }
}
```

### 3.3 Actor System with Message Type Constraints

```rust
/// Actor system with heterogeneous message handling
pub struct ActorSystem<M> 
where 
    M: Message + Send + Sync + Clone + 'static,
{
    actors: HashMap<ActorId, Box<dyn Actor<Message = M>>>,
    mailbox_factory: MailboxFactory<M>,
    dispatcher: Dispatcher<M>,
    supervision_strategy: SupervisionStrategy,
    event_bus: Option<Arc<EventBus<SystemEvent>>>,
}

/// Type-safe actor reference with message constraints
pub struct ActorRef<M: Message + Send + Sync + 'static> {
    actor_id: ActorId,
    mailbox: Arc<Mailbox<M>>,
    system_ref: Weak<ActorSystem<M>>,
}

impl<M: Message + Send + Sync + 'static> ActorRef<M> {
    pub async fn send(&self, message: M) -> Result<(), ActorError> {
        if self.mailbox.is_full() {
            return Err(ActorError::MailboxFull);
        }
        
        self.mailbox.enqueue(message.into()).await?;
        Ok(())
    }
    
    pub async fn ask<R>(&self, message: M) -> Result<R, ActorError> 
    where 
        R: Send + 'static,
        M: AskMessage<Response = R>,
    {
        let (tx, rx) = oneshot::channel();
        let wrapped_message = AskMessage::new(message, tx);
        
        self.mailbox.enqueue(wrapped_message.into()).await?;
        let result = rx.await.map_err(|_| ActorError::ResponseChannelClosed)?;
        
        Ok(result)
    }
}
```

---

## 4. Dependency Injection Architecture

### 4.1 Service Registry with Type Safety

```rust
/// Central service registry with type-safe dependency injection
pub struct ServiceRegistry {
    services: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    factories: HashMap<TypeId, Box<dyn ServiceFactory>>,
    singletons: HashSet<TypeId>,
    dependency_graph: DependencyGraph,
}

/// Service factory trait for dynamic service creation
pub trait ServiceFactory: Send + Sync {
    fn create(&self, registry: &ServiceRegistry) -> Result<Box<dyn Any + Send + Sync>, ServiceError>;
    fn dependencies(&self) -> Vec<TypeId>;
    fn service_type(&self) -> TypeId;
}

impl ServiceRegistry {
    pub fn register<T>(&mut self, service: T) -> Result<(), ServiceError>
    where T: Send + Sync + 'static {
        let type_id = TypeId::of::<T>();
        self.services.insert(type_id, Box::new(service));
        Ok(())
    }
    
    pub fn register_factory<T, F>(&mut self, factory: F) -> Result<(), ServiceError>
    where 
        T: Send + Sync + 'static,
        F: ServiceFactory + 'static,
    {
        let type_id = TypeId::of::<T>();
        
        // Validate dependencies to prevent cycles
        self.dependency_graph.add_service(type_id, factory.dependencies())?;
        
        self.factories.insert(type_id, Box::new(factory));
        Ok(())
    }
    
    pub fn resolve<T>(&self) -> Result<Arc<T>, ServiceError>
    where T: Send + Sync + 'static {
        let type_id = TypeId::of::<T>();
        
        if let Some(service) = self.services.get(&type_id) {
            service.downcast_ref::<T>()
                .map(|s| Arc::new(s.clone()))
                .ok_or(ServiceError::TypeMismatch)
        } else if let Some(factory) = self.factories.get(&type_id) {
            let instance = factory.create(self)?;
            let service = instance.downcast::<T>()
                .map_err(|_| ServiceError::TypeMismatch)?;
            Ok(Arc::new(*service))
        } else {
            Err(ServiceError::ServiceNotFound(type_id))
        }
    }
    
    /// Resolve all dependencies for a service in topological order
    pub fn resolve_dependencies(&self, type_id: TypeId) -> Result<Vec<TypeId>, ServiceError> {
        self.dependency_graph.topological_sort(type_id)
    }
}
```

### 4.2 System Builder with Fluent API

```rust
/// System builder with comprehensive dependency management
pub struct SystemBuilder {
    registry: ServiceRegistry,
    config: Option<SystemConfig>,
    runtime_config: Option<RuntimeConfig>,
    features: HashSet<Feature>,
}

impl SystemBuilder {
    pub fn new() -> Self {
        Self {
            registry: ServiceRegistry::new(),
            config: None,
            runtime_config: None,
            features: HashSet::new(),
        }
    }
    
    pub fn with_config(mut self, config: SystemConfig) -> Self {
        self.config = Some(config);
        self
    }
    
    pub fn with_runtime(mut self, config: RuntimeConfig) -> Self {
        self.runtime_config = Some(config);
        self
    }
    
    pub fn enable_feature(mut self, feature: Feature) -> Self {
        self.features.insert(feature);
        self
    }
    
    pub fn register_service<T>(mut self, service: T) -> Self 
    where T: Send + Sync + 'static {
        self.registry.register(service).expect("Failed to register service");
        self
    }
    
    pub fn register_agent_factory<A>(mut self, factory: AgentFactory<A>) -> Self
    where A: Agent + 'static {
        self.registry.register_factory::<A, _>(factory)
            .expect("Failed to register agent factory");
        self
    }
    
    pub async fn build(mut self) -> Result<SystemCore, SystemError> {
        // Validate configuration
        let config = self.config.ok_or(SystemError::MissingConfiguration)?;
        let runtime_config = self.runtime_config.unwrap_or_default();
        
        // Register core services based on enabled features
        self.register_core_services(&config, &runtime_config)?;
        
        // Resolve all dependencies in correct order
        let runtime_manager = self.registry.resolve::<RuntimeManager>()?;
        let actor_system = self.registry.resolve::<ActorSystem>()?;
        let supervision_tree = self.registry.resolve::<SupervisionTree>()?;
        let event_bus = self.registry.resolve::<EventBus>()?;
        let configuration_manager = self.registry.resolve::<ConfigurationManager>()?;
        
        SystemCore::new(
            (*runtime_manager).clone(),
            (*actor_system).clone(),
            (*supervision_tree).clone(), 
            (*event_bus).clone(),
            (*configuration_manager).clone(),
        ).await
    }
    
    fn register_core_services(
        &mut self, 
        config: &SystemConfig, 
        runtime_config: &RuntimeConfig
    ) -> Result<(), ServiceError> {
        // Register runtime manager
        let runtime_manager = RuntimeManager::new(runtime_config.clone())?;
        self.registry.register(runtime_manager)?;
        
        // Register actor system
        let actor_system = ActorSystem::new(config.actor_config.clone())?;
        self.registry.register(actor_system)?;
        
        // Register other core services...
        
        Ok(())
    }
}
```

---

## 5. Module Dependency Graph with Explicit Imports

### 5.1 Dependency Visualization

```
Dependency Flow (← indicates "depends on"):

errors ← (foundation module, no dependencies)
config ← errors
events ← errors, config  
runtime ← config, monitoring, errors
monitoring ← events, errors
async_patterns ← runtime, errors, monitoring
resources ← config, monitoring, errors
transport ← events, errors
security ← config, errors
actor ← async_patterns, events, supervision, errors
supervision ← actor, events, monitoring, errors
tools ← agents, security, errors
agents ← actor, tools, supervision, config, errors
```

### 5.2 Explicit Module Imports

```rust
// src/runtime/mod.rs
use crate::{
    config::{Configuration, RuntimeConfig},
    monitoring::{HealthMonitor, MetricsCollector},
    errors::{SystemError, SystemResult, RuntimeError},
};
use tokio::runtime::{Runtime, Builder, Handle};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

// src/async_patterns/mod.rs
use crate::{
    runtime::RuntimeManager,
    errors::{SystemError, TaskError, PatternError},
    monitoring::{TaskMetrics, MetricsRegistry},
};
use tokio::{
    sync::{Semaphore, Mutex},
    task::JoinHandle,
    time::{timeout, Duration},
};
use futures::{Stream, Sink, StreamExt, SinkExt, Future};

// src/actor/mod.rs
use crate::{
    async_patterns::{TaskExecutor, TaskHandle},
    events::{EventBus, EventHandler, SystemEvent},
    supervision::{SupervisionStrategy, SupervisionResult},
    errors::{ActorError, SystemResult, MessageError},
};
use tokio::sync::{mpsc, oneshot, RwLock};
use std::collections::HashMap;

// src/supervision/mod.rs
use crate::{
    actor::{ActorSystem, ActorRef, ActorId},
    events::{EventBus, SystemEvent, SupervisionEvent},
    monitoring::{HealthCheck, FailureDetector, PhiAccrualDetector},
    errors::{SupervisionError, SystemResult, FailureReason},
};
use std::collections::{HashMap, BinaryHeap};
use tokio::time::Instant;

// src/events/mod.rs
use crate::{
    errors::{EventError, SystemResult, SerializationError},
    config::{EventConfig, DeadLetterConfig},
};
use serde::{Serialize, Deserialize};
use futures::future::join_all;
use std::collections::HashMap;

// src/tools/mod.rs
use crate::{
    agents::{Agent, AgentRole, AgentId},
    security::{PermissionSystem, AuthService},
    errors::{ToolError, SystemResult, PermissionError},
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use semver::Version;

// src/transport/mod.rs
use crate::{
    events::{EventBus, Message, MessageType},
    errors::{TransportError, SystemResult, RoutingError},
};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// src/config/mod.rs
use crate::{
    errors::{ConfigError, SystemResult, ValidationError},
    events::{EventHandler, ConfigurationEvent},
};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock, Mutex};
use notify::{Watcher, RecommendedWatcher};

// src/resources/mod.rs
use crate::{
    config::{Configuration, ResourceConfig},
    monitoring::{HealthCheck, ResourceMetrics, MetricsCollector},
    errors::{ResourceError, SystemResult, PoolError},
};
use tokio::sync::Mutex;
use std::collections::{VecDeque, HashMap};
use std::time::Duration;

// src/monitoring/mod.rs
use crate::{
    events::{EventBus, HealthEvent, MetricsEvent},
    errors::{MonitoringError, SystemResult, HealthError},
};
use tokio::time::{interval, Duration, Instant};
use prometheus::{Registry, Counter, Histogram, Gauge};

// src/agents/mod.rs
use crate::{
    actor::{Actor, ActorSystem, ActorRef},
    tools::{Tool, ToolBus, ToolCapabilities},
    supervision::{Supervisor, SupervisionStrategy},
    config::{AgentConfig, TeamConfig},
    errors::{AgentError, SystemResult, SpawnError},
};
use std::collections::HashMap;
use uuid::Uuid;

// src/security/mod.rs
use crate::{
    config::{SecurityConfig, AuthConfig},
    errors::{SecurityError, SystemResult, AuthError},
};
use tokio::sync::RwLock;
use ring::{digest, hmac};
use jwt::{Token, Claims};
```

---

## 6. Trait Object Specifications with Concrete Bounds

### 6.1 Dynamic Type Storage

```rust
/// Type-erased tool storage with complete trait bounds
pub type DynTool = Box<dyn Tool + Send + Sync + 'static>;
pub type DynAgent = Box<dyn Agent + Send + Sync + 'static>;
pub type DynActor<M> = Box<dyn Actor<Message = M> + Send + 'static>
where M: Message + Send + 'static;

/// Event handler with specific event constraints
pub type DynEventHandler<E> = Box<dyn EventHandler<Event = E> + Send + Sync + 'static>
where E: Event + Send + Sync + Clone + 'static;

/// Resource with lifecycle and error bounds
pub type DynResource = Box<dyn Resource<Config = ResourceConfig, Error = ResourceError> + Send + Sync + 'static>;

/// Health check with timeout constraints
pub type DynHealthCheck = Box<dyn HealthCheck + Send + Sync + 'static>;

/// Configuration with serialization bounds
pub type DynConfiguration = Box<dyn Configuration + Send + Sync + 'static>;

/// Supervisor with child type constraints
pub type DynSupervisor<C> = Box<dyn Supervisor<Child = C> + Send + Sync + 'static>
where C: Send + 'static;
```

### 6.2 Complex Generic Constraints and Lifetime Parameters

```rust
/// Connection pool with comprehensive resource constraints
pub struct ConnectionPool<R, C> 
where 
    R: Resource<Config = C> + Clone + Send + Sync + 'static,
    C: Send + Sync + Clone + 'static,
    R::Error: Send + Sync + 'static,
{
    pool: Arc<Mutex<VecDeque<R>>>,
    config: C,
    max_size: usize,
    min_size: usize,
    acquire_timeout: Duration,
    idle_timeout: Duration,
    health_check_interval: Duration,
    factory: Box<dyn Fn(C) -> BoxFuture<'static, Result<R, R::Error>> + Send + Sync>,
    metrics: PoolMetrics,
}

/// RAII wrapper for pooled resources with automatic cleanup
pub struct PooledResource<R: Resource> {
    resource: Option<R>,
    pool: Weak<Mutex<VecDeque<R>>>,
    acquired_at: Instant,
    metrics: Arc<PoolMetrics>,
}

impl<R: Resource> Drop for PooledResource<R> {
    fn drop(&mut self) {
        if let (Some(resource), Some(pool)) = (self.resource.take(), self.pool.upgrade()) {
            let duration = self.acquired_at.elapsed();
            self.metrics.record_usage_duration(duration);
            
            if let Ok(mut pool) = pool.try_lock() {
                if resource.is_healthy() && pool.len() < pool.capacity() {
                    pool.push_back(resource);
                    self.metrics.increment_returned();
                } else {
                    self.metrics.increment_discarded();
                    // Resource will be dropped here
                }
            }
        }
    }
}

/// Event bus with heterogeneous handler storage
pub struct EventBus<E> 
where E: Event + Send + Sync + Clone + 'static 
{
    handlers: HashMap<EventType, Vec<DynEventHandler<E>>>,
    dead_letter_queue: DeadLetterQueue<E>,
    serializer: Box<dyn EventSerializer<E> + Send + Sync>,
    metrics: EventMetrics,
    config: EventBusConfig,
}

/// Tool container with schema validation and capability tracking
pub struct ToolContainer {
    tools: HashMap<ToolId, DynTool>,
    schemas: HashMap<ToolId, ToolSchema>,
    capabilities: HashMap<ToolId, ToolCapabilities>,
    permissions: PermissionMatrix,
    metrics: ToolMetrics,
}

impl ToolContainer {
    pub fn register_tool<T>(&mut self, id: ToolId, tool: T) -> Result<(), ToolError>
    where T: Tool + 'static {
        // Validate schema
        let schema = tool.schema();
        schema.validate()?;
        
        // Extract capabilities
        let capabilities = tool.capabilities();
        
        // Store with type erasure
        self.tools.insert(id.clone(), Box::new(tool));
        self.schemas.insert(id.clone(), schema);
        self.capabilities.insert(id, capabilities);
        
        Ok(())
    }
    
    pub async fn execute_tool(&self, id: &ToolId, params: Value) -> Result<Value, ToolError> {
        let tool = self.tools.get(id).ok_or(ToolError::NotFound)?;
        let schema = self.schemas.get(id).ok_or(ToolError::SchemaNotFound)?;
        
        // Validate parameters against schema
        schema.validate_params(&params)?;
        
        // Execute with metrics
        let start = Instant::now();
        let result = tool.execute(params).await;
        self.metrics.record_execution(id, start.elapsed(), result.is_ok());
        
        result
    }
}
```

---

## 7. Cargo.toml Structure Preview

```toml
[package]
name = "mister-smith-framework"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
authors = ["Mister Smith AI Framework Team"]
description = "AI Agent Framework with Tokio-based async architecture, supervision trees, and tool integration"
license = "MIT OR Apache-2.0"
repository = "https://github.com/mister-smith/framework"
documentation = "https://docs.rs/mister-smith-framework"
keywords = ["ai", "agents", "async", "tokio", "supervision"]
categories = ["asynchronous", "development-tools"]
readme = "README.md"

[features]
default = ["runtime", "actors", "tools", "monitoring"]
full = [
    "default", "security", "encryption", "metrics", 
    "tracing", "persistence", "clustering"
]

# Core features
runtime = ["tokio/full"]
actors = ["dep:async-trait"]
tools = ["dep:serde_json", "dep:jsonschema"]
monitoring = ["dep:metrics", "dep:prometheus"]
supervision = ["dep:crossbeam-channel"]

# Security features  
security = ["dep:ring", "dep:jwt-simple"]
encryption = ["security", "dep:aes-gcm", "dep:chacha20poly1305"]
auth = ["security", "dep:oauth2", "dep:jsonwebtoken"]

# Observability features
metrics = ["dep:metrics", "dep:metrics-exporter-prometheus"]
tracing = [
    "dep:tracing", "dep:tracing-subscriber", 
    "dep:tracing-opentelemetry", "dep:opentelemetry"
]

# Storage and persistence
persistence = ["dep:sqlx", "dep:redis", "dep:sled"]
clustering = ["dep:raft", "dep:async-nats"]

# Development and testing
testing = ["dep:mockall", "dep:tokio-test", "dep:proptest"]
dev = ["testing", "dep:criterion", "dep:cargo-fuzz"]

[dependencies]
# Core async runtime
tokio = { version = "1.45", features = ["full"] }
futures = "0.3"
async-trait = { version = "0.1", optional = true }
pin-project = "1.1"

# Serialization and data structures
serde = { version = "1.0", features = ["derive"] }
serde_json = { version = "1.0", optional = true }
toml = "0.8"
jsonschema = { version = "0.18", optional = true }
semver = { version = "1.0", features = ["serde"] }

# Error handling and logging
thiserror = "1.0"
anyhow = "1.0"
tracing = { version = "0.1", optional = true }
tracing-subscriber = { version = "0.3", optional = true }

# Collections and utilities
indexmap = "2.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
once_cell = "1.19"
parking_lot = "0.12"
smallvec = "1.11"

# Concurrency primitives
crossbeam-channel = { version = "0.5", optional = true }
crossbeam-utils = "0.8"
atomic_float = "1.0"
dashmap = "5.5"

# Time and scheduling
chrono = { version = "0.4", features = ["serde"] }
cron = "0.12"

# Configuration management
config = "0.14"
notify = "6.0"
dirs = "5.0"

# HTTP client for tools
reqwest = { version = "0.12", features = ["json", "stream"] }
url = "2.4"

# Metrics and monitoring (optional)
metrics = { version = "0.23", optional = true }
prometheus = { version = "0.13", optional = true }
metrics-exporter-prometheus = { version = "0.15", optional = true }

# Security dependencies (optional)
ring = { version = "0.17", optional = true }
jwt-simple = { version = "0.12", optional = true }
aes-gcm = { version = "0.10", optional = true }
chacha20poly1305 = { version = "0.10", optional = true }

# Database and persistence (optional)
sqlx = { version = "0.7", optional = true, features = ["runtime-tokio-rustls"] }
redis = { version = "0.24", optional = true, features = ["tokio-comp"] }
sled = { version = "0.34", optional = true }

# Clustering and messaging (optional)
raft = { version = "0.7", optional = true }
async-nats = { version = "0.33", optional = true }

[dev-dependencies]
tokio-test = "0.4"
mockall = { version = "0.12", optional = true }
criterion = { version = "0.5", features = ["html_reports"], optional = true }
proptest = { version = "1.4", optional = true }
test-log = "0.2"
env_logger = "0.11"
wiremock = "0.6"
tempfile = "3.8"

[build-dependencies]
prost-build = "0.12"

# Benchmarks
[[bench]]
name = "actor_system"
harness = false
required-features = ["dev"]

[[bench]]
name = "task_executor"
harness = false  
required-features = ["dev"]

[[bench]]
name = "tool_bus"
harness = false
required-features = ["dev"]

[[bench]]
name = "event_bus"
harness = false
required-features = ["dev"]

# Examples
[[example]]
name = "basic_agent"
required-features = ["default"]

[[example]]
name = "multi_agent_system"
required-features = ["full"]

[[example]]
name = "tool_integration"
required-features = ["tools"]

# Performance profiles
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
opt-level = 3

[profile.bench]
debug = true
overflow-checks = false

[profile.test]
opt-level = 1

# Workspace configuration
[workspace]
members = [
    "examples/basic-agent",
    "examples/multi-agent-system", 
    "examples/tool-integration",
    "examples/supervision-patterns",
    "tools/agent-cli",
    "tools/framework-generator",
    "tools/config-validator",
    "benches",
]

[workspace.dependencies]
mister-smith-framework = { path = "." }
clap = { version = "4.0", features = ["derive"] }
serde_yaml = "0.9"
```

---

## 8. Implementation Guidelines

### 8.1 Module Initialization Order

```rust
/// Recommended system initialization sequence
async fn initialize_system() -> Result<SystemCore, SystemError> {
    // 1. Initialize error handling and logging
    init_error_handling()?;
    
    // 2. Load and validate configuration
    let config = load_system_config().await?;
    
    // 3. Initialize core services in dependency order
    let builder = SystemBuilder::new()
        .with_config(config)
        .enable_feature(Feature::Monitoring)
        .enable_feature(Feature::Security);
    
    // 4. Register core services
    let system = builder
        .register_runtime_services()
        .register_actor_services()
        .register_tool_services()
        .register_monitoring_services()
        .build()
        .await?;
    
    // 5. Start system components
    system.start().await?;
    
    Ok(system)
}
```

### 8.2 Best Practices for Type Safety

1. **Always use explicit generic constraints**: Never rely on inference for public APIs
2. **Implement proper Drop semantics**: Ensure resources are cleaned up correctly  
3. **Use Arc<T> for shared ownership**: Avoid cloning expensive resources
4. **Prefer trait objects for heterogeneous collections**: Use `Box<dyn Trait>` appropriately
5. **Validate configurations at compile time**: Use type system to prevent runtime errors

### 8.3 Error Handling Patterns

```rust
/// Custom result type for system operations
pub type SystemResult<T> = Result<T, SystemError>;

/// Error hierarchy with proper context
#[derive(Error, Debug)]
pub enum SystemError {
    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),
    
    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigError),
    
    #[error("Actor system error: {0}")]
    Actor(#[from] ActorError),
    
    #[error("Tool execution error: {0}")]
    Tool(#[from] ToolError),
    
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),
}
```

---

## Summary

This comprehensive module organization and type system specification provides:

- **Complete src/ directory structure** with clear separation of concerns
- **Type-safe trait hierarchy** with proper generic constraints and lifetime parameters
- **Dependency injection architecture** enabling modular composition
- **Explicit module dependencies** showing exact import relationships
- **Trait object specifications** for dynamic dispatch with concrete bounds
- **Cargo.toml structure** supporting optional features and development workflows

The design enables autonomous developers to:
1. Understand exact project structure and file organization
2. Implement type-safe components with proper constraints  
3. Use dependency injection for modular architecture
4. Follow established patterns for error handling and resource management
5. Extend the framework through well-defined interfaces

This specification serves as the authoritative reference for implementing the Mister Smith AI Agent Framework with Rust best practices and type safety guarantees.