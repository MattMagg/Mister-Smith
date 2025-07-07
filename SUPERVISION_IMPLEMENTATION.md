# Supervision Tree Implementation

## Overview

This document describes the supervision tree implementation for the MisterSmith framework, providing fault tolerance and automatic recovery capabilities based on Erlang/OTP patterns adapted for Rust's async ecosystem.

## Architecture

### Core Components

1. **SupervisionTree** (`src/supervision/tree.rs`)
   - Root structure managing the entire supervision hierarchy
   - Handles node registration and failure event routing
   - Coordinates restart policies across the tree

2. **Supervisor Trait** (`src/supervision/types.rs`)
   - Defines core supervision behavior
   - Supports different supervision strategies
   - Implements hub-and-spoke routing pattern

3. **SupervisorNode** (`src/supervision/node.rs`)
   - Individual nodes in the supervision tree
   - Manages child processes and applies strategies
   - Tracks metrics and failure history

4. **FailureDetector** (`src/supervision/failure_detector.rs`)
   - Phi Accrual based adaptive failure detection
   - Monitors node health via heartbeats
   - Minimizes false positives through statistical analysis

5. **CircuitBreaker** (`src/supervision/circuit_breaker.rs`)
   - Prevents cascading failures
   - Three states: Closed, Open, Half-Open
   - Configurable thresholds and timeouts

### Supervision Strategies

The implementation supports four core strategies:

- **OneForOne**: Restart only the failed child
- **OneForAll**: Restart all children when one fails  
- **RestForOne**: Restart failed child and all started after it
- **Escalate**: Propagate failure to parent supervisor

### Restart Policies

Flexible restart policies with:
- Maximum restart limits
- Time window constraints
- Backoff strategies (Fixed, Linear, Exponential)
- Configurable delays

## Usage Examples

### Basic Setup

```rust
use mister_smith::supervision::prelude::*;

// Create supervision tree
let tree = SupervisionTree::new(SupervisionTreeConfig::default());

// Create root supervisor
let root = Arc::new(RootSupervisor::new(SupervisionStrategy::OneForAll));
let root_node = Arc::new(SupervisorNode::new(
    NodeId("root".to_string()),
    NodeType::RootSupervisor,
    None,
    root,
));

// Set root and start
tree.set_root(root_node).await?;
tree.start().await?;
```

### Creating Agent Supervisors

```rust
// Research agent supervisor with exponential backoff
let research_supervisor = Arc::new(AgentSupervisor::new(
    "research",
    SupervisionStrategy::OneForOne,
    RestartPolicy::exponential_backoff(),
));

// Code agent supervisor with restart limit
let code_supervisor = Arc::new(AgentSupervisor::new(
    "code",
    SupervisionStrategy::OneForOne,
    RestartPolicy::max_restarts(5),
));
```

### Circuit Breaker Protection

```rust
let circuit_breaker = CircuitBreaker::new(CircuitBreakerConfig {
    failure_threshold: 5,
    reset_timeout: Duration::from_secs(30),
    half_open_max_calls: 3,
    success_threshold_to_close: 2,
    operation_timeout: Some(Duration::from_secs(10)),
});

// Execute with protection
let result = circuit_breaker.call(async {
    external_service.process_request(request).await
}).await;
```

## Implementation Details

### Phi Accrual Failure Detection

The failure detector uses the Phi Accrual algorithm which:
- Adapts to network conditions dynamically
- Calculates a phi value based on heartbeat intervals
- Uses statistical distribution (CDF) for accuracy
- Configurable threshold (typical: 8-16)

### Hub-and-Spoke Routing

The supervision tree implements hub-and-spoke routing where:
- Supervisors act as hubs for task distribution
- Workers are spokes receiving routed tasks
- Central routing decisions based on task type
- Load balancing capabilities (extensible)

### Fault Isolation

Each supervisor node provides:
- Failure boundaries preventing cascade
- Isolated restart domains
- Configurable escalation policies
- Metrics tracking for monitoring

## Testing

Comprehensive test suite in `src/tests/supervision/`:

- **tree_tests.rs**: Tree structure and operations
- **strategy_tests.rs**: Supervision strategies
- **failure_detection_tests.rs**: Failure detector behavior
- **circuit_breaker_tests.rs**: Circuit breaker patterns
- **integration_tests.rs**: End-to-end scenarios

## Performance Considerations

1. **Lock-free operations** where possible using atomics
2. **Bounded collections** to prevent memory leaks
3. **Configurable timeouts** for all async operations
4. **Metrics collection** with minimal overhead

## Future Enhancements

1. **Distributed supervision** across nodes
2. **Dynamic reconfiguration** without restart
3. **Advanced routing algorithms** (consistent hashing, etc.)
4. **Integration with external monitoring** (Prometheus, etc.)
5. **State snapshots** for faster recovery

## Dependencies

Core dependencies used:
- `tokio`: Async runtime
- `async-trait`: Async trait support
- `tracing`: Structured logging
- `thiserror`: Error handling
- `serde`: Serialization support

## References

Based on supervision patterns from:
- Erlang/OTP supervision principles
- Akka supervision strategies
- Circuit breaker pattern (Michael Nygard)
- Phi Accrual failure detector (Naohiro Hayashibara et al.)