# Sub-Phase 2.5: Resource Sizing - Tracker

## Summary
Calculate AWS resource requirements based on MisterSmith performance baselines and scaling needs.

## Status
🔄 IN PROGRESS - Started: 2025-01-10

## Key Findings from Phase 1 Analysis

### Performance Baselines
1. **Connection Limits**
   - SSE Connections: Up to 1000 concurrent
   - Database Connections: 30 connection pool
   - NATS Connections: 10-100 concurrent
   - WebSocket/gRPC: TBD based on agent count

2. **Timeout Settings**
   - NATS Connect Timeout: 10 seconds
   - NATS Ping Interval: 30 seconds
   - Health Check Interval: 60 seconds
   - Max Reconnects: 10 attempts

3. **Agent Configuration**
   - Max Workers: 8 (default), scalable to 1000
   - Memory per Agent: 100MB default
   - Consensus Algorithm: majority/unanimous/quorum
   - Auto-scaling: Enabled by default

4. **Service Ports**
   - API Server: 8080
   - Metrics: 9090
   - OTLP gRPC: 4317
   - OTLP HTTP: 4318
   - NATS: 4222

## Calculated Resource Requirements

### 1. ECS Task Definitions

#### Supervisor Service (Control Plane)
```yaml
Scale Factor | vCPU | Memory | Expected Load
-------------|------|--------|---------------
1x (Base)    | 0.5  | 1 GB   | 1-10 agents
10x          | 1    | 2 GB   | 10-50 agents
50x          | 2    | 4 GB   | 50-100 agents
100x         | 4    | 8 GB   | 100+ agents

Recommended Production: 2 vCPU, 4 GB Memory
```

#### Worker Service (Agent Execution)
```yaml
Per Agent    | vCPU | Memory | Concurrent Tasks
-------------|------|--------|------------------
Base         | 0.25 | 512 MB | 1-5 tasks
Standard     | 0.5  | 1 GB   | 5-10 tasks
Performance  | 1    | 2 GB   | 10-20 tasks

Scale: 1-1000 agents
Recommended: 0.5 vCPU, 1 GB per agent
Max Cluster: 500 vCPU, 1 TB memory (1000 agents)
```

#### Gateway Service (API/WebSocket)
```yaml
Connections  | vCPU | Memory | Bandwidth
-------------|------|--------|----------
100          | 0.5  | 1 GB   | 10 Mbps
1000         | 2    | 4 GB   | 100 Mbps
10000        | 4    | 8 GB   | 1 Gbps

Recommended: 2 vCPU, 4 GB (handles 1000 SSE)
```

### 2. Aurora Serverless v2 Sizing

#### ACU Requirements
```yaml
Workload          | Min ACU | Max ACU | Storage
------------------|---------|---------|--------
Development       | 0.5     | 2       | 10 GB
Staging           | 1       | 8       | 50 GB
Production (Base) | 2       | 16      | 100 GB
Production (Peak) | 4       | 32      | 500 GB

Connection Pool: 30 per service instance
Max Connections: 960 (32 ACU)
IOPS: 3000 baseline, burstable to 10000
```

#### Database Calculations
- Agent State: ~1KB per agent per update
- Discovery Events: ~2KB per event
- Metrics: ~500 bytes per metric
- Estimated Growth: 10GB/month at 1000 agents

### 3. Message Queue Capacity

#### Amazon MQ (NATS Alternative)
```yaml
Broker Type   | Instance    | Connections | Throughput
--------------|-------------|-------------|------------
Development   | mq.t3.micro | 100         | 1K msg/sec
Staging       | mq.m5.large | 1000        | 10K msg/sec
Production    | mq.m5.xlarge| 5000        | 50K msg/sec

Message Size: 1-10KB average
Retention: 24 hours
Storage: 100GB EBS
```

#### EventBridge (Alternative)
```yaml
Event Bus     | Events/sec | Size Limit | Cost Model
--------------|------------|------------|------------
Custom Bus    | 10,000     | 256KB      | Per event
Partner Bus   | 2,400      | 256KB      | Subscription

Recommended: Custom Event Bus
Estimated: 1M events/day at peak
```

### 4. Auto-Scaling Configuration

#### ECS Service Scaling
```yaml
Service       | Metric            | Target | Min | Max
--------------|-------------------|--------|-----|-----
Supervisor    | CPU Utilization   | 70%    | 1   | 10
Worker        | Custom (Agents)   | N/A    | 1   | 100
Gateway       | Connection Count  | 800    | 2   | 20

Scale-out: 60 seconds
Scale-in: 300 seconds
```

#### Aurora Scaling
```yaml
Metric: Average CPU Utilization
Target: 70%
Scale-up: When > 70% for 60 seconds
Scale-down: When < 50% for 300 seconds
```

### 5. Monitoring Requirements

#### CloudWatch Metrics
```yaml
Custom Metrics:
- Agent count by state
- Task execution time
- Discovery events/sec
- Consensus latency
- Memory usage per agent

Dashboards:
- System Overview
- Agent Performance
- API Gateway
- Database Health

Alarms:
- High CPU (>80%)
- Memory pressure (>90%)
- Connection exhaustion
- Error rate (>1%)
- Latency (>1s p99)
```

#### X-Ray Tracing
```yaml
Sampling Rate: 10% (adjustable)
Trace Storage: 30 days
Segments/month: ~10M at 1000 agents
```

### 6. Network Bandwidth

```yaml
Component     | Ingress    | Egress     | Total/month
--------------|------------|------------|------------
API Gateway   | 100 Mbps   | 200 Mbps   | 1 TB
NATS/MQ       | 50 Mbps    | 50 Mbps    | 500 GB
Aurora        | 20 Mbps    | 20 Mbps    | 200 GB
Total         | 170 Mbps   | 270 Mbps   | 1.7 TB
```

## Cost Optimization Strategies

1. **Compute**
   - Use Spot instances for worker agents (70% savings)
   - Reserved Instances for supervisor/gateway
   - Implement aggressive auto-scaling

2. **Database**
   - Aurora Serverless v2 for automatic scaling
   - Enable RDS Proxy for connection pooling
   - Use read replicas for analytics

3. **Messaging**
   - Consider EventBridge for event-driven
   - Use SQS/SNS for simpler patterns
   - Batch message processing

4. **Monitoring**
   - Set appropriate metric retention
   - Use metric filters for aggregation
   - Sample traces intelligently

## Next Steps

- [ ] Create CloudFormation templates with resource definitions
- [ ] Set up auto-scaling policies
- [ ] Configure CloudWatch dashboards
- [ ] Implement cost allocation tags
- [ ] Design capacity testing scenarios

## Dependencies
- Phase 1.4: Configuration extraction ✅
- Phase 2.1: Service decomposition ✅
- Phase 2.2: Container specifications ✅

## Outputs
- ECS task definitions with resource allocations
- Aurora Serverless v2 configuration
- Auto-scaling policies
- CloudWatch dashboard definitions
- Cost estimation model