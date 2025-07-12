# Messaging System Capacity Planning

## Current NATS Configuration Analysis

Based on MisterSmith's NATS usage patterns, we calculate AWS messaging service requirements.

### NATS Usage Patterns
```rust
// Current NATS configuration
ping_interval: Duration::from_secs(30)
max_reconnects: Some(10)
connect_timeout: Duration::from_secs(10)

// Message types identified:
// 1. Agent coordination messages
// 2. Discovery events
// 3. Task assignment/completion
// 4. Health check pings
// 5. Metrics reporting
```

## Message Volume Analysis

### 1. Agent Coordination Messages
```yaml
coordination_messages:
  frequency: 2-10 per minute per agent
  size: 1-5 KB
  pattern: pub/sub with fanout
  durability: transient
  
  volume_calculation:
    agents: 1000
    messages_per_minute: 5000
    messages_per_second: 83
    data_rate: 415 KB/s
    daily_volume: 7.2M messages
    monthly_volume: 216M messages
```

### 2. Discovery Events
```yaml
discovery_events:
  frequency: 1-5 per minute per agent
  size: 2-10 KB
  pattern: pub/sub with persistence
  durability: persistent (24h)
  
  volume_calculation:
    agents: 1000
    events_per_minute: 2500
    events_per_second: 42
    data_rate: 420 KB/s
    daily_volume: 3.6M events
    monthly_volume: 108M events
```

### 3. Task Messages
```yaml
task_messages:
  frequency: 10-50 per minute per agent
  size: 5-20 KB
  pattern: queue with acknowledgment
  durability: persistent (7 days)
  
  volume_calculation:
    agents: 1000
    tasks_per_minute: 25000
    tasks_per_second: 417
    data_rate: 8.3 MB/s
    daily_volume: 36M tasks
    monthly_volume: 1.08B tasks
```

### 4. Health Check Pings
```yaml
health_pings:
  frequency: 1 per 30 seconds per agent
  size: 100-500 bytes
  pattern: pub/sub
  durability: transient
  
  volume_calculation:
    agents: 1000
    pings_per_minute: 2000
    pings_per_second: 33
    data_rate: 16.5 KB/s
    daily_volume: 2.9M pings
    monthly_volume: 86.4M pings
```

### 5. Metrics Reporting
```yaml
metrics_messages:
  frequency: 6 per minute per agent
  size: 0.5-2 KB
  pattern: pub/sub with aggregation
  durability: transient
  
  volume_calculation:
    agents: 1000
    metrics_per_minute: 6000
    metrics_per_second: 100
    data_rate: 150 KB/s
    daily_volume: 8.6M metrics
    monthly_volume: 259M metrics
```

## Total System Capacity

```yaml
total_capacity:
  messages_per_second: 675
  data_rate: 9.4 MB/s
  daily_volume: 58.3M messages
  monthly_volume: 1.75B messages
  
  peak_factors:
    normal_operations: 1.5x
    high_activity: 3x
    stress_conditions: 5x
    
  peak_requirements:
    messages_per_second: 3,375
    data_rate: 47 MB/s
    daily_volume: 291M messages
```

## AWS Messaging Service Options

### Option 1: Amazon MQ (NATS Alternative)

#### Configuration Options
```yaml
# Development
mq_dev:
  broker_type: mq.t3.micro
  deployment_mode: SINGLE_INSTANCE
  engine_type: ACTIVEMQ
  
  capacity:
    connections: 100
    throughput: 1000 msg/sec
    storage: 20 GB EBS
    
  cost_monthly: $13.50
  
# Staging
mq_staging:
  broker_type: mq.m5.large
  deployment_mode: ACTIVE_STANDBY_MULTI_AZ
  engine_type: ACTIVEMQ
  
  capacity:
    connections: 1000
    throughput: 10000 msg/sec
    storage: 200 GB EBS
    
  cost_monthly: $540.00
  
# Production
mq_production:
  broker_type: mq.m5.2xlarge
  deployment_mode: ACTIVE_STANDBY_MULTI_AZ
  engine_type: ACTIVEMQ
  
  capacity:
    connections: 10000
    throughput: 50000 msg/sec
    storage: 1000 GB EBS
    
  cost_monthly: $2,160.00
```

