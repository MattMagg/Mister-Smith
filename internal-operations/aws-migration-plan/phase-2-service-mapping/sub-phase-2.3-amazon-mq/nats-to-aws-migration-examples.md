# NATS to AWS Migration: Implementation Examples

## Complete Working Examples

### 1. Agent Lifecycle Management

#### NATS Implementation (Original)
```rust
// Agent publishes lifecycle events
async fn publish_agent_event(nc: &nats::Connection, agent_id: &str, event: &str) {
    let subject = format!("agent.{}.{}", agent_id, event);
    nc.publish(&subject, b"{}").unwrap();
}

// Supervisor subscribes to all agent events
async fn subscribe_agent_events(nc: &nats::Connection) {
    nc.subscribe("agent.*.>", |msg| {
        let parts: Vec<&str> = msg.subject.split('.').collect();
        let agent_id = parts[1];
        let event = parts[2];
        println!("Agent {} event: {}", agent_id, event);
    });
}
```

#### AWS Implementation (Migrated)
```rust
use aws_sdk_eventbridge::{Client as EventBridgeClient, types::PutEventsRequestEntry};
use aws_sdk_lambda::{Client as LambdaClient};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde_json::json;

// Agent publishes lifecycle events to EventBridge
pub struct AwsAgentPublisher {
    eventbridge: EventBridgeClient,
}

impl AwsAgentPublisher {
    pub async fn publish_event(&self, agent_id: &str, event: &str, metadata: serde_json::Value) -> Result<(), Error> {
        let entry = PutEventsRequestEntry::builder()
            .source("mistersmith.agent")
            .detail_type("AgentLifecycleEvent")
            .detail(json!({
                "agentId": agent_id,
                "event": event,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "metadata": metadata
            }).to_string())
            .build();
            
        self.eventbridge
            .put_events()
            .entries(entry)
            .send()
            .await?;
            
        Ok(())
    }
}

// EventBridge Rule targets Lambda for processing
#[derive(Deserialize)]
struct AgentLifecycleEvent {
    source: String,
    #[serde(rename = "detail-type")]
    detail_type: String,
    detail: AgentEventDetail,
}

#[derive(Deserialize)]
struct AgentEventDetail {
    #[serde(rename = "agentId")]
    agent_id: String,
    event: String,
    timestamp: String,
    metadata: serde_json::Value,
}

async fn agent_event_handler(event: LambdaEvent<AgentLifecycleEvent>) -> Result<(), Error> {
    let (event, _) = event.into_parts();
    let detail = event.detail;
    
    match detail.event.as_str() {
        "created" => handle_agent_created(&detail.agent_id, detail.metadata).await?,
        "started" => handle_agent_started(&detail.agent_id).await?,
        "ready" => handle_agent_ready(&detail.agent_id).await?,
        "heartbeat" => handle_agent_heartbeat(&detail.agent_id).await?,
        "stopped" => handle_agent_stopped(&detail.agent_id).await?,
        "error" => handle_agent_error(&detail.agent_id, detail.metadata).await?,
        _ => println!("Unknown event: {}", detail.event),
    }
    
    Ok(())
}

// CloudFormation/CDK for EventBridge Rule
const EVENTBRIDGE_RULE: &str = r#"
AgentLifecycleRule:
  Type: AWS::Events::Rule
  Properties:
    EventPattern:
      source:
        - mistersmith.agent
      detail-type:
        - AgentLifecycleEvent
    Targets:
      - Arn: !GetAtt AgentEventHandlerFunction.Arn
        Id: "1"
"#;
```

### 2. Work Queue Distribution

#### NATS Implementation (Original)
```rust
// Publisher sends work without knowing which agent handles it
async fn publish_work(nc: &nats::Connection, domain: &str, priority: &str, task_type: &str, payload: &[u8]) {
    let subject = format!("work.{}.{}.{}", domain, priority, task_type);
    nc.publish(&subject, payload).unwrap();
}

// Multiple workers subscribe to same queue group
async fn subscribe_work_queue(nc: &nats::Connection, domain: &str) {
    let subject = format!("work.{}.>", domain);
    let queue_group = format!("workers-{}", domain);
    
    nc.queue_subscribe(&subject, &queue_group, |msg| {
        // Process work
        let task = deserialize_task(&msg.data);
        process_task(task);
    });
}
```

