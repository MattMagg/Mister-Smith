# NATS to AWS Protocol Translation Layer - Implementation Tracker

## Overview
This document tracks the implementation of a comprehensive NATS to AWS messaging protocol translation layer for MisterSmith's AWS migration. The translation layer enables seamless communication between NATS-based components and AWS native services.

## Subject Mapping Architecture

### 1. Core Subject Translation Rules

#### Agent Heartbeat Mapping
```rust
// NATS: agent.*.heartbeat -> EventBridge: /mistersmith/agent/{agent_id}/heartbeat
pub fn translate_agent_heartbeat(subject: &str) -> Result<EventBridgePattern, TranslationError> {
    let parts: Vec<&str> = subject.split('.').collect();
    if parts.len() >= 3 && parts[0] == "agent" && parts[2] == "heartbeat" {
        let agent_id = parts[1];
        Ok(EventBridgePattern {
            source: vec!["mistersmith.agent".to_string()],
            detail_type: vec!["Agent Heartbeat".to_string()],
            detail: json!({
                "agentId": agent_id,
                "type": "heartbeat"
            })
        })
    } else {
        Err(TranslationError::InvalidSubjectFormat)
    }
}
```

#### Work Queue Mapping
```rust
// NATS: work.nlp.high.* -> SQS: mistersmith-work-nlp-high
pub fn translate_work_queue(subject: &str) -> Result<SqsQueueConfig, TranslationError> {
    let parts: Vec<&str> = subject.split('.').collect();
    if parts.len() >= 3 && parts[0] == "work" {
        let queue_name = format!("mistersmith-work-{}-{}", parts[1], parts[2]);
        let priority = match parts[2] {
            "high" => MessagePriority::High,
            "medium" => MessagePriority::Medium,
            "low" => MessagePriority::Low,
            _ => MessagePriority::Medium,
        };
        
        Ok(SqsQueueConfig {
            queue_name,
            priority,
            visibility_timeout: 300,
            message_group_id: Some(parts[1].to_string()),
            fifo: true,
        })
    } else {
        Err(TranslationError::InvalidSubjectFormat)
    }
}
```

#### Coordination Complete Mapping
```rust
// NATS: coord.*.complete -> DynamoDB: CoordinationState table
pub fn translate_coord_complete(subject: &str) -> Result<DynamoDbUpdate, TranslationError> {
    let parts: Vec<&str> = subject.split('.').collect();
    if parts.len() >= 3 && parts[0] == "coord" && parts[2] == "complete" {
        let coord_id = parts[1];
        Ok(DynamoDbUpdate {
            table_name: "CoordinationState".to_string(),
            key: hashmap! {
                "coordinationId" => AttributeValue::S(coord_id.to_string()),
            },
            update_expression: "SET #status = :status, #completed = :timestamp",
            expression_attribute_names: hashmap! {
                "#status" => "status",
                "#completed" => "completedAt",
            },
            expression_attribute_values: hashmap! {
                ":status" => AttributeValue::S("COMPLETE".to_string()),
                ":timestamp" => AttributeValue::N(Utc::now().timestamp().to_string()),
            },
        })
    } else {
        Err(TranslationError::InvalidSubjectFormat)
    }
}
```

### 2. Wildcard Translation Algorithms

