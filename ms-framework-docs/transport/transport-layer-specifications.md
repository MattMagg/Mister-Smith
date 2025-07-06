---
title: transport-layer-specifications-revised
type: note
permalink: revision-swarm/transport/transport-layer-specifications-revised
---

# Transport Layer Specifications - Foundation Patterns
## Agent Implementation Framework

> **Canonical Reference**: See `/Users/mac-main/Mister-Smith/Mister-Smith/tech-framework.md` for authoritative technology stack specifications

As stated in the canonical source:
> "This is the authoritative source for the Claude-Flow tech stack. All implementations should reference this document."

## 🔍 VALIDATION STATUS

**Implementation Readiness**: 93% ✅ - Production ready with minor enhancement opportunities

**Validation Details**:
- **Validator**: Agent 25 - MS Framework Validation Swarm
- **Validation Date**: 2025-07-05
- **Score**: 14/15 - Excellent implementation readiness

**Key Findings**:
- ✅ All three protocols (NATS, gRPC, HTTP) fully specified
- ✅ Advanced connection management with enterprise features
- ✅ Comprehensive error handling and resilience patterns
- ✅ Performance optimization strategies clearly defined
- ✅ Security and data layer integration properly specified
- ✅ Testing specifications comprehensive across all protocols

**Critical Issues**: Minor gaps in integration testing guidance
- Missing chaos engineering scenarios for protocol failures
- Need end-to-end latency testing across all layers
- Resource exhaustion testing under extreme loads needed

**Minor Enhancements**: 
- Adaptive configuration based on load metrics
- Enhanced routing algorithms with ML-based optimization
- Cross-protocol metrics correlation for unified monitoring

Reference: `/Users/mac-main/Mister-Smith/MisterSmith/validation-swarm/batch5-specialized-domains/agent25-transport-layer-validation.md`

## Overview

This document defines foundational transport patterns for agent communication using the Claude-Flow Rust Stack. Focus is on basic communication patterns suitable for learning distributed systems.

**Technology Stack** (from tech-framework.md):
- async-nats 0.34
- Tonic 0.11 (gRPC)
- Axum 0.8 (HTTP)
- Tokio 1.38 (async runtime)

## 1. Basic Message Transport Patterns

### 1.1 Request-Response Pattern

```pseudocode
PATTERN RequestResponse:
    AGENT sends REQUEST to TARGET_AGENT
    TARGET_AGENT processes REQUEST
    TARGET_AGENT returns RESPONSE to AGENT
    
    Properties:
        - Synchronous communication
        - Direct addressing
        - Guaranteed response or timeout
```

### 1.2 Publish-Subscribe Pattern

```pseudocode
PATTERN PublishSubscribe:
    PUBLISHER_AGENT publishes MESSAGE to TOPIC
    ALL SUBSCRIBER_AGENTS on TOPIC receive MESSAGE
    
    Properties:
        - Asynchronous broadcast
        - Topic-based routing
        - Multiple consumers
```

### 1.3 Queue Group Pattern

```pseudocode
PATTERN QueueGroup:
    PRODUCER_AGENT sends TASK to QUEUE
    ONE WORKER_AGENT from GROUP receives TASK
    
    Properties:
        - Load balancing
        - Work distribution
        - Single consumer per message
```

### 1.4 Blackboard Pattern

```pseudocode
PATTERN Blackboard:
    AGENTS write/read from SHARED_KNOWLEDGE_SPACE
    ALL AGENTS can observe changes to BLACKBOARD
    
    Properties:
        - Shared memory coordination
        - Decoupled collaboration
        - Knowledge persistence
        
    Implementation:
        - Shared store with versioning
        - Watch notifications on changes
        - Concurrent access control
```

## 2. NATS Messaging Foundations

### 2.1 Basic NATS Subjects

```pseudocode
SUBJECT_HIERARCHY:
    agents.{agent_id}.commands    # Direct agent commands
    agents.{agent_id}.status      # Agent status updates
    tasks.{task_type}.queue       # Task distribution
    events.{event_type}           # System events
    cmd.{type}.{target}           # Command routing

    # Claude-CLI Hook System Integration
    control.startup               # CLI initialization and capabilities
    agent.{id}.pre                # Pre-task hook processing
    agent.{id}.post               # Post-task hook processing
    agent.{id}.error              # Error hook handling
    agent.{id}.hook_response      # Hook mutation responses
    ctx.{gid}.file_change         # File change notifications

WILDCARD_PATTERNS:
    agents.*.status               # All agent statuses (single token)
    tasks.>                       # All task queues (multiple tokens)
    task.*                        # All task types
    agent.*.pre                   # All pre-task hooks
    agent.*.post                  # All post-task hooks
    agent.*.error                 # All error hooks
    ctx.*.file_change             # All file change events

TOPIC_CONVENTIONS:
    task.created                  # Task lifecycle events
    task.assigned
    task.completed
    agent.ready                   # Agent state transitions
    agent.busy
    system.metrics                # System observability

    # Hook Lifecycle Events
    hook.startup.triggered        # Startup hook execution
    hook.pre_task.triggered       # Pre-task hook execution
    hook.post_task.triggered      # Post-task hook execution
    hook.on_error.triggered       # Error hook execution
    hook.on_file_change.triggered # File change hook execution
```

### 2.1.1 Claude CLI Hook Message Formats

```pseudocode
HOOK_EVENT_MESSAGE_FORMAT:
    {
        "hook_type": "startup" | "pre_task" | "post_task" | "on_error" | "on_file_change",
        "agent_id": "string",
        "tool_name": "optional_string",
        "tool_input": "optional_json_object",
        "tool_response": "optional_json_object",
        "session_info": {
            "agent_id": "string",
            "session_id": "string",
            "model": "string",
            "start_time": "iso8601_timestamp"
        },
        "timestamp": "iso8601_timestamp",
        "context_id": "optional_string"
    }

HOOK_RESPONSE_MESSAGE_FORMAT:
    {
        "decision": "approve" | "block" | "continue",
        "reason": "optional_string",
        "continue": "boolean",
        "stop_reason": "optional_string",
        "modifications": "optional_json_object"
    }

TASK_OUTPUT_MESSAGE_FORMAT:
    {
        "task_info": {
            "type": "agent_id" | "description",
            "value": "string_or_number"
        },
        "output_line": "string",
        "task_status": "running" | "completed" | "failed",
        "timestamp": "iso8601_timestamp",
        "agent_id": "string"
    }
```

### 2.2 NATS Performance Characteristics

```pseudocode
PERFORMANCE_BENCHMARKS:
    CORE_NATS:
        throughput: 3+ million msgs/sec
        latency: microseconds to low milliseconds
        delivery: at-most-once (fire-and-forget)
        
    JETSTREAM:
        throughput: ~200k msgs/sec (fsync overhead)
        latency: P99 low milliseconds
        delivery: at-least-once with acknowledgments
        failure_mode: Publishers drop if fsync overwhelmed
```

### 2.3 JetStream Persistence

```pseudocode
STREAM_CONFIGURATION:
    CREATE STREAM "agent-events"
        subjects: ["agents.*.events"]
        storage: file
        retention: limits
        max_age: 7_days
        max_bytes: configurable
        discard_policy: old_on_full
        
    CREATE CONSUMER "event-processor"
        stream: "agent-events"
        deliver: all
        ack_policy: explicit
        
    RESOURCE_LIMITS:
        max_memory: 512M
        max_disk: 1G
        max_streams: 10
        max_consumers: 100
```

### 2.4 Basic Message Flow

```pseudocode
AGENT_COMMUNICATION_FLOW:
    1. SENDER creates MESSAGE
    2. MESSAGE includes:
        - agent_id
        - message_type
        - payload
        - timestamp
    3. SENDER publishes to SUBJECT
    4. NATS routes to SUBSCRIBERS
    5. RECEIVERS process MESSAGE
    
MESSAGE_SCHEMAS:
    TaskAssignment:
        task_id: string
        task: Task
        deadline: optional<Duration>
        
    StatusUpdate:
        agent_id: string
        status: AgentStatus
        capacity: float
        
    ResultNotification:
        task_id: string
        result: TaskOutput
        metrics: ExecutionMetrics
        
    CapabilityQuery:
        required_capabilities: List<Capability>
        
    CapabilityResponse:
        agent_id: string
        capabilities: List<Capability>
        availability: Availability
```

## 3. gRPC Service Patterns

### 3.1 Basic Service Definition

```pseudocode
SERVICE AgentCommunication:
    METHOD send_message(request) -> response
    METHOD get_status(agent_id) -> status
    METHOD list_agents() -> agent_list
    
MESSAGE TYPES:
    - Simple request/response
    - Status queries
    - List operations
```

### 3.2 Streaming Patterns

```pseudocode
STREAMING_PATTERNS:
    
    SERVER_STREAMING:
        CLIENT requests updates
        SERVER streams responses
        Use case: Status monitoring
        
    CLIENT_STREAMING:
        CLIENT streams requests
        SERVER returns summary
        Use case: Batch operations
        
    BIDIRECTIONAL_STREAMING:
        Both stream concurrently
        Use case: Real-time chat
```

## 4. HTTP API Patterns

### 4.1 RESTful Endpoints

