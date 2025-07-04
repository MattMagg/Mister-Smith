---
title: agent-communication
type: note
permalink: revision-swarm/data-management/agent-communication
tags:
- '#revised-document #agent-communication #message-passing #task-distribution #coordination'
---

# Agent Communication Architecture
## Message Passing, Task Distribution & Coordination

> **Canonical Reference**: See `tech-framework.md` for authoritative technology stack specifications

## Executive Summary

This document defines the communication protocols, message passing patterns, task distribution mechanisms, and coordination strategies for agent interactions within the Mister Smith framework. It covers direct RPC, publish/subscribe, blackboard patterns, comprehensive message schemas, and coordination mechanisms for distributed agent systems.

## 3. Message Passing and Communication Patterns

### 3.1 Direct RPC Pattern

Point-to-point communication for immediate responses:

```rust
struct DirectChannel {
    target_endpoint: String,
    timeout: Duration,
}

impl DirectChannel {
    async fn call(&self, request: Request) -> Result<Response, Error> {
        // Direct HTTP/gRPC call
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()?;
        
        let response = client
            .post(&self.target_endpoint)
            .json(&request)
            .send()
            .await?;
            
        Ok(response.json().await?)
    }
}
```

### 3.2 Publish/Subscribe Pattern

> **Supervision Integration**: See `agent-lifecycle.md` section 2.2 for event-driven message bus patterns and supervisor coordination

Topic-based message distribution:

```rust
struct PubSubBus {
    broker_url: String,
    subscriptions: HashMap<Topic, Vec<CallbackFn>>,
}

impl PubSubBus {
    async fn publish(&self, topic: Topic, message: Message) -> Result<(), Error> {
        // Publish to broker (e.g., NATS)
        let nc = nats::connect(&self.broker_url)?;
        nc.publish(&topic.as_str(), &message.serialize()?)?;
        Ok(())
    }
    
    async fn subscribe<F>(&mut self, topic: Topic, callback: F) 
    where F: Fn(Message) + Send + 'static {
        let nc = nats::connect(&self.broker_url)?;
        let sub = nc.subscribe(&topic.as_str())?;
        
        tokio::spawn(async move {
            for msg in sub.messages() {
                callback(Message::deserialize(&msg.data).unwrap());
            }
        });
    }
}
```

### 3.3 Blackboard Pattern

Shared memory coordination:

```rust
struct Blackboard {
    store: Arc<RwLock<HashMap<String, BlackboardEntry>>>,
    watchers: Arc<RwLock<HashMap<Pattern, Vec<WatcherFn>>>>,
}

struct BlackboardEntry {
    value: Value,
    timestamp: Instant,
    author: AgentId,
    version: u64,
}

impl Blackboard {
    async fn write(&self, key: String, value: Value, agent_id: AgentId) -> Result<(), Error> {
        let mut store = self.store.write().await;
        let version = store.get(&key).map(|e| e.version + 1).unwrap_or(1);
        
        store.insert(key.clone(), BlackboardEntry {
            value: value.clone(),
            timestamp: Instant::now(),
            author: agent_id,
            version,
        });
        
        // Notify watchers
        self.notify_watchers(&key, &value).await;
        Ok(())
    }
    
    async fn read(&self, key: &str) -> Option<BlackboardEntry> {
        self.store.read().await.get(key).cloned()
    }
}
```

### 3.4 Complete Message Schema Definitions

#### 3.4.1 Base Message Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mister-smith.ai/schemas/base-message",
  "title": "Base Agent Message",
  "description": "Core message structure for all agent communications",
  "type": "object",
  "required": ["id", "type", "sender", "timestamp", "routing"],
  "properties": {
    "id": {
      "type": "string",
      "format": "uuid",
      "description": "Unique message identifier"
    },
    "type": {
      "type": "string",
      "description": "Message type discriminator"
    },
    "payload": {
      "type": "object",
      "description": "Message-specific data",
      "additionalProperties": true
    },
    "sender": {
      "$ref": "#/$defs/AgentId",
      "description": "Agent ID of message sender"
    },
    "timestamp": {
      "type": "string",
      "format": "date-time",
      "description": "ISO 8601 timestamp of message creation"
    },
    "routing": {
      "$ref": "#/$defs/RoutingInfo",
      "description": "Message routing configuration"
    },
    "correlation_id": {
      "type": "string",
      "format": "uuid",
      "description": "Optional correlation ID for request/response tracking"
    },
    "ttl": {
      "type": "integer",
      "minimum": 0,
      "description": "Time-to-live in seconds (0 = no expiration)",
      "default": 300
    },
    "priority": {
      "type": "integer",
      "minimum": 0,
      "maximum": 9,
      "description": "Message priority (0=lowest, 9=highest)",
      "default": 5
    }
  },
  "$defs": {
    "AgentId": {
      "type": "string",
      "pattern": "^agent-[a-zA-Z0-9]{8}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{12}$",
      "description": "Unique agent identifier"
    },
    "RoutingInfo": {
      "oneOf": [
        {
          "type": "object",
          "properties": {
            "type": { "const": "broadcast" },
            "exclude": {
              "type": "array",
              "items": { "$ref": "#/$defs/AgentId" },
              "description": "Agents to exclude from broadcast"
            }
          },
          "required": ["type"]
        },
        {
          "type": "object",
          "properties": {
            "type": { "const": "target" },
            "target": { "$ref": "#/$defs/AgentId" }
          },
          "required": ["type", "target"]
        },
        {
          "type": "object",
          "properties": {
            "type": { "const": "round_robin" },
            "pool": {
              "type": "array",
              "items": { "$ref": "#/$defs/AgentId" },
              "minItems": 1
            }
          },
          "required": ["type", "pool"]
        }
      ]
    }
  }
}
```

#### 3.4.2 System Messages Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mister-smith.ai/schemas/system-message",
  "title": "System Control Message",
  "allOf": [
    { "$ref": "https://mister-smith.ai/schemas/base-message" }
  ],
  "properties": {
    "type": {
      "enum": ["START", "STOP", "PAUSE", "RESUME", "HEALTH_CHECK", "RESTART", "SHUTDOWN"]
    },
    "payload": {
      "type": "object",
      "properties": {
        "reason": {
          "type": "string",
          "description": "Human-readable reason for the system command"
        },
        "force": {
          "type": "boolean",
          "description": "Whether to force the action (bypass graceful shutdown)",
          "default": false
        },
        "timeout": {
          "type": "integer",
          "minimum": 0,
          "description": "Timeout in seconds for the operation"
        }
      }
    }
  }
}
```

