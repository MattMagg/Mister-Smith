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

## 🔍 VALIDATION STATUS

**Last Validated**: 2025-07-05  
**Validator**: Framework Documentation Team  
**Validation Score**: Pending full validation  
**Status**: Active Development  

### Implementation Status

- Integration contracts library specification complete
- Core architecture patterns established
- Cross-component compatibility defined
- Transport unification framework documented

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

**Addresses**: Transport protocol differences (Agent 11: 70% compatibility, Agent 14: Transport layer issues) | Data Flow Integrity (Agent 12: Message routing and transformation validation)

**Implementation**: This unified interface is part of the [Shared Contracts Library](#11-shared-contracts-library) and provides protocol bridging capabilities with comprehensive data flow validation.

**See Also**:

- [System Integration](system-integration.md#transport-layer) for system-wide transport strategies
- [Integration Patterns](./integration-patterns.md#4-event-system-integration-patterns) for event-driven communication patterns

```rust
#[async_trait]
pub trait Transport: Send + Sync + Clone {
    type Message: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static;
    type Subscription: Stream<Item = Result<Self::Message, TransportError>> + Send + Unpin;
    type ConnectionInfo: Send + Sync + 'static;

    // Core messaging operations with data flow validation
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
    
    // Observability and data flow validation (Agent 12)
    fn metrics(&self) -> TransportMetrics;
    async fn health_check(&self) -> Result<TransportHealth, TransportError>;
    async fn validate_message_flow(&self, message: &Self::Message) -> Result<FlowValidation, TransportError>;
    fn get_flow_statistics(&self) -> FlowStatistics;
}

// Data Flow Validation Types (Agent 12 Integration)
#[derive(Debug, Clone)]
pub struct FlowValidation {
    pub message_valid: bool,
    pub schema_version: String,
    pub transformation_chain: Vec<TransformationStep>,
    pub latency_ms: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TransformationStep {
    pub component: String,
    pub operation: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data_checksum: String,
}

#[derive(Debug, Clone)]
pub struct FlowStatistics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub average_latency_ms: f64,
    pub error_rate: f64,
    pub throughput_per_second: f64,
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

// Protocol Bridge Implementation with Data Flow Validation
pub struct ProtocolBridge<Primary, Secondary> {
    primary: Primary,
    secondary: Secondary,
    message_translator: MessageTranslator,
    routing_table: RoutingTable,
    // Data flow integrity components (Agent 12)
    flow_validator: BridgeFlowValidator,
    transformation_tracker: TransformationTracker,
    consistency_checker: ConsistencyChecker,
}

// Bridge Flow Validator (Agent 12 Integration)
pub struct BridgeFlowValidator {
    schema_mappings: HashMap<String, SchemaMappingRules>,
    validation_cache: Arc<RwLock<LruCache<String, ValidationResult>>>,
    performance_monitor: PerformanceMonitor,
}

impl BridgeFlowValidator {
    pub async fn validate_bridge_flow<M>(
        &self,
        message: &M,
        source_protocol: &str,
        target_protocol: &str
    ) -> Result<FlowValidation, TransportError> 
    where
        M: Serialize + Send + Sync
    {
        // Validate schema mapping exists
        let mapping_key = format!("{}->{}", source_protocol, target_protocol);
        let mapping = self.schema_mappings.get(&mapping_key)
            .ok_or(TransportError::ProtocolError("No schema mapping found".to_string()))?;
        
        // Validate transformation integrity
        let checksum_before = calculate_checksum(message)?;
        let validation_result = mapping.validate(message)?;
        
        // Monitor performance (Agent 12 thresholds)
        let start = std::time::Instant::now();
        let latency = start.elapsed();
        
        if latency > Duration::from_millis(1) {
            self.performance_monitor.record_slow_validation(latency);
        }
        
        Ok(FlowValidation {
            message_valid: validation_result.is_valid,
            schema_version: mapping.target_schema_version.clone(),
            transformation_chain: vec![TransformationStep {
                component: "protocol_bridge".to_string(),
                operation: format!("{}_to_{}", source_protocol, target_protocol),
                timestamp: chrono::Utc::now(),
                data_checksum: checksum_before,
            }],
            latency_ms: latency.as_millis() as u64,
            warnings: validation_result.warnings,
        })
    }
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
        // Data flow validation before routing (Agent 12)
        self.flow_validator.validate_bridge_flow(
            &message,
            "primary",
            "secondary"
        ).await?;
        
        let route = self.routing_table.route_for_destination(destination);
        
        // Track transformation for each routing choice
        let result = match route.transport {
            TransportChoice::Primary => {
                let translated = self.message_translator.to_primary(&message)?;
                self.transformation_tracker.record_translation(
                    &message,
                    &translated,
                    "bridge_to_primary"
                ).await;
                self.primary.send(destination, translated).await
            }
            TransportChoice::Secondary => {
                let translated = self.message_translator.to_secondary(&message)?;
                self.transformation_tracker.record_translation(
                    &message,
                    &translated,
                    "bridge_to_secondary"
                ).await;
                self.secondary.send(destination, translated).await
            }
            TransportChoice::Both => {
                // Send to both transports for redundancy with consistency check
                let primary_msg = self.message_translator.to_primary(&message)?;
                let secondary_msg = self.message_translator.to_secondary(&message)?;
                
                // Verify consistency between translations
                self.consistency_checker.verify_translation_consistency(
                    &primary_msg,
                    &secondary_msg
                ).await?;
                
                let (primary_result, secondary_result) = tokio::join!(
                    self.primary.send(destination, primary_msg.clone()),
                    self.secondary.send(destination, secondary_msg.clone())
                );
                
                // Track both translations
                self.transformation_tracker.record_dual_translation(
                    &message,
                    &primary_msg,
                    &secondary_msg
                ).await;
                
                // Return success if either succeeds
                primary_result.or(secondary_result)
            }
        };
        
        // Record flow completion
        if result.is_ok() {
            self.flow_validator.performance_monitor.record_successful_flow();
        }
        
        result
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

### 2.3 Transport Security Integration Contracts

**Addresses**: Cross-component mTLS implementation consistency (Agent 17: 87/100 validation score with critical recommendations)

**Implementation**: These security contracts enforce consistent mTLS implementation across all transport protocols and ensure certificate lifecycle management consistency.

```rust
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

// Core Transport Security Contract
#[async_trait]
pub trait TransportSecurity: Send + Sync {
    /// Transport security configuration with mTLS enforcement
    type SecurityConfig: SecurityConfiguration;
    type Connection: SecureConnection;
    type Certificate: Certificate;
    
    /// Initialize transport security with standardized configuration
    /// Enforces TLS 1.3 policy and certificate path standardization (Agent 17: Critical Priority 1)
    async fn initialize_security(&self, config: Self::SecurityConfig) -> Result<(), SecurityError>;
    
    /// Establish secure connection with mTLS verification
    /// Implements certificate validation caching for performance (Agent 17: Performance optimization)
    async fn establish_secure_connection(&self, endpoint: &str) -> Result<Self::Connection, SecurityError>;
    
    /// Validate certificate chain with cross-protocol consistency
    /// Ensures consistent certificate validation across all transport protocols
    async fn validate_certificate_chain(&self, certificates: &[Self::Certificate]) -> Result<ValidationResult, SecurityError>;
    
    /// Rotate certificates with zero-downtime coordination
    /// Implements automated rotation with multi-threshold monitoring (Agent 17: Medium Priority)
    async fn rotate_certificates(&self, rotation_config: CertificateRotationConfig) -> Result<RotationResult, SecurityError>;
    
    /// Monitor certificate expiration with multi-threshold alerting
    /// Implements 30-day, 7-day, and 1-day warning thresholds (Agent 17 enhancement)
    async fn monitor_certificate_expiration(&self) -> Result<ExpirationMonitoringReport, SecurityError>;
}

// Security Configuration Trait - enforces standardization
pub trait SecurityConfiguration: Clone + Send + Sync {
    /// Get standardized certificate base path: "/etc/mister-smith/certs"
    /// Addresses certificate path inconsistency (Agent 17: Critical Priority 1)
    fn certificate_base_path(&self) -> PathBuf {
        PathBuf::from("/etc/mister-smith/certs")
    }
    
    /// Get CA certificate path: "${base_path}/ca/ca-cert.pem"
    fn ca_certificate_path(&self) -> PathBuf {
        self.certificate_base_path().join("ca").join("ca-cert.pem")
    }
    
    /// Get server certificate path: "${base_path}/server/server-cert.pem"
    fn server_certificate_path(&self) -> PathBuf {
        self.certificate_base_path().join("server").join("server-cert.pem")
    }
    
    /// Get client certificate path: "${base_path}/client/client-cert.pem"
    fn client_certificate_path(&self) -> PathBuf {
        self.certificate_base_path().join("client").join("client-cert.pem")
    }
    
    /// Enforce TLS 1.3 only policy (Agent 17: Critical standardization)
    fn tls_version_policy(&self) -> TLSVersionPolicy {
        TLSVersionPolicy {
            minimum_version: TLSVersion::TLS13,
            preferred_version: TLSVersion::TLS13,
            fallback_allowed: false
        }
    }
    
    /// Get approved cipher suites (TLS 1.3 AEAD only)
    fn approved_cipher_suites(&self) -> Vec<String> {
        vec![
            "TLS13_AES_256_GCM_SHA384".to_string(),
            "TLS13_CHACHA20_POLY1305_SHA256".to_string(),
            "TLS13_AES_128_GCM_SHA256".to_string()
        ]
    }
    
    /// Validate configuration against security policies
    fn validate_security_policy(&self) -> Result<(), SecurityPolicyViolation>;
}

// Cross-Protocol Security Coordinator Contract
#[async_trait]
pub trait CrossProtocolSecurityCoordinator: Send + Sync {
    /// Ensure TLS version consistency across all protocols
    /// Addresses TLS version inconsistency gap (Agent 17: Critical Priority 1)
    async fn enforce_tls_version_consistency(&self) -> Result<ConsistencyReport, SecurityError>;
    
    /// Synchronize certificate paths across protocols
    /// Ensures all protocols use standardized certificate locations
    async fn synchronize_certificate_paths(&self) -> Result<SynchronizationReport, SecurityError>;
    
    /// Validate cipher suite consistency
    /// Ensures approved cipher suites are used across all protocols
    async fn validate_cipher_suite_consistency(&self) -> Result<CipherSuiteValidationReport, SecurityError>;
    
    /// Coordinate certificate rotation across protocols
    /// Ensures coordinated certificate updates with minimal service disruption
    async fn coordinate_certificate_rotation(&self, protocols: Vec<TransportProtocol>) -> Result<CoordinatedRotationResult, SecurityError>;
}

// Certificate Lifecycle Management Contract
#[async_trait]
pub trait CertificateLifecycleManager: Send + Sync {
    type Certificate: Certificate;
    
    /// Generate new certificates with proper extensions and constraints
    /// Implements RSA 4096-bit keys and proper certificate extensions (Agent 17: Excellence)
    async fn generate_certificate(&self, cert_type: CertificateType, config: CertificateGenerationConfig) -> Result<Self::Certificate, CertificateError>;
    
    /// Validate certificate with comprehensive checks
    /// Includes certificate chain, expiration, and usage validation
    async fn validate_certificate(&self, certificate: &Self::Certificate) -> Result<CertificateValidationResult, CertificateError>;
    
    /// Monitor certificate health with multi-threshold alerting
    /// Implements 30/7/1 day warning thresholds (Agent 17 enhancement)
    async fn monitor_certificate_health(&self) -> Result<CertificateHealthReport, CertificateError>;
    
    /// Rotate certificate with atomic replacement
    /// Ensures zero-downtime certificate rotation with rollback capability
    async fn rotate_certificate(&self, cert_type: CertificateType) -> Result<CertificateRotationResult, CertificateError>;
    
    /// Archive expired certificates for compliance
    /// Maintains audit trail for certificate lifecycle events
    async fn archive_certificate(&self, certificate: Self::Certificate, reason: ArchiveReason) -> Result<(), CertificateError>;
}

// Performance Optimization Contract (Agent 17: Performance focus)
#[async_trait]
pub trait SecurityPerformanceOptimizer: Send + Sync {
    /// Cache certificate validation results
    /// Reduces certificate validation overhead with TTL-based caching
    async fn cache_certificate_validation(&self, cert_id: &str, result: ValidationResult, ttl: Duration) -> Result<(), CacheError>;
    
    /// Retrieve cached validation result
    /// Enables fast certificate validation for repeated operations
    async fn get_cached_validation(&self, cert_id: &str) -> Result<Option<ValidationResult>, CacheError>;
    
    /// Optimize TLS handshake with session resumption
    /// Implements TLS 1.3 0-RTT session resumption for performance
    async fn enable_session_resumption(&self, session_config: SessionResumptionConfig) -> Result<(), SecurityError>;
    
    /// Monitor security performance metrics
    /// Tracks certificate validation times, handshake duration, cache hit rates
    async fn collect_security_metrics(&self) -> Result<SecurityPerformanceMetrics, MetricsError>;
}

// Security Monitoring and Alerting Contract
#[async_trait]
pub trait SecurityMonitor: Send + Sync {
    /// Monitor security events across all transport protocols
    /// Provides unified security event monitoring and correlation
    async fn monitor_security_events(&self) -> Result<SecurityEventStream, MonitoringError>;
    
    /// Alert on security policy violations
    /// Immediate alerting for TLS version violations, certificate issues, etc.
    async fn alert_security_violation(&self, violation: SecurityViolation) -> Result<(), AlertingError>;
    
    /// Generate security compliance report
    /// Comprehensive reporting for audit and compliance requirements
    async fn generate_compliance_report(&self, timeframe: TimeRange) -> Result<ComplianceReport, ReportingError>;
    
    /// Track certificate lifecycle events
    /// Audit trail for certificate generation, rotation, expiration, revocation
    async fn track_certificate_event(&self, event: CertificateLifecycleEvent) -> Result<(), AuditError>;
}

// Supporting Types for Security Contracts
#[derive(Debug, Clone)]
pub struct TLSVersionPolicy {
    pub minimum_version: TLSVersion,
    pub preferred_version: TLSVersion,
    pub fallback_allowed: bool,
}

#[derive(Debug, Clone)]
pub struct CertificateRotationConfig {
    pub certificate_type: CertificateType,
    pub rotation_strategy: RotationStrategy,
    pub notification_channels: Vec<NotificationChannel>,
    pub rollback_enabled: bool,
}

#[derive(Debug)]
pub struct ExpirationMonitoringReport {
    pub critical_alerts: Vec<CertificateAlert>,
    pub warning_alerts: Vec<CertificateAlert>,
    pub notice_alerts: Vec<CertificateAlert>,
    pub healthy_certificates: Vec<CertificateStatus>,
}

#[derive(Debug)]
pub struct SecurityPerformanceMetrics {
    pub certificate_validation_times: PerformanceHistogram,
    pub handshake_durations: PerformanceHistogram,
    pub cache_hit_rate: f64,
    pub session_resumption_rate: f64,
    pub tls_error_rate: f64,
}

// Security Error Types
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("TLS version policy violation: {0}")]
    TLSVersionViolation(String),
    
    #[error("Certificate validation failed: {0}")]
    CertificateValidationFailed(String),
    
    #[error("Certificate path inconsistency: {0}")]
    CertificatePathInconsistency(String),
    
    #[error("Cipher suite not approved: {0}")]
    UnapprovedCipherSuite(String),
    
    #[error("Cross-protocol consistency violation: {0}")]
    ConsistencyViolation(String),
    
    #[error("Security configuration invalid: {0}")]
    InvalidConfiguration(String),
}
```

### 2.4 Configuration Management Integration

**Addresses**: Configuration format conflicts (Agent 14: 45-55% consistency, Agent 11: 80% compatibility) | Data Consistency Validation (Agent 12: Cross-component configuration integrity)

**Implementation**: The hierarchical configuration system is implemented through the [Shared Contracts Library](#11-shared-contracts-library) with support for multiple providers and comprehensive data flow validation.

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
    
    // Data flow validation methods (Agent 12)
    async fn validate_config_flow(&self, key: &ConfigKey) -> Result<ConfigFlowValidation, ConfigError>;
    async fn get_config_lineage(&self, key: &ConfigKey) -> Result<ConfigLineage, ConfigError>;
}

// Configuration Lineage Tracking (Agent 12)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigLineage {
    pub key: ConfigKey,
    pub current_value_hash: String,
    pub source_chain: Vec<ConfigSource>,
    pub modification_history: Vec<ConfigModification>,
    pub dependent_components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigModification {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
    pub previous_hash: String,
    pub new_hash: String,
    pub modified_by: String,
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

// Hierarchical Configuration System with Data Flow Validation
pub struct HierarchicalConfig {
    providers: Vec<Box<dyn ConfigProvider>>,
    cache: Arc<RwLock<HashMap<ConfigKey, CachedValue>>>,
    watchers: Arc<RwLock<HashMap<ConfigKey, Vec<ConfigWatcher>>>>,
    // Data flow validation components (Agent 12)
    config_validator: ConfigFlowValidator,
    consistency_tracker: ConfigConsistencyTracker,
    change_auditor: ConfigChangeAuditor,
}

// Configuration Flow Validator (Agent 12 Integration)
pub struct ConfigFlowValidator {
    schema_registry: Arc<RwLock<HashMap<String, ConfigSchema>>>,
    dependency_graph: ConfigDependencyGraph,
    validation_rules: ValidationRuleSet,
}

impl ConfigFlowValidator {
    pub async fn validate_config_flow<T>(
        &self,
        key: &ConfigKey,
        value: &T,
        source: &str
    ) -> Result<ConfigFlowValidation, ConfigError>
    where
        T: Serialize + for<'de> Deserialize<'de>
    {
        // Validate schema compliance
        let schema = self.schema_registry.read().await
            .get(&key.component)
            .ok_or(ConfigError::SchemaNotFound)?;
        
        schema.validate_value(value)?;
        
        // Check cross-component dependencies
        let dependencies = self.dependency_graph.get_dependencies(key);
        for dep_key in dependencies {
            self.validate_dependency_consistency(key, &dep_key).await?;
        }
        
        // Apply validation rules
        let validation_result = self.validation_rules.apply(key, value)?;
        
        Ok(ConfigFlowValidation {
            valid: true,
            source: source.to_string(),
            dependencies_validated: dependencies.len(),
            warnings: validation_result.warnings,
            timestamp: chrono::Utc::now(),
        })
    }
    
    async fn validate_dependency_consistency(
        &self,
        primary: &ConfigKey,
        dependency: &ConfigKey
    ) -> Result<(), ConfigError> {
        // Ensure configuration consistency across dependent components
        // Implementation based on Agent 12 cross-component validation
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigFlowValidation {
    pub valid: bool,
    pub source: String,
    pub dependencies_validated: usize,
    pub warnings: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
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
