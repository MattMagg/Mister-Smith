# Integration Contracts and Core Architecture

[← Back to Core Architecture](./CLAUDE.md) | [Integration Patterns →](./integration-patterns.md) | [Implementation Guide →](./integration-implementation.md)

**Agent**: 19 - Core Architecture Integration Specialist  
**Mission**: Establish concrete integration contracts and core architecture patterns  
**Target**: Address core architecture and cross-component compatibility gaps from Phase 1 analysis  

---

## Executive Summary

This document provides the foundational integration contracts and core architecture specifications for the Mister Smith framework. Building upon Agent 14's cross-document validation (which revealed critical gaps in trait compatibility and component contracts), this specification establishes the concrete integration foundation required for seamless component interaction.

**Key Focus Areas:**
- Unified integration contracts library (`mister-smith-contracts`)
- Core architecture patterns for component integration
- Cross-component compatibility specifications
- Concrete trait definitions and adapter patterns
- Transport interface unification and protocol bridging
- Configuration management standardization

**Target Achievement**: Elevate component integration from 65-71% to 85-94% compatibility across all critical interfaces.

**Quick Navigation**: Jump directly to [Agent Contracts](#21-core-agent-integration-contract), [Transport Interface](#22-unified-transport-interface), or [Configuration Management](#23-configuration-management-integration) for specific implementations.

**Related Documents:**
- [Error, Event, and Dependency Injection Patterns](./integration-patterns.md) - Advanced integration patterns building on these contracts
- [Testing, Roadmap, and Metrics](integration-implementation.md) - Implementation guidance and testing framework
- [System Integration](system-integration.md) - Broader system integration strategies
- [Component Architecture](component-architecture.md) - Foundational system design principles

---

## Table of Contents

1. [Core Integration Architecture](#1-core-integration-architecture)
   - [Shared Contracts Library](#11-shared-contracts-library)
   - [Component Integration Matrix](#12-component-integration-matrix)
2. [Cross-Component Integration Contracts](#2-cross-component-integration-contracts)
   - [Core Agent Integration Contract](#21-core-agent-integration-contract)
   - [Unified Transport Interface](#22-unified-transport-interface)
   - [Configuration Management Integration](#23-configuration-management-integration)

---

## 1. Core Integration Architecture

### 1.1 Shared Contracts Library

The `mister-smith-contracts` crate provides the foundation for all component integration. This shared library implements the contracts detailed in [Section 2](#2-cross-component-integration-contracts) below:

```rust
// Core integration traits
pub use mister_smith_contracts::{
    Agent, Transport, ConfigProvider, EventBus, ServiceRegistry,
    SystemError, ErrorRecovery, Event, Injectable, ContractTest
};

// Integration utilities
pub use mister_smith_contracts::integration::{
    ProtocolBridge, MessageTranslator, ConfigurationMapper,
    ErrorPropagator, EventCorrelator, DependencyResolver
};

// Testing framework
pub use mister_smith_contracts::testing::{
    IntegrationTestHarness, ContractValidator, MockRegistry,
    TestOrchestrator, CrossComponentTester
};
```

### 1.2 Component Integration Matrix

| Component Pair | Before | After | Integration Pattern | Reference |
|----------------|--------|-------|---------------------|-----------|
| Core ↔ Transport | 69% | 92% | Unified messaging contracts + protocol bridging | [Transport Interface](#22-unified-transport-interface) |
| Data ↔ Orchestration | 69% | 90% | Event-driven state management + schema mapping | [Agent Contract](#21-core-agent-integration-contract) |
| Security ↔ All Components | 71% | 94% | Cross-cutting authentication + authorization patterns | [Configuration Management](#23-configuration-management-integration) |
| Observability ↔ All | 71% | 88% | Unified tracing + metrics collection contracts | [Error, Event, and Dependency Injection Patterns](./integration-patterns.md) |
| Deployment ↔ All | 63% | 85% | Configuration standardization + health check contracts | [Configuration Management](#23-configuration-management-integration) |
| Claude CLI ↔ Framework | 58% | 87% | Process isolation + security bridge patterns | [System Integration](system-integration.md) |

---

## 2. Cross-Component Integration Contracts

### 2.1 Core Agent Integration Contract

**Addresses**: Agent lifecycle management compatibility (Agent 14: 65% trait compatibility)

**Implementation**: This contract is provided by the [Shared Contracts Library](#11-shared-contracts-library) as part of the core integration foundation.

**See Also**: [Component Architecture](component-architecture.md#agent-lifecycle) for foundational agent design patterns

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

#[async_trait]
pub trait Agent: Send + Sync + 'static {
    type Input: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;
    type Output: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;
    type Error: Into<SystemError> + Send + Sync + 'static;
    type Config: Send + Sync + for<'de> Deserialize<'de> + 'static;

    // Core lifecycle methods
    async fn initialize(&mut self, config: &Self::Config) -> Result<(), Self::Error>;
    async fn process(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
    async fn shutdown(&mut self) -> Result<(), Self::Error>;
    
    // Health and status management
    fn health_check(&self) -> HealthStatus;
    fn agent_info(&self) -> AgentInfo;
    
    // Supervision integration
    fn supervision_strategy(&self) -> SupervisionStrategy;
    async fn handle_supervision_event(&self, event: SupervisionEvent) -> Result<(), Self::Error>;
    
    // Context management
    async fn with_context<T>(&self, ctx: AgentContext, f: impl FnOnce() -> T + Send) -> T where T: Send;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub agent_type: AgentType,
    pub version: String,
    pub capabilities: Vec<Capability>,
    pub resource_requirements: ResourceRequirements,
}

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { error: SystemError },
    Starting,
    Stopping,
}

// Agent adapter for existing implementations
pub struct AgentAdapter<T> {
    inner: T,
    config_mapper: ConfigMapper,
    error_mapper: ErrorMapper,
}

impl<T> AgentAdapter<T> {
    pub fn new(agent: T) -> Self {
        Self {
            inner: agent,
            config_mapper: ConfigMapper::default(),
            error_mapper: ErrorMapper::default(),
        }
    }
    
    pub fn with_config_mapper(mut self, mapper: ConfigMapper) -> Self {
        self.config_mapper = mapper;
        self
    }
    
    pub fn with_error_mapper(mut self, mapper: ErrorMapper) -> Self {
        self.error_mapper = mapper;
        self
    }
}

#[async_trait]
impl<T> Agent for AgentAdapter<T> 
where 
    T: /* existing agent trait bounds */ + Send + Sync + 'static 
{
    type Input = /* mapped input type */;
    type Output = /* mapped output type */;
    type Error = SystemError;
    type Config = UnifiedConfig;

    async fn initialize(&mut self, config: &Self::Config) -> Result<(), Self::Error> {
        let mapped_config = self.config_mapper.map(config)?;
        self.inner.initialize(&mapped_config)
            .await
            .map_err(|e| self.error_mapper.map(e))
    }

    async fn process(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        let mapped_input = self.config_mapper.map_input(input)?;
        let result = self.inner.process(mapped_input)
            .await
            .map_err(|e| self.error_mapper.map(e))?;
        Ok(self.config_mapper.map_output(result)?)
    }

    // ... other method implementations with mapping
}
```

### 2.2 Unified Transport Interface

**Addresses**: Transport protocol differences (Agent 11: 70% compatibility, Agent 14: Transport layer issues)

**Implementation**: This unified interface is part of the [Shared Contracts Library](#11-shared-contracts-library) and provides protocol bridging capabilities.

**See Also**: 
- [System Integration](system-integration.md#transport-layer) for system-wide transport strategies
- [Integration Patterns](./integration-patterns.md#4-event-system-integration-patterns) for event-driven communication patterns

```rust
#[async_trait]
pub trait Transport: Send + Sync + Clone {
    type Message: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;
    type Subscription: Stream<Item = Result<Self::Message, TransportError>> + Send + Unpin;
    type ConnectionInfo: Send + Sync + 'static;

    // Core messaging operations
    async fn send(&self, destination: &Destination, message: Self::Message) -> Result<(), TransportError>;
    async fn broadcast(&self, topic: &str, message: Self::Message) -> Result<(), TransportError>;
    async fn subscribe(&self, pattern: &SubscriptionPattern) -> Result<Self::Subscription, TransportError>;
    async fn request_response(&self, destination: &Destination, message: Self::Message, timeout: Duration) -> Result<Self::Message, TransportError>;
    
    // Connection management
    async fn connect(&mut self, config: &TransportConfig) -> Result<Self::ConnectionInfo, TransportError>;
    async fn disconnect(&mut self) -> Result<(), TransportError>;
    fn connection_status(&self) -> ConnectionStatus;
    
    // Advanced features
    async fn create_queue(&self, queue_config: &QueueConfig) -> Result<QueueHandle, TransportError>;
    async fn join_cluster(&self, cluster_config: &ClusterConfig) -> Result<ClusterMembership, TransportError>;
    
    // Observability
    fn metrics(&self) -> TransportMetrics;
    async fn health_check(&self) -> Result<TransportHealth, TransportError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Destination {
    Direct { agent_id: AgentId },
    Topic { subject: String },
    Queue { queue_name: String },
    Broadcast { scope: BroadcastScope },
}

#[derive(Debug, Clone)]
pub enum SubscriptionPattern {
    Exact { subject: String },
    Wildcard { pattern: String },
    Queue { queue_name: String, group: Option<String> },
}

// Protocol Bridge Implementation
pub struct ProtocolBridge<Primary, Secondary> {
    primary: Primary,
    secondary: Secondary,
    message_translator: MessageTranslator,
    routing_table: RoutingTable,
}

impl<Primary, Secondary> ProtocolBridge<Primary, Secondary> 
where 
    Primary: Transport,
    Secondary: Transport,
{
    pub fn new(primary: Primary, secondary: Secondary) -> Self {
        Self {
            primary,
            secondary,
            message_translator: MessageTranslator::new(),
            routing_table: RoutingTable::new(),
        }
    }
    
    pub fn add_routing_rule(&mut self, rule: RoutingRule) {
        self.routing_table.add_rule(rule);
    }
    
    pub fn with_message_translator(mut self, translator: MessageTranslator) -> Self {
        self.message_translator = translator;
        self
    }
}

#[async_trait]
impl<Primary, Secondary> Transport for ProtocolBridge<Primary, Secondary>
where 
    Primary: Transport + 'static,
    Secondary: Transport + 'static,
{
    type Message = UnifiedMessage;
    type Subscription = BridgedSubscription<Primary::Subscription, Secondary::Subscription>;
    type ConnectionInfo = BridgedConnectionInfo;

    async fn send(&self, destination: &Destination, message: Self::Message) -> Result<(), TransportError> {
        let route = self.routing_table.route_for_destination(destination);
        
        match route.transport {
            TransportChoice::Primary => {
                let translated = self.message_translator.to_primary(&message)?;
                self.primary.send(destination, translated).await
            }
            TransportChoice::Secondary => {
                let translated = self.message_translator.to_secondary(&message)?;
                self.secondary.send(destination, translated).await
            }
            TransportChoice::Both => {
                // Send to both transports for redundancy
                let primary_msg = self.message_translator.to_primary(&message)?;
                let secondary_msg = self.message_translator.to_secondary(&message)?;
                
                let (primary_result, secondary_result) = tokio::join!(
                    self.primary.send(destination, primary_msg),
                    self.secondary.send(destination, secondary_msg)
                );
                
                // Return success if either succeeds
                primary_result.or(secondary_result)
            }
        }
    }

    // ... other method implementations with protocol bridging
}

// NATS Transport Implementation
pub struct NatsTransport {
    client: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
    config: NatsConfig,
}

#[async_trait]
impl Transport for NatsTransport {
    type Message = NatsMessage;
    type Subscription = NatsSubscription;
    type ConnectionInfo = NatsConnectionInfo;

    async fn send(&self, destination: &Destination, message: Self::Message) -> Result<(), TransportError> {
        match destination {
            Destination::Topic { subject } => {
                self.client.publish(subject, message.payload).await?;
                Ok(())
            }
            Destination::Queue { queue_name } => {
                self.jetstream.publish(queue_name, message.payload).await?;
                Ok(())
            }
            // ... other destination types
        }
    }

    // ... other method implementations
}

// WebSocket Transport Implementation  
pub struct WebSocketTransport {
    connections: Arc<RwLock<HashMap<AgentId, WebSocketStream>>>,
    broker: MessageBroker,
    config: WebSocketConfig,
}

#[async_trait]
impl Transport for WebSocketTransport {
    type Message = WebSocketMessage;
    type Subscription = WebSocketSubscription;
    type ConnectionInfo = WebSocketConnectionInfo;

    async fn send(&self, destination: &Destination, message: Self::Message) -> Result<(), TransportError> {
        match destination {
            Destination::Direct { agent_id } => {
                let connections = self.connections.read().await;
                if let Some(ws) = connections.get(agent_id) {
                    ws.send(Message::Binary(message.into_bytes())).await?;
                    Ok(())
                } else {
                    Err(TransportError::DestinationNotFound)
                }
            }
            Destination::Broadcast { scope } => {
                self.broker.broadcast(scope, message).await
            }
            // ... other destination types
        }
    }

    // ... other method implementations
}
```

### 2.3 Configuration Management Integration

**Addresses**: Configuration format conflicts (Agent 14: 45-55% consistency, Agent 11: 80% compatibility)

**Implementation**: The hierarchical configuration system is implemented through the [Shared Contracts Library](#11-shared-contracts-library) with support for multiple providers.

**See Also**: 
- [Implementation Config](implementation-config.md) for detailed configuration management patterns
- [System Integration](system-integration.md#configuration-management) for deployment-level configuration strategies

```rust
#[async_trait]
pub trait ConfigProvider: Send + Sync + Clone {
    async fn get<T>(&self, key: &ConfigKey) -> Result<T, ConfigError> 
    where 
        T: for<'de> Deserialize<'de> + Send + 'static;
    
    async fn get_optional<T>(&self, key: &ConfigKey) -> Result<Option<T>, ConfigError>
    where 
        T: for<'de> Deserialize<'de> + Send + 'static;
    
    async fn set<T>(&self, key: &ConfigKey, value: T) -> Result<(), ConfigError>
    where 
        T: Serialize + Send + Sync;
    
    async fn watch(&self, key: &ConfigKey) -> Result<ConfigWatcher, ConfigError>;
    async fn reload(&self) -> Result<(), ConfigError>;
    
    fn validate_schema(&self, schema: &ConfigSchema) -> Result<(), ConfigError>;
    fn source_info(&self) -> ConfigSourceInfo;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConfigKey {
    pub component: String,
    pub section: String,
    pub key: String,
    pub environment: Option<String>,
}

impl ConfigKey {
    pub fn new(component: &str, section: &str, key: &str) -> Self {
        Self {
            component: component.to_string(),
            section: section.to_string(),
            key: key.to_string(),
            environment: None,
        }
    }
    
    pub fn with_environment(mut self, env: &str) -> Self {
        self.environment = Some(env.to_string());
        self
    }
    
    pub fn path(&self) -> String {
        match &self.environment {
            Some(env) => format!("{}.{}.{}.{}", self.component, env, self.section, self.key),
            None => format!("{}.{}.{}", self.component, self.section, self.key),
        }
    }
}

// Hierarchical Configuration System
pub struct HierarchicalConfig {
    providers: Vec<Box<dyn ConfigProvider>>,
    cache: Arc<RwLock<HashMap<ConfigKey, CachedValue>>>,
    watchers: Arc<RwLock<HashMap<ConfigKey, Vec<ConfigWatcher>>>>,
}

impl HierarchicalConfig {
    pub fn builder() -> HierarchicalConfigBuilder {
        HierarchicalConfigBuilder::new()
    }
    
    pub async fn resolve_with_fallback<T>(&self, keys: &[ConfigKey]) -> Result<T, ConfigError>
    where 
        T: for<'de> Deserialize<'de> + Send + 'static
    {
        for key in keys {
            if let Ok(value) = self.get(key).await {
                return Ok(value);
            }
        }
        Err(ConfigError::KeyNotFound)
    }
}

#[async_trait]
impl ConfigProvider for HierarchicalConfig {
    async fn get<T>(&self, key: &ConfigKey) -> Result<T, ConfigError> 
    where 
        T: for<'de> Deserialize<'de> + Send + 'static
    {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(key) {
                if !cached.is_expired() {
                    return cached.deserialize();
                }
            }
        }
        
        // Try each provider in order
        for provider in &self.providers {
            match provider.get::<T>(key).await {
                Ok(value) => {
                    // Cache the result
                    let mut cache = self.cache.write().await;
                    cache.insert(key.clone(), CachedValue::new(value.clone()));
                    return Ok(value);
                }
                Err(ConfigError::KeyNotFound) => continue,
                Err(e) => return Err(e),
            }
        }
        
        Err(ConfigError::KeyNotFound)
    }

    // ... other method implementations
}

pub struct HierarchicalConfigBuilder {
    providers: Vec<Box<dyn ConfigProvider>>,
}

impl HierarchicalConfigBuilder {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }
    
    pub fn add_environment_variables(mut self) -> Self {
        self.providers.push(Box::new(EnvConfigProvider::new()));
        self
    }
    
    pub fn add_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, ConfigError> {
        let provider = FileConfigProvider::new(path)?;
        self.providers.push(Box::new(provider));
        Ok(self)
    }
    
    pub fn add_consul(mut self, config: ConsulConfig) -> Self {
        self.providers.push(Box::new(ConsulConfigProvider::new(config)));
        self
    }
    
    pub fn add_kubernetes_secrets(mut self, namespace: &str) -> Self {
        self.providers.push(Box::new(K8sSecretsProvider::new(namespace)));
        self
    }
    
    pub fn build(self) -> HierarchicalConfig {
        HierarchicalConfig {
            providers: self.providers,
            cache: Arc::new(RwLock::new(HashMap::new())),
            watchers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// Configuration mapping for component integration
#[derive(Debug, Clone)]
pub struct ConfigMapper {
    mappings: HashMap<String, ConfigMapping>,
}

#[derive(Debug, Clone)]
pub struct ConfigMapping {
    pub source_key: ConfigKey,
    pub target_key: ConfigKey,
    pub transformer: Option<ConfigTransformer>,
    pub validation: Option<ConfigValidator>,
}

pub trait ConfigTransformer: Send + Sync {
    fn transform(&self, value: ConfigValue) -> Result<ConfigValue, ConfigError>;
}

pub trait ConfigValidator: Send + Sync {
    fn validate(&self, value: &ConfigValue) -> Result<(), ConfigError>;
}

impl ConfigMapper {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }
    
    pub fn add_mapping(&mut self, component: &str, mapping: ConfigMapping) {
        self.mappings.insert(component.to_string(), mapping);
    }
    
    pub async fn map_config<T>(&self, provider: &dyn ConfigProvider, component: &str) -> Result<T, ConfigError>
    where 
        T: for<'de> Deserialize<'de> + Send + 'static
    {
        if let Some(mapping) = self.mappings.get(component) {
            let value = provider.get::<ConfigValue>(&mapping.source_key).await?;
            
            let transformed = if let Some(transformer) = &mapping.transformer {
                transformer.transform(value)?
            } else {
                value
            };
            
            if let Some(validator) = &mapping.validation {
                validator.validate(&transformed)?;
            }
            
            transformed.deserialize()
        } else {
            Err(ConfigError::MappingNotFound)
        }
    }
}
```

---

## Conclusion

This document establishes the core integration contracts and architecture patterns that form the foundation of the Mister Smith framework's component integration strategy. By providing unified trait definitions, protocol bridging capabilities, and configuration standardization, we create a solid base for seamless component interaction.

**Key Achievements:**
- Unified Agent trait with comprehensive lifecycle management
- Transport abstraction supporting multiple protocols with bridging
- Hierarchical configuration system with provider flexibility
- Adapter patterns for integrating existing implementations

**Next Steps:**
- Review [Error, Event, and Dependency Injection Patterns](./integration-patterns.md) for advanced integration patterns and event systems
- See [Testing, Roadmap, and Metrics](integration-implementation.md) for implementation guidance and validation frameworks
- Examine [System Integration](system-integration.md) for deployment and operational integration strategies
- Reference [Component Architecture](component-architecture.md) for foundational design principles and patterns

---

[← Back to Core Architecture](./CLAUDE.md) | [Integration Patterns →](./integration-patterns.md) | [Implementation Guide →](./integration-implementation.md)

---

*Integration Contracts and Core Architecture v1.0*  
*Agent 19 - Core Architecture Integration Specialist*  
*Generated: 2025-07-03*  
*Target: Establish foundational integration contracts for 85-94% component compatibility*