```rust
use regex::Regex;

pub struct WildcardTranslator {
    patterns: Vec<(Regex, Box<dyn Fn(&str) -> Result<AwsTarget, TranslationError>>)>,
}

impl WildcardTranslator {
    pub fn new() -> Self {
        let mut translator = WildcardTranslator {
            patterns: vec![],
        };
        
        // Agent wildcard patterns
        translator.add_pattern(
            r"^agent\.([^.]+)\.(.+)$",
            Box::new(|subject| {
                let caps = Regex::new(r"^agent\.([^.]+)\.(.+)$")?.captures(subject).unwrap();
                let agent_id = &caps[1];
                let action = &caps[2];
                
                Ok(AwsTarget::EventBridge(EventBridgePattern {
                    source: vec!["mistersmith.agent".to_string()],
                    detail_type: vec![format!("Agent {}", action.to_case(Case::Title))],
                    detail: json!({
                        "agentId": agent_id,
                        "action": action
                    })
                }))
            })
        );
        
        // Work queue wildcard patterns
        translator.add_pattern(
            r"^work\.([^.]+)\.([^.]+)\.(.+)$",
            Box::new(|subject| {
                let caps = Regex::new(r"^work\.([^.]+)\.([^.]+)\.(.+)$")?.captures(subject).unwrap();
                let domain = &caps[1];
                let priority = &caps[2];
                let task_type = &caps[3];
                
                Ok(AwsTarget::Sqs(SqsMessage {
                    queue_url: format!("https://sqs.{}.amazonaws.com/{}/mistersmith-work-{}-{}",
                        std::env::var("AWS_REGION")?,
                        std::env::var("AWS_ACCOUNT_ID")?,
                        domain,
                        priority
                    ),
                    message_body: json!({
                        "domain": domain,
                        "priority": priority,
                        "taskType": task_type
                    }),
                    message_attributes: hashmap! {
                        "Domain" => MessageAttributeValue {
                            data_type: "String".to_string(),
                            string_value: Some(domain.to_string()),
                            ..Default::default()
                        }
                    },
                }))
            })
        );
        
        translator
    }
    
    pub fn translate(&self, subject: &str) -> Result<AwsTarget, TranslationError> {
        for (pattern, translator) in &self.patterns {
            if pattern.is_match(subject) {
                return translator(subject);
            }
        }
        Err(TranslationError::NoMatchingPattern)
    }
}
```

## Message Transformation

### 1. NATS MessageEnvelope to EventBridge Event

```rust
use aws_sdk_eventbridge::types::{PutEventsRequestEntry};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NatsMessageEnvelope {
    pub id: String,
    pub subject: String,
    pub timestamp: i64,
    pub sender_id: String,
    pub correlation_id: Option<String>,
    pub payload: serde_json::Value,
    pub headers: HashMap<String, String>,
}

impl NatsMessageEnvelope {
    pub fn to_eventbridge_event(&self) -> Result<PutEventsRequestEntry, TransformError> {
        let detail = json!({
            "messageId": self.id,
            "subject": self.subject,
            "senderId": self.sender_id,
            "correlationId": self.correlation_id,
            "payload": self.payload,
            "headers": self.headers,
            "originalTimestamp": self.timestamp,
        });
        
        let source = self.extract_source()?;
        let detail_type = self.extract_detail_type()?;
        
        Ok(PutEventsRequestEntry::builder()
            .time(DateTime::<Utc>::from_timestamp(self.timestamp, 0))
            .source(source)
            .detail_type(detail_type)
            .detail(serde_json::to_string(&detail)?)
            .event_bus_name("mistersmith-event-bus")
            .build())
    }
    
    fn extract_source(&self) -> Result<String, TransformError> {
        let parts: Vec<&str> = self.subject.split('.').collect();
        if parts.is_empty() {
            return Err(TransformError::InvalidSubject);
        }
        Ok(format!("mistersmith.{}", parts[0]))
    }
    
    fn extract_detail_type(&self) -> Result<String, TransformError> {
        let parts: Vec<&str> = self.subject.split('.').collect();
        if parts.len() < 2 {
            return Err(TransformError::InvalidSubject);
        }
        Ok(parts[1..].join(".").to_case(Case::Title))
    }
}
```

### 2. Request/Reply to Lambda Invocation

```rust
use aws_sdk_lambda::types::InvocationType;
use tokio::sync::oneshot;

pub struct RequestReplyTranslator {
    lambda_client: aws_sdk_lambda::Client,
    reply_channels: Arc<Mutex<HashMap<String, oneshot::Sender<Vec<u8>>>>>,
}

impl RequestReplyTranslator {
    pub async fn handle_request(
        &self,
        request: NatsRequest,
    ) -> Result<Vec<u8>, TranslationError> {
        // Map NATS subject to Lambda function
        let function_name = self.map_subject_to_function(&request.subject)?;
        
        // Create correlation ID for tracking
        let correlation_id = Uuid::new_v4().to_string();
        
        // Prepare Lambda payload
        let payload = json!({
            "correlationId": correlation_id,
            "subject": request.subject,
            "data": request.data,
            "headers": request.headers,
            "replySubject": request.reply,
        });
        
        // Invoke Lambda function
        let response = self.lambda_client
            .invoke()
            .function_name(function_name)
            .invocation_type(InvocationType::RequestResponse)
            .payload(serde_json::to_vec(&payload)?)
            .send()
            .await?;
        
        // Extract response
        let response_payload = response.payload()
            .ok_or(TranslationError::NoLambdaResponse)?;
        
        Ok(response_payload.to_vec())
    }
    
    fn map_subject_to_function(&self, subject: &str) -> Result<String, TranslationError> {
        let mappings = hashmap! {
            "api.user.get" => "mistersmith-api-user-get",
            "api.task.create" => "mistersmith-api-task-create",
            "api.agent.status" => "mistersmith-api-agent-status",
        };
        
        // Direct mapping
        if let Some(function) = mappings.get(subject) {
            return Ok(function.to_string());
        }
        
        // Pattern matching
        if subject.starts_with("api.") {
            let parts: Vec<&str> = subject.split('.').collect();
            if parts.len() >= 3 {
                return Ok(format!("mistersmith-api-{}-{}", parts[1], parts[2]));
            }
        }
        
        Err(TranslationError::NoFunctionMapping(subject.to_string()))
    }
}
```

