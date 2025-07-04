---
title: tokio-runtime-architecture
type: implementation
permalink: core-architecture/tokio-runtime
tags:
- '#rust #tokio #runtime #async #implementation'
---

# Tokio Runtime Architecture

**Navigation**: [Home](../../README.md) > [Core Architecture](./CLAUDE.md) > Tokio Runtime Architecture

**Quick Links**: [Async Patterns](async-patterns.md) | [Supervision Trees](supervision-trees.md) | [Component Architecture](component-architecture.md)

Implementation-ready Rust specifications for the Mister Smith AI Agent Framework's Tokio runtime system. This module provides concrete implementations for runtime configuration and lifecycle management using Tokio's async runtime.

## Dependencies

```toml
[package]
name = "mister-smith-core"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.45.1", features = ["full"] }
futures = "0.3"
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
dashmap = "6.0"
num_cpus = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
metrics = "0.23"
```

## Core Error Types

```rust
// src/errors.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SystemError {
    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),
    #[error("Supervision error: {0}")]
    Supervision(#[from] SupervisionError),
    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigError),
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    #[error("Persistence error: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("Actor system error: {0}")]
    Actor(#[from] ActorError),
    #[error("Task execution error: {0}")]
    Task(#[from] TaskError),
    #[error("Stream processing error: {0}")]
    Stream(#[from] StreamError),
    #[error("Event system error: {0}")]
    Event(#[from] EventError),
    #[error("Tool system error: {0}")]
    Tool(#[from] ToolError),
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Failed to build runtime: {0}")]
    BuildFailed(#[from] std::io::Error),
    #[error("Runtime startup failed: {0}")]
    StartupFailed(String),
    #[error("Runtime shutdown failed: {0}")]
    ShutdownFailed(String),
    #[error("Runtime configuration invalid: {0}")]
    ConfigurationInvalid(String),
}

// Error severity for system-wide error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    Retry { max_attempts: u32, delay: std::time::Duration },
    Restart,
    Escalate,
    Reload,
    CircuitBreaker,
    Failover,
    Ignore,
}

impl SystemError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            SystemError::Runtime(_) => ErrorSeverity::Critical,
            SystemError::Supervision(_) => ErrorSeverity::High,
            SystemError::Configuration(_) => ErrorSeverity::Medium,
            SystemError::Resource(_) => ErrorSeverity::Medium,
            SystemError::Network(_) => ErrorSeverity::Low,
            SystemError::Persistence(_) => ErrorSeverity::High,
            SystemError::Actor(_) => ErrorSeverity::Medium,
            SystemError::Task(_) => ErrorSeverity::Low,
            SystemError::Stream(_) => ErrorSeverity::Medium,
            SystemError::Event(_) => ErrorSeverity::Low,
            SystemError::Tool(_) => ErrorSeverity::Low,
        }
    }
    
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            SystemError::Runtime(_) => RecoveryStrategy::Restart,
            SystemError::Supervision(_) => RecoveryStrategy::Escalate,
            SystemError::Configuration(_) => RecoveryStrategy::Reload,
            SystemError::Resource(_) => RecoveryStrategy::Retry { 
                max_attempts: 3, 
                delay: std::time::Duration::from_millis(1000) 
            },
            SystemError::Network(_) => RecoveryStrategy::CircuitBreaker,
            SystemError::Persistence(_) => RecoveryStrategy::Failover,
            _ => RecoveryStrategy::Retry { 
                max_attempts: 1, 
                delay: std::time::Duration::from_millis(100) 
            },
        }
    }
}
```

## 1. Tokio Runtime Architecture

### 1.1 Core Runtime Configuration

```rust
// src/core/runtime.rs
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use tokio::runtime::Runtime;
use serde::{Serialize, Deserialize};

// Runtime constants
pub const DEFAULT_WORKER_THREADS: usize = num_cpus::get();
pub const DEFAULT_MAX_BLOCKING_THREADS: usize = 512;
pub const DEFAULT_THREAD_KEEP_ALIVE: Duration = Duration::from_secs(60);
pub const DEFAULT_THREAD_STACK_SIZE: usize = 2 * 1024 * 1024; // 2MB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub worker_threads: Option<usize>,
    pub max_blocking_threads: usize,
    pub thread_keep_alive: Duration,
    pub thread_stack_size: Option<usize>,
    pub enable_all: bool,
    pub enable_time: bool,
    pub enable_io: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: None, // Uses Tokio default
            max_blocking_threads: DEFAULT_MAX_BLOCKING_THREADS,
            thread_keep_alive: DEFAULT_THREAD_KEEP_ALIVE,
            thread_stack_size: Some(DEFAULT_THREAD_STACK_SIZE),
            enable_all: true,
            enable_time: true,
            enable_io: true,
        }
    }
}

impl RuntimeConfig {
    pub fn build_runtime(&self) -> Result<Runtime, RuntimeError> {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        
        if let Some(worker_threads) = self.worker_threads {
            builder.worker_threads(worker_threads);
        }
        
        builder
            .max_blocking_threads(self.max_blocking_threads)
            .thread_keep_alive(self.thread_keep_alive);
            
        if let Some(stack_size) = self.thread_stack_size {
            builder.thread_stack_size(stack_size);
        }
        
        if self.enable_all {
            builder.enable_all();
        } else {
            if self.enable_time {
                builder.enable_time();
            }
            if self.enable_io {
                builder.enable_io();
            }
        }
        
        builder.build().map_err(RuntimeError::BuildFailed)
    }
}
```