#### Amazon MQ Configuration
```xml
<!-- activemq.xml configuration -->
<broker xmlns="http://activemq.apache.org/schema/core">
  <destinationPolicy>
    <policyMap>
      <policyEntries>
        <policyEntry topic="agent.>" producerFlowControl="false">
          <subscriptionRecoveryPolicy>
            <lastImageSubscriptionRecoveryPolicy/>
          </subscriptionRecoveryPolicy>
        </policyEntry>
        <policyEntry queue="task.>" producerFlowControl="false">
          <deadLetterStrategy>
            <individualDeadLetterStrategy queuePrefix="DLQ."/>
          </deadLetterStrategy>
        </policyEntry>
      </policyEntries>
    </policyMap>
  </destinationPolicy>
  
  <systemUsage>
    <systemUsage>
      <memoryUsage>
        <memoryUsage percentOfJvmHeap="70"/>
      </memoryUsage>
      <storeUsage>
        <storeUsage limit="100 gb"/>
      </storeUsage>
      <tempUsage>
        <tempUsage limit="50 gb"/>
      </tempUsage>
    </systemUsage>
  </systemUsage>
</broker>
```

### Option 2: EventBridge + SQS/SNS

#### EventBridge Configuration
```yaml
eventbridge:
  event_bus: mistersmith-events
  
  rules:
    - name: agent-coordination
      pattern:
        source: ["mistersmith.agent"]
        detail-type: ["Coordination Message"]
      targets:
        - arn: ${coordination_topic_arn}
        
    - name: discovery-events
      pattern:
        source: ["mistersmith.discovery"]
        detail-type: ["Discovery Event"]
      targets:
        - arn: ${discovery_queue_arn}
        
    - name: task-routing
      pattern:
        source: ["mistersmith.task"]
        detail-type: ["Task Assignment"]
      targets:
        - arn: ${task_queue_arn}
  
  capacity:
    events_per_second: 10000
    event_size_limit: 256 KB
    
  cost_model:
    events_per_month: 1.75B
    cost_per_million: $1.00
    monthly_cost: $1,750.00
```

#### SQS Queue Configuration
```yaml
sqs_queues:
  task_queue:
    name: mistersmith-tasks
    type: standard
    visibility_timeout: 300
    message_retention: 1209600  # 14 days
    max_message_size: 262144    # 256 KB
    
    dlq:
      name: mistersmith-tasks-dlq
      max_receive_count: 3
      
    scaling:
      max_concurrent_batches: 1000
      batch_size: 10
      
  coordination_queue:
    name: mistersmith-coordination
    type: FIFO
    visibility_timeout: 60
    message_retention: 3600     # 1 hour
    
    deduplication:
      content_based: true
      deduplication_scope: messageGroup
```

#### SNS Topic Configuration
```yaml
sns_topics:
  discovery_events:
    name: mistersmith-discovery
    type: standard
    
    subscriptions:
      - protocol: sqs
        endpoint: ${discovery_processing_queue}
        filter_policy:
          confidence: [{"numeric": [">=", 0.8]}]
          
      - protocol: lambda
        endpoint: ${real_time_processor_arn}
        filter_policy:
          event_type: ["urgent", "critical"]
          
  agent_coordination:
    name: mistersmith-coordination
    type: standard
    
    subscriptions:
      - protocol: sqs
        endpoint: ${coordination_queue}
        
  fan_out_capacity:
    subscribers_per_topic: 100
    messages_per_second: 3000
    fan_out_ratio: 1:100
```

### Option 3: Amazon MSK (Managed Kafka)