#### 3.4.3 Task Messages Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mister-smith.ai/schemas/task-message",
  "title": "Task Assignment and Execution Message",
  "allOf": [
    { "$ref": "https://mister-smith.ai/schemas/base-message" }
  ],
  "properties": {
    "type": {
      "enum": ["TASK_ASSIGN", "TASK_PROGRESS", "TASK_COMPLETE", "TASK_FAILED", "TASK_CANCEL"]
    },
    "payload": {
      "type": "object",
      "properties": {
        "task_id": {
          "type": "string",
          "format": "uuid",
          "description": "Unique task identifier"
        },
        "task_type": {
          "enum": ["RESEARCH", "CODE", "ANALYZE", "REVIEW", "DEPLOY", "MONITOR", "CUSTOM"]
        },
        "description": {
          "type": "string",
          "maxLength": 1000,
          "description": "Human-readable task description"
        },
        "requirements": {
          "type": "object",
          "properties": {
            "capabilities": {
              "type": "array",
              "items": { "type": "string" },
              "description": "Required agent capabilities"
            },
            "resources": {
              "type": "object",
              "properties": {
                "memory_mb": { "type": "integer", "minimum": 0 },
                "cpu_cores": { "type": "number", "minimum": 0 },
                "storage_mb": { "type": "integer", "minimum": 0 }
              }
            },
            "deadline": {
              "type": "string",
              "format": "date-time",
              "description": "Task completion deadline"
            }
          }
        },
        "dependencies": {
          "type": "array",
          "items": {
            "type": "string",
            "format": "uuid"
          },
          "description": "Task IDs that must complete before this task"
        },
        "progress": {
          "type": "number",
          "minimum": 0,
          "maximum": 100,
          "description": "Task completion percentage"
        },
        "result": {
          "type": "object",
          "description": "Task execution result (for completion messages)",
          "additionalProperties": true
        },
        "error": {
          "type": "object",
          "properties": {
            "code": { "type": "string" },
            "message": { "type": "string" },
            "details": { "type": "object" },
            "recoverable": { "type": "boolean" }
          },
          "required": ["code", "message"]
        }
      },
      "required": ["task_id", "task_type"]
    }
  }
}
```

#### 3.4.4 Agent Communication Messages Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mister-smith.ai/schemas/agent-communication",
  "title": "Inter-Agent Communication Message",
  "allOf": [
    { "$ref": "https://mister-smith.ai/schemas/base-message" }
  ],
  "properties": {
    "type": {
      "enum": ["REQUEST", "RESPONSE", "NOTIFICATION", "COLLABORATION"]
    },
    "payload": {
      "type": "object",
      "properties": {
        "method": {
          "type": "string",
          "description": "Method or action being requested"
        },
        "parameters": {
          "type": "object",
          "description": "Method parameters",
          "additionalProperties": true
        },
        "response_data": {
          "type": "object",
          "description": "Response payload (for RESPONSE type)",
          "additionalProperties": true
        },
        "status_code": {
          "type": "integer",
          "description": "HTTP-style status code for responses"
        },
        "notification_type": {
          "enum": ["INFO", "WARNING", "ERROR", "SUCCESS"],
          "description": "Type of notification"
        },
        "collaboration_mode": {
          "enum": ["PEER_REVIEW", "PAIR_WORK", "KNOWLEDGE_SHARE", "DELEGATION"]
        }
      }
    }
  }
}
```