### 1.2 Runtime Lifecycle Management

```rust
// src/core/runtime.rs (continued)
use crate::supervision::SupervisionTree;
use crate::events::EventBus;
use crate::resources::health::{HealthMonitor, MetricsCollector};
use crate::errors::{RuntimeError, SystemError};
use tokio::signal;
use tokio::task::JoinHandle;
use tracing::{info, warn, error};

pub const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct RuntimeManager {
    runtime: Arc<Runtime>,
    shutdown_signal: Arc<AtomicBool>,
    health_monitor: Arc<HealthMonitor>,
    metrics_collector: Arc<MetricsCollector>,
    supervision_tree: Arc<SupervisionTree>,
    event_bus: Arc<EventBus>,
    tasks: Vec<JoinHandle<()>>,
}

impl RuntimeManager {
    pub fn initialize(config: RuntimeConfig) -> Result<Self, RuntimeError> {
        let runtime = Arc::new(config.build_runtime()?);
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        let health_monitor = Arc::new(HealthMonitor::new());
        let metrics_collector = Arc::new(MetricsCollector::new());
        let supervision_tree = Arc::new(SupervisionTree::new());
        let event_bus = Arc::new(EventBus::new());
        
        Ok(Self {
            runtime,
            shutdown_signal,
            health_monitor,
            metrics_collector,
            supervision_tree,
            event_bus,
            tasks: Vec::new(),
        })
    }
    
    pub async fn start_system(&mut self) -> Result<(), RuntimeError> {
        info!("Starting runtime system components");
        
        // Start health monitoring
        let health_task = {
            let monitor = Arc::clone(&self.health_monitor);
            let shutdown = Arc::clone(&self.shutdown_signal);
            self.runtime.spawn(async move {
                monitor.run(shutdown).await;
            })
        };
        
        // Start metrics collection
        let metrics_task = {
            let collector = Arc::clone(&self.metrics_collector);
            let shutdown = Arc::clone(&self.shutdown_signal);
            self.runtime.spawn(async move {
                collector.run(shutdown).await;
            })
        };
        
        // Start supervision tree
        let supervision_task = {
            let tree = Arc::clone(&self.supervision_tree);
            let shutdown = Arc::clone(&self.shutdown_signal);
            self.runtime.spawn(async move {
                if let Err(e) = tree.start(shutdown).await {
                    error!("Supervision tree failed: {}", e);
                }
            })
        };
        
        // Start signal handler
        let signal_task = {
            let shutdown = Arc::clone(&self.shutdown_signal);
            self.runtime.spawn(async move {
                Self::signal_handler(shutdown).await;
            })
        };
        
        self.tasks.extend([health_task, metrics_task, supervision_task, signal_task]);
        
        info!("Runtime system started successfully");
        Ok(())
    }
    
    pub async fn graceful_shutdown(self) -> Result<(), RuntimeError> {
        info!("Initiating graceful shutdown");
        
        // Signal shutdown to all components
        self.shutdown_signal.store(true, Ordering::SeqCst);
        
        // Shutdown supervision tree first
        if let Err(e) = self.supervision_tree.shutdown().await {
            warn!("Error during supervision tree shutdown: {}", e);
        }
        
        // Flush metrics
        if let Err(e) = self.metrics_collector.flush().await {
            warn!("Error flushing metrics: {}", e);
        }
        
        // Wait for all tasks to complete
        for task in self.tasks {
            if let Err(e) = task.await {
                warn!("Task failed during shutdown: {}", e);
            }
        }
        
        // Shutdown runtime with timeout
        self.runtime.shutdown_timeout(DEFAULT_SHUTDOWN_TIMEOUT);
        
        info!("Graceful shutdown completed");
        Ok(())
    }
    
    async fn signal_handler(shutdown_signal: Arc<AtomicBool>) {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
            .expect("Failed to register SIGINT handler");
        
        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating shutdown");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, initiating shutdown");
            }
        }
        
        shutdown_signal.store(true, Ordering::SeqCst);
    }
    
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }
}
```

## External Dependencies

This module references the following components from other modules in the framework:

- `crate::supervision::SupervisionTree` - From [supervision-trees.md](supervision-trees.md)
- `crate::events::EventBus` - From event-system.md *(requires extraction)*
- `crate::resources::health::{HealthMonitor, MetricsCollector}` - From resource-management.md *(requires extraction)*
- Additional error types (`SupervisionError`, `ConfigError`, etc.) - From their respective modules

## Cross-References

### Internal Framework References
- **Async Patterns**: [async-patterns.md](async-patterns.md) - Async programming patterns used with this runtime
- **Supervision Trees**: [supervision-trees.md](supervision-trees.md) - Supervision tree integration
- **Component Architecture**: [component-architecture.md](component-architecture.md) - How runtime fits into overall component design
- **System Integration**: [system-integration.md](system-integration.md) - Runtime integration with other system components

### Related Architecture Documents
- **System Architecture**: [system-architecture.md](system-architecture.md) - Complete system design overview
- **Type Definitions**: [type-definitions.md](type-definitions.md) - Core type system used by runtime
- **Integration Patterns**: [integration-patterns.md](./integration-patterns.md) - Patterns for integrating with runtime

## Navigation

- **Up**: [Core Architecture](./CLAUDE.md)
- **Previous**: [Component Architecture](component-architecture.md)
- **Next**: [Async Patterns](async-patterns.md)
- **Related**: [Supervision Trees](supervision-trees.md)

---

*Extracted from system-architecture.md - Mister Smith AI Agent Framework*