### 3. Queue Groups to SQS Consumer Groups

```rust
pub struct QueueGroupTranslator {
    sqs_client: aws_sdk_sqs::Client,
    consumer_registry: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl QueueGroupTranslator {
    pub async fn subscribe_queue_group(
        &self,
        subject: &str,
        queue_group: &str,
        handler: Box<dyn MessageHandler>,
    ) -> Result<(), TranslationError> {
        // Map to SQS queue
        let queue_url = self.get_or_create_queue(subject).await?;
        
        // Register consumer in group
        let consumer_id = format!("{}-{}-{}", queue_group, subject, Uuid::new_v4());
        self.register_consumer(&queue_url, &consumer_id).await?;
        
        // Start polling with visibility timeout for queue group behavior
        let visibility_timeout = self.calculate_visibility_timeout(queue_group);
        
        tokio::spawn(async move {
            loop {
                let messages = self.sqs_client
                    .receive_message()
                    .queue_url(&queue_url)
                    .max_number_of_messages(10)
                    .visibility_timeout(visibility_timeout)
                    .wait_time_seconds(20)
                    .message_attribute_names("All")
                    .send()
                    .await?;
                
                for message in messages.messages.unwrap_or_default() {
                    // Process message
                    if let Err(e) = handler.handle(&message).await {
                        error!("Failed to handle message: {}", e);
                        continue;
                    }
                    
                    // Delete message on success
                    self.sqs_client
                        .delete_message()
                        .queue_url(&queue_url)
                        .receipt_handle(message.receipt_handle.unwrap())
                        .send()
                        .await?;
                }
            }
        });
        
        Ok(())
    }
    
    fn calculate_visibility_timeout(&self, queue_group: &str) -> i32 {
        // Different timeouts for different queue groups
        match queue_group {
            "fast-processors" => 30,
            "batch-processors" => 300,
            "heavy-workers" => 900,
            _ => 60,
        }
    }
}
```

### 4. JetStream KV to DynamoDB Operations

