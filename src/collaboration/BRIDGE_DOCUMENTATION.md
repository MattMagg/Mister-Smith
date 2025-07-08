# MCP-Collaboration Bridge Documentation

## Overview

The MCP-Collaboration Bridge provides seamless integration between MCP (Model Context Protocol) server handlers and the MisterSmith collaboration infrastructure. This bridge enables unified discovery flows, agent lifecycle management, and real-time collaboration features.

## Architecture

### Core Components

1. **CollaborationBridge** - Main service that integrates all collaboration infrastructure
2. **BridgedHandlers** - Enhanced MCP handlers that use collaboration features
3. **AgentLifecycleManager** - Manages agent spawning and coordination
4. **UnifiedDiscoveryFlow** - Seamless discovery sharing across layers

### Data Flow

```
MCP Client → Bridged Handler → CollaborationBridge → Collaboration Infrastructure
                                                  ↓
                                              NATS → SSE → Real-time Clients
```

## Key Features

### 1. Unified Discovery Flow

**Before (Disconnected):**
```
MCP Client → share_discovery → DISCOVERY_STORE → NATS → (no SSE integration)
```

**After (Unified):**
```
MCP Client → bridged_share_discovery → CollaborationBridge → DiscoveryBroadcaster → NATS → SSE → Clients
```

### 2. Collaboration-Aware Subscriptions

- Real SSE endpoints connected to DiscoverySSEBroadcaster
- Agent role-based filtering
- Context-aware discovery routing
- Proper client lifecycle management

### 3. Agent Lifecycle Integration

- MCP agents register with LiveOrchestrator
- Support for spawning collaborative agents
- Context sharing between MCP and collaboration layers
- Cleanup and resource management

### 4. Enhanced Context Management

- Shared context across MCP and collaboration layers
- Agent-to-agent communication
- Broadcast and directed context sharing
- Persistence and retrieval

## API Reference

### Bridged Handlers

#### `bridged_share_discovery_handler`

Enhanced version of `share_discovery_handler` that integrates with collaboration infrastructure.

**Parameters:**
```rust
ShareDiscoveryParams {
    agent_id: String,
    agent_role: String,
    discovery_type: String, // "pattern", "anomaly", "connection", "solution", "question", "insight"
    content: String,
    confidence: f32, // 0.0 to 1.0
    related_to: Option<String>,
}
```

**Enhanced Features:**
- Publishes via DiscoveryBroadcaster (not direct NATS)
- Updates agent activity tracking
- Shares context with collaboration layer
- Provides orchestrator guidance for questions
- Returns collaboration metadata

#### `bridged_subscribe_discoveries_handler`

Enhanced version of `subscribe_discoveries_handler` that connects to actual SSE broadcaster.

**Parameters:**
```rust
SubscribeDiscoveriesParams {
    discovery_types: Option<Vec<String>>,
    agent_roles: Option<Vec<String>>,
    min_confidence: Option<f32>,
    subscriber_agent_id: String,
}
```

**Enhanced Features:**
- Connects to actual DiscoverySSEBroadcaster
- Registers agent with lifecycle manager
- Returns real SSE endpoints
- Includes collaboration metadata

#### `spawn_collaborative_agent_handler`

Spawns agents through LiveOrchestrator with collaboration features.

**Parameters:**
```rust
SpawnAgentParams {
    agent_role: String,
    initial_context: String,
}
```

**Features:**
- Uses LiveOrchestrator for agent management
- Registers with AgentLifecycleManager
- Shares initial context
- Returns agent coordination information

### Additional Handlers

#### `orchestrate_task_handler`

Orchestrates complex tasks across multiple agents.

#### `share_context_handler`

Enables agent-to-agent context sharing.

#### `get_collaboration_status_handler`

Returns comprehensive collaboration infrastructure status.

## Usage Examples

### Basic Integration

```rust
use mistersmith::collaboration::{initialize_bridge, get_bridge};
use mistersmith::handlers::*;

// Initialize bridge
let nats_client = async_nats::connect("nats://localhost:4222").await?;
initialize_bridge(nats_client.clone()).await?;

// Use bridged handlers
let result = bridged_share_discovery_handler(
    Parameters {
        params: ShareDiscoveryParams {
            agent_id: "security-agent".to_string(),
            agent_role: "Security".to_string(),
            discovery_type: "anomaly".to_string(),
            content: "Unusual authentication patterns detected".to_string(),
            confidence: 0.85,
            related_to: None,
        }
    },
    nats_client.clone()
).await?;
```