```pseudocode
API_STRUCTURE:
    GET  /agents              # List agents
    POST /agents              # Register agent
    GET  /agents/{id}         # Get agent details
    POST /agents/{id}/message # Send message
    
    GET  /tasks               # List tasks
    POST /tasks               # Create task
    GET  /tasks/{id}          # Get task status
```

### 4.2 WebSocket Communication

```pseudocode
WEBSOCKET_PROTOCOL:
    CONNECTION:
        Client connects to /ws
        Server accepts connection
        
    MESSAGE_FORMAT:
        type: "request" | "response" | "event"
        action: string
        payload: data
        
    PATTERNS:
        - Event notification
        - Real-time updates
        - Bidirectional messaging
```

## 5. Transport Abstraction

### 5.1 Generic Transport Interface

```pseudocode
INTERFACE Transport:
    connect(config)
    disconnect()
    send(message) -> response
    subscribe(topic, handler)
    
IMPLEMENTATIONS:
    - NatsTransport
    - GrpcTransport
    - HttpTransport
```

### 5.2 Message Routing

```pseudocode
ROUTING_LOGIC:
    RECEIVE message
    EXTRACT destination
    LOOKUP transport for destination
    FORWARD using appropriate protocol
    
FALLBACK:
    If primary transport fails
    Try secondary transport
    Log routing decision
```

## 6. Agent Communication Patterns

### 6.1 Core Agent Message Interfaces

```pseudocode
AGENT_INTERFACES:
    Planner:
        create_plan(goal) -> TaskList
        refine_plan(feedback) -> TaskList
        
    Executor:
        execute_task(task) -> TaskOutput
        can_execute(task_type) -> bool
        
    Critic:
        evaluate(output, criteria) -> Feedback
        validate_plan(plan) -> ValidationResult
        
    Router:
        route_task(task) -> AgentId
        balance_load(tasks) -> TaskAssignments
        
    Memory:
        store(key, value, metadata) -> Result
        retrieve(key) -> Option<Value>
        query(pattern) -> List<Results>
```

### 6.2 Coordination Topologies

```pseudocode
CENTRALIZED_PATTERN:
    ORCHESTRATOR maintains global_state
    ORCHESTRATOR assigns tasks to AGENTS
    AGENTS report results to ORCHESTRATOR
    
HIERARCHICAL_PATTERN:
    PARENT delegates to CHILDREN
    CHILDREN report to PARENT
    Multiple levels of delegation
    
PEER_TO_PEER_PATTERN:
    AGENTS communicate directly
    No central coordinator
    Self-organizing behavior
```

## 7. Advanced Connection Management

### 7.1 Connection Pool Architecture

```pseudocode
INTERFACE ConnectionPoolManager {
    create_pool(config: PoolConfig) -> Pool
    monitor_health() -> HealthMetrics
    handle_backpressure() -> BackpressureAction
    scale_connections(metrics: LoadMetrics) -> ScalingDecision
}

CLASS AdvancedConnectionPool {
    PRIVATE config: PoolConfiguration
    PRIVATE connections: ConnectionSet
    PRIVATE health_monitor: HealthMonitor
    PRIVATE metrics_collector: MetricsCollector
    
    STRUCT PoolConfiguration {
        max_connections: Integer = 10
        min_connections: Integer = 2
        acquire_timeout: Duration = 30_seconds
        idle_timeout: Duration = 600_seconds
        max_lifetime: Duration = 3600_seconds
        health_check_interval: Duration = 30_seconds
        connection_recycling: RecyclingStrategy = FAST
    }
}
```

### 7.2 Protocol-Specific Pool Configurations

```pseudocode
-- NATS Connection Pool with Deadpool Pattern
CLASS NatsConnectionPool IMPLEMENTS ConnectionPoolManager {
    FUNCTION create_nats_pool(config: NatsConfig) -> NatsPool {
        pool_config = {
            max_connections: config.max_connections,
            reconnect_strategy: ExponentialBackoff {
                initial_delay: Duration.millis(100),
                max_delay: Duration.seconds(5),
                max_attempts: 10
            },
            connection_timeout: Duration.seconds(10),
            subscription_capacity: 1000,
            jetstream_domain: config.jetstream_domain
        }
        
        RETURN create_managed_pool(pool_config)
    }
    
    FUNCTION configure_nats_connection(conn: NatsConnection) -> Result {
        -- Post-connection configuration
        conn.set_subscription_capacity(1000)
        conn.configure_jetstream(js_config)
        conn.enable_heartbeat(Duration.seconds(30))
        RETURN Success()
    }
}

-- PostgreSQL Connection Pool (coordinated with data layer)
CLASS PostgresConnectionPool IMPLEMENTS ConnectionPoolManager {
    FUNCTION create_postgres_pool(config: PgConfig) -> PgPool {
        pool_config = {
            max_connections: config.max_connections,
            min_connections: config.min_connections,
            acquire_timeout: Duration.seconds(30),
            idle_timeout: Duration.minutes(10),
            max_lifetime: Duration.hours(2),
            after_connect: configure_session
        }
        
        RETURN deadpool_postgres.create_pool(pool_config)
    }
    
    FUNCTION configure_session(conn: PgConnection) -> Result {
        -- Session-level configuration
        conn.execute("SET application_name = 'agent_transport'")
        conn.execute("SET statement_timeout = '30s'")
        conn.execute("SET idle_in_transaction_session_timeout = '60s'")
        RETURN Success()
    }
}

-- gRPC Connection Pool
CLASS GrpcConnectionPool IMPLEMENTS ConnectionPoolManager {
    FUNCTION create_grpc_pool(config: GrpcConfig) -> GrpcPool {
        pool_config = {
            max_connections: config.max_connections,
            keep_alive_interval: Duration.seconds(60),
            keep_alive_timeout: Duration.seconds(5),
            connect_timeout: Duration.seconds(10),
            http2_flow_control: true,
            max_message_size: 4_MB
        }
        
        RETURN create_grpc_pool_with_config(pool_config)
    }
}
```

### 7.3 Connection String Templates and Configuration

```pseudocode
CLASS ConnectionStringManager {
    ENUM ProtocolType {
        NATS,
        POSTGRESQL,
        GRPC,
        HTTP
    }
    
    FUNCTION build_connection_string(
        protocol: ProtocolType, 
        config: Map<String, String>
    ) -> String {
        SWITCH protocol {
            CASE NATS:
                RETURN build_nats_url(config)
            CASE POSTGRESQL:
                RETURN build_postgres_url(config)
            CASE GRPC:
                RETURN build_grpc_url(config)
            CASE HTTP:
                RETURN build_http_url(config)
        }
    }
    
    FUNCTION build_nats_url(config: Map<String, String>) -> String {
        -- Support multiple formats:
        -- nats://user:pass@host:4222
        -- nats://host1:4222,host2:4222,host3:4222 (cluster)
        -- tls://user:pass@host:4222 (secure)
        
        IF config.contains("cluster_hosts") THEN
            hosts = config.get("cluster_hosts").split(",")
            RETURN "nats://" + hosts.join(",")
        ELSE
            user_pass = build_auth_string(config)
            host_port = config.get("host", "localhost") + ":" + config.get("port", "4222")
            protocol = config.get("tls", "false") == "true" ? "tls" : "nats"
            RETURN protocol + "://" + user_pass + host_port
        END IF
    }
    
    FUNCTION build_postgres_url(config: Map<String, String>) -> String {
        -- Support multiple formats:
        -- postgres://user:pass@host:port/database
        -- postgres://%2Fvar%2Frun%2Fpostgresql/database (Unix socket)
        -- postgres://host/db?application_name=agent&sslmode=require
        
        IF config.contains("socket_path") THEN
            encoded_path = url_encode(config.get("socket_path"))
            database = config.get("database", "postgres")
            RETURN "postgres://" + encoded_path + "/" + database
        ELSE
            user_pass = build_auth_string(config)
            host_port = config.get("host", "localhost") + ":" + config.get("port", "5432")
            database = config.get("database", "postgres")
            query_params = build_query_params(config)
            RETURN "postgres://" + user_pass + host_port + "/" + database + query_params
        END IF
    }
}

-- Environment-based configuration loading
CLASS EnvironmentConfigLoader {
    FUNCTION load_nats_config() -> Map<String, String> {
        config = Map()
        config.put("host", env.get("NATS_HOST", "localhost"))
        config.put("port", env.get("NATS_PORT", "4222"))
        config.put("user", env.get("NATS_USER", ""))
        config.put("password", env.get("NATS_PASSWORD", ""))
        config.put("cluster_hosts", env.get("NATS_CLUSTER", ""))
        config.put("tls", env.get("NATS_TLS", "false"))
        RETURN config
    }
    
    FUNCTION load_postgres_config() -> Map<String, String> {
        config = Map()
        config.put("host", env.get("PG_HOST", "localhost"))
        config.put("port", env.get("PG_PORT", "5432"))
        config.put("user", env.get("PG_USER", "postgres"))
        config.put("password", env.get("PG_PASSWORD", ""))
        config.put("database", env.get("PG_DATABASE", "postgres"))
        config.put("socket_path", env.get("PG_SOCKET_PATH", ""))
        config.put("application_name", env.get("PG_APP_NAME", "agent_transport"))
        config.put("sslmode", env.get("PG_SSL_MODE", "prefer"))
        RETURN config
    }
}
```