```rust
pub struct JetStreamKvTranslator {
    dynamodb_client: aws_sdk_dynamodb::Client,
    table_prefix: String,
}

impl JetStreamKvTranslator {
    pub async fn put(&self, bucket: &str, key: &str, value: Vec<u8>) -> Result<u64, TranslationError> {
        let table_name = format!("{}-{}", self.table_prefix, bucket);
        
        // Store with revision tracking
        let revision = self.get_next_revision(&table_name, key).await?;
        
        let item = hashmap! {
            "pk" => AttributeValue::S(key.to_string()),
            "sk" => AttributeValue::S(format!("rev#{:020}", revision)),
            "value" => AttributeValue::B(Blob::new(value)),
            "timestamp" => AttributeValue::N(Utc::now().timestamp().to_string()),
            "ttl" => AttributeValue::N((Utc::now().timestamp() + 86400 * 30).to_string()),
        };
        
        self.dynamodb_client
            .put_item()
            .table_name(&table_name)
            .set_item(Some(item))
            .send()
            .await?;
        
        // Update latest pointer
        self.update_latest_pointer(&table_name, key, revision).await?;
        
        Ok(revision)
    }
    
    pub async fn get(&self, bucket: &str, key: &str) -> Result<Option<KvEntry>, TranslationError> {
        let table_name = format!("{}-{}", self.table_prefix, bucket);
        
        // Get latest revision
        let latest = self.dynamodb_client
            .get_item()
            .table_name(&table_name)
            .key("pk", AttributeValue::S(key.to_string()))
            .key("sk", AttributeValue::S("latest".to_string()))
            .send()
            .await?;
        
        if let Some(item) = latest.item {
            if let Some(AttributeValue::N(rev)) = item.get("revision") {
                let revision: u64 = rev.parse()?;
                
                // Get actual value
                let result = self.dynamodb_client
                    .get_item()
                    .table_name(&table_name)
                    .key("pk", AttributeValue::S(key.to_string()))
                    .key("sk", AttributeValue::S(format!("rev#{:020}", revision)))
                    .send()
                    .await?;
                
                if let Some(value_item) = result.item {
                    return Ok(Some(KvEntry {
                        bucket: bucket.to_string(),
                        key: key.to_string(),
                        value: value_item.get("value")
                            .and_then(|v| v.as_b().ok())
                            .map(|b| b.as_ref().to_vec())
                            .ok_or(TranslationError::InvalidDynamoDbItem)?,
                        revision,
                        created: value_item.get("timestamp")
                            .and_then(|v| v.as_n().ok())
                            .and_then(|n| n.parse::<i64>().ok())
                            .ok_or(TranslationError::InvalidDynamoDbItem)?,
                    }));
                }
            }
        }
        
        Ok(None)
    }
    
    pub async fn watch(&self, bucket: &str, key: &str) -> Result<KvWatcher, TranslationError> {
        // Use DynamoDB Streams for watching
        let stream_arn = self.get_stream_arn(&format!("{}-{}", self.table_prefix, bucket)).await?;
        
        Ok(KvWatcher {
            stream_arn,
            key: key.to_string(),
            dynamodb_streams_client: self.create_streams_client()?,
        })
    }
}
```

## Connection Management

### 1. NATS Connection Pool to AWS SDK Clients

```rust
use bb8::{Pool, PooledConnection};
use bb8_lapin::LapinConnectionManager;

pub struct ConnectionManager {
    nats_pool: Pool<NatsConnectionManager>,
    aws_config: aws_config::Config,
    eventbridge_client: aws_sdk_eventbridge::Client,
    sqs_client: aws_sdk_sqs::Client,
    lambda_client: aws_sdk_lambda::Client,
    dynamodb_client: aws_sdk_dynamodb::Client,
}

impl ConnectionManager {
    pub async fn new(config: Config) -> Result<Self, Error> {
        // Initialize NATS connection pool
        let nats_manager = NatsConnectionManager::new(config.nats_urls.clone());
        let nats_pool = Pool::builder()
            .max_size(config.nats_pool_size)
            .min_idle(Some(config.nats_min_idle))
            .connection_timeout(Duration::from_secs(30))
            .idle_timeout(Some(Duration::from_secs(600)))
            .build(nats_manager)
            .await?;
        
        // Initialize AWS SDK with retry configuration
        let retry_config = RetryConfig::standard()
            .with_max_attempts(3)
            .with_initial_backoff(Duration::from_millis(100));
        
        let aws_config = aws_config::from_env()
            .region(Region::new(config.aws_region.clone()))
            .retry_config(retry_config)
            .load()
            .await;
        
        Ok(Self {
            nats_pool,
            aws_config: aws_config.clone(),
            eventbridge_client: aws_sdk_eventbridge::Client::new(&aws_config),
            sqs_client: aws_sdk_sqs::Client::new(&aws_config),
            lambda_client: aws_sdk_lambda::Client::new(&aws_config),
            dynamodb_client: aws_sdk_dynamodb::Client::new(&aws_config),
        })
    }
    
    pub async fn get_nats_connection(&self) -> Result<PooledConnection<'_, NatsConnectionManager>, Error> {
        self.nats_pool.get().await.map_err(Into::into)
    }
    
    pub fn eventbridge(&self) -> &aws_sdk_eventbridge::Client {
        &self.eventbridge_client
    }
    
    pub fn sqs(&self) -> &aws_sdk_sqs::Client {
        &self.sqs_client
    }
    
    pub fn lambda(&self) -> &aws_sdk_lambda::Client {
        &self.lambda_client
    }
    
    pub fn dynamodb(&self) -> &aws_sdk_dynamodb::Client {
        &self.dynamodb_client
    }
}
```

### 2. Subscription Handling to EventBridge Rules