### Real-time Collaboration

```rust
// Set up subscription
let subscription = bridged_subscribe_discoveries_handler(
    Parameters {
        params: SubscribeDiscoveriesParams {
            discovery_types: Some(vec!["pattern".to_string()]),
            agent_roles: Some(vec!["Security".to_string()]),
            min_confidence: Some(0.7),
            subscriber_agent_id: "monitor-agent".to_string(),
        }
    },
    nats_client.clone()
).await?;

// Connect to SSE endpoint from subscription response
// The bridge ensures this connects to the actual SSE broadcaster
```

### Agent Spawning

```rust
// Spawn collaborative agent
let agent_result = spawn_collaborative_agent_handler(
    Parameters {
        params: SpawnAgentParams {
            agent_role: "Performance".to_string(),
            initial_context: "Monitor system performance metrics".to_string(),
        }
    },
    nats_client.clone()
).await?;

// Agent is now managed by LiveOrchestrator
```

### Task Orchestration

```rust
// Orchestrate complex task
let task_result = orchestrate_task_handler(
    Parameters {
        params: OrchestateTaskParams {
            task_description: "Investigate performance degradation".to_string(),
            required_agents: Some(vec!["Performance".to_string(), "Security".to_string()]),
            timeout_seconds: Some(300),
        }
    },
    nats_client.clone()
).await?;
```

## Data Consistency Guarantees

### Discovery Consistency

1. **Atomic Storage**: Discoveries stored in DISCOVERY_STORE first
2. **Reliable Broadcast**: Published via DiscoveryBroadcaster with retry logic
3. **Eventual Consistency**: SSE clients receive updates via NATS stream
4. **Failure Handling**: Stored discoveries remain available even if broadcast fails

### Agent State Consistency

1. **Lifecycle Management**: LiveOrchestrator manages agent state
2. **Activity Tracking**: Agent activity timestamps updated on each operation
3. **Context Synchronization**: Shared context via ContextManager
4. **Cleanup**: Proper resource cleanup on agent termination

### Subscription Consistency

1. **Filter Persistence**: Subscription filters stored in DiscoveryStore
2. **SSE Coordination**: Same filters used for SSE streaming
3. **Client Tracking**: Connected clients tracked by SSE broadcaster
4. **Graceful Disconnect**: Cleanup on client disconnection

## Integration Points

### MCP Layer Integration

- **Handlers**: Enhanced MCP handlers use collaboration features
- **Persistence**: Shared DISCOVERY_STORE for data consistency
- **Error Handling**: Graceful fallback to original handlers
- **API Compatibility**: Maintains MCP protocol compliance

### Collaboration Layer Integration

- **DiscoveryBroadcaster**: Used for all discovery publishing
- **LiveOrchestrator**: Manages agent lifecycle and coordination
- **ContextManager**: Handles inter-agent communication
- **SSEBroadcaster**: Provides real-time streaming to clients

### NATS Integration

- **Unified Messaging**: All communication flows through NATS
- **Subject Routing**: Consistent subject patterns across layers
- **Backpressure**: Natural flow control via NATS
- **Reliability**: Message persistence and delivery guarantees

## Performance Characteristics

### Scalability

- **Concurrent Agents**: Support for multiple agents simultaneously
- **Discovery Throughput**: Efficient discovery sharing via NATS
- **SSE Connections**: Multiple concurrent SSE clients supported
- **Memory Management**: LRU eviction for discovery storage

### Latency

- **Discovery Sharing**: Low latency via NATS pub/sub
- **SSE Streaming**: Near real-time updates to clients
- **Context Sharing**: Efficient broadcast and directed messaging
- **Agent Coordination**: Minimal orchestration overhead

### Reliability

- **Fault Tolerance**: Continues operation if collaboration features fail
- **Circuit Breaker**: Protects against cascade failures
- **Retry Logic**: Automatic retry for transient failures
- **Graceful Degradation**: MCP functionality preserved

