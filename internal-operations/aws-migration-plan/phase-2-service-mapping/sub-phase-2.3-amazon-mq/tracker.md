# Sub-Phase 2.3: Amazon MQ (NATS to AWS Messaging Migration)

## Overview

This tracker documents the migration from NATS messaging patterns to AWS managed messaging services for the MisterSmith Phase 2 implementation.

## Key Finding: Amazon MQ Limitations

Amazon MQ **does not support NATS protocol**. It only supports:
- Apache ActiveMQ Classic
- RabbitMQ

This requires a complete protocol translation approach rather than a simple service replacement.

## Recommended Architecture

### Primary: Amazon EventBridge
- **Purpose**: Event-driven pub/sub messaging
- **Benefits**: 
  - Serverless, no infrastructure management
  - Native AWS service integration
  - Content-based routing and filtering
  - Schema registry for event validation
- **Limitations**:
  - Different paradigm than NATS subjects
  - No built-in request/reply pattern
  - Maximum event size: 256 KB

### Secondary: Amazon SQS
- **Purpose**: Work queue distribution
- **Benefits**:
  - FIFO queues for ordered processing
  - Dead letter queues for error handling
  - Long polling for efficient message retrieval
- **Limitations**:
  - Pull-based model (vs NATS push)
  - No wildcard subscriptions

### Persistence: DynamoDB
- **Purpose**: Replace JetStream KV buckets
- **Benefits**:
  - Fully managed NoSQL
  - TTL support
  - Global tables for multi-region
- **Mapping**: AGENT_STATE bucket → DynamoDB table

### Real-time: API Gateway WebSockets
- **Purpose**: Replace NATS SSE patterns
- **Benefits**:
  - Managed WebSocket connections
  - Integration with Lambda
  - Connection state management

## NATS Pattern Mapping

### 1. Subject Hierarchy → EventBridge Patterns

#### NATS Subjects
```
agent.<agent_id>.<lifecycle_event>
work.<domain>.<priority>.<task_type>
coord.<workflow_id>.<stage>
```

#### EventBridge Event Patterns
```json
{
  "source": ["mistersmith.agent"],
  "detail-type": ["AgentLifecycle"],
  "detail": {
    "agentId": ["<agent_id>"],
    "event": ["created", "started", "ready", "heartbeat", "stopped", "error"]
  }
}
```

### 2. Work Queue Pattern → SQS Queue Groups

#### NATS Queue Groups
```rust
nc.queue_subscribe("work.nlp.>", "workers-nlp", handler);
```

#### SQS Implementation
```rust
// Create domain-specific queues
let nlp_queue_url = "https://sqs.region.amazonaws.com/account/work-nlp-queue";
let nlp_high_priority_queue_url = "https://sqs.region.amazonaws.com/account/work-nlp-high-queue";

// Worker polls from queue
let messages = sqs_client
    .receive_message()
    .queue_url(nlp_queue_url)
    .max_number_of_messages(10)
    .wait_time_seconds(20) // Long polling
    .send()
    .await?;
```

### 3. Request-Reply Pattern → Lambda + SQS

#### NATS Request-Reply
```rust
let response = nc.request("agent.nlp-001.cmd.status", empty, Duration::from_secs(5))?;
```

#### AWS Implementation
```rust
// Invoke Lambda directly for synchronous replies
let response = lambda_client
    .invoke()
    .function_name("agent-nlp-001-handler")
    .invocation_type(InvocationType::RequestResponse)
    .payload(command_payload)
    .send()
    .await?;
```

### 4. Broadcast Pattern → EventBridge Rules

#### NATS Broadcast
```rust
nc.publish("broadcast.global.config_update", config_data);
```

#### EventBridge Implementation
```rust
let event = PutEventsRequestEntry::builder()
    .source("mistersmith.broadcast")
    .detail_type("GlobalConfigUpdate")
    .detail(json!(config_data).to_string())
    .build();

eventbridge_client
    .put_events()
    .entries(event)
    .send()
    .await?;
```

## Implementation Code Examples

### 1. Protocol Translation Layer