### 7.4 Advanced Health Checking and Recovery

```pseudocode
CLASS AdvancedHealthMonitor {
    PRIVATE health_checkers: Map<ProtocolType, HealthChecker>
    PRIVATE circuit_breakers: Map<String, CircuitBreaker>
    PRIVATE recovery_strategies: Map<ProtocolType, RecoveryStrategy>
    
    INTERFACE HealthChecker {
        check_health() -> HealthStatus
        recover_connection() -> RecoveryResult
        get_metrics() -> HealthMetrics
    }
    
    CLASS NatsHealthChecker IMPLEMENTS HealthChecker {
        FUNCTION check_health() -> HealthStatus {
            TRY {
                start_time = now()
                response = connection.ping()
                latency = now() - start_time
                
                IF latency > Duration.seconds(1) THEN
                    RETURN HealthStatus.DEGRADED
                ELSE
                    RETURN HealthStatus.HEALTHY
                END IF
            } CATCH (error) {
                RETURN HealthStatus.UNHEALTHY
            }
        }
        
        FUNCTION recover_connection() -> RecoveryResult {
            -- Implement exponential backoff reconnection
            attempts = 0
            max_attempts = 5
            backoff = Duration.millis(100)
            
            WHILE attempts < max_attempts {
                TRY {
                    connection.reconnect()
                    IF check_health() == HEALTHY THEN
                        RETURN RecoveryResult.SUCCESS
                    END IF
                } CATCH (error) {
                    log_error("Reconnection attempt failed", error)
                }
                
                sleep(backoff)
                backoff = min(backoff * 2, Duration.seconds(5))
                attempts += 1
            }
            
            RETURN RecoveryResult.FAILED
        }
    }
    
    CLASS PostgresHealthChecker IMPLEMENTS HealthChecker {
        FUNCTION check_health() -> HealthStatus {
            TRY {
                start_time = now()
                result = connection.execute("SELECT 1")
                latency = now() - start_time
                
                -- Check for slow queries
                IF latency > Duration.millis(100) THEN
                    RETURN HealthStatus.DEGRADED
                ELSE
                    RETURN HealthStatus.HEALTHY
                END IF
            } CATCH (error) {
                RETURN HealthStatus.UNHEALTHY
            }
        }
    }
}

-- Circuit breaker pattern for connection management
CLASS ConnectionCircuitBreaker {
    ENUM CircuitState {
        CLOSED,     -- Normal operation
        OPEN,       -- Failing fast, not attempting connections
        HALF_OPEN   -- Testing if service has recovered
    }
    
    PRIVATE state: CircuitState = CLOSED
    PRIVATE failure_count: Integer = 0
    PRIVATE last_failure_time: Timestamp
    PRIVATE failure_threshold: Integer = 5
    PRIVATE recovery_timeout: Duration = Duration.seconds(60)
    
    FUNCTION attempt_connection() -> ConnectionResult {
        SWITCH state {
            CASE CLOSED:
                RETURN try_connection()
            CASE OPEN:
                IF now() - last_failure_time > recovery_timeout THEN
                    state = HALF_OPEN
                    RETURN try_connection()
                ELSE
                    RETURN ConnectionResult.CIRCUIT_OPEN
                END IF
            CASE HALF_OPEN:
                RETURN try_connection()
        }
    }
    
    FUNCTION record_success() {
        failure_count = 0
        state = CLOSED
    }
    
    FUNCTION record_failure() {
        failure_count += 1
        last_failure_time = now()
        
        IF failure_count >= failure_threshold THEN
            state = OPEN
        END IF
    }
}
```

### 7.5 Resource Limits and Backpressure Management

```pseudocode
CLASS ResourceLimitManager {
    STRUCT ResourceLimits {
        max_memory_usage: Bytes = 512_MB
        max_connection_queue_size: Integer = 1000
        max_pending_requests: Integer = 10000
        connection_acquire_timeout: Duration = Duration.seconds(30)
        request_timeout: Duration = Duration.seconds(30)
        max_concurrent_operations: Integer = 100
    }
    
    CLASS BackpressureController {
        PRIVATE current_load: LoadMetrics
        PRIVATE resource_limits: ResourceLimits
        PRIVATE adaptive_throttling: AdaptiveThrottler
        
        FUNCTION handle_backpressure(request: ConnectionRequest) -> BackpressureAction {
            current_metrics = collect_current_metrics()
            
            -- Check memory usage
            IF current_metrics.memory_usage > resource_limits.max_memory_usage * 0.9 THEN
                RETURN BackpressureAction.REJECT_REQUEST
            END IF
            
            -- Check connection queue size
            IF current_metrics.queue_size > resource_limits.max_connection_queue_size THEN
                RETURN BackpressureAction.QUEUE_FULL
            END IF
            
            -- Check pending request count
            IF current_metrics.pending_requests > resource_limits.max_pending_requests THEN
                RETURN BackpressureAction.THROTTLE_REQUEST
            END IF
            
            -- Apply adaptive throttling based on success rate
            IF current_metrics.success_rate < 0.95 THEN
                throttle_delay = adaptive_throttling.calculate_delay(current_metrics)
                RETURN BackpressureAction.DELAY_REQUEST(throttle_delay)
            END IF
            
            RETURN BackpressureAction.ALLOW_REQUEST
        }
        
        FUNCTION collect_current_metrics() -> LoadMetrics {
            RETURN LoadMetrics {
                memory_usage: get_memory_usage(),
                queue_size: get_connection_queue_size(),
                pending_requests: get_pending_request_count(),
                success_rate: get_recent_success_rate(),
                average_latency: get_average_latency()
            }
        }
    }
    
    -- Protocol-specific backpressure handling
    CLASS NatsBackpressureHandler {
        FUNCTION handle_slow_consumer() {
            -- NATS built-in slow consumer protection
            subscription.set_pending_limits(1000, 50_MB)
            
            -- Custom overflow handling
            IF subscription.pending_messages() > 800 THEN
                log_warning("Approaching NATS subscription limit")
                trigger_load_shedding()
            END IF
        }
        
        FUNCTION trigger_load_shedding() {
            -- Drop non-critical messages
            -- Increase processing parallelism
            -- Signal upstream producers to slow down
        }
    }
}
```

### 7.6 Performance Monitoring and Metrics

```pseudocode
CLASS ConnectionPerformanceMonitor {
    PRIVATE metrics_collector: MetricsCollector
    PRIVATE alert_manager: AlertManager
    
    STRUCT ConnectionMetrics {
        pool_size: Integer
        active_connections: Integer
        idle_connections: Integer
        pending_acquisitions: Integer
        total_acquisitions: Counter
        failed_acquisitions: Counter
        acquisition_time_histogram: Histogram
        connection_lifetime_histogram: Histogram
        health_check_success_rate: Gauge
    }
    
    FUNCTION collect_metrics() -> ConnectionMetrics {
        RETURN ConnectionMetrics {
            pool_size: connection_pool.size(),
            active_connections: connection_pool.active_count(),
            idle_connections: connection_pool.idle_count(),
            pending_acquisitions: connection_pool.pending_count(),
            total_acquisitions: acquisition_counter.value(),
            failed_acquisitions: failure_counter.value(),
            acquisition_time_histogram: acquisition_timer.snapshot(),
            connection_lifetime_histogram: lifetime_timer.snapshot(),
            health_check_success_rate: health_success_rate.value()
        }
    }
    
    FUNCTION monitor_thresholds() {
        metrics = collect_metrics()
        
        -- Connection pool utilization alerts
        utilization = metrics.active_connections / metrics.pool_size
        IF utilization > 0.9 THEN
            alert_manager.trigger_alert(AlertType.HIGH_POOL_UTILIZATION, utilization)
        END IF
        
        -- Acquisition time alerts
        p95_acquisition_time = metrics.acquisition_time_histogram.percentile(95)
        IF p95_acquisition_time > Duration.seconds(5) THEN
            alert_manager.trigger_alert(AlertType.SLOW_ACQUISITION, p95_acquisition_time)
        END IF
        
        -- Failed acquisition rate alerts
        failure_rate = metrics.failed_acquisitions / metrics.total_acquisitions
        IF failure_rate > 0.05 THEN
            alert_manager.trigger_alert(AlertType.HIGH_FAILURE_RATE, failure_rate)
        END IF
        
        -- Health check alerts
        IF metrics.health_check_success_rate < 0.95 THEN
            alert_manager.trigger_alert(AlertType.HEALTH_CHECK_FAILURES, 
                                      metrics.health_check_success_rate)
        END IF
    }
    
    FUNCTION export_metrics_for_prometheus() -> PrometheusMetrics {
        -- Export metrics in Prometheus format for monitoring
        metrics = collect_metrics()
        
        prometheus_metrics = PrometheusMetrics()
        prometheus_metrics.add_gauge("connection_pool_size", metrics.pool_size)
        prometheus_metrics.add_gauge("connection_pool_active", metrics.active_connections)
        prometheus_metrics.add_gauge("connection_pool_idle", metrics.idle_connections)
        prometheus_metrics.add_counter("connection_acquisitions_total", metrics.total_acquisitions)
        prometheus_metrics.add_counter("connection_acquisition_failures_total", metrics.failed_acquisitions)
        prometheus_metrics.add_histogram("connection_acquisition_duration_seconds", 
                                        metrics.acquisition_time_histogram)
        
        RETURN prometheus_metrics
    }
}
```