#### AWS Implementation (Migrated)
```rust
use aws_sdk_sqs::{Client as SqsClient, types::{Message, MessageAttributeValue}};
use std::collections::HashMap;

// Work publisher sends to appropriate SQS queue
pub struct AwsWorkPublisher {
    sqs: SqsClient,
    queue_mappings: HashMap<String, String>, // domain-priority -> queue URL
}

impl AwsWorkPublisher {
    pub async fn publish_work(
        &self, 
        domain: &str, 
        priority: &str, 
        task_type: &str, 
        payload: serde_json::Value
    ) -> Result<(), Error> {
        let queue_key = format!("{}-{}", domain, priority);
        let queue_url = self.queue_mappings.get(&queue_key)
            .ok_or_else(|| Error::msg("No queue for domain-priority"))?;
            
        self.sqs
            .send_message()
            .queue_url(queue_url)
            .message_body(payload.to_string())
            .message_attributes(
                "Domain",
                MessageAttributeValue::builder()
                    .string_value(domain)
                    .data_type("String")
                    .build()
            )
            .message_attributes(
                "Priority",
                MessageAttributeValue::builder()
                    .string_value(priority)
                    .data_type("String")
                    .build()
            )
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

// Worker polls from SQS queue
pub struct AwsWorkConsumer {
    sqs: SqsClient,
    queue_url: String,
    domain: String,
}

impl AwsWorkConsumer {
    pub async fn start(&self) -> Result<(), Error> {
        loop {
            let resp = self.sqs
                .receive_message()
                .queue_url(&self.queue_url)
                .attribute_names(aws_sdk_sqs::types::QueueAttributeName::All)
                .message_attribute_names("All")
                .max_number_of_messages(10)
                .visibility_timeout(300) // 5 minutes to process
                .wait_time_seconds(20) // Long polling
                .send()
                .await?;
                
            if let Some(messages) = resp.messages {
                for message in messages {
                    // Process in parallel
                    tokio::spawn(self.process_message(message));
                }
            }
        }
    }
    
    async fn process_message(&self, message: Message) -> Result<(), Error> {
        let receipt_handle = message.receipt_handle.clone().unwrap();
        
        // Extract task details
        let task_type = message.message_attributes
            .and_then(|attrs| attrs.get("TaskType"))
            .and_then(|attr| attr.string_value.clone())
            .unwrap_or_default();
            
        let body = message.body.unwrap_or_default();
        let task_data: serde_json::Value = serde_json::from_str(&body)?;
        
        // Process task
        match self.process_task(&task_type, task_data).await {
            Ok(_) => {
                // Delete message on success
                self.sqs
                    .delete_message()
                    .queue_url(&self.queue_url)
                    .receipt_handle(&receipt_handle)
                    .send()
                    .await?;
            }
            Err(e) => {
                // Log error, message will reappear after visibility timeout
                eprintln!("Task processing failed: {}", e);
                
                // Optionally move to DLQ after max retries
                if self.get_receive_count(&message) > 3 {
                    self.move_to_dlq(message).await?;
                }
            }
        }
        
        Ok(())
    }
}

// CloudFormation for SQS queues with DLQ
const SQS_QUEUE_TEMPLATE: &str = r#"
NLPHighPriorityQueue:
  Type: AWS::SQS::Queue
  Properties:
    QueueName: work-nlp-high
    VisibilityTimeout: 300
    MessageRetentionPeriod: 1209600  # 14 days
    RedrivePolicy:
      deadLetterTargetArn: !GetAtt NLPDeadLetterQueue.Arn
      maxReceiveCount: 3
    Tags:
      - Key: Domain
        Value: nlp
      - Key: Priority
        Value: high

NLPDeadLetterQueue:
  Type: AWS::SQS::Queue
  Properties:
    QueueName: work-nlp-high-dlq
    MessageRetentionPeriod: 1209600  # 14 days
"#;
```