```rust
use aws_sdk_eventbridge::{Client as EventBridgeClient, types::PutEventsRequestEntry};
use aws_sdk_sqs::{Client as SqsClient};
use std::collections::HashMap;

/// Translates NATS subjects to AWS EventBridge patterns
pub struct NatsToAwsTranslator {
    eventbridge: EventBridgeClient,
    sqs: SqsClient,
    queue_mappings: HashMap<String, String>,
}

impl NatsToAwsTranslator {
    /// Translates NATS subject to EventBridge event
    pub async fn publish(&self, subject: &str, payload: Vec<u8>) -> Result<(), Error> {
        let parts: Vec<&str> = subject.split('.').collect();
        
        match parts.get(0) {
            Some(&"agent") => self.publish_agent_event(&parts, payload).await,
            Some(&"work") => self.publish_work_event(&parts, payload).await,
            Some(&"coord") => self.publish_coordination_event(&parts, payload).await,
            Some(&"broadcast") => self.publish_broadcast_event(&parts, payload).await,
            _ => Err(Error::UnknownSubjectPattern),
        }
    }
    
    async fn publish_agent_event(&self, parts: &[&str], payload: Vec<u8>) -> Result<(), Error> {
        let agent_id = parts.get(1).unwrap_or(&"unknown");
        let event_type = parts.get(2).unwrap_or(&"unknown");
        
        let event = PutEventsRequestEntry::builder()
            .source("mistersmith.agent")
            .detail_type("AgentLifecycle")
            .detail(json!({
                "agentId": agent_id,
                "event": event_type,
                "payload": base64::encode(&payload),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string())
            .build();
            
        self.eventbridge
            .put_events()
            .entries(event)
            .send()
            .await?;
            
        Ok(())
    }
    
    async fn publish_work_event(&self, parts: &[&str], payload: Vec<u8>) -> Result<(), Error> {
        let domain = parts.get(1).unwrap_or(&"general");
        let priority = parts.get(2).unwrap_or(&"normal");
        let task_type = parts.get(3).unwrap_or(&"default");
        
        // Route to appropriate SQS queue based on domain and priority
        let queue_key = format!("{}-{}", domain, priority);
        let queue_url = self.queue_mappings.get(&queue_key)
            .ok_or(Error::NoQueueMapping)?;
            
        self.sqs
            .send_message()
            .queue_url(queue_url)
            .message_body(String::from_utf8_lossy(&payload).to_string())
            .message_attributes(
                "TaskType",
                MessageAttributeValue::builder()
                    .string_value(task_type)
                    .data_type("String")
                    .build()
            )
            .send()
            .await?;
            
        Ok(())
    }
}
```

### 2. Work Queue Consumer

```rust
use aws_sdk_sqs::{Client as SqsClient, types::Message};
use tokio::time::{interval, Duration};

pub struct AwsWorkQueueConsumer {
    sqs: SqsClient,
    queue_url: String,
    handler: Box<dyn Fn(Message) -> BoxFuture<'static, Result<(), Error>> + Send + Sync>,
}

impl AwsWorkQueueConsumer {
    pub async fn start(&self) -> Result<(), Error> {
        let mut ticker = interval(Duration::from_millis(100));
        
        loop {
            ticker.tick().await;
            
            // Long polling for efficiency
            let resp = self.sqs
                .receive_message()
                .queue_url(&self.queue_url)
                .max_number_of_messages(10)
                .wait_time_seconds(20)
                .send()
                .await?;
                
            if let Some(messages) = resp.messages {
                for message in messages {
                    let receipt_handle = message.receipt_handle.clone();
                    
                    // Process message
                    match (self.handler)(message).await {
                        Ok(_) => {
                            // Delete message on success
                            if let Some(handle) = receipt_handle {
                                self.sqs
                                    .delete_message()
                                    .queue_url(&self.queue_url)
                                    .receipt_handle(handle)
                                    .send()
                                    .await?;
                            }
                        }
                        Err(e) => {
                            // Message will become visible again after visibility timeout
                            error!("Failed to process message: {}", e);
                        }
                    }
                }
            }
        }
    }
}
```

### 3. EventBridge Rule Handler (Lambda)

```rust
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct EventBridgeEvent {
    source: String,
    #[serde(rename = "detail-type")]
    detail_type: String,
    detail: AgentEvent,
}

#[derive(Deserialize, Serialize)]
struct AgentEvent {
    #[serde(rename = "agentId")]
    agent_id: String,
    event: String,
    payload: String,
    timestamp: String,
}

async fn function_handler(event: LambdaEvent<EventBridgeEvent>) -> Result<(), Error> {
    let (event, _context) = event.into_parts();
    
    match event.detail_type.as_str() {
        "AgentLifecycle" => handle_agent_lifecycle(event.detail).await,
        "CoordinationEvent" => handle_coordination(event.detail).await,
        _ => {
            warn!("Unknown event type: {}", event.detail_type);
            Ok(())
        }
    }
}

async fn handle_agent_lifecycle(event: AgentEvent) -> Result<(), Error> {
    match event.event.as_str() {
        "heartbeat" => update_agent_state(&event.agent_id).await,
        "ready" => register_agent_ready(&event.agent_id).await,
        "error" => handle_agent_error(&event.agent_id, &event.payload).await,
        _ => Ok(()),
    }
}
```

### 4. State Management with DynamoDB