## 8. Error Handling Patterns

### 8.1 Basic Retry Logic

```pseudocode
RETRY_PATTERN:
    attempts = 0
    WHILE attempts < max_retries:
        TRY:
            SEND message
            RETURN success
        CATCH error:
            attempts += 1
            WAIT backoff_time
    RETURN failure
```

### 8.2 Timeout Handling

```pseudocode
TIMEOUT_PATTERN:
    START timer(timeout_duration)
    SEND request
    WAIT for response OR timeout
    IF timeout:
        CANCEL request
        RETURN timeout_error
    ELSE:
        RETURN response
```

## 9. Basic Security

### 9.1 TLS Configuration

```pseudocode
TLS_SETUP:
    LOAD certificates
    CONFIGURE tls_config:
        - server_cert
        - server_key
        - ca_cert (required for mTLS)
    APPLY to transport layer
    
mTLS_PATTERN:
    SERVER_CONFIG:
        cert_file: "./certs/nats-server.crt"
        key_file:  "./certs/nats-server.key"
        ca_file:   "./certs/ca.crt"
        verify: true  # Enforce client certificates
        
    CLIENT_CONFIG:
        require_tls: true
        root_certificates: ca.crt
        client_certificate: client.crt
        client_key: client.key
```

### 9.2 Authentication Pattern

```pseudocode
AUTH_FLOW:
    CLIENT sends credentials
    SERVER validates credentials
    SERVER returns token
    CLIENT includes token in requests
    SERVER validates token on each request
    
SUBJECT_AUTHORIZATION:
    PER_USER_ACL:
        admin_permissions:
            publish: [ ">" ]      # Full access
            subscribe: [ ">" ]
            
        tenant_permissions:
            publish: { allow: ["tenantA.>"] }
            subscribe: { allow: ["tenantA.>"] }
            
    ACCOUNT_ISOLATION:
        - Each tenant gets NATS account
        - Complete namespace separation
        - Built-in multi-tenancy
        
    SECURITY_PRINCIPLES:
        - Never share accounts between tenants
        - Always enforce mTLS
        - Apply least privilege subjects
        - Set resource quotas per account
        - Monitor wildcard usage
```

### 9.3 Zero-Downtime Key Rotation

```pseudocode
KEY_ROTATION_PATTERN:
    STATE_MACHINE:
        KeyAActive -> StagingNewKey
        StagingNewKey -> ReloadingConfig
        ReloadingConfig -> KeyBActive
        
    IMPLEMENTATION:
        SIGNAL_HANDLER(SIGHUP):
            LOAD new_certificates
            ATOMIC_SWAP certificates
            UPDATE connections
            NO_DOWNTIME
```

## 10. Implementation Guidelines

### 10.1 Agent Integration

Agents implementing transport should:
1. Choose appropriate pattern for use case
2. Handle connection failures gracefully
3. Implement basic retry logic
4. Log communication events

### 10.2 Testing Patterns

```pseudocode
TEST_SCENARIOS:
    - Connection establishment
    - Message delivery
    - Timeout handling
    - Reconnection logic
    - Basic error cases
```

## 11. Configuration Templates

### 11.1 NATS Configuration

```pseudocode
NATS_CONFIG:
    servers: ["nats://localhost:4222"]
    max_reconnects: 5
    reconnect_wait: 2_seconds
    timeout: 10_seconds
```

### 11.2 gRPC Configuration

```pseudocode
GRPC_CONFIG:
    address: "localhost:50051"
    timeout: 30_seconds
    keepalive: 60_seconds
    max_message_size: 4_mb
```

### 11.3 HTTP Configuration

```pseudocode
HTTP_CONFIG:
    bind_address: "0.0.0.0:8080"
    request_timeout: 30_seconds
    max_connections: 100
```

## Summary

This document provides foundational transport patterns for agent communication. Focus is on:
- Basic messaging patterns
- Simple protocol usage
- Foundation error handling
- Essential connection management

Agents should implement these patterns incrementally, starting with basic request-response and expanding as needed.

## 12. NATS Concrete Implementation

### 12.1 Complete Subject Taxonomy

```json
AGENT_SUBJECTS: {
  "commands": "agents.{agent_id}.commands.{command_type}",
  "status": "agents.{agent_id}.status.{status_type}",
  "heartbeat": "agents.{agent_id}.heartbeat",
  "capabilities": "agents.{agent_id}.capabilities",
  "metrics": "agents.{agent_id}.metrics.{metric_type}",
  "logs": "agents.{agent_id}.logs.{level}",
  "events": "agents.{agent_id}.events.{event_type}"
}

TASK_SUBJECTS: {
  "assignment": "tasks.{task_type}.assignment",
  "queue": "tasks.{task_type}.queue.{priority}",
  "progress": "tasks.{task_id}.progress",
  "result": "tasks.{task_id}.result",
  "error": "tasks.{task_id}.error",
  "cancel": "tasks.{task_id}.cancel"
}

SYSTEM_SUBJECTS: {
  "control": "system.control.{operation}",
  "discovery": "system.discovery.{service_type}",
  "health": "system.health.{component}",
  "config": "system.config.{component}.{action}",
  "alerts": "system.alerts.{severity}",
  "audit": "system.audit.{action}"
}

WORKFLOW_SUBJECTS: {
  "orchestration": "workflow.{workflow_id}.orchestration",
  "coordination": "workflow.{workflow_id}.coordination",
  "dependencies": "workflow.{workflow_id}.dependencies",
  "rollback": "workflow.{workflow_id}.rollback"
}

CLAUDE_CLI_SUBJECTS: {
  "startup": "cli.startup",
  "hooks": "cli.hooks.{hook_type}.{agent_id}",
  "responses": "cli.responses.{agent_id}",
  "mutations": "cli.mutations.{agent_id}",
  "context": "cli.context.{group_id}.{context_type}"
}
```

### 12.2 Message Payload Schemas

```json
AGENT_COMMAND_SCHEMA: {
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["command_id", "command_type", "agent_id", "timestamp"],
  "properties": {
    "command_id": {"type": "string", "format": "uuid"},
    "command_type": {"type": "string", "enum": ["execute", "stop", "pause", "resume", "configure"]},
    "agent_id": {"type": "string", "pattern": "^[a-zA-Z0-9_-]+$"},
    "timestamp": {"type": "string", "format": "date-time"},
    "payload": {"type": "object"},
    "priority": {"type": "integer", "minimum": 1, "maximum": 10},
    "timeout_ms": {"type": "integer", "minimum": 1000},
    "reply_to": {"type": "string", "pattern": "^[a-zA-Z0-9._-]+$"},
    "correlation_id": {"type": "string", "format": "uuid"},
    "metadata": {
      "type": "object",
      "properties": {
        "source": {"type": "string"},
        "trace_id": {"type": "string", "format": "uuid"},
        "user_id": {"type": "string"},
        "session_id": {"type": "string", "format": "uuid"}
      }
    }
  }
}

AGENT_STATUS_SCHEMA: {
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["agent_id", "status", "timestamp"],
  "properties": {
    "agent_id": {"type": "string", "pattern": "^[a-zA-Z0-9_-]+$"},
    "status": {"type": "string", "enum": ["idle", "busy", "error", "offline", "starting", "stopping"]},
    "timestamp": {"type": "string", "format": "date-time"},
    "capacity": {"type": "number", "minimum": 0.0, "maximum": 1.0},
    "current_tasks": {"type": "integer", "minimum": 0},
    "max_tasks": {"type": "integer", "minimum": 1},
    "uptime_seconds": {"type": "integer", "minimum": 0},
    "last_heartbeat": {"type": "string", "format": "date-time"},
    "capabilities": {
      "type": "array",
      "items": {"type": "string"}
    },
    "resource_usage": {
      "type": "object",
      "properties": {
        "cpu_percent": {"type": "number", "minimum": 0.0, "maximum": 100.0},
        "memory_mb": {"type": "number", "minimum": 0},
        "disk_mb": {"type": "number", "minimum": 0}
      }
    }
  }
}

TASK_ASSIGNMENT_SCHEMA: {
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["task_id", "task_type", "assigned_agent", "timestamp"],
  "properties": {
    "task_id": {"type": "string", "format": "uuid"},
    "task_type": {"type": "string", "enum": ["analysis", "synthesis", "execution", "validation", "monitoring"]},
    "assigned_agent": {"type": "string", "pattern": "^[a-zA-Z0-9_-]+$"},
    "timestamp": {"type": "string", "format": "date-time"},
    "priority": {"type": "integer", "minimum": 1, "maximum": 10},
    "deadline": {"type": "string", "format": "date-time"},
    "requirements": {
      "type": "object",
      "properties": {
        "capabilities": {"type": "array", "items": {"type": "string"}},
        "resources": {
          "type": "object",
          "properties": {
            "cpu_cores": {"type": "integer", "minimum": 1},
            "memory_mb": {"type": "integer", "minimum": 256},
            "disk_mb": {"type": "integer", "minimum": 100}
          }
        }
      }
    },
    "task_data": {"type": "object"},
    "dependencies": {"type": "array", "items": {"type": "string", "format": "uuid"}},
    "callback_subject": {"type": "string"}
  }
}

TASK_RESULT_SCHEMA: {
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["task_id", "agent_id", "status", "timestamp"],
  "properties": {
    "task_id": {"type": "string", "format": "uuid"},
    "agent_id": {"type": "string", "pattern": "^[a-zA-Z0-9_-]+$"},
    "status": {"type": "string", "enum": ["completed", "failed", "cancelled", "timeout"]},
    "timestamp": {"type": "string", "format": "date-time"},
    "execution_time_ms": {"type": "integer", "minimum": 0},
    "result_data": {"type": "object"},
    "error_details": {
      "type": "object",
      "properties": {
        "error_code": {"type": "string"},
        "error_message": {"type": "string"},
        "stack_trace": {"type": "string"},
        "retry_count": {"type": "integer", "minimum": 0}
      }
    },
    "metrics": {
      "type": "object",
      "properties": {
        "cpu_usage_percent": {"type": "number"},
        "memory_peak_mb": {"type": "number"},
        "io_operations": {"type": "integer"}
      }
    }
  }
}
```