#### 3.4.5 Supervision Messages Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mister-smith.ai/schemas/supervision-message",
  "title": "Agent Supervision and Lifecycle Message",
  "allOf": [
    { "$ref": "https://mister-smith.ai/schemas/base-message" }
  ],
  "properties": {
    "type": {
      "enum": ["SPAWN", "TERMINATE", "RESTART", "HEALTH_CHECK", "RESOURCE_UPDATE", "CAPABILITY_CHANGE"]
    },
    "payload": {
      "type": "object",
      "properties": {
        "agent_spec": {
          "type": "object",
          "properties": {
            "agent_type": {
              "enum": ["SUPERVISOR", "WORKER", "COORDINATOR", "MONITOR", "PLANNER", "EXECUTOR", "CRITIC", "ROUTER", "MEMORY"]
            },
            "capabilities": {
              "type": "array",
              "items": { "type": "string" }
            },
            "configuration": {
              "type": "object",
              "additionalProperties": true
            }
          },
          "required": ["agent_type"]
        },
        "health_status": {
          "enum": ["HEALTHY", "DEGRADED", "UNHEALTHY", "UNKNOWN"]
        },
        "metrics": {
          "type": "object",
          "properties": {
            "cpu_usage": { "type": "number", "minimum": 0, "maximum": 100 },
            "memory_usage": { "type": "number", "minimum": 0, "maximum": 100 },
            "active_tasks": { "type": "integer", "minimum": 0 },
            "messages_processed": { "type": "integer", "minimum": 0 },
            "error_rate": { "type": "number", "minimum": 0, "maximum": 1 }
          }
        },
        "restart_reason": {
          "enum": ["TIMEOUT", "RESOURCE_EXHAUSTED", "FATAL_ERROR", "CONFIGURATION_CHANGE", "MANUAL"]
        }
      }
    }
  }
}
```

#### 3.4.6 Hook Integration Messages Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mister-smith.ai/schemas/hook-message",
  "title": "Claude-CLI Hook Integration Message",
  "allOf": [
    { "$ref": "https://mister-smith.ai/schemas/base-message" }
  ],
  "properties": {
    "type": {
      "enum": ["PRE_TASK", "POST_TASK", "ON_ERROR", "ON_FILE_CHANGE", "STARTUP"]
    },
    "payload": {
      "type": "object",
      "properties": {
        "hook_context": {
          "type": "object",
          "properties": {
            "cwd": { "type": "string" },
            "project_config": { "type": "object" },
            "environment": {
              "type": "object",
              "additionalProperties": { "type": "string" }
            }
          },
          "required": ["cwd"]
        },
        "task_info": {
          "type": "object",
          "properties": {
            "task_id": { "type": "string", "format": "uuid" },
            "description": { "type": "string" },
            "agent_id": { "$ref": "https://mister-smith.ai/schemas/base-message#/$defs/AgentId" }
          }
        },
        "file_changes": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "path": { "type": "string" },
              "change_type": { "enum": ["CREATED", "MODIFIED", "DELETED"] },
              "timestamp": { "type": "string", "format": "date-time" }
            },
            "required": ["path", "change_type"]
          }
        },
        "error_info": {
          "type": "object",
          "properties": {
            "error_type": { "type": "string" },
            "message": { "type": "string" },
            "stack_trace": { "type": "string" },
            "context": { "type": "object" }
          },
          "required": ["error_type", "message"]
        }
      }
    }
  }
}
```

### 3.5 Enhanced Message Validation & Processing

#### 3.5.1 Runtime Message Validation