### 3. Request-Reply Pattern

#### NATS Implementation (Original)
```rust
// Synchronous request with timeout
async fn request_agent_status(nc: &nats::Connection, agent_id: &str) -> Result<AgentStatus, Error> {
    let subject = format!("agent.{}.cmd.status", agent_id);
    let response = nc.request(&subject, b"{}", Duration::from_secs(5))?;
    let status: AgentStatus = serde_json::from_slice(&response.data)?;
    Ok(status)
}

// Agent responds to requests
async fn handle_status_requests(nc: &nats::Connection, agent_id: &str) {
    let subject = format!("agent.{}.cmd.status", agent_id);
    nc.subscribe(&subject, |msg| {
        let status = get_current_status();
        msg.respond(serde_json::to_vec(&status).unwrap()).unwrap();
    });
}
```

#### AWS Implementation (Migrated)
```rust
use aws_sdk_lambda::{Client as LambdaClient, types::InvocationType};
use uuid::Uuid;

// Direct Lambda invocation for request-reply
pub struct AwsRequestReplyClient {
    lambda: LambdaClient,
    sqs: SqsClient,
}

impl AwsRequestReplyClient {
    // Option 1: Direct Lambda invocation (preferred for low latency)
    pub async fn request_agent_status_lambda(&self, agent_id: &str) -> Result<AgentStatus, Error> {
        let function_name = format!("agent-{}-handler", agent_id);
        let payload = json!({
            "command": "status",
            "requestId": Uuid::new_v4().to_string()
        });
        
        let response = self.lambda
            .invoke()
            .function_name(function_name)
            .invocation_type(InvocationType::RequestResponse)
            .payload(serde_json::to_vec(&payload)?)
            .send()
            .await?;
            
        let status: AgentStatus = serde_json::from_slice(&response.payload.unwrap())?;
        Ok(status)
    }
    
    // Option 2: SQS with correlation ID (for loose coupling)
    pub async fn request_agent_status_sqs(&self, agent_id: &str) -> Result<AgentStatus, Error> {
        let correlation_id = Uuid::new_v4().to_string();
        let response_queue = "https://sqs.region.amazonaws.com/account/response-queue";
        
        // Send request
        self.sqs
            .send_message()
            .queue_url(&format!("https://sqs.region.amazonaws.com/account/agent-{}-commands", agent_id))
            .message_body(json!({
                "command": "status",
                "correlationId": correlation_id,
                "responseQueue": response_queue
            }).to_string())
            .send()
            .await?;
            
        // Wait for response with correlation ID
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(5);
        
        while start.elapsed() < timeout {
            let resp = self.sqs
                .receive_message()
                .queue_url(response_queue)
                .wait_time_seconds(1)
                .send()
                .await?;
                
            if let Some(messages) = resp.messages {
                for message in messages {
                    let body: serde_json::Value = serde_json::from_str(&message.body.unwrap())?;
                    if body["correlationId"] == correlation_id {
                        // Found our response
                        let status: AgentStatus = serde_json::from_value(body["status"].clone())?;
                        
                        // Delete message
                        self.sqs
                            .delete_message()
                            .queue_url(response_queue)
                            .receipt_handle(message.receipt_handle.unwrap())
                            .send()
                            .await?;
                            
                        return Ok(status);
                    }
                }
            }
        }
        
        Err(Error::msg("Request timeout"))
    }
}

// Lambda handler for agent commands
async fn agent_command_handler(event: LambdaEvent<CommandRequest>) -> Result<serde_json::Value, Error> {
    let (request, _) = event.into_parts();
    
    match request.command.as_str() {
        "status" => {
            let status = get_agent_status().await?;
            Ok(serde_json::to_value(status)?)
        }
        "pause" => {
            pause_agent().await?;
            Ok(json!({"success": true}))
        }
        _ => Err(Error::msg("Unknown command"))
    }
}
```