### 12.3 JetStream Configuration Specifications

```yaml
JETSTREAM_STREAMS:
  agent_events:
    subjects: ["agents.*.events.>", "agents.*.status", "agents.*.heartbeat"]
    storage: file
    retention: limits
    max_age: 604800  # 7 days in seconds
    max_bytes: 1073741824  # 1GB
    max_msgs: 1000000
    max_msg_size: 1048576  # 1MB
    discard: old
    num_replicas: 3
    duplicate_window: 120  # 2 minutes

  task_lifecycle:
    subjects: ["tasks.*.assignment", "tasks.*.result", "tasks.*.progress", "tasks.*.error"]
    storage: file
    retention: limits
    max_age: 2592000  # 30 days
    max_bytes: 5368709120  # 5GB
    max_msgs: 10000000
    max_msg_size: 10485760  # 10MB
    discard: old
    num_replicas: 3
    duplicate_window: 300  # 5 minutes

  system_monitoring:
    subjects: ["system.>.health", "system.>.alerts", "system.>.audit"]
    storage: file
    retention: limits
    max_age: 7776000  # 90 days
    max_bytes: 2147483648  # 2GB
    max_msgs: 5000000
    max_msg_size: 524288  # 512KB
    discard: old
    num_replicas: 3
    duplicate_window: 60  # 1 minute

  claude_cli_integration:
    subjects: ["cli.>"]
    storage: memory
    retention: limits
    max_age: 3600  # 1 hour
    max_bytes: 268435456  # 256MB
    max_msgs: 100000
    max_msg_size: 1048576  # 1MB
    discard: new
    num_replicas: 1
    duplicate_window: 30  # 30 seconds

JETSTREAM_CONSUMERS:
  agent_status_monitor:
    stream: agent_events
    filter_subject: "agents.*.status"
    deliver_policy: new
    ack_policy: explicit
    ack_wait: 30
    max_deliver: 3
    replay_policy: instant
    max_ack_pending: 1000

  task_processor:
    stream: task_lifecycle
    filter_subject: "tasks.*.assignment"
    deliver_policy: all
    ack_policy: explicit
    ack_wait: 300  # 5 minutes
    max_deliver: 5
    replay_policy: instant
    max_ack_pending: 100

  system_alerting:
    stream: system_monitoring
    filter_subject: "system.*.alerts.>"
    deliver_policy: new
    ack_policy: explicit
    ack_wait: 60
    max_deliver: 3
    replay_policy: instant
    max_ack_pending: 500

RESOURCE_LIMITS:
  max_memory: 2147483648  # 2GB
  max_storage: 107374182400  # 100GB
  max_streams: 50
  max_consumers: 500
  max_connections: 10000
  max_subscriptions: 100000
```

## 13. gRPC Service Definitions

### 13.1 Agent Communication Services

```protobuf
syntax = "proto3";
package mister_smith.agent;

import "google/protobuf/timestamp.proto";
import "google/protobuf/any.proto";
import "google/protobuf/empty.proto";

// Core Agent Communication Service
service AgentCommunication {
  // Unary RPCs
  rpc SendCommand(CommandRequest) returns (CommandResponse);
  rpc GetStatus(StatusRequest) returns (AgentStatus);
  rpc RegisterAgent(AgentRegistration) returns (RegistrationResponse);
  rpc GetCapabilities(AgentIdentifier) returns (AgentCapabilities);
  
  // Server streaming
  rpc StreamStatus(StatusRequest) returns (stream AgentStatus);
  rpc StreamEvents(EventSubscription) returns (stream AgentEvent);
  rpc StreamLogs(LogSubscription) returns (stream LogEntry);
  
  // Client streaming
  rpc UploadResults(stream TaskResult) returns (UploadSummary);
  rpc StreamMetrics(stream AgentMetrics) returns (MetricsSummary);
  
  // Bidirectional streaming
  rpc AgentChat(stream AgentMessage) returns (stream AgentMessage);
  rpc TaskCoordination(stream CoordinationMessage) returns (stream CoordinationMessage);
}

// Task Management Service
service TaskManagement {
  rpc AssignTask(TaskAssignment) returns (TaskAcceptance);
  rpc GetTaskStatus(TaskIdentifier) returns (TaskStatus);
  rpc CancelTask(TaskIdentifier) returns (TaskCancellation);
  rpc StreamTaskProgress(TaskIdentifier) returns (stream TaskProgress);
  rpc SubmitResult(TaskResult) returns (ResultAcknowledgment);
}

// Agent Discovery Service
service AgentDiscovery {
  rpc DiscoverAgents(DiscoveryQuery) returns (AgentList);
  rpc RegisterCapability(CapabilityRegistration) returns (google.protobuf.Empty);
  rpc FindAgentsWithCapability(CapabilityQuery) returns (AgentList);
  rpc StreamAgentUpdates(DiscoverySubscription) returns (stream AgentUpdate);
}

// Health Check Service (following gRPC health checking protocol)
service Health {
  rpc Check(HealthCheckRequest) returns (HealthCheckResponse);
  rpc Watch(HealthCheckRequest) returns (stream HealthCheckResponse);
}

// Message Definitions
message CommandRequest {
  string command_id = 1;
  string agent_id = 2;
  CommandType command_type = 3;
  google.protobuf.Any payload = 4;
  int32 priority = 5;
  int64 timeout_ms = 6;
  string correlation_id = 7;
  RequestMetadata metadata = 8;
}

message CommandResponse {
  string command_id = 1;
  ResponseStatus status = 2;
  string message = 3;
  google.protobuf.Any result = 4;
  int64 execution_time_ms = 5;
}

message AgentStatus {
  string agent_id = 1;
  AgentState state = 2;
  google.protobuf.Timestamp timestamp = 3;
  float capacity = 4;
  int32 current_tasks = 5;
  int32 max_tasks = 6;
  int64 uptime_seconds = 7;
  ResourceUsage resource_usage = 8;
  repeated string capabilities = 9;
}

message TaskAssignment {
  string task_id = 1;
  TaskType task_type = 2;
  string assigned_agent = 3;
  google.protobuf.Timestamp deadline = 4;
  int32 priority = 5;
  TaskRequirements requirements = 6;
  google.protobuf.Any task_data = 7;
  repeated string dependencies = 8;
}

message TaskResult {
  string task_id = 1;
  string agent_id = 2;
  TaskStatus status = 3;
  google.protobuf.Timestamp completion_time = 4;
  int64 execution_time_ms = 5;
  google.protobuf.Any result_data = 6;
  ErrorDetails error = 7;
  TaskMetrics metrics = 8;
}

message AgentEvent {
  string event_id = 1;
  string agent_id = 2;
  EventType event_type = 3;
  google.protobuf.Timestamp timestamp = 4;
  google.protobuf.Any event_data = 5;
  string correlation_id = 6;
}

// Enums
enum CommandType {
  COMMAND_TYPE_UNSPECIFIED = 0;
  EXECUTE = 1;
  STOP = 2;
  PAUSE = 3;
  RESUME = 4;
  CONFIGURE = 5;
  SHUTDOWN = 6;
}

enum AgentState {
  AGENT_STATE_UNSPECIFIED = 0;
  IDLE = 1;
  BUSY = 2;
  ERROR = 3;
  OFFLINE = 4;
  STARTING = 5;
  STOPPING = 6;
}

enum TaskType {
  TASK_TYPE_UNSPECIFIED = 0;
  ANALYSIS = 1;
  SYNTHESIS = 2;
  EXECUTION = 3;
  VALIDATION = 4;
  MONITORING = 5;
}

enum TaskStatus {
  TASK_STATUS_UNSPECIFIED = 0;
  PENDING = 1;
  RUNNING = 2;
  COMPLETED = 3;
  FAILED = 4;
  CANCELLED = 5;
  TIMEOUT = 6;
}

enum ResponseStatus {
  RESPONSE_STATUS_UNSPECIFIED = 0;
  SUCCESS = 1;
  ERROR = 2;
  TIMEOUT = 3;
  REJECTED = 4;
}

enum EventType {
  EVENT_TYPE_UNSPECIFIED = 0;
  STARTUP = 1;
  SHUTDOWN = 2;
  TASK_START = 3;
  TASK_END = 4;
  ERROR_OCCURRED = 5;
  CAPABILITY_CHANGE = 6;
}

// Nested message types
message RequestMetadata {
  string source = 1;
  string trace_id = 2;
  string user_id = 3;
  string session_id = 4;
  map<string, string> custom_headers = 5;
}

message ResourceUsage {
  float cpu_percent = 1;
  int64 memory_mb = 2;
  int64 disk_mb = 3;
  int32 network_connections = 4;
}

message TaskRequirements {
  repeated string capabilities = 1;
  ResourceRequirements resources = 2;
  SecurityRequirements security = 3;
}

message ResourceRequirements {
  int32 cpu_cores = 1;
  int64 memory_mb = 2;
  int64 disk_mb = 3;
  int32 network_bandwidth_mbps = 4;
}

message SecurityRequirements {
  string security_level = 1;
  repeated string required_permissions = 2;
  bool requires_encryption = 3;
}

message ErrorDetails {
  string error_code = 1;
  string error_message = 2;
  string stack_trace = 3;
  int32 retry_count = 4;
  repeated string error_tags = 5;
}

message TaskMetrics {
  float cpu_usage_percent = 1;
  int64 memory_peak_mb = 2;
  int64 io_operations = 3;
  int64 network_bytes_sent = 4;
  int64 network_bytes_received = 5;
}

message HealthCheckRequest {
  string service = 1;
}

message HealthCheckResponse {
  enum ServingStatus {
    UNKNOWN = 0;
    SERVING = 1;
    NOT_SERVING = 2;
    SERVICE_UNKNOWN = 3;
  }
  ServingStatus status = 1;
}
```