```pseudocode
STRUCT MessageValidator {
    schemas: HashMap<String, MessageSchema>,
    validation_cache: Arc<RwLock<LruCache<String, ValidationResult>>>,
    custom_rules: Vec<Box<dyn ValidationRule>>,
    metrics: ValidationMetrics
}

TRAIT ValidationRule {
    FUNCTION rule_name(&self) -> &str
    ASYNC FUNCTION validate(&self, message: &Message) -> ValidationResult
    FUNCTION severity(&self) -> ValidationSeverity
}

IMPL MessageValidator {
    #[tracing::instrument(skip(self, message))]
    ASYNC FUNCTION validate_message(&self, message: &Message) -> ValidationResult {
        // Check cache first
        cache_key = format!("{}-{}", message.message_type, message.content_hash())
        IF LET Some(cached_result) = self.validation_cache.read().await.get(&cache_key) {
            RETURN cached_result.clone()
        }
        
        // Get schema for message type
        schema = self.schemas.get(&message.message_type)
            .ok_or(ValidationError::UnknownMessageType(message.message_type.clone()))?
        
        validation_errors = Vec::new()
        
        // Validate required fields
        FOR field IN &schema.required_fields {
            IF !message.has_field(&field.name) {
                validation_errors.push(ValidationError::MissingField(field.name.clone()))
            } ELSE {
                field_value = message.get_field(&field.name)
                IF !field.validates(field_value) {
                    validation_errors.push(ValidationError::InvalidField {
                        field: field.name.clone(),
                        expected: field.field_type.clone(),
                        actual: field_value.type_name()
                    })
                }
            }
        }
        
        // Apply custom validation rules
        FOR rule IN &self.custom_rules {
            rule_result = rule.validate(message).await
            IF rule_result.is_invalid() {
                validation_errors.extend(rule_result.errors)
            }
        }
        
        result = IF validation_errors.is_empty() {
            ValidationResult::Valid
        } ELSE {
            ValidationResult::Invalid {
                errors: validation_errors,
                message_id: message.id.clone()
            }
        }
        
        // Cache result
        self.validation_cache.write().await.insert(cache_key, result.clone())
        self.metrics.record_validation(&message.message_type, &result)
        
        result
    }
}

// Built-in validation rules
STRUCT AgentCapabilityRule {
    capability_registry: Arc<CapabilityRegistry>
}

IMPL ValidationRule FOR AgentCapabilityRule {
    ASYNC FUNCTION validate(&self, message: &Message) -> ValidationResult {
        // Validate sender has required capabilities for message type
        required_capabilities = self.get_required_capabilities(&message.message_type)
        agent_capabilities = self.capability_registry.get_agent_capabilities(&message.sender).await?
        
        missing_capabilities = required_capabilities.iter()
            .filter(|cap| !agent_capabilities.contains(cap))
            .cloned()
            .collect::<Vec<_>>()
        
        IF !missing_capabilities.is_empty() {
            ValidationResult::Invalid {
                errors: vec![ValidationError::InsufficientCapabilities {
                    agent_id: message.sender.clone(),
                    required: required_capabilities,
                    missing: missing_capabilities
                }],
                message_id: message.id.clone()
            }
        } ELSE {
            ValidationResult::Valid
        }
    }
}
```

#### 3.5.2 Priority-Based Mailbox with Backpressure

