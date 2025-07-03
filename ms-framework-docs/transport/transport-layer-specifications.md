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

## 7. Connection Management

### 7.1 Basic Connection Pool

```pseudocode
CONNECTION_POOL:
    maintain CONNECTIONS[]
    
    GET_CONNECTION:
        IF available in pool:
            RETURN connection
        ELSE:
            CREATE new connection
            ADD to pool
            RETURN connection
            
    RELEASE_CONNECTION:
        RETURN connection to pool
```

### 7.2 Health Checking

```pseudocode
HEALTH_CHECK_PATTERN:
    EVERY interval:
        FOR EACH connection:
            SEND ping
            IF no response in timeout:
                MARK unhealthy
                RECONNECT
```

### 7.3 Back-Pressure Management

```pseudocode
BACK_PRESSURE_TACTICS:
    CONNECTION_LEVEL:
        - NATS drops slow subscribers
        - Set subscription_capacity(1000)
        - Configure output buffers
        
    FLOW_CONTROL:
        - Pull consumers for rate control
        - Consumer prefetch limits
        - HTTP/2 flow control windows
        
    MONITORING_THRESHOLDS:
        - consumer_lag > 1000 messages
        - pending_messages > 10k
        - reconnection_count increasing
        - memory_buffer > 80% utilized
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

---

*Transport Layer Specifications - Foundation Patterns for Agent Implementation*