### 13.2 gRPC Server Configuration

```yaml
GRPC_SERVER_CONFIG:
  address: "0.0.0.0:50051"
  max_connections: 1000
  max_message_size: 16777216  # 16MB
  max_frame_size: 2097152     # 2MB
  keepalive:
    time: 60                  # seconds
    timeout: 5                # seconds
    permit_without_stream: true
  tls:
    enabled: true
    cert_file: "certs/server.crt"
    key_file: "certs/server.key"
    ca_file: "certs/ca.crt"
    client_auth: require_and_verify
  interceptors:
    - authentication
    - authorization
    - rate_limiting
    - metrics
    - tracing
  reflection: true
  health_check: true
```

## 14. HTTP API Specifications

### 14.1 OpenAPI 3.0 Specification

```yaml
openapi: 3.0.3
info:
  title: Mister Smith Agent Framework API
  description: RESTful API for agent communication and management
  version: 1.0.0
  contact:
    name: Mister Smith Framework
    url: https://github.com/mister-smith/framework
  license:
    name: Apache 2.0
    url: https://www.apache.org/licenses/LICENSE-2.0.html

servers:
  - url: https://api.mister-smith.dev/v1
    description: Production server
  - url: https://staging-api.mister-smith.dev/v1
    description: Staging server
  - url: http://localhost:8080/v1
    description: Development server

security:
  - bearerAuth: []
  - apiKeyAuth: []

paths:
  /agents:
    get:
      summary: List all agents
      description: Retrieve a paginated list of all registered agents
      operationId: listAgents
      parameters:
        - name: page
          in: query
          schema:
            type: integer
            minimum: 1
            default: 1
        - name: limit
          in: query
          schema:
            type: integer
            minimum: 1
            maximum: 100
            default: 20
        - name: status
          in: query
          schema:
            type: string
            enum: [idle, busy, error, offline, starting, stopping]
        - name: capability
          in: query
          schema:
            type: string
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/AgentListResponse'
        '400':
          $ref: '#/components/responses/BadRequest'
        '401':
          $ref: '#/components/responses/Unauthorized'
        '500':
          $ref: '#/components/responses/InternalServerError'

    post:
      summary: Register a new agent
      description: Register a new agent with the framework
      operationId: registerAgent
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/AgentRegistration'
      responses:
        '201':
          description: Agent registered successfully
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/AgentRegistrationResponse'
        '400':
          $ref: '#/components/responses/BadRequest'
        '409':
          $ref: '#/components/responses/Conflict'
        '500':
          $ref: '#/components/responses/InternalServerError'

  /agents/{agentId}:
    get:
      summary: Get agent details
      description: Retrieve detailed information about a specific agent
      operationId: getAgent
      parameters:
        - name: agentId
          in: path
          required: true
          schema:
            type: string
            pattern: '^[a-zA-Z0-9_-]+$'
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Agent'
        '404':
          $ref: '#/components/responses/NotFound'
        '500':
          $ref: '#/components/responses/InternalServerError'

    put:
      summary: Update agent configuration
      description: Update the configuration of an existing agent
      operationId: updateAgent
      parameters:
        - name: agentId
          in: path
          required: true
          schema:
            type: string
            pattern: '^[a-zA-Z0-9_-]+$'
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/AgentUpdate'
      responses:
        '200':
          description: Agent updated successfully
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Agent'
        '400':
          $ref: '#/components/responses/BadRequest'
        '404':
          $ref: '#/components/responses/NotFound'
        '500':
          $ref: '#/components/responses/InternalServerError'

    delete:
      summary: Deregister agent
      description: Remove an agent from the framework
      operationId: deregisterAgent
      parameters:
        - name: agentId
          in: path
          required: true
          schema:
            type: string
            pattern: '^[a-zA-Z0-9_-]+$'
      responses:
        '204':
          description: Agent deregistered successfully
        '404':
          $ref: '#/components/responses/NotFound'
        '500':
          $ref: '#/components/responses/InternalServerError'

  /agents/{agentId}/commands:
    post:
      summary: Send command to agent
      description: Send a command to a specific agent
      operationId: sendCommand
      parameters:
        - name: agentId
          in: path
          required: true
          schema:
            type: string
            pattern: '^[a-zA-Z0-9_-]+$'
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/Command'
      responses:
        '202':
          description: Command accepted
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/CommandResponse'
        '400':
          $ref: '#/components/responses/BadRequest'
        '404':
          $ref: '#/components/responses/NotFound'
        '500':
          $ref: '#/components/responses/InternalServerError'

  /agents/{agentId}/status:
    get:
      summary: Get agent status
      description: Retrieve the current status of an agent
      operationId: getAgentStatus
      parameters:
        - name: agentId
          in: path
          required: true
          schema:
            type: string
            pattern: '^[a-zA-Z0-9_-]+$'
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/AgentStatus'
        '404':
          $ref: '#/components/responses/NotFound'
        '500':
          $ref: '#/components/responses/InternalServerError'

  /tasks:
    get:
      summary: List tasks
      description: Retrieve a paginated list of tasks
      operationId: listTasks
      parameters:
        - name: page
          in: query
          schema:
            type: integer
            minimum: 1
            default: 1
        - name: limit
          in: query
          schema:
            type: integer
            minimum: 1
            maximum: 100
            default: 20
        - name: status
          in: query
          schema:
            type: string
            enum: [pending, running, completed, failed, cancelled, timeout]
        - name: type
          in: query
          schema:
            type: string
            enum: [analysis, synthesis, execution, validation, monitoring]
        - name: agent_id
          in: query
          schema:
            type: string
            pattern: '^[a-zA-Z0-9_-]+$'
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TaskListResponse'
        '400':
          $ref: '#/components/responses/BadRequest'
        '500':
          $ref: '#/components/responses/InternalServerError'

    post:
      summary: Create a new task
      description: Create and assign a new task
      operationId: createTask
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/TaskCreation'
      responses:
        '201':
          description: Task created successfully
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Task'
        '400':
          $ref: '#/components/responses/BadRequest'
        '500':
          $ref: '#/components/responses/InternalServerError'

  /tasks/{taskId}:
    get:
      summary: Get task details
      description: Retrieve detailed information about a specific task
      operationId: getTask
      parameters:
        - name: taskId
          in: path
          required: true
          schema:
            type: string
            format: uuid
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Task'
        '404':
          $ref: '#/components/responses/NotFound'
        '500':
          $ref: '#/components/responses/InternalServerError'

    delete:
      summary: Cancel task
      description: Cancel a pending or running task
      operationId: cancelTask
      parameters:
        - name: taskId
          in: path
          required: true
          schema:
            type: string
            format: uuid
      responses:
        '200':
          description: Task cancelled successfully
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TaskCancellation'
        '404':
          $ref: '#/components/responses/NotFound'
        '409':
          $ref: '#/components/responses/Conflict'
        '500':
          $ref: '#/components/responses/InternalServerError'

  /ws:
    get:
      summary: WebSocket endpoint
      description: WebSocket connection for real-time communication
      operationId: websocketConnect
      parameters:
        - name: protocol
          in: query
          schema:
            type: string
            enum: [events, commands, status]
            default: events
      responses:
        '101':
          description: Switching protocols to WebSocket
        '400':
          $ref: '#/components/responses/BadRequest'
        '401':
          $ref: '#/components/responses/Unauthorized'

components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
    apiKeyAuth:
      type: apiKey
      in: header
      name: X-API-Key

  schemas:
    Agent:
      type: object
      required: [agent_id, status, capabilities, registered_at]
      properties:
        agent_id:
          type: string
          pattern: '^[a-zA-Z0-9_-]+$'
          example: "analyzer-001"
        status:
          type: string
          enum: [idle, busy, error, offline, starting, stopping]
          example: "idle"
        capabilities:
          type: array
          items:
            type: string
          example: ["text-analysis", "data-processing"]
        registered_at:
          type: string
          format: date-time
        last_heartbeat:
          type: string
          format: date-time
        resource_usage:
          $ref: '#/components/schemas/ResourceUsage'
        current_tasks:
          type: integer
          minimum: 0
          example: 2
        max_tasks:
          type: integer
          minimum: 1
          example: 10

    AgentRegistration:
      type: object
      required: [agent_id, capabilities]
      properties:
        agent_id:
          type: string
          pattern: '^[a-zA-Z0-9_-]+$'
        capabilities:
          type: array
          items:
            type: string
          minItems: 1
        max_tasks:
          type: integer
          minimum: 1
          default: 5
        metadata:
          type: object
          additionalProperties: true

    Task:
      type: object
      required: [task_id, type, status, created_at]
      properties:
        task_id:
          type: string
          format: uuid
        type:
          type: string
          enum: [analysis, synthesis, execution, validation, monitoring]
        status:
          type: string
          enum: [pending, running, completed, failed, cancelled, timeout]
        assigned_agent:
          type: string
          pattern: '^[a-zA-Z0-9_-]+$'
        created_at:
          type: string
          format: date-time
        deadline:
          type: string
          format: date-time
        priority:
          type: integer
          minimum: 1
          maximum: 10
        task_data:
          type: object
          additionalProperties: true
        result:
          type: object
          additionalProperties: true

    Command:
      type: object
      required: [command_type]
      properties:
        command_type:
          type: string
          enum: [execute, stop, pause, resume, configure, shutdown]
        payload:
          type: object
          additionalProperties: true
        priority:
          type: integer
          minimum: 1
          maximum: 10
          default: 5
        timeout_ms:
          type: integer
          minimum: 1000
          default: 30000

    ResourceUsage:
      type: object
      properties:
        cpu_percent:
          type: number
          minimum: 0
          maximum: 100
        memory_mb:
          type: number
          minimum: 0
        disk_mb:
          type: number
          minimum: 0

    Error:
      type: object
      required: [error_code, message]
      properties:
        error_code:
          type: string
          example: "INVALID_REQUEST"
        message:
          type: string
          example: "The request payload is invalid"
        details:
          type: array
          items:
            type: object
            properties:
              field:
                type: string
              issue:
                type: string
        trace_id:
          type: string
          format: uuid

  responses:
    BadRequest:
      description: Bad request
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/Error'
    Unauthorized:
      description: Unauthorized
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/Error'
    NotFound:
      description: Resource not found
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/Error'
    Conflict:
      description: Resource conflict
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/Error'
    InternalServerError:
      description: Internal server error
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/Error'
```