```pseudocode
STRUCT AgentMailbox {
    priority_queues: [VecDeque<Message>; 5], // One queue per priority level
    capacity_per_priority: [usize; 5],
    total_capacity: usize,
    current_size: AtomicUsize,
    backpressure_strategy: BackpressureStrategy,
    message_validator: MessageValidator,
    metrics: MailboxMetrics,
    notification_channel: mpsc::Sender<MailboxEvent>
}

ENUM BackpressureStrategy {
    Block,                    // Block sender until space available
    Drop(MessagePriority),    // Drop messages below specified priority
    Overflow(usize),          // Allow temporary overflow up to limit
    Reject,                   // Reject new messages immediately
    SpillToSecondary(String)  // Spill to secondary storage
}

ENUM MailboxEvent {
    MessageEnqueued { message_id: String, priority: MessagePriority },
    MessageDequeued { message_id: String, wait_time: Duration },
    BackpressureActivated { strategy: String, queue_size: usize },
    MessageDropped { message_id: String, reason: String },
    QueueFull { priority: MessagePriority, size: usize }
}

IMPL AgentMailbox {
    #[tracing::instrument(skip(self, message))]
    ASYNC FUNCTION send(&self, message: Message) -> Result<SendResult, MailboxError> {
        // Validate message first
        validation_result = self.message_validator.validate_message(&message).await
        IF !validation_result.is_valid() {
            self.metrics.record_validation_failure(&message.message_type)
            RETURN Err(MailboxError::ValidationFailed(validation_result))
        }
        
        priority_index = message.priority as usize
        current_size = self.current_size.load(Ordering::Acquire)
        
        // Check if message has expired
        IF message.is_expired() {
            self.metrics.record_expired_message(&message.message_type)
            RETURN Err(MailboxError::MessageExpired)
        }
        
        // Handle capacity constraints
        IF current_size >= self.total_capacity {
            RETURN self.handle_capacity_exceeded(message).await
        }
        
        // Check priority-specific capacity
        priority_queue_size = self.priority_queues[priority_index].len()
        IF priority_queue_size >= self.capacity_per_priority[priority_index] {
            RETURN self.handle_priority_queue_full(message, priority_index).await
        }
        
        // Enqueue message
        self.priority_queues[priority_index].push_back(message.clone())
        new_size = self.current_size.fetch_add(1, Ordering::Release) + 1
        
        // Send notification
        mailbox_event = MailboxEvent::MessageEnqueued {
            message_id: message.id.clone(),
            priority: message.priority
        }
        self.notification_channel.try_send(mailbox_event).ok() // Non-blocking
        
        // Record metrics
        self.metrics.record_message_enqueued(&message.message_type, message.priority)
        
        tracing::debug!(
            message_id = %message.id,
            message_type = %message.message_type,
            priority = ?message.priority,
            queue_size = new_size,
            "Message enqueued successfully"
        )
        
        Ok(SendResult::Enqueued { queue_position: self.estimate_queue_position(&message) })
    }
    
    #[tracing::instrument(skip(self))]
    ASYNC FUNCTION receive(&self, timeout: Option<Duration>) -> Result<Message, MailboxError> {
        deadline = timeout.map(|t| Instant::now() + t)
        start_time = Instant::now()
        
        LOOP {
            // Check priority queues in order (highest priority first)
            FOR priority_index IN 0..5 {
                IF LET Some(message) = self.priority_queues[priority_index].pop_front() {
                    self.current_size.fetch_sub(1, Ordering::Release)
                    wait_time = start_time.elapsed()
                    
                    // Send notification
                    mailbox_event = MailboxEvent::MessageDequeued {
                        message_id: message.id.clone(),
                        wait_time
                    }
                    self.notification_channel.try_send(mailbox_event).ok()
                    
                    // Record metrics
                    self.metrics.record_message_dequeued(&message.message_type, message.priority, wait_time)
                    
                    tracing::debug!(
                        message_id = %message.id,
                        message_type = %message.message_type,
                        priority = ?message.priority,
                        wait_time = ?wait_time,
                        "Message dequeued"
                    )
                    
                    RETURN Ok(message)
                }
            }
            
            // No messages available, check timeout
            IF LET Some(deadline) = deadline {
                IF Instant::now() >= deadline {
                    RETURN Err(MailboxError::ReceiveTimeout)
                }
            }
            
            // Wait for new messages with exponential backoff
            wait_duration = Duration::from_millis(1 << min(5, start_time.elapsed().as_millis() / 100))
            tokio::time::sleep(wait_duration).await
        }
    }
    
    ASYNC FUNCTION handle_capacity_exceeded(&self, message: Message) -> Result<SendResult, MailboxError> {
        MATCH &self.backpressure_strategy {
            BackpressureStrategy::Block => {
                // Wait for space with exponential backoff
                backoff = ExponentialBackoff::new(Duration::from_millis(1), Duration::from_secs(1))
                
                WHILE self.current_size.load(Ordering::Acquire) >= self.total_capacity {
                    tokio::time::sleep(backoff.next_delay()).await
                }
                
                self.send(message).await
            },
            BackpressureStrategy::Drop(min_priority) => {
                IF message.priority >= *min_priority {
                    // Try to make space by dropping lower priority messages
                    dropped_count = self.drop_lower_priority_messages(message.priority)
                    IF dropped_count > 0 {
                        self.send(message).await
                    } ELSE {
                        self.metrics.record_message_dropped(&message.message_type, "no_space_after_drop")
                        Err(MailboxError::Dropped("Unable to make space".to_string()))
                    }
                } ELSE {
                    self.metrics.record_message_dropped(&message.message_type, "low_priority")
                    Err(MailboxError::Dropped("Message priority too low".to_string()))
                }
            },
            BackpressureStrategy::Overflow(max_overflow) => {
                current_overflow = self.current_size.load(Ordering::Acquire) - self.total_capacity
                IF current_overflow < *max_overflow {
                    // Allow temporary overflow
                    self.priority_queues[message.priority as usize].push_back(message.clone())
                    self.current_size.fetch_add(1, Ordering::Release)
                    
                    // Schedule cleanup task
                    self.schedule_overflow_cleanup().await
                    
                    Ok(SendResult::Overflowed { overflow_count: current_overflow + 1 })
                } ELSE {
                    Err(MailboxError::OverflowLimitExceeded)
                }
            },
            BackpressureStrategy::Reject => {
                self.metrics.record_message_rejected(&message.message_type)
                Err(MailboxError::Rejected("Mailbox at capacity".to_string()))
            },
            BackpressureStrategy::SpillToSecondary(storage_path) => {
                // Spill to secondary storage
                spill_result = self.spill_to_storage(&message, storage_path).await
                MATCH spill_result {
                    Ok(spill_id) => Ok(SendResult::Spilled { spill_id }),
                    Err(e) => Err(MailboxError::SpillFailed(e))
                }
            }
        }
    }
}
```

### 3.6 Agent State Machine Management

#### 3.6.1 State Persistence with Event Sourcing