## Error Handling

### Bridge Failures

- **Initialization**: Clear error messages if bridge init fails
- **Runtime**: Graceful fallback to original handlers
- **Recovery**: Automatic recovery when services restore
- **Logging**: Comprehensive error logging and tracing

### Collaboration Failures

- **Service Unavailable**: MCP handlers continue to work
- **Partial Failures**: Individual feature failures isolated
- **Network Issues**: Retry logic for transient network problems
- **State Corruption**: Automatic state recovery mechanisms

## Configuration

### Environment Variables

```bash
# NATS configuration
NATS_URL=nats://localhost:4222

# Bridge configuration
BRIDGE_MAX_DISCOVERIES=1000
BRIDGE_CLEANUP_INTERVAL=300
BRIDGE_RETRY_ATTEMPTS=3

# SSE configuration
SSE_KEEPALIVE_INTERVAL=30
SSE_MAX_CONNECTIONS=100
```

### Initialization Options

```rust
// Initialize with custom configuration
let bridge = CollaborationBridge::new(nats_client)
    .with_max_discoveries(2000)
    .with_cleanup_interval(Duration::from_secs(600))
    .with_retry_attempts(5)
    .build()
    .await?;
```

## Testing

### Unit Tests

- **Handler Validation**: Parameter validation and error handling
- **Bridge Integration**: Component interaction testing
- **State Management**: Agent lifecycle and discovery storage
- **Error Scenarios**: Failure handling and recovery

### Integration Tests

- **End-to-End Flow**: Complete discovery sharing workflow
- **Multi-Agent**: Multiple agents collaborating simultaneously
- **SSE Streaming**: Real-time updates via SSE
- **Task Orchestration**: Complex task coordination

### Performance Tests

- **Discovery Throughput**: High-volume discovery sharing
- **Concurrent Agents**: Multiple agents operating simultaneously
- **Memory Usage**: Discovery storage and eviction
- **Latency**: End-to-end response times

## Monitoring

### Metrics

- **Discovery Volume**: Total discoveries shared
- **Agent Activity**: Active agent count and operations
- **SSE Connections**: Connected clients and throughput
- **Error Rates**: Failure rates and types

### Health Checks

- **Bridge Status**: Overall bridge health
- **Component Health**: Individual service status
- **NATS Connectivity**: Message bus availability
- **Resource Usage**: Memory and CPU utilization

## Troubleshooting

### Common Issues

1. **Bridge Not Initialized**: Ensure `initialize_bridge()` called
2. **NATS Connection**: Verify NATS server is running
3. **SSE Connections**: Check firewall and proxy settings
4. **Discovery Not Received**: Verify subscription filters
5. **Agent Spawn Failures**: Check orchestrator status

### Debug Information

```rust
// Get detailed status
let status = get_collaboration_status_handler(
    Parameters { params: serde_json::Value::Null },
    nats_client
).await?;
```

### Logging

Enable detailed logging with:
```bash
RUST_LOG=mistersmith::collaboration=debug
```

## Migration Guide

### From Direct Handlers

```rust
// Old approach
share_discovery_handler(params, nats_client).await?;

// New approach
bridged_share_discovery_handler(params, nats_client).await?;
```

### From Manual Integration

```rust
// Old approach - manual setup
let broadcaster = DiscoveryBroadcaster::new(...);
let orchestrator = LiveOrchestrator::new(...);

// New approach - bridge handles setup
initialize_bridge(nats_client).await?;
let bridge = get_bridge().await.unwrap();
```

## Best Practices

1. **Initialize Early**: Set up bridge at application startup
2. **Error Handling**: Always handle bridge initialization failures
3. **Resource Cleanup**: Properly clean up agents and subscriptions
4. **Monitoring**: Monitor bridge health and performance
5. **Testing**: Test both success and failure scenarios

## Future Enhancements

1. **Persistence**: Database backing for discovery storage
2. **Security**: Authentication and authorization for agents
3. **Scaling**: Horizontal scaling across multiple nodes
4. **Analytics**: Advanced discovery analytics and insights
5. **Integration**: Additional protocol support beyond MCP