```rust
pub struct SubscriptionManager {
    eventbridge_client: aws_sdk_eventbridge::Client,
    rule_prefix: String,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
}

impl SubscriptionManager {
    pub async fn create_subscription(
        &self,
        subject: &str,
        target_arn: &str,
    ) -> Result<String, Error> {
        // Generate rule name
        let rule_name = format!("{}-{}", self.rule_prefix, subject.replace('.', "-"));
        
        // Create EventBridge rule pattern
        let event_pattern = self.subject_to_event_pattern(subject)?;
        
        // Create or update rule
        self.eventbridge_client
            .put_rule()
            .name(&rule_name)
            .event_pattern(serde_json::to_string(&event_pattern)?)
            .state(RuleState::Enabled)
            .event_bus_name("mistersmith-event-bus")
            .send()
            .await?;
        
        // Add target
        let target = Target::builder()
            .id(format!("target-{}", Uuid::new_v4()))
            .arn(target_arn)
            .retry_policy(
                RetryPolicy::builder()
                    .maximum_retry_attempts(2)
                    .maximum_event_age(3600)
                    .build()
            )
            .build();
        
        self.eventbridge_client
            .put_targets()
            .rule(&rule_name)
            .targets(target)
            .event_bus_name("mistersmith-event-bus")
            .send()
            .await?;
        
        // Store subscription info
        let mut subs = self.subscriptions.write().await;
        subs.insert(rule_name.clone(), SubscriptionInfo {
            subject: subject.to_string(),
            target_arn: target_arn.to_string(),
            created_at: Utc::now(),
        });
        
        Ok(rule_name)
    }
    
    fn subject_to_event_pattern(&self, subject: &str) -> Result<serde_json::Value, Error> {
        let parts: Vec<&str> = subject.split('.').collect();
        
        // Handle wildcards
        let source_pattern = if parts[0] == "*" {
            vec![]
        } else {
            vec![format!("mistersmith.{}", parts[0])]
        };
        
        let detail_type_pattern = if parts.len() > 1 && parts[1] != "*" {
            vec![parts[1..].join(".").to_case(Case::Title)]
        } else {
            vec![]
        };
        
        let mut pattern = json!({});
        
        if !source_pattern.is_empty() {
            pattern["source"] = json!(source_pattern);
        }
        
        if !detail_type_pattern.is_empty() {
            pattern["detail-type"] = json!(detail_type_pattern);
        }
        
        // Add subject filter in detail
        pattern["detail"] = json!({
            "subject": [subject]
        });
        
        Ok(pattern)
    }
}
```

### 3. Error Handling and Dead Letter Queues

```rust
pub struct ErrorHandler {
    sqs_client: aws_sdk_sqs::Client,
    dlq_prefix: String,
    metrics: Arc<Metrics>,
}

impl ErrorHandler {
    pub async fn handle_translation_error(
        &self,
        error: TranslationError,
        original_message: &[u8],
        context: ErrorContext,
    ) -> Result<(), Error> {
        // Increment error metrics
        self.metrics.increment_counter("translation_errors", &[
            ("error_type", error.error_type()),
            ("source", &context.source),
        ]);
        
        // Determine DLQ based on error type
        let dlq_url = self.get_dlq_url(&error, &context).await?;
        
        // Send to DLQ with metadata
        let message_body = json!({
            "error": {
                "type": error.error_type(),
                "message": error.to_string(),
                "timestamp": Utc::now().to_rfc3339(),
            },
            "context": context,
            "original_message": base64::encode(original_message),
        });
        
        self.sqs_client
            .send_message()
            .queue_url(dlq_url)
            .message_body(serde_json::to_string(&message_body)?)
            .message_attributes(
                "ErrorType",
                MessageAttributeValue::builder()
                    .string_value(error.error_type())
                    .data_type("String")
                    .build()
            )
            .send()
            .await?;
        
        // Log error
        error!(
            "Translation error: {} - Context: {:?}",
            error, context
        );
        
        Ok(())
    }
    
    async fn get_dlq_url(&self, error: &TranslationError, context: &ErrorContext) -> Result<String, Error> {
        let queue_name = match error {
            TranslationError::InvalidSubjectFormat => format!("{}-invalid-subject", self.dlq_prefix),
            TranslationError::NoMatchingPattern => format!("{}-no-pattern", self.dlq_prefix),
            TranslationError::AwsError(_) => format!("{}-aws-error", self.dlq_prefix),
            _ => format!("{}-general", self.dlq_prefix),
        };
        
        let result = self.sqs_client
            .get_queue_url()
            .queue_name(queue_name)
            .send()
            .await?;
        
        Ok(result.queue_url.unwrap())
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorContext {
    pub source: String,
    pub operation: String,
    pub correlation_id: Option<String>,
    pub retry_count: u32,
}
```