```pseudocode
// Event sourcing for agent state management
STRUCT AgentStateManager {
    event_store: EventStore,
    current_states: Arc<RwLock<HashMap<AgentId, AgentState>>>,
    state_cache: Arc<RwLock<LruCache<AgentId, CachedState>>>,
    snapshot_store: SnapshotStore,
    state_validators: Vec<Box<dyn StateValidator>>
}

TRAIT StateEvent {
    FUNCTION event_type(&self) -> &str
    FUNCTION agent_id(&self) -> &AgentId
    FUNCTION apply_to_state(&self, state: &mut AgentState) -> Result<()>
    FUNCTION timestamp(&self) -> DateTime<Utc>
    FUNCTION version(&self) -> u64
}

ENUM AgentStateEvent {
    AgentSpawned {
        agent_id: AgentId,
        agent_type: String,
        configuration: AgentConfig,
        supervisor_id: Option<AgentId>,
        timestamp: DateTime<Utc>
    },
    StateTransition {
        agent_id: AgentId,
        from_state: AgentLifecycleState,
        to_state: AgentLifecycleState,
        reason: String,
        timestamp: DateTime<Utc>
    },
    TaskAssigned {
        agent_id: AgentId,
        task_id: TaskId,
        task_spec: TaskSpecification,
        timestamp: DateTime<Utc>
    },
    TaskCompleted {
        agent_id: AgentId,
        task_id: TaskId,
        result: TaskResult,
        execution_time: Duration,
        timestamp: DateTime<Utc>
    },
    CapabilityUpdated {
        agent_id: AgentId,
        capability: String,
        operation: CapabilityOperation,
        timestamp: DateTime<Utc>
    },
    ConfigurationChanged {
        agent_id: AgentId,
        config_changes: HashMap<String, Value>,
        timestamp: DateTime<Utc>
    }
}

IMPL AgentStateManager {
    #[tracing::instrument(skip(self, event))]
    ASYNC FUNCTION persist_state_event(&self, event: Box<dyn StateEvent>) -> Result<()> {
        // Validate event
        FOR validator IN &self.state_validators {
            validator.validate_event(&*event)?
        }
        
        // Store event in event store
        event_id = self.event_store.append_event(event.clone()).await?
        
        // Apply event to current state
        current_states = self.current_states.write().await
        state = current_states.entry(event.agent_id().clone())
            .or_insert_with(|| AgentState::new(event.agent_id().clone()))
        
        event.apply_to_state(state)?
        state.last_event_id = Some(event_id)
        state.version += 1
        
        // Update cache
        self.state_cache.write().await.insert(
            event.agent_id().clone(),
            CachedState {
                state: state.clone(),
                cached_at: Instant::now(),
                ttl: Duration::from_secs(300)
            }
        )
        
        // Check if snapshot needed
        IF state.version % SNAPSHOT_INTERVAL == 0 {
            self.create_state_snapshot(event.agent_id().clone()).await?
        }
        
        tracing::info!(
            agent_id = %event.agent_id(),
            event_type = %event.event_type(),
            version = state.version,
            "State event persisted"
        )
        
        Ok(())
    }
    
    #[tracing::instrument(skip(self))]
    ASYNC FUNCTION restore_agent_state(&self, agent_id: &AgentId) -> Result<AgentState> {
        // Check cache first
        IF LET Some(cached) = self.state_cache.read().await.get(agent_id) {
            IF !cached.is_expired() {
                RETURN Ok(cached.state.clone())
            }
        }
        
        // Try to load latest snapshot
        state = IF LET Some(snapshot) = self.snapshot_store.load_latest(agent_id).await? {
            snapshot.state
        } ELSE {
            AgentState::new(agent_id.clone())
        }
        
        // Apply events since snapshot
        last_event_id = state.last_event_id.clone()
        events = self.event_store.load_events_since(agent_id, last_event_id).await?
        
        FOR event IN events {
            event.apply_to_state(&mut state)?
            state.version += 1
        }
        
        // Cache restored state
        self.state_cache.write().await.insert(
            agent_id.clone(),
            CachedState {
                state: state.clone(),
                cached_at: Instant::now(),
                ttl: Duration::from_secs(300)
            }
        )
        
        // Update current states
        self.current_states.write().await.insert(agent_id.clone(), state.clone())
        
        Ok(state)
    }
    
    ASYNC FUNCTION create_state_snapshot(&self, agent_id: AgentId) -> Result<()> {
        current_states = self.current_states.read().await
        IF LET Some(state) = current_states.get(&agent_id) {
            snapshot = StateSnapshot {
                agent_id: agent_id.clone(),
                state: state.clone(),
                created_at: Utc::now(),
                version: state.version
            }
            
            self.snapshot_store.save_snapshot(snapshot).await?
            tracing::info!(agent_id = %agent_id, version = state.version, "State snapshot created")
        }
        
        Ok(())
    }
}
```

#### 3.6.2 State Machine with Supervision Integration