```rust
use aws_sdk_dynamodb::{Client as DynamoClient, types::AttributeValue};
use std::collections::HashMap;

pub struct AgentStateManager {
    dynamo: DynamoClient,
    table_name: String,
}

impl AgentStateManager {
    pub async fn update_agent_state(&self, agent_id: &str, state: &AgentState) -> Result<(), Error> {
        let mut item = HashMap::new();
        item.insert("agent_id".to_string(), AttributeValue::S(agent_id.to_string()));
        item.insert("status".to_string(), AttributeValue::S(state.status.clone()));
        item.insert("capabilities".to_string(), AttributeValue::L(
            state.capabilities.iter()
                .map(|c| AttributeValue::S(c.clone()))
                .collect()
        ));
        item.insert("current_load".to_string(), AttributeValue::N(state.current_load.to_string()));
        item.insert("last_heartbeat".to_string(), AttributeValue::S(state.last_heartbeat.clone()));
        item.insert("ttl".to_string(), AttributeValue::N(
            (chrono::Utc::now().timestamp() + 300).to_string() // 5 minute TTL
        ));
        
        self.dynamo
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await?;
            
        Ok(())
    }
    
    pub async fn get_agent_state(&self, agent_id: &str) -> Result<Option<AgentState>, Error> {
        let resp = self.dynamo
            .get_item()
            .table_name(&self.table_name)
            .key("agent_id", AttributeValue::S(agent_id.to_string()))
            .send()
            .await?;
            
        if let Some(item) = resp.item {
            Ok(Some(AgentState::from_dynamodb_item(item)?))
        } else {
            Ok(None)
        }
    }
}
```

## Migration Complexity Analysis

### High Complexity Items
1. **Subject Wildcard Patterns**: NATS wildcards (`>`, `*`) don't map directly to EventBridge
2. **Queue Groups**: Requires multiple SQS queues instead of single NATS subject
3. **Request-Reply**: Must use Lambda invocation or correlation IDs with SQS
4. **Connection Pooling**: Not applicable in serverless model

### Medium Complexity Items
1. **Message Ordering**: Use SQS FIFO queues
2. **Dead Letter Handling**: Built into SQS
3. **Heartbeat Patterns**: Use EventBridge scheduled rules
4. **Circuit Breaker**: Implement in Lambda with DynamoDB state

### Low Complexity Items
1. **Message Persistence**: DynamoDB provides better guarantees than JetStream KV
2. **Monitoring**: CloudWatch integration is superior
3. **Security**: IAM provides fine-grained access control

## Performance Considerations

### NATS Performance Baseline
- Core NATS: 3M+ messages/second
- JetStream: 200K messages/second

### AWS Services Performance
- EventBridge: 10,000 events/second per region (soft limit)
- SQS: 3,000 messages/second per queue (can scale with multiple queues)
- DynamoDB: 40,000 read/write units per table

### Mitigation Strategies
1. **Fan-out with SNS**: For high-volume broadcasts
2. **Kinesis Data Streams**: For ultra-high throughput requirements
3. **Batch Operations**: Aggregate messages before sending
4. **Regional Distribution**: Use multiple regions for scale

## Cost Implications

### EventBridge Pricing
- $1.00 per million events published
- No charge for rules or targets

### SQS Pricing
- $0.40 per million requests (after 1M free)
- $0.00 for data transfer within region

### DynamoDB Pricing
- On-demand: $0.25 per million writes, $0.25 per million reads
- Provisioned: More cost-effective for steady workloads

### Estimated Monthly Cost (1000 agents, moderate load)
- EventBridge: ~$50 (50M events)
- SQS: ~$100 (250M messages)
- DynamoDB: ~$75 (on-demand mode)
- **Total: ~$225/month**

## Migration Steps

1. **Phase 1**: Implement translation layer
   - Build NatsToAwsTranslator
   - Map all subject patterns
   - Create EventBridge rules

2. **Phase 2**: Set up work queues
   - Create SQS queues per domain/priority
   - Implement queue consumers
   - Add dead letter queues

3. **Phase 3**: State management
   - Create DynamoDB tables
   - Implement TTL for heartbeats
   - Add global secondary indexes

4. **Phase 4**: Monitoring and observability
   - CloudWatch dashboards
   - X-Ray tracing
   - CloudWatch Logs insights

## Recommendations

1. **Start with EventBridge**: Best fit for event-driven patterns
2. **Use SQS for work distribution**: More reliable than EventBridge for high-volume work
3. **Consider Step Functions**: For complex coordination patterns
4. **Implement gradually**: Start with agent lifecycle events, then work queues
5. **Monitor costs closely**: AWS managed services can be more expensive than self-hosted NATS

## Next Steps

1. Create detailed CloudFormation/CDK templates
2. Build proof-of-concept for critical paths
3. Performance test with expected load
4. Create runbooks for operations
5. Plan rollback strategy

---

*Last Updated: 2025-01-10*
*Status: Initial Analysis Complete*
*Next Review: After POC Implementation*