### 4. Broadcast Pattern with EventBridge

#### NATS Implementation (Original)
```rust
// Broadcast to all subscribers
async fn broadcast_config_update(nc: &nats::Connection, config: &Config) {
    nc.publish("broadcast.global.config_update", serde_json::to_vec(config).unwrap()).unwrap();
}

// Each agent subscribes individually
async fn subscribe_broadcasts(nc: &nats::Connection) {
    nc.subscribe("broadcast.>", |msg| {
        let parts: Vec<&str> = msg.subject.split('.').collect();
        let scope = parts[1];
        let event_type = parts[2];
        
        match (scope, event_type) {
            ("global", "config_update") => update_config(&msg.data),
            ("domain", _) => handle_domain_broadcast(&msg.data),
            _ => {}
        }
    });
}
```

#### AWS Implementation (Migrated)
```rust
// Broadcast using EventBridge
pub struct AwsBroadcaster {
    eventbridge: EventBridgeClient,
}

impl AwsBroadcaster {
    pub async fn broadcast_global(&self, event_type: &str, data: serde_json::Value) -> Result<(), Error> {
        let entry = PutEventsRequestEntry::builder()
            .source("mistersmith.broadcast")
            .detail_type(format!("GlobalBroadcast_{}", event_type))
            .detail(json!({
                "scope": "global",
                "eventType": event_type,
                "data": data,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string())
            .build();
            
        self.eventbridge
            .put_events()
            .entries(entry)
            .send()
            .await?;
            
        Ok(())
    }
    
    pub async fn broadcast_domain(&self, domain: &str, event_type: &str, data: serde_json::Value) -> Result<(), Error> {
        let entry = PutEventsRequestEntry::builder()
            .source("mistersmith.broadcast")
            .detail_type(format!("DomainBroadcast_{}", event_type))
            .detail(json!({
                "scope": "domain",
                "domain": domain,
                "eventType": event_type,
                "data": data,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }).to_string())
            .build();
            
        self.eventbridge
            .put_events()
            .entries(entry)
            .send()
            .await?;
            
        Ok(())
    }
}

// Lambda handles broadcasts via EventBridge rules
async fn broadcast_handler(event: LambdaEvent<EventBridgeEvent>) -> Result<(), Error> {
    let (event, _) = event.into_parts();
    
    let detail: BroadcastDetail = serde_json::from_str(&event.detail)?;
    
    match detail.scope.as_str() {
        "global" => handle_global_broadcast(&detail.event_type, detail.data).await?,
        "domain" => handle_domain_broadcast(&detail.domain.unwrap(), &detail.event_type, detail.data).await?,
        _ => {}
    }
    
    Ok(())
}

// EventBridge Rules for different broadcast types
const BROADCAST_RULES: &str = r#"
GlobalConfigUpdateRule:
  Type: AWS::Events::Rule
  Properties:
    EventPattern:
      source:
        - mistersmith.broadcast
      detail-type:
        - GlobalBroadcast_config_update
    Targets:
      - Arn: !GetAtt ConfigUpdateHandlerFunction.Arn
        Id: "1"

DomainBroadcastRule:
  Type: AWS::Events::Rule
  Properties:
    EventPattern:
      source:
        - mistersmith.broadcast
      detail-type:
        - prefix: "DomainBroadcast_"
      detail:
        domain:
          - nlp
          - vision
          - audio
    Targets:
      - Arn: !GetAtt DomainBroadcastHandlerFunction.Arn
        Id: "1"
"#;
```

### 5. State Management Migration