```pseudocode
STRUCT AgentStateMachine {
    current_state: Arc<RwLock<AgentLifecycleState>>,
    allowed_transitions: HashMap<AgentLifecycleState, Vec<AgentLifecycleState>>,
    state_handlers: HashMap<AgentLifecycleState, Box<dyn StateHandler>>,
    transition_guards: HashMap<(AgentLifecycleState, AgentLifecycleState), Box<dyn TransitionGuard>>,
    state_manager: AgentStateManager,
    metrics: StateMachineMetrics
}

ENUM AgentLifecycleState {
    Initializing,
    Idle,
    Busy(TaskId),
    Paused,
    Error(ErrorInfo),
    Restarting,
    Terminating,
    Terminated
}

TRAIT StateHandler {
    ASYNC FUNCTION on_enter(&self, agent_id: &AgentId, previous_state: Option<AgentLifecycleState>) -> Result<()>
    ASYNC FUNCTION on_exit(&self, agent_id: &AgentId, next_state: AgentLifecycleState) -> Result<()>
    ASYNC FUNCTION handle_message(&self, agent_id: &AgentId, message: Message) -> Result<StateHandlerResult>
}

TRAIT TransitionGuard {
    ASYNC FUNCTION can_transition(&self, agent_id: &AgentId, from: &AgentLifecycleState, to: &AgentLifecycleState) -> bool
    FUNCTION guard_name(&self) -> &str
}

ENUM StateHandlerResult {
    Handled,
    TransitionTo(AgentLifecycleState),
    Forward(Message),
    Error(String)
}

IMPL AgentStateMachine {
    #[tracing::instrument(skip(self))]
    ASYNC FUNCTION transition_to(
        &self, 
        agent_id: &AgentId, 
        new_state: AgentLifecycleState,
        reason: String
    ) -> Result<(), StateMachineError> {
        current_state_guard = self.current_state.read().await
        current_state = current_state_guard.clone()
        drop(current_state_guard)
        
        // Check if transition is allowed
        allowed_states = self.allowed_transitions.get(&current_state)
            .ok_or(StateMachineError::InvalidCurrentState(current_state.clone()))?
        
        IF !allowed_states.contains(&new_state) {
            RETURN Err(StateMachineError::TransitionNotAllowed {
                from: current_state,
                to: new_state,
                allowed: allowed_states.clone()
            })
        }
        
        // Check transition guards
        guard_key = (current_state.clone(), new_state.clone())
        IF LET Some(guard) = self.transition_guards.get(&guard_key) {
            can_transition = guard.can_transition(agent_id, &current_state, &new_state).await
            IF !can_transition {
                RETURN Err(StateMachineError::TransitionBlocked {
                    guard: guard.guard_name().to_string(),
                    reason: format!("Transition from {:?} to {:?} blocked", current_state, new_state)
                })
            }
        }
        
        // Execute state exit handler
        IF LET Some(current_handler) = self.state_handlers.get(&current_state) {
            current_handler.on_exit(agent_id, new_state.clone()).await?
        }
        
        // Perform the transition
        {
            current_state_guard = self.current_state.write().await
            *current_state_guard = new_state.clone()
        }
        
        // Persist state change event
        state_event = AgentStateEvent::StateTransition {
            agent_id: agent_id.clone(),
            from_state: current_state.clone(),
            to_state: new_state.clone(),
            reason,
            timestamp: Utc::now()
        }
        
        self.state_manager.persist_state_event(Box::new(state_event)).await?
        
        // Execute state enter handler
        IF LET Some(new_handler) = self.state_handlers.get(&new_state) {
            new_handler.on_enter(agent_id, Some(current_state.clone())).await?
        }
        
        // Record metrics
        self.metrics.record_state_transition(&current_state, &new_state)
        
        tracing::info!(
            agent_id = %agent_id,
            from_state = ?current_state,
            to_state = ?new_state,
            "State transition completed"
        )
        
        Ok(())
    }
    
    #[tracing::instrument(skip(self, message))]
    ASYNC FUNCTION handle_message(&self, agent_id: &AgentId, message: Message) -> Result<()> {
        current_state = self.current_state.read().await.clone()
        
        IF LET Some(handler) = self.state_handlers.get(&current_state) {
            result = handler.handle_message(agent_id, message.clone()).await?
            
            MATCH result {
                StateHandlerResult::Handled => {
                    tracing::debug!(agent_id = %agent_id, "Message handled in current state")
                },
                StateHandlerResult::TransitionTo(new_state) => {
                    self.transition_to(agent_id, new_state, "Message-triggered transition".to_string()).await?
                },
                StateHandlerResult::Forward(forwarded_message) => {
                    // Forward message to supervisor or other agents
                    self.forward_message(agent_id, forwarded_message).await?
                },
                StateHandlerResult::Error(error_msg) => {
                    error_state = AgentLifecycleState::Error(ErrorInfo {
                        error_type: "MessageHandlingError".to_string(),
                        message: error_msg,
                        timestamp: Utc::now(),
                        recoverable: true
                    })
                    self.transition_to(agent_id, error_state, "Message handling error".to_string()).await?
                }
            }
        } ELSE {
            tracing::warn!(
                agent_id = %agent_id,
                state = ?current_state,
                "No handler for current state"
            )
        }
        
        Ok(())
    }
}

// Built-in state handlers
STRUCT IdleStateHandler;

IMPL StateHandler FOR IdleStateHandler {
    ASYNC FUNCTION on_enter(&self, agent_id: &AgentId, _previous_state: Option<AgentLifecycleState>) -> Result<()> {
        tracing::info!(agent_id = %agent_id, "Agent entered idle state")
        Ok(())
    }
    
    ASYNC FUNCTION handle_message(&self, _agent_id: &AgentId, message: Message) -> Result<StateHandlerResult> {
        MATCH message.message_type.as_str() {
            "TASK_ASSIGN" => {
                task_id = message.payload.get("task_id").unwrap().as_str().unwrap()
                Ok(StateHandlerResult::TransitionTo(AgentLifecycleState::Busy(task_id.to_string())))
            },
            "PAUSE" => {
                Ok(StateHandlerResult::TransitionTo(AgentLifecycleState::Paused))
            },
            "TERMINATE" => {
                Ok(StateHandlerResult::TransitionTo(AgentLifecycleState::Terminating))
            },
            _ => Ok(StateHandlerResult::Handled)
        }
    }
}
```