#### MSK Cluster Configuration
```yaml
msk_cluster:
  name: mistersmith-kafka
  kafka_version: 2.8.1
  
  broker_config:
    instance_type: kafka.m5.large
    instance_count: 3
    storage_per_broker: 1000 GB
    
  topics:
    - name: agent-coordination
      partitions: 10
      replication_factor: 3
      cleanup_policy: delete
      retention_ms: 3600000  # 1 hour
      
    - name: discovery-events
      partitions: 50
      replication_factor: 3
      cleanup_policy: delete
      retention_ms: 86400000  # 24 hours
      
    - name: task-assignments
      partitions: 100
      replication_factor: 3
      cleanup_policy: delete
      retention_ms: 604800000  # 7 days
      
  capacity:
    throughput: 1 GB/s
    storage: 3 TB
    monthly_cost: $1,200
```

## Performance Comparison

### Throughput Analysis
```yaml
throughput_comparison:
  amazon_mq:
    max_throughput: 50000 msg/sec
    latency: 10-50ms
    durability: persistent
    ordering: limited
    
  eventbridge_sqs:
    max_throughput: 10000 msg/sec
    latency: 100-500ms
    durability: persistent
    ordering: FIFO queues only
    
  msk_kafka:
    max_throughput: 1000000 msg/sec
    latency: 1-10ms
    durability: configurable
    ordering: per partition
```

### Cost Analysis (Monthly)
```yaml
cost_comparison:
  amazon_mq:
    base_cost: $2160
    data_transfer: $100
    storage: $100
    total: $2360
    
  eventbridge_sqs:
    eventbridge: $1750
    sqs: $500
    sns: $200
    data_transfer: $150
    total: $2600
    
  msk_kafka:
    cluster: $1200
    data_transfer: $200
    storage: $300
    total: $1700
```

## Recommended Architecture

### Hybrid Approach
```yaml
recommended_solution:
  real_time_coordination:
    service: Amazon MQ
    use_case: Agent coordination, health checks
    volume: 20% of total
    
  async_processing:
    service: EventBridge + SQS
    use_case: Discovery events, task assignments
    volume: 70% of total
    
  high_throughput:
    service: MSK (if needed)
    use_case: Metrics, bulk data
    volume: 10% of total
```

### Migration Strategy
```yaml
phase_1:
  - Deploy Amazon MQ for critical paths
  - Implement EventBridge for async processing
  - Keep NATS for development/testing
  
phase_2:
  - Migrate remaining NATS usage
  - Optimize message routing
  - Implement monitoring
  
phase_3:
  - Add MSK if high throughput needed
  - Implement advanced routing
  - Optimize costs
```

## Monitoring and Alerting

### CloudWatch Metrics
```yaml
custom_metrics:
  - name: MessageProcessingLatency
    namespace: MisterSmith/Messaging
    dimensions:
      - MessageType
      - Source
      - Target
      
  - name: MessageThroughput
    namespace: MisterSmith/Messaging
    dimensions:
      - Service
      - Environment
      
  - name: QueueDepth
    namespace: MisterSmith/Messaging
    dimensions:
      - QueueName
      - MessageType
      
  - name: DeadLetterCount
    namespace: MisterSmith/Messaging
    dimensions:
      - SourceQueue
      - ErrorType
```

### Alerting Rules
```yaml
alerts:
  - name: HighMessageLatency
    metric: MessageProcessingLatency
    threshold: 1000ms
    comparison: GreaterThanThreshold
    
  - name: QueueBacklog
    metric: QueueDepth
    threshold: 1000
    comparison: GreaterThanThreshold
    
  - name: DeadLetterSpike
    metric: DeadLetterCount
    threshold: 10
    comparison: GreaterThanThreshold
    period: 300
    
  - name: ThroughputDrop
    metric: MessageThroughput
    threshold: 100
    comparison: LessThanThreshold
```