### 4. Reconnection Logic and AWS Retry Policies

```rust
pub struct ReconnectionManager {
    connection_manager: Arc<ConnectionManager>,
    health_checker: Arc<HealthChecker>,
    retry_policy: ExponentialBackoff,
}

impl ReconnectionManager {
    pub async fn maintain_connections(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            // Check NATS connections
            if let Err(e) = self.check_nats_health().await {
                warn!("NATS health check failed: {}", e);
                self.reconnect_nats().await;
            }
            
            // AWS SDK handles reconnection automatically, but we can check service health
            if let Err(e) = self.check_aws_health().await {
                warn!("AWS health check failed: {}", e);
                // AWS SDK will handle retries automatically
            }
        }
    }
    
    async fn reconnect_nats(&self) {
        let mut backoff = self.retry_policy.clone();
        
        loop {
            match self.connection_manager.get_nats_connection().await {
                Ok(conn) => {
                    info!("NATS connection restored");
                    break;
                }
                Err(e) => {
                    let delay = backoff.next_backoff().unwrap_or(Duration::from_secs(60));
                    error!("Failed to reconnect to NATS: {}. Retrying in {:?}", e, delay);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    async fn check_aws_health(&self) -> Result<(), Error> {
        // Check EventBridge
        self.connection_manager
            .eventbridge()
            .list_rules()
            .limit(1)
            .send()
            .await?;
        
        // Check SQS
        self.connection_manager
            .sqs()
            .list_queues()
            .max_results(1)
            .send()
            .await?;
        
        // Check Lambda
        self.connection_manager
            .lambda()
            .list_functions()
            .max_items(1)
            .send()
            .await?;
        
        // Check DynamoDB
        self.connection_manager
            .dynamodb()
            .list_tables()
            .limit(1)
            .send()
            .await?;
        
        Ok(())
    }
}
```

## Performance Considerations

### 1. Batch Message Processing

```rust
pub struct BatchProcessor {
    eventbridge_client: aws_sdk_eventbridge::Client,
    sqs_client: aws_sdk_sqs::Client,
    batch_size: usize,
    batch_timeout: Duration,
}

impl BatchProcessor {
    pub async fn process_eventbridge_batch(&self, messages: Vec<NatsMessageEnvelope>) -> Result<BatchResult, Error> {
        let mut entries = Vec::with_capacity(messages.len());
        let mut failed_indices = Vec::new();
        
        // Transform messages to EventBridge entries
        for (idx, msg) in messages.iter().enumerate() {
            match msg.to_eventbridge_event() {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    error!("Failed to transform message {}: {}", idx, e);
                    failed_indices.push(idx);
                }
            }
        }
        
        // Batch send to EventBridge (max 10 per request)
        let mut success_count = 0;
        for chunk in entries.chunks(10) {
            let result = self.eventbridge_client
                .put_events()
                .entries(chunk.to_vec())
                .send()
                .await?;
            
            success_count += result.entries.unwrap_or_default()
                .iter()
                .filter(|e| e.error_code.is_none())
                .count();
        }
        
        Ok(BatchResult {
            total: messages.len(),
            success: success_count,
            failed: failed_indices,
        })
    }
    
    pub async fn process_sqs_batch(&self, queue_url: &str, messages: Vec<SqsMessage>) -> Result<BatchResult, Error> {
        let mut success_count = 0;
        let mut failed_indices = Vec::new();
        
        // SQS batch send (max 10 per request)
        for (chunk_idx, chunk) in messages.chunks(10).enumerate() {
            let entries: Vec<_> = chunk.iter().enumerate()
                .map(|(idx, msg)| {
                    SendMessageBatchRequestEntry::builder()
                        .id(format!("{}", chunk_idx * 10 + idx))
                        .message_body(&msg.body)
                        .set_message_attributes(Some(msg.attributes.clone()))
                        .build()
                })
                .collect();
            
            let result = self.sqs_client
                .send_message_batch()
                .queue_url(queue_url)
                .entries(entries)
                .send()
                .await?;
            
            // Track failures
            if let Some(failed) = result.failed {
                for failure in failed {
                    if let Ok(id) = failure.id.unwrap_or_default().parse::<usize>() {
                        failed_indices.push(chunk_idx * 10 + id);
                    }
                }
            }
            
            success_count += result.successful.unwrap_or_default().len();
        }
        
        Ok(BatchResult {
            total: messages.len(),
            success: success_count,
            failed: failed_indices,
        })
    }
}
```