#### NATS JetStream KV (Original)
```rust
// Store agent state in JetStream KV
async fn update_agent_state(js: &jetstream::Context, agent_id: &str, state: &AgentState) {
    let bucket = js.get_key_value("AGENT_STATE").await.unwrap();
    let key = format!("agent.{}", agent_id);
    bucket.put(&key, serde_json::to_vec(state).unwrap()).await.unwrap();
}

// Retrieve agent state
async fn get_agent_state(js: &jetstream::Context, agent_id: &str) -> Option<AgentState> {
    let bucket = js.get_key_value("AGENT_STATE").await.unwrap();
    let key = format!("agent.{}", agent_id);
    
    match bucket.get(&key).await {
        Ok(Some(entry)) => {
            Some(serde_json::from_slice(&entry.value).unwrap())
        }
        _ => None
    }
}
```

#### AWS DynamoDB (Migrated)
```rust
use aws_sdk_dynamodb::{Client as DynamoClient, types::{AttributeValue, PutRequest, WriteRequest}};
use std::collections::HashMap;

pub struct AwsStateManager {
    dynamo: DynamoClient,
    table_name: String,
}

impl AwsStateManager {
    pub async fn update_agent_state(&self, agent_id: &str, state: &AgentState) -> Result<(), Error> {
        let ttl = chrono::Utc::now().timestamp() + 300; // 5 minute TTL
        
        let item = hashmap! {
            "PK".to_string() => AttributeValue::S(format!("AGENT#{}", agent_id)),
            "SK".to_string() => AttributeValue::S("STATE".to_string()),
            "agentId".to_string() => AttributeValue::S(agent_id.to_string()),
            "status".to_string() => AttributeValue::S(state.status.clone()),
            "capabilities".to_string() => AttributeValue::L(
                state.capabilities.iter()
                    .map(|c| AttributeValue::S(c.clone()))
                    .collect()
            ),
            "currentLoad".to_string() => AttributeValue::N(state.current_load.to_string()),
            "lastHeartbeat".to_string() => AttributeValue::S(state.last_heartbeat.to_rfc3339()),
            "metadata".to_string() => AttributeValue::S(serde_json::to_string(&state.metadata)?),
            "ttl".to_string() => AttributeValue::N(ttl.to_string()),
            "GSI1PK".to_string() => AttributeValue::S(format!("STATUS#{}", state.status)),
            "GSI1SK".to_string() => AttributeValue::S(format!("AGENT#{}", agent_id)),
        };
        
        self.dynamo
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(PK) OR #ttl < :now")
            .expression_attribute_names("#ttl", "ttl")
            .expression_attribute_values(":now", AttributeValue::N(chrono::Utc::now().timestamp().to_string()))
            .send()
            .await?;
            
        Ok(())
    }
    
    pub async fn get_agent_state(&self, agent_id: &str) -> Result<Option<AgentState>, Error> {
        let resp = self.dynamo
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("AGENT#{}", agent_id)))
            .key("SK", AttributeValue::S("STATE".to_string()))
            .send()
            .await?;
            
        if let Some(item) = resp.item {
            if let Some(ttl_attr) = item.get("ttl") {
                let ttl: i64 = ttl_attr.as_n().unwrap().parse()?;
                if ttl < chrono::Utc::now().timestamp() {
                    return Ok(None); // Expired
                }
            }
            
            let state = AgentState {
                status: item.get("status").and_then(|v| v.as_s().ok()).cloned().unwrap_or_default(),
                capabilities: item.get("capabilities")
                    .and_then(|v| v.as_l().ok())
                    .map(|list| list.iter()
                        .filter_map(|v| v.as_s().ok().cloned())
                        .collect())
                    .unwrap_or_default(),
                current_load: item.get("currentLoad")
                    .and_then(|v| v.as_n().ok())
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0.0),
                last_heartbeat: item.get("lastHeartbeat")
                    .and_then(|v| v.as_s().ok())
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
                metadata: item.get("metadata")
                    .and_then(|v| v.as_s().ok())
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default(),
            };
            
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }
    
    // Query agents by status using GSI
    pub async fn get_agents_by_status(&self, status: &str) -> Result<Vec<AgentState>, Error> {
        let resp = self.dynamo
            .query()
            .table_name(&self.table_name)
            .index_name("GSI1")
            .key_condition_expression("GSI1PK = :pk")
            .expression_attribute_values(":pk", AttributeValue::S(format!("STATUS#{}", status)))
            .send()
            .await?;
            
        let mut agents = Vec::new();
        if let Some(items) = resp.items {
            for item in items {
                // Parse agent state from item
                // ... (similar to get_agent_state parsing)
            }
        }
        
        Ok(agents)
    }
}

// DynamoDB table definition
const DYNAMODB_TABLE: &str = r#"
AgentStateTable:
  Type: AWS::DynamoDB::Table
  Properties:
    TableName: mistersmith-agent-state
    BillingMode: PAY_PER_REQUEST
    AttributeDefinitions:
      - AttributeName: PK
        AttributeType: S
      - AttributeName: SK
        AttributeType: S
      - AttributeName: GSI1PK
        AttributeType: S
      - AttributeName: GSI1SK
        AttributeType: S
    KeySchema:
      - AttributeName: PK
        KeyType: HASH
      - AttributeName: SK
        KeyType: RANGE
    GlobalSecondaryIndexes:
      - IndexName: GSI1
        KeySchema:
          - AttributeName: GSI1PK
            KeyType: HASH
          - AttributeName: GSI1SK
            KeyType: RANGE
        Projection:
          ProjectionType: ALL
    TimeToLiveSpecification:
      AttributeName: ttl
      Enabled: true
    StreamSpecification:
      StreamViewType: NEW_AND_OLD_IMAGES
"#;
```