### 14.2 WebSocket Protocol Specification

```json
WEBSOCKET_PROTOCOL: {
  "connection": {
    "url": "wss://api.mister-smith.dev/v1/ws",
    "protocols": ["events", "commands", "status"],
    "authentication": {
      "type": "bearer_token",
      "header": "Authorization"
    },
    "heartbeat_interval": 30,
    "reconnect_policy": {
      "initial_delay": 1000,
      "max_delay": 30000,
      "backoff_factor": 2.0,
      "max_attempts": 10
    }
  },
  "message_format": {
    "type": "object",
    "required": ["type", "id", "timestamp"],
    "properties": {
      "type": {
        "type": "string",
        "enum": ["event", "command", "response", "heartbeat", "error"]
      },
      "id": {
        "type": "string",
        "format": "uuid"
      },
      "timestamp": {
        "type": "string",
        "format": "date-time"
      },
      "payload": {
        "type": "object"
      },
      "correlation_id": {
        "type": "string",
        "format": "uuid"
      }
    }
  },
  "event_types": {
    "agent_status_changed": {
      "payload": {
        "agent_id": "string",
        "old_status": "string",
        "new_status": "string",
        "timestamp": "date-time"
      }
    },
    "task_assigned": {
      "payload": {
        "task_id": "uuid",
        "agent_id": "string",
        "task_type": "string",
        "priority": "integer"
      }
    },
    "task_completed": {
      "payload": {
        "task_id": "uuid",
        "agent_id": "string",
        "status": "string",
        "execution_time_ms": "integer"
      }
    },
    "system_alert": {
      "payload": {
        "severity": "string",
        "component": "string",
        "message": "string",
        "details": "object"
      }
    }
  }
}
```

## 15. Error Response Standards

### 15.1 Standardized Error Codes

```json
ERROR_CODES: {
  "VALIDATION": {
    "INVALID_REQUEST": {
      "code": "E1001",
      "http_status": 400,
      "message": "Request validation failed",
      "retryable": false
    },
    "MISSING_REQUIRED_FIELD": {
      "code": "E1002",
      "http_status": 400,
      "message": "Required field is missing",
      "retryable": false
    },
    "INVALID_FIELD_FORMAT": {
      "code": "E1003",
      "http_status": 400,
      "message": "Field format is invalid",
      "retryable": false
    }
  },
  "AUTHENTICATION": {
    "INVALID_TOKEN": {
      "code": "E2001",
      "http_status": 401,
      "message": "Authentication token is invalid",
      "retryable": false
    },
    "TOKEN_EXPIRED": {
      "code": "E2002",
      "http_status": 401,
      "message": "Authentication token has expired",
      "retryable": true
    },
    "INSUFFICIENT_PERMISSIONS": {
      "code": "E2003",
      "http_status": 403,
      "message": "Insufficient permissions for this operation",
      "retryable": false
    }
  },
  "RESOURCE": {
    "AGENT_NOT_FOUND": {
      "code": "E3001",
      "http_status": 404,
      "message": "Agent not found",
      "retryable": false
    },
    "TASK_NOT_FOUND": {
      "code": "E3002",
      "http_status": 404,
      "message": "Task not found",
      "retryable": false
    },
    "AGENT_ALREADY_EXISTS": {
      "code": "E3003",
      "http_status": 409,
      "message": "Agent already exists",
      "retryable": false
    }
  },
  "SYSTEM": {
    "INTERNAL_ERROR": {
      "code": "E5001",
      "http_status": 500,
      "message": "Internal server error",
      "retryable": true
    },
    "SERVICE_UNAVAILABLE": {
      "code": "E5002",
      "http_status": 503,
      "message": "Service temporarily unavailable",
      "retryable": true
    },
    "TIMEOUT": {
      "code": "E5003",
      "http_status": 504,
      "message": "Request timeout",
      "retryable": true
    }
  },
  "AGENT": {
    "AGENT_OFFLINE": {
      "code": "E4001",
      "http_status": 503,
      "message": "Agent is offline",
      "retryable": true
    },
    "AGENT_BUSY": {
      "code": "E4002",
      "http_status": 429,
      "message": "Agent is at capacity",
      "retryable": true
    },
    "CAPABILITY_NOT_SUPPORTED": {
      "code": "E4003",
      "http_status": 400,
      "message": "Agent does not support required capability",
      "retryable": false
    }
  }
}
```

### 15.2 Error Response Format

```json
ERROR_RESPONSE_SCHEMA: {
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["error_code", "message", "timestamp"],
  "properties": {
    "error_code": {
      "type": "string",
      "pattern": "^E\\d{4}$",
      "description": "Standardized error code"
    },
    "message": {
      "type": "string",
      "description": "Human-readable error message"
    },
    "timestamp": {
      "type": "string",
      "format": "date-time",
      "description": "When the error occurred"
    },
    "trace_id": {
      "type": "string",
      "format": "uuid",
      "description": "Unique identifier for tracing this error"
    },
    "details": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "field": {
            "type": "string",
            "description": "Field that caused the error"
          },
          "issue": {
            "type": "string",
            "description": "Specific issue with the field"
          },
          "code": {
            "type": "string",
            "description": "Field-specific error code"
          }
        }
      }
    },
    "retry_after": {
      "type": "integer",
      "description": "Seconds to wait before retrying (for retryable errors)"
    },
    "documentation_url": {
      "type": "string",
      "format": "uri",
      "description": "Link to relevant documentation"
    }
  }
}
```

### 15.3 Retry Policies

```yaml
RETRY_POLICIES:
  exponential_backoff:
    initial_delay: 1000      # milliseconds
    max_delay: 30000         # milliseconds
    backoff_factor: 2.0
    jitter: true
    max_attempts: 5

  linear_backoff:
    initial_delay: 1000      # milliseconds
    delay_increment: 1000    # milliseconds
    max_delay: 10000         # milliseconds
    max_attempts: 3

  immediate_retry:
    delay: 0
    max_attempts: 2

RETRY_CONDITIONS:
  retryable_errors:
    - "E2002"  # TOKEN_EXPIRED
    - "E5001"  # INTERNAL_ERROR
    - "E5002"  # SERVICE_UNAVAILABLE
    - "E5003"  # TIMEOUT
    - "E4001"  # AGENT_OFFLINE
    - "E4002"  # AGENT_BUSY

  non_retryable_errors:
    - "E1001"  # INVALID_REQUEST
    - "E1002"  # MISSING_REQUIRED_FIELD
    - "E2001"  # INVALID_TOKEN
    - "E2003"  # INSUFFICIENT_PERMISSIONS
    - "E3001"  # AGENT_NOT_FOUND
    - "E4003"  # CAPABILITY_NOT_SUPPORTED

CIRCUIT_BREAKER:
  failure_threshold: 5
  recovery_timeout: 30000    # milliseconds
  test_request_volume: 3
  minimum_throughput: 10
```