### 2. Connection Pooling Strategies

```rust
pub struct PoolingStrategy {
    nats_pools: HashMap<String, Pool<NatsConnectionManager>>,
    aws_client_cache: Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
}

impl PoolingStrategy {
    pub async fn get_nats_pool(&self, cluster: &str) -> Result<&Pool<NatsConnectionManager>, Error> {
        self.nats_pools.get(cluster)
            .ok_or_else(|| Error::NoPoolForCluster(cluster.to_string()))
    }
    
    pub async fn get_or_create_aws_client<T, F>(&self, key: &str, factory: F) -> Result<Arc<T>, Error>
    where
        T: Clone + Send + Sync + 'static,
        F: FnOnce() -> T,
    {
        let cache = self.aws_client_cache.read().await;
        if let Some(client) = cache.get(key) {
            if let Some(typed_client) = client.downcast_ref::<Arc<T>>() {
                return Ok(typed_client.clone());
            }
        }
        drop(cache);
        
        // Create new client
        let new_client = Arc::new(factory());
        let mut cache = self.aws_client_cache.write().await;
        cache.insert(key.to_string(), Box::new(new_client.clone()));
        
        Ok(new_client)
    }
}
```

### 3. Message Ordering Preservation

```rust
pub struct OrderingPreserver {
    sequence_tracker: Arc<RwLock<HashMap<String, SequenceInfo>>>,
    pending_messages: Arc<RwLock<HashMap<String, BTreeMap<u64, PendingMessage>>>>,
}

impl OrderingPreserver {
    pub async fn process_ordered_message(
        &self,
        partition_key: &str,
        sequence: u64,
        message: NatsMessageEnvelope,
        processor: impl MessageProcessor,
    ) -> Result<(), Error> {
        let mut tracker = self.sequence_tracker.write().await;
        let seq_info = tracker.entry(partition_key.to_string())
            .or_insert(SequenceInfo {
                last_processed: 0,
                highest_seen: 0,
            });
        
        seq_info.highest_seen = seq_info.highest_seen.max(sequence);
        
        if sequence == seq_info.last_processed + 1 {
            // Process immediately
            processor.process(message).await?;
            seq_info.last_processed = sequence;
            
            // Process any pending messages
            let mut pending = self.pending_messages.write().await;
            if let Some(partition_pending) = pending.get_mut(partition_key) {
                while let Some((next_seq, _)) = partition_pending.iter().next() {
                    if *next_seq == seq_info.last_processed + 1 {
                        let (_, msg) = partition_pending.pop_first().unwrap();
                        processor.process(msg.message).await?;
                        seq_info.last_processed = *next_seq;
                    } else {
                        break;
                    }
                }
            }
        } else if sequence > seq_info.last_processed + 1 {
            // Store for later processing
            let mut pending = self.pending_messages.write().await;
            pending.entry(partition_key.to_string())
                .or_insert_with(BTreeMap::new)
                .insert(sequence, PendingMessage {
                    message,
                    received_at: Utc::now(),
                });
        }
        
        Ok(())
    }
}
```

### 4. Latency Compensation Techniques

