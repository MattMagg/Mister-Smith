# NATS Requirements Analysis for MisterSmith Phase 2

**Agent 1 Report** - Context Reader & NATS Requirements Analyst  
**Date**: 2025-07-07  
**Analysis**: Basic NATS vs JetStream for Phase 2 Implementation

## Executive Summary

For MisterSmith Phase 2 (Single Agent Implementation), **basic NATS is sufficient**. JetStream is NOT required and would add unnecessary complexity.

## Phase 1 Completion Status (from basic-memory)

✅ **Phase 1 Foundation - COMPLETED**
- Cargo.toml with standardized dependencies (including async-nats = "0.37.0")
- Basic directory structure created
- Hello World agent working
- Tokio runtime proof functional
- Clean compilation achieved

## Phase 2 Requirements Analysis

### What Phase 2 Needs

1. **Single Claude CLI Process Management**
   - Spawn/stop/pause commands
   - State tracking (Starting, Active, Paused, Stopping, Terminated, Error)
   - Process supervision with restart capability
   - Resource limits enforcement

2. **Real-time Messaging**
   ```
   agent.001.spawn     → Start Claude CLI process
   agent.001.stop      → Graceful shutdown
   agent.001.status    → Health monitoring
   agent.001.input     → Send to Claude stdin
   agent.001.output    → Receive from Claude stdout
   ```

3. **Hook Bridge Integration**
   - Claude CLI hooks publish events to NATS
   - Framework subscribes and responds
   - All communication is ephemeral

### What Phase 2 Does NOT Need

- Message persistence
- Message replay capabilities
- Multi-agent coordination
- State synchronization
- Audit trails
- Historical data retention

## Technical Comparison

### Basic NATS (Core)
- **Throughput**: 3M+ messages/sec
- **Latency**: 50-500 microseconds
- **Delivery**: At-most-once (fire-and-forget)
- **Storage**: None (ephemeral only)
- **Complexity**: Minimal
- **Use Case**: Real-time messaging

### JetStream
- **Throughput**: ~200K messages/sec
- **Latency**: 1-15 milliseconds
- **Delivery**: At-least-once with acknowledgment
- **Storage**: Persistent streams with retention
- **Complexity**: Higher (streams, consumers, configuration)
- **Use Case**: Durable messaging, event sourcing

## Recommendation

### Use Basic NATS for Phase 2

**Rationale**:

1. **Performance**: 50μs latency ideal for interactive CLI experience
2. **Simplicity**: Aligns with "minimal, verifiable functionality first" principle
3. **Appropriateness**: Fire-and-forget perfect for process commands
4. **Foundation**: Easier to verify and debug single agent behavior

### Implementation Strategy

```rust
// Phase 2: Basic NATS implementation
use async_nats::Client;

pub struct NatsTransport {
    client: Client,
}

impl NatsTransport {
    pub async fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        let client = async_nats::connect("127.0.0.1:4222").await?;
        Ok(Self { client })
    }
    
    pub async fn publish_command(&self, agent_id: &str, command: &str, payload: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        let subject = format!("agent.{}.{}", agent_id, command);
        self.client.publish(subject, payload.into()).await?;
        Ok(())
    }
    
    pub async fn subscribe_status(&self, agent_id: &str) -> Result<async_nats::Subscriber, Box<dyn std::error::Error>> {
        let subject = format!("agent.{}.status", agent_id);
        self.client.subscribe(subject).await
    }
}
```

### Future JetStream Integration (Phase 3+)

Design subjects to be JetStream-compatible now:
- Use hierarchical subject naming
- Plan for stream configurations
- Keep message schemas consistent

Add JetStream when needed for:
- Multiple agent coordination
- Message persistence requirements
- Audit trail capabilities
- State recovery from history

## Key Integration Patterns

From `claude-cli-integration.md` analysis:

1. **Hook Bridge Pattern**:
   - Claude CLI hooks → Shell script → NATS publish
   - NATS subscribe → Framework → Process management

2. **Process Supervision**:
   - Tokio spawn for process management
   - NATS for command/status communication
   - Restart on failure without message replay

3. **Message Flow**:
   - Commands are one-way (no persistence needed)
   - Status updates are real-time (old values irrelevant)
   - I/O streaming requires low latency

## Conclusion

Basic NATS provides everything needed for Phase 2's single agent implementation while maintaining simplicity and performance. JetStream features documented in the framework are for future phases when distributed coordination becomes necessary.

**Dependencies remain unchanged**:
```toml
async-nats = "0.37.0"  # Supports both basic NATS and JetStream
```

This allows seamless upgrade to JetStream in Phase 3 without dependency changes.