## 16. Connection Pool Configurations

### 16.1 NATS Connection Pool

```yaml
NATS_CONNECTION_POOL:
  servers:
    - "nats://nats-01.internal:4222"
    - "nats://nats-02.internal:4222"
    - "nats://nats-03.internal:4222"
  
  connection_settings:
    max_connections: 100
    max_idle_connections: 20
    connection_timeout: 10000    # milliseconds
    ping_interval: 60000         # milliseconds
    max_reconnects: 10
    reconnect_wait: 2000         # milliseconds
    reconnect_jitter: 1000       # milliseconds
    disconnect_error_handler: true
    async_error_handler: true

  subscription_settings:
    max_subscriptions: 1000
    subscription_timeout: 5000   # milliseconds
    pending_message_limit: 1000
    pending_byte_limit: 10485760 # 10MB
    subscription_capacity: 1000

  tls_config:
    enabled: true
    cert_file: "certs/client.crt"
    key_file: "certs/client.key"
    ca_file: "certs/ca.crt"
    verify_certificates: true
    cipher_suites:
      - "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384"
      - "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256"

  jetstream_config:
    max_ack_pending: 1000
    ack_wait: 30000             # milliseconds
    max_deliver: 3
    delivery_policy: "new"
    replay_policy: "instant"

  monitoring:
    stats_interval: 60000       # milliseconds
    slow_consumer_threshold: 1000
    enable_metrics: true
    enable_tracing: true
```

### 16.2 gRPC Connection Pool

```yaml
GRPC_CONNECTION_POOL:
  target_addresses:
    - "grpc-01.internal:50051"
    - "grpc-02.internal:50051"
    - "grpc-03.internal:50051"

  connection_settings:
    max_connections_per_target: 10
    max_idle_connections: 5
    connection_timeout: 15000    # milliseconds
    keepalive_time: 60000        # milliseconds
    keepalive_timeout: 5000      # milliseconds
    keepalive_without_stream: true
    max_connection_idle: 300000  # milliseconds
    max_connection_age: 600000   # milliseconds

  call_settings:
    max_message_size: 16777216   # 16MB
    max_header_list_size: 8192   # 8KB
    call_timeout: 30000          # milliseconds
    retry_enabled: true
    max_retry_attempts: 3
    initial_backoff: 1000        # milliseconds
    max_backoff: 10000           # milliseconds
    backoff_multiplier: 2.0

  tls_config:
    enabled: true
    server_name_override: ""
    cert_file: "certs/client.crt"
    key_file: "certs/client.key"
    ca_file: "certs/ca.crt"
    insecure_skip_verify: false

  load_balancing:
    policy: "round_robin"       # round_robin, least_request, consistent_hash
    health_check_enabled: true
    health_check_interval: 30000 # milliseconds

  monitoring:
    enable_stats: true
    stats_tags:
      - "service"
      - "method"
      - "status"
    enable_tracing: true
    trace_sampling_rate: 0.1
```

### 16.3 HTTP Connection Pool

```yaml
HTTP_CONNECTION_POOL:
  max_connections: 200
  max_idle_connections: 50
  max_connections_per_host: 20
  idle_connection_timeout: 90000  # milliseconds
  connection_timeout: 10000       # milliseconds
  request_timeout: 30000          # milliseconds
  response_header_timeout: 10000  # milliseconds
  tls_handshake_timeout: 10000    # milliseconds
  expect_continue_timeout: 1000   # milliseconds

  retry_config:
    max_retries: 3
    initial_backoff: 1000         # milliseconds
    max_backoff: 10000           # milliseconds
    backoff_multiplier: 2.0
    retryable_status_codes:
      - 429  # Too Many Requests
      - 502  # Bad Gateway
      - 503  # Service Unavailable
      - 504  # Gateway Timeout

  rate_limiting:
    requests_per_second: 100
    burst_size: 200
    per_host_limit: 50

  compression:
    enabled: true
    algorithms:
      - "gzip"
      - "deflate"
    min_size: 1024              # bytes

  tls_config:
    min_version: "TLS1.2"
    max_version: "TLS1.3"
    cipher_suites:
      - "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384"
      - "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256"
    verify_certificates: true
    ca_file: "certs/ca.crt"
    cert_file: "certs/client.crt"
    key_file: "certs/client.key"

  monitoring:
    enable_metrics: true
    metrics_interval: 60000       # milliseconds
    enable_access_logs: true
    log_request_body: false
    log_response_body: false
```

## 17. Serialization Specifications

### 17.1 JSON Schema Definitions

```json
SERIALIZATION_STANDARDS: {
  "encoding": "UTF-8",
  "format": "JSON",
  "validation": "JSON Schema Draft 07",
  "date_format": "ISO 8601 (RFC 3339)",
  "number_precision": "IEEE 754 double precision",
  "string_max_length": 1048576,
  "object_max_depth": 32,
  "array_max_length": 10000
}

BASE_MESSAGE_SCHEMA: {
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://schemas.mister-smith.dev/base-message.json",
  "type": "object",
  "required": ["message_id", "timestamp", "version"],
  "properties": {
    "message_id": {
      "type": "string",
      "format": "uuid",
      "description": "Unique message identifier"
    },
    "timestamp": {
      "type": "string",
      "format": "date-time",
      "description": "Message creation timestamp in ISO 8601 format"
    },
    "version": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+\\.\\d+$",
      "description": "Schema version (semantic versioning)"
    },
    "correlation_id": {
      "type": "string",
      "format": "uuid",
      "description": "Optional correlation identifier for message tracking"
    },
    "trace_id": {
      "type": "string",
      "format": "uuid",
      "description": "Distributed tracing identifier"
    }
  }
}

VALIDATION_RULES: {
  "strict_mode": true,
  "additional_properties": false,
  "validate_formats": true,
  "validate_required": true,
  "fail_on_unknown_keywords": true,
  "error_aggregation": true,
  "max_validation_errors": 100
}

COMPRESSION_SETTINGS: {
  "algorithm": "gzip",
  "compression_level": 6,
  "min_size_threshold": 1024,
  "content_types": [
    "application/json",
    "text/plain",
    "application/xml"
  ]
}

SECURITY_CONSTRAINTS: {
  "max_string_length": 1048576,
  "max_array_length": 10000,
  "max_object_properties": 1000,
  "max_nesting_depth": 32,
  "prohibited_patterns": [
    "javascript:",
    "data:",
    "vbscript:",
    "<script",
    "</script>"
  ],
  "sanitization": {
    "html_entities": true,
    "sql_injection": true,
    "xss_prevention": true
  }
}
```

### 17.2 Binary Serialization (Protocol Buffers)

```protobuf
syntax = "proto3";
package mister_smith.serialization;

import "google/protobuf/timestamp.proto";
import "google/protobuf/any.proto";

// Base message wrapper for all protocol buffer messages
message BaseMessage {
  string message_id = 1;
  google.protobuf.Timestamp timestamp = 2;
  string version = 3;
  string correlation_id = 4;
  string trace_id = 5;
  google.protobuf.Any payload = 6;
  map<string, string> metadata = 7;
}

// Performance optimized message for high-frequency data
message CompactMessage {
  bytes message_id = 1;  // 16-byte UUID as bytes
  int64 timestamp_unix = 2;  // Unix timestamp in nanoseconds
  bytes payload = 3;  // Compressed payload
  uint32 checksum = 4;  // CRC32 checksum
}

// Message envelope for batch processing
message MessageBatch {
  repeated BaseMessage messages = 1;
  uint32 batch_size = 2;
  string batch_id = 3;
  google.protobuf.Timestamp created_at = 4;
  CompressionType compression = 5;
}

enum CompressionType {
  COMPRESSION_TYPE_UNSPECIFIED = 0;
  NONE = 1;
  GZIP = 2;
  LZ4 = 3;
  SNAPPY = 4;
}
```

### 17.3 Serialization Performance Configuration

```yaml
SERIALIZATION_PERFORMANCE:
  json:
    parser: "serde_json"
    writer_buffer_size: 8192
    reader_buffer_size: 8192
    pretty_print: false
    escape_unicode: false
    floating_point_precision: 15
    use_compact_representation: true

  protobuf:
    codec: "prost"
    max_message_size: 67108864  # 64MB
    use_varint_encoding: true
    preserve_unknown_fields: false
    deterministic_serialization: true
    lazy_parsing: true

  messagepack:
    codec: "rmp-serde"
    use_compact_integers: true
    use_string_keys: true
    preserve_order: false

  compression:
    threshold_bytes: 1024
    algorithms:
      gzip:
        level: 6
        window_bits: 15
        memory_level: 8
      lz4:
        acceleration: 1
        compression_level: 0
      snappy:
        enable_checksum: true

VALIDATION_PERFORMANCE:
  schema_cache_size: 1000
  schema_cache_ttl: 3600      # seconds
  parallel_validation: true
  max_validation_threads: 4
  validation_timeout: 5000    # milliseconds
  enable_fast_path: true
  skip_optional_validation: false
```

---

*Transport Layer Specifications - Complete Protocol Implementation for Agent Communication*