```rust
pub struct LatencyCompensator {
    metrics: Arc<Metrics>,
    predictive_cache: Arc<PredictiveCache>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl LatencyCompensator {
    pub async fn compensate_request<T, F, Fut>(
        &self,
        operation: &str,
        request: F,
    ) -> Result<T, Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
        T: Clone + Send + Sync + 'static,
    {
        // Check predictive cache
        if let Some(cached) = self.predictive_cache.get::<T>(operation).await {
            self.metrics.increment_counter("cache_hits", &[("operation", operation)]);
            return Ok(cached);
        }
        
        // Check circuit breaker
        if !self.circuit_breaker.is_available(operation).await {
            return Err(Error::ServiceUnavailable(operation.to_string()));
        }
        
        // Execute with timing
        let start = Instant::now();
        let result = match tokio::time::timeout(Duration::from_secs(30), request()).await {
            Ok(Ok(value)) => {
                let latency = start.elapsed();
                self.metrics.record_histogram("operation_latency", latency.as_millis() as f64, &[
                    ("operation", operation),
                    ("status", "success"),
                ]);
                
                // Update circuit breaker
                self.circuit_breaker.record_success(operation).await;
                
                // Cache if appropriate
                if latency > Duration::from_millis(100) {
                    self.predictive_cache.put(operation, value.clone()).await;
                }
                
                Ok(value)
            }
            Ok(Err(e)) => {
                self.metrics.increment_counter("operation_errors", &[
                    ("operation", operation),
                    ("error_type", "application"),
                ]);
                self.circuit_breaker.record_failure(operation).await;
                Err(e)
            }
            Err(_) => {
                self.metrics.increment_counter("operation_errors", &[
                    ("operation", operation),
                    ("error_type", "timeout"),
                ]);
                self.circuit_breaker.record_failure(operation).await;
                Err(Error::Timeout(operation.to_string()))
            }
        };
        
        result
    }
}

pub struct PredictiveCache {
    cache: Arc<DashMap<String, (Box<dyn Any + Send + Sync>, Instant)>>,
    ttl: Duration,
}

impl PredictiveCache {
    pub async fn get<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        if let Some((value, timestamp)) = self.cache.get(key) {
            if timestamp.elapsed() < self.ttl {
                return value.downcast_ref::<T>().cloned();
            }
        }
        None
    }
    
    pub async fn put<T: Clone + Send + Sync + 'static>(&self, key: &str, value: T) {
        self.cache.insert(key.to_string(), (Box::new(value), Instant::now()));
    }
}
```

## Implementation Status

### Phase 1: Core Translation Layer ✅
- [x] Subject mapping rules
- [x] Wildcard pattern matching
- [x] Message envelope transformation
- [x] Basic error handling

### Phase 2: Advanced Features 🚧
- [x] Request/Reply translation
- [x] Queue group mapping
- [x] JetStream KV operations
- [ ] Stream processing translation

### Phase 3: Performance Optimization 🚧
- [x] Connection pooling
- [x] Batch processing
- [x] Message ordering
- [ ] Predictive caching

### Phase 4: Production Readiness 📋
- [ ] Comprehensive testing
- [ ] Performance benchmarking
- [ ] Documentation
- [ ] Deployment automation

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_heartbeat_translation() {
        let subject = "agent.agent-123.heartbeat";
        let result = translate_agent_heartbeat(subject).unwrap();
        
        assert_eq!(result.source, vec!["mistersmith.agent"]);
        assert_eq!(result.detail_type, vec!["Agent Heartbeat"]);
        assert_eq!(result.detail["agentId"], "agent-123");
    }
    
    #[tokio::test]
    async fn test_batch_eventbridge_processing() {
        let processor = BatchProcessor::new(/* clients */);
        let messages = vec![
            // Create test messages
        ];
        
        let result = processor.process_eventbridge_batch(messages).await.unwrap();
        assert_eq!(result.success, result.total);
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_end_to_end_translation() {
    // Setup test environment
    let translator = setup_test_translator().await;
    
    // Send NATS message
    let nats_msg = create_test_message();
    translator.process(nats_msg).await.unwrap();
    
    // Verify AWS side received message
    let aws_msg = poll_aws_service().await;
    assert_eq!(aws_msg.content, expected_content);
}
```

## Monitoring and Observability

### Metrics Collection
```rust
pub struct TranslationMetrics {
    messages_translated: Counter,
    translation_errors: Counter,
    translation_latency: Histogram,
    active_connections: Gauge,
}

impl TranslationMetrics {
    pub fn record_translation(&self, subject: &str, target: &str, duration: Duration) {
        self.messages_translated.increment(1);
        self.translation_latency.record(duration.as_millis() as f64);
        
        // Add labels
        metrics::counter!("protocol_translation_total", 1,
            "source_protocol" => "nats",
            "target_protocol" => "aws",
            "subject" => subject.to_string(),
            "target_service" => target.to_string(),
        );
    }
}
```

## Next Steps

1. **Complete Implementation**
   - Implement remaining translation patterns
   - Add comprehensive error recovery
   - Optimize performance bottlenecks

2. **Testing**
   - Write comprehensive unit tests
   - Perform load testing
   - Validate ordering guarantees

3. **Documentation**
   - API documentation
   - Deployment guide
   - Performance tuning guide

4. **Production Deployment**
   - Container packaging
   - Kubernetes deployment manifests
   - Monitoring dashboard setup