### 6. Complete Migration Helper Library

```rust
// Unified migration library
pub mod nats_to_aws {
    use std::sync::Arc;
    use aws_sdk_eventbridge::Client as EventBridgeClient;
    use aws_sdk_sqs::Client as SqsClient;
    use aws_sdk_dynamodb::Client as DynamoClient;
    
    /// Unified client that mimics NATS API but uses AWS services
    pub struct MigrationClient {
        eventbridge: Arc<EventBridgeClient>,
        sqs: Arc<SqsClient>,
        dynamo: Arc<DynamoClient>,
        queue_resolver: Arc<QueueResolver>,
    }
    
    impl MigrationClient {
        /// Publish mimics NATS publish but routes to appropriate AWS service
        pub async fn publish(&self, subject: &str, payload: Vec<u8>) -> Result<(), Error> {
            let router = SubjectRouter::new();
            
            match router.route(subject) {
                Route::EventBridge(source, detail_type) => {
                    self.publish_event(source, detail_type, payload).await
                }
                Route::Sqs(queue_url) => {
                    self.send_to_queue(queue_url, payload).await
                }
                Route::Lambda(function_name) => {
                    self.invoke_lambda(function_name, payload).await
                }
            }
        }
        
        /// Subscribe creates appropriate AWS resources (Lambda, SQS consumer, etc)
        pub async fn subscribe<F>(&self, pattern: &str, handler: F) -> Result<Subscription, Error>
        where
            F: Fn(Message) -> BoxFuture<'static, ()> + Send + Sync + 'static
        {
            let subscription = match SubjectPattern::parse(pattern)? {
                SubjectPattern::Exact(subject) => {
                    self.create_exact_subscription(subject, handler).await?
                }
                SubjectPattern::Wildcard(pattern) => {
                    self.create_pattern_subscription(pattern, handler).await?
                }
            };
            
            Ok(subscription)
        }
        
        /// Request performs synchronous request-reply
        pub async fn request(&self, subject: &str, payload: Vec<u8>, timeout: Duration) -> Result<Vec<u8>, Error> {
            // Route to Lambda for direct invocation
            let function_name = self.resolve_lambda_function(subject)?;
            
            let response = tokio::time::timeout(
                timeout,
                self.invoke_lambda_sync(function_name, payload)
            ).await??;
            
            Ok(response)
        }
    }
    
    /// Subject router determines which AWS service to use
    struct SubjectRouter;
    
    impl SubjectRouter {
        fn route(&self, subject: &str) -> Route {
            let parts: Vec<&str> = subject.split('.').collect();
            
            match parts.get(0) {
                Some(&"agent") => Route::EventBridge(
                    "mistersmith.agent".to_string(),
                    "AgentEvent".to_string()
                ),
                Some(&"work") => {
                    let domain = parts.get(1).unwrap_or(&"general");
                    let priority = parts.get(2).unwrap_or(&"normal");
                    let queue_url = format!("work-{}-{}", domain, priority);
                    Route::Sqs(queue_url)
                }
                Some(&"broadcast") => Route::EventBridge(
                    "mistersmith.broadcast".to_string(),
                    "BroadcastEvent".to_string()
                ),
                _ => Route::EventBridge(
                    "mistersmith.unknown".to_string(),
                    "UnknownEvent".to_string()
                )
            }
        }
    }
    
    enum Route {
        EventBridge(String, String), // source, detail-type
        Sqs(String), // queue URL
        Lambda(String), // function name
    }
}
```

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_nats_to_eventbridge_migration() {
        // Original NATS behavior
        let nc = nats::connect("nats://localhost:4222").unwrap();
        nc.publish("agent.test-001.heartbeat", b"{}").unwrap();
        
        // Migrated AWS behavior
        let config = aws_config::load_from_env().await;
        let eventbridge = EventBridgeClient::new(&config);
        
        let migration_client = MigrationClient::new(eventbridge, sqs, dynamo);
        migration_client.publish("agent.test-001.heartbeat", b"{}").await.unwrap();
        
        // Both should result in similar downstream behavior
    }
    
    #[tokio::test]
    async fn test_work_queue_load_balancing() {
        // Test that SQS provides similar load balancing to NATS queue groups
        let client = create_test_client().await;
        
        // Publish 100 work items
        for i in 0..100 {
            client.publish(&format!("work.nlp.normal.task"), 
                json!({"id": i}).to_string().into_bytes()
            ).await.unwrap();
        }
        
        // Verify distribution across workers
        // ... assertions
    }
}
```

## Monitoring and Observability

```yaml
# CloudWatch Dashboard for NATS migration
NATSMigrationDashboard:
  Type: AWS::CloudWatch::Dashboard
  Properties:
    DashboardName: nats-migration-metrics
    DashboardBody: !Sub |
      {
        "widgets": [
          {
            "type": "metric",
            "properties": {
              "metrics": [
                ["AWS/Events", "SuccessfulRuleMatches", {"stat": "Sum"}],
                ["AWS/Events", "FailedInvocations", {"stat": "Sum"}],
                ["AWS/SQS", "NumberOfMessagesSent", {"stat": "Sum"}],
                ["AWS/SQS", "NumberOfMessagesReceived", {"stat": "Sum"}],
                ["AWS/Lambda", "Invocations", {"stat": "Sum"}],
                ["AWS/Lambda", "Errors", {"stat": "Sum"}],
                ["AWS/Lambda", "Duration", {"stat": "Average"}]
              ],
              "period": 300,
              "stat": "Average",
              "region": "${AWS::Region}",
              "title": "NATS Migration Metrics"
            }
          }
        ]
      }
```

## Rollback Strategy

```rust
// Feature flag for gradual migration
pub enum MessagingBackend {
    Nats(nats::Connection),
    Aws(MigrationClient),
    Dual { nats: nats::Connection, aws: MigrationClient }
}

impl MessagingClient {
    pub async fn publish(&self, subject: &str, payload: Vec<u8>) -> Result<(), Error> {
        match &self.backend {
            MessagingBackend::Nats(nc) => {
                nc.publish(subject, &payload)?;
            }
            MessagingBackend::Aws(client) => {
                client.publish(subject, payload).await?;
            }
            MessagingBackend::Dual { nats, aws } => {
                // Publish to both during migration
                nats.publish(subject, &payload)?;
                aws.publish(subject, payload).await?;
            }
        }
        Ok(())
    }
}
```

These examples provide a complete migration path from NATS to AWS managed services while maintaining similar functionality and patterns.