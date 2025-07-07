# MisterSmith Performance Baselines

This document defines the performance baselines and monitoring metrics for the MisterSmith framework based on the specifications in `ms-framework-docs/operations/observability-monitoring-framework.md`.

## Performance Baseline Categories

### 1. Agent Performance Baselines

#### Task Completion
- **P95 Duration**: 10 seconds
- **P99 Duration**: 30 seconds  
- **Success Rate**: 99%

#### Agent Spawn
- **P95 Duration**: 2 seconds
- **P99 Duration**: 5 seconds
- **Success Rate**: 99.5%

#### Resource Usage
- **Memory Baseline**: 256 MB
- **Memory Max**: 512 MB
- **CPU Baseline**: 15%
- **CPU Max**: 80%

### 2. Communication Baselines

#### Message Delivery
- **P95 Latency**: 50ms
- **P99 Latency**: 100ms
- **Delivery Success Rate**: 99.9%

#### Queue Processing
- **Processing Time**: 25ms
- **Max Queue Depth**: 100 messages

#### Context Propagation
- **Overhead**: 1ms
- **Success Rate**: 99.99%

### 3. System Baselines

#### Throughput
- **Tasks per Minute**: 100
- **Burst Capacity**: 500

#### Utilization
- **Optimal Range**: 70-85%
- **Max Sustainable**: 90%

#### Error Rates
- **Baseline Error Rate**: 0.01 (1%)
- **Alert Threshold**: 0.05 (5%)
- **Critical Threshold**: 0.10 (10%)

## Metrics Collection Implementation

The metrics system is implemented in `src/metrics/` with the following components:

### Core Modules

1. **`metrics/mod.rs`**: Main module exposing the metrics API
2. **`metrics/types.rs`**: Core metric types and structures
3. **`metrics/baselines.rs`**: Performance baseline definitions
4. **`metrics/collector.rs`**: Main metrics aggregation
5. **`metrics/agent_metrics.rs`**: Agent-specific metrics collection
6. **`metrics/system_metrics.rs`**: System-wide metrics collection
7. **`metrics/exporter.rs`**: Prometheus metrics exporter

### Key Metrics Collected

#### Agent Metrics
- `agent_operation_count`: Total operations performed
- `agent_operation_duration_ms`: Operation latency histogram
- `agent_error_count`: Error counter
- `agent_memory_usage_mb`: Current memory usage
- `agent_cpu_usage_percent`: Current CPU usage
- `agent_queue_depth`: Message queue depth

#### System Metrics
- `system_active_agents`: Number of active agents
- `system_total_tasks`: Total task counter
- `system_completed_tasks`: Successful task counter
- `system_failed_tasks`: Failed task counter
- `system_throughput_per_second`: Current throughput
- `system_average_latency_ms`: Average task latency

#### Backplane Metrics
- `backplane_consumer_lag_ms`: Consumer lag measurement
- `backplane_pending_messages`: Pending message count
- `backplane_ack_rate_per_second`: Message acknowledgment rate
- `backplane_reconnection_count`: Connection failures
- `backplane_message_size_p95`: 95th percentile message size
- `backplane_throughput_mb_per_second`: Data throughput

## Performance Testing

Performance tests are located in `src/tests/performance/` and include:

### Test Scenarios

1. **Agent Spawn Performance** (`agent_performance.rs`)
   - Tests agent creation latency
   - Validates against spawn baselines
   - Measures resource allocation

2. **Task Completion Performance** (`agent_performance.rs`)
   - Tests end-to-end task processing
   - Measures completion latency
   - Validates success rates

3. **Message Throughput** (`message_throughput.rs`)
   - Tests message delivery performance
   - Measures queue processing times
   - Validates delivery guarantees

4. **System Load** (`system_load.rs`)
   - Tests resource utilization under load
   - Measures CPU and memory usage
   - Validates against resource baselines

5. **Error Rate Testing** (`system_load.rs`)
   - Tests error handling performance
   - Validates error rate thresholds
   - Measures recovery times

### Running Performance Tests

```bash
# Run all performance tests
cargo test --package mistersmith --test performance -- --nocapture

# Run specific test
cargo test --package mistersmith --test performance agent_spawn -- --nocapture

# Run with custom configuration
PERF_TEST_DURATION=300 cargo test --package mistersmith --test performance
```

## Prometheus Integration

The metrics are exposed via Prometheus on port 9090 by default:

```bash
# Start metrics exporter
curl http://localhost:9090/metrics

# Health check
curl http://localhost:9090/health
```

### Grafana Dashboard

A Grafana dashboard configuration is available for visualizing these metrics:
- Agent performance overview
- System resource utilization
- Message throughput graphs
- Error rate monitoring
- Performance baseline comparisons

## Baseline Validation

The system automatically validates metrics against baselines and reports violations:

### Violation Severities

1. **Warning**: Performance degradation but within acceptable limits
2. **Error**: Performance below acceptable thresholds
3. **Critical**: Severe performance issues requiring immediate attention

### Automated Alerts

The monitoring system can be configured to send alerts when baseline violations occur:
- Slack notifications for warnings
- PagerDuty escalation for critical violations
- Email reports for trend analysis

## Continuous Monitoring

Performance baselines should be reviewed and updated based on:
- Production performance data
- Hardware improvements
- Framework optimizations
- Workload changes

Regular performance regression testing ensures the system maintains its performance characteristics across updates.