## 4. Task Distribution

> **Agent Types**: See `agent-lifecycle.md` section 1.2 for detailed Planner, Executor, and Router agent role taxonomies

### 4.1 Work Queue Pattern

```pseudocode
CLASS TaskDistributor {
    PRIVATE workers: List<WorkerAgent>
    PRIVATE taskQueue: Queue<Task>
    
    FUNCTION distribute(task: Task) {
        // Find available worker
        worker = findAvailableWorker()
        
        IF worker != NULL THEN
            worker.assign(task)
        ELSE
            taskQueue.enqueue(task)
        END IF
    }
    
    FUNCTION findAvailableWorker() -> WorkerAgent? {
        FOR worker IN workers {
            IF worker.isAvailable() THEN
                RETURN worker
            END IF
        }
        RETURN NULL
    }
}
```

### 4.2 Load Balancing

```pseudocode
CLASS LoadBalancer {
    PRIVATE agents: List<Agent>
    PRIVATE currentIndex: Integer = 0
    
    FUNCTION selectAgent() -> Agent {
        // Round-robin selection
        agent = agents[currentIndex]
        currentIndex = (currentIndex + 1) % agents.size()
        RETURN agent
    }
    
    FUNCTION selectLeastLoaded() -> Agent {
        // Select agent with fewest active tasks
        RETURN agents.minBy(agent => agent.getActiveTaskCount())
    }
}
```

## 5. Coordination Patterns

### 5.1 Request-Response Pattern

```pseudocode
CLASS RequestHandler {
    PRIVATE pendingRequests: Map<UUID, ResponseCallback>
    
    FUNCTION sendRequest(target: Agent, request: Request) -> Future<Response> {
        requestId = generateUUID()
        message = RequestMessage(requestId, request)
        
        future = Future<Response>()
        pendingRequests[requestId] = future.callback
        
        target.send(message)
        RETURN future
    }
    
    FUNCTION handleResponse(response: Response) {
        callback = pendingRequests.remove(response.requestId)
        IF callback != NULL THEN
            callback(response)
        END IF
    }
}
```

### 5.2 Publish-Subscribe Pattern

```pseudocode
CLASS EventBus {
    PRIVATE subscribers: Map<String, List<Agent>>
    
    FUNCTION subscribe(topic: String, agent: Agent) {
        IF NOT subscribers.contains(topic) THEN
            subscribers[topic] = []
        END IF
        subscribers[topic].add(agent)
    }
    
    FUNCTION publish(topic: String, event: Event) {
        agents = subscribers.get(topic, [])
        FOR agent IN agents {
            agent.send(EventMessage(topic, event))
        }
    }
}
```

## Related Documentation

### Framework Components
- **Agent Lifecycle**: `agent-lifecycle.md` - Basic agent types, supervision patterns, and lifecycle management
- **Agent Operations**: `agent-operations.md` - Discovery, workflow management, and operational patterns
- **Agent Integration**: `agent-integration.md` - Resource management, tool bus integration, and extension patterns
- **Security Protocols**: `../security/` - Authentication and authorization for agent communications
- **Transport Layer**: `../transport/` - Low-level messaging infrastructure
- **Operations**: `../operations/` - Deployment and monitoring considerations

### Implementation References
- **Technology Stack**: `tech-framework.md` - Rust/Tokio runtime specifications
- **Claude CLI Integration**: `../../research/claude-cli-integration/` - Hook system for external tool integration
- **Data Management**: `data-persistence.md` - State storage and persistence strategies

### Cross-References
- **Message Schemas**: All JSON schemas in section 3.4 are self-contained and reference the base message schema
- **Agent Types**: References agent type enums defined in `agent-lifecycle.md`
- **Task Types**: Task message schemas support extensible task type definitions
- **Error Handling**: State machine error states integrate with supervision restart policies from `agent-lifecycle.md`

### Navigation Links
- **← Previous**: [Agent Lifecycle](agent-lifecycle.md) - Basic agent architecture and supervision
- **→ Next**: [Agent Operations](agent-operations.md) - Discovery and workflow management
- **↑ Parent**: [Data Management](CLAUDE.md) - Data management overview

---

**File Size Optimization Notes**:
- Current size: 1,304 lines (exceeds recommended 1,000 line limit)
- **Suggested extraction**: Section 3.4 (Complete Message Schema Definitions) could be moved to `core-message-schemas.md`
- **Suggested extraction**: Section 3.5 (Enhanced Message Validation) could be moved to a dedicated `message-validation.md`
- **Suggested extraction**: Section 3.6 (Agent State Machine Management) has overlap with `agent-lifecycle.md` and could be consolidated

*Mister Smith AI Agent Framework - Communication Architecture*
*Modularized from agent-orchestration.md sections 3-5*
*Agent 12, Phase 1, Group 1B - Framework Modularization Operation*
*Updated by Agent 31, Phase 2, Batch 2 - Cross-reference optimization*