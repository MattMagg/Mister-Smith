---
title: System Integration Patterns & Implementation
type: implementation
permalink: core-architecture/system-integration
tags:
- '#integration #implementation #patterns #extensions #messaging #event-sourcing #saga'
---

# System Integration Patterns & Implementation

*Extracted from system-architecture.md - Sections 5-7*

This document provides comprehensive integration patterns, implementation guidelines, and extension mechanisms for the Mister Smith AI Agent Framework. It covers advanced message routing, state persistence patterns, implementation best practices, and extension points for customization.

## Table of Contents

- 5. Integration Patterns
  - 5.1 Enhanced Message Routing & Addressing
  - 5.2 Health Check and Monitoring
  - 5.3 Shared Tool Registry Pattern
  - 5.4 State Persistence & Recovery
  - 5.5 Async Message Flow Patterns
- 6. Implementation Guidelines
  - 6.1 Error Handling Strategy
  - 6.2 Testing Framework
  - 6.3 Critical Anti-Patterns to Avoid
- 7. Extension Mechanisms
  - 7.1 Middleware Pattern
  - 7.2 Event Emitter Pattern
  - 7.3 Custom Routing Strategies

## 5. Integration Patterns

### 5.1 Enhanced Message Routing & Addressing

> **Note**: This message routing system integrates with the [Implementation Configuration](implementation-config.md#module-organization-structure) to provide a complete messaging framework. See [Transport Core](../transport/transport-core.md) for protocol-specific implementations.

#### 5.1.1 Hierarchical Message Addressing

```pseudocode
// AsyncAPI-inspired addressing scheme with NATS subject patterns
ENUM MessageAddress {
    // Agent lifecycle: agents.{supervisor_id}.{operation}.{agent_type}.{agent_id}
    AgentSpawn(supervisor_id: String, agent_type: String, agent_id: String),
    AgentTerminate(supervisor_id: String, agent_id: String),
    
    // Task management: tasks.{agent_id}.{operation}.{task_type}.{task_id}
    TaskAssign(agent_id: String, task_type: String, task_id: String),
    TaskComplete(agent_id: String, task_id: String),
    TaskFailed(agent_id: String, task_id: String),
    
    // State management: state.{domain}.{operation}.{entity_id}
    StateSnapshot(domain: String, entity_id: String),
    StateTransition(domain: String, entity_id: String, from_state: String, to_state: String),
    
    // System events: system.{service}.{operation}.{scope}
    SystemHealth(service: String, scope: String),
    SystemShutdown(scope: String),
    
    // Control messages: control.{operation}.{target}
    ControlPause(target: String),
    ControlResume(target: String)
}

IMPL MessageAddress {
    FUNCTION to_subject(&self) -> String {
        MATCH self {
            AgentSpawn(supervisor, agent_type, agent_id) => 
                format!("agents.{}.spawn.{}.{}", supervisor, agent_type, agent_id),
            TaskAssign(agent_id, task_type, task_id) => 
                format!("tasks.{}.assign.{}.{}", agent_id, task_type, task_id),
            StateTransition(domain, entity_id, from, to) => 
                format!("state.{}.transition.{}.{}.{}", domain, entity_id, from, to),
            // ... other patterns
        }
    }
    
    FUNCTION from_subject(subject: &str) -> Result<Self> {
        parts = subject.split('.').collect::<Vec<_>>()
        MATCH parts.as_slice() {
            ["agents", supervisor, "spawn", agent_type, agent_id] => 
                Ok(AgentSpawn(supervisor.to_string(), agent_type.to_string(), agent_id.to_string())),
            ["tasks", agent_id, "assign", task_type, task_id] => 
                Ok(TaskAssign(agent_id.to_string(), task_type.to_string(), task_id.to_string())),
            // ... other patterns
            _ => Err(AddressingError::InvalidSubject(subject.to_string()))
        }
    }
    
    FUNCTION supports_wildcard(&self) -> bool {
        // Enable subscription patterns like "agents.*.spawn.*.*"
        true
    }
}
```

#### 5.1.2 Message Schema Validation

```pseudocode
// AsyncAPI-inspired message schema with validation
STRUCT MessageSchema {
    message_type: String,
    version: String,
    required_headers: Vec<String>,
    optional_headers: Vec<String>,
    payload_schema: JsonSchema,
    examples: Vec<MessageExample>
}

STRUCT MessageValidator {
    schemas: HashMap<String, MessageSchema>,
    validation_cache: Arc<RwLock<HashMap<String, ValidationResult>>>
}

IMPL MessageValidator {
    ASYNC FUNCTION validate_message(&self, message: &Message) -> ValidationResult {
        // Check cache first
        cache_key = format!("{}-{}", message.message_type, message.checksum())
        IF LET Some(cached_result) = self.validation_cache.read().await.get(&cache_key) {
            RETURN cached_result.clone()
        }
        
        schema = self.schemas.get(&message.message_type)
            .ok_or(ValidationError::UnknownMessageType)?
        
        // Validate headers
        FOR required_header IN &schema.required_headers {
            IF !message.headers.contains_key(required_header) {
                RETURN ValidationResult::Failed(ValidationError::MissingHeader(required_header.clone()))
            }
        }
        
        // Validate payload against JSON schema
        validation_result = schema.payload_schema.validate(&message.payload)?
        
        // Cache result
        self.validation_cache.write().await.insert(cache_key, validation_result.clone())
        
        RETURN validation_result
    }
    
    FUNCTION register_schema(&mut self, schema: MessageSchema) {
        self.schemas.insert(schema.message_type.clone(), schema)
    }
}
```

#### 5.1.3 Enhanced Message Bridge

```pseudocode
STRUCT MessageBridge {
    routing_table: Arc<RwLock<HashMap<String, Vec<ComponentId>>>>,
    message_validator: MessageValidator,
    message_serializer: MessageSerializer,
    transport: Transport,
    dead_letter_queue: DeadLetterQueue,
    metrics: MessageMetrics,
    correlation_tracker: CorrelationTracker
}

IMPL MessageBridge {
    #[tracing::instrument(skip(self, message))]
    ASYNC FUNCTION route_message<M: Message>(&self, message: M, address: MessageAddress) -> Result<()> {
        // Validate message
        validation_result = self.message_validator.validate_message(&message).await?
        IF validation_result.is_failed() {
            self.handle_validation_failure(message, validation_result).await?
            RETURN Err(MessageError::ValidationFailed)
        }
        
        // Serialize message
        serialized = self.message_serializer.serialize(&message)?
        
        // Create routing info with correlation tracking
        routing_info = RoutingInfo {
            subject: address.to_subject(),
            correlation_id: message.correlation_id.clone(),
            reply_to: message.reply_to.clone(),
            priority: message.priority,
            timestamp: Utc::now()
        }
        
        // Track correlation for request-reply patterns
        IF LET Some(correlation_id) = &routing_info.correlation_id {
            self.correlation_tracker.track_outbound(correlation_id.clone(), routing_info.clone()).await
        }
        
        // Send with retry and timeout
        send_result = tokio::time::timeout(
            message.timeout.unwrap_or(DEFAULT_MESSAGE_TIMEOUT),
            self.transport.send_with_retry(serialized, routing_info.clone(), RETRY_POLICY)
        ).await
        
        MATCH send_result {
            Ok(Ok(())) => {
                self.metrics.record_successful_send(&address.to_subject())
                Ok(())
            },
            Ok(Err(transport_error)) => {
                self.handle_transport_error(message, transport_error).await?
                Err(MessageError::TransportFailed(transport_error))
            },
            Err(timeout_error) => {
                self.dead_letter_queue.enqueue(message, "timeout").await?
                self.metrics.record_timeout(&address.to_subject())
                Err(MessageError::Timeout)
            }
        }
    }
    
    #[tracing::instrument(skip(self, message))]
    ASYNC FUNCTION broadcast<M: Message>(&self, message: M, pattern: &str) -> Result<BroadcastResult> {
        routing_table = self.routing_table.read().await
        matching_targets = routing_table.keys()
            .filter(|subject| self.subject_matches_pattern(subject, pattern))
            .cloned()
            .collect::<Vec<_>>()
        
        // Create futures for parallel sending
        send_futures = matching_targets.iter().map(|subject| {
            address = MessageAddress::from_subject(subject).unwrap()
            self.route_message(message.clone(), address)
        }).collect::<Vec<_>>()
        
        // Execute with partial failure handling
        results = join_all(send_futures).await
        
        successes = results.iter().filter(|r| r.is_ok()).count()
        failures = results.iter().filter(|r| r.is_err()).count()
        
        BroadcastResult {
            total_targets: matching_targets.len(),
            successful_sends: successes,
            failed_sends: failures,
            errors: results.into_iter().filter_map(|r| r.err()).collect()
        }
    }
    
    ASYNC FUNCTION subscribe(&self, pattern: &str, handler: MessageHandler) -> Result<SubscriptionId> {
        subscription_id = SubscriptionId::new()
        
        // Setup NATS subscription with pattern
        subscription = self.transport.subscribe(pattern).await?
        
        // Spawn handler task
        handler_task = tokio::spawn(async move {
            WHILE LET Some(message) = subscription.next().await {
                IF LET Err(e) = handler.handle(message).await {
                    tracing::error!(error = %e, "Message handler failed")
                }
            }
        })
        
        // Track subscription for cleanup
        self.track_subscription(subscription_id, handler_task).await
        
        RETURN Ok(subscription_id)
    }
    
    ASYNC FUNCTION request_reply<Req: Message, Resp: Message>(
        &self, 
        request: Req, 
        address: MessageAddress,
        timeout: Duration
    ) -> Result<Resp> {
        // Generate correlation ID
        correlation_id = Uuid::new_v4().to_string()
        
        // Setup reply subscription
        reply_subject = format!("_INBOX.{}", correlation_id)
        reply_subscription = self.transport.subscribe(&reply_subject).await?
        
        // Modify request with reply information
        request_with_reply = request.with_correlation_id(correlation_id.clone())
            .with_reply_to(reply_subject.clone())
        
        // Send request
        self.route_message(request_with_reply, address).await?
        
        // Wait for reply with timeout
        reply_result = tokio::time::timeout(timeout, async {
            WHILE LET Some(reply_message) = reply_subscription.next().await {
                IF reply_message.correlation_id == Some(correlation_id.clone()) {
                    RETURN self.message_serializer.deserialize::<Resp>(&reply_message.payload)
                }
            }
            Err(MessageError::NoReply)
        }).await
        
        MATCH reply_result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(MessageError::ReplyTimeout)
        }
    }
}
```

### 5.3 Shared Tool Registry Pattern

> **Configuration Reference**: Tool bus configuration is defined in [Implementation Configuration](implementation-config.md#implementation-completeness-checklist) under the "Tool System" section.

```pseudocode
STRUCT ToolBus {
    tools: Arc<RwLock<HashMap<ToolId, Box<dyn Tool>>>>,
    permissions: HashMap<AgentId, Vec<ToolId>>
}

TRAIT Tool: Send + Sync {
    ASYNC FUNCTION execute(&self, params: Value) -> Result<Value>
    FUNCTION schema(&self) -> ToolSchema
}

// Extension mechanism
IMPL ToolBus {
    FUNCTION register_tool<T: Tool + 'static>(&mut self, id: ToolId, tool: T) {
        self.tools.write().unwrap().insert(id, Box::new(tool))
    }
    
    ASYNC FUNCTION call(&self, agent_id: AgentId, tool_id: ToolId, params: Value) -> Result<Value> {
        // Permission check
        IF !self.has_permission(agent_id, tool_id) {
            RETURN Err("Unauthorized tool access")
        }
        
        tools = self.tools.read().unwrap()
        RETURN tools.get(&tool_id)?.execute(params).await
    }
}
```

### 5.4 Role-Based Agent Spawning

```pseudocode
ENUM AgentRole {
    ProductManager { sop: StandardProcedure },
    Architect { design_patterns: Vec<Pattern> },
    Engineer { toolchain: ToolSet },
    Researcher { knowledge_base: KnowledgeBase },
    Analyst { metrics_tools: MetricsSet }
}

STRUCT RoleSpawner {
    role_registry: HashMap<String, AgentRole>,
    spawn_controller: SpawnController,
    
    ASYNC FUNCTION spawn_team(&self, project: ProjectSpec) -> Team {
        agents = vec![]
        
        // Dynamic team composition based on project needs
        FOR role IN project.required_roles() {
            agent = self.spawn_role(role).await?
            agents.push(agent)
        }
        
        RETURN Team::new(agents, project.coordination_mode())
    }
    
    ASYNC FUNCTION spawn_role(&self, role: AgentRole) -> Result<Agent> {
        // Use spawn controller for resource-bounded spawning
        RETURN self.spawn_controller.spawn_bounded(role).await
    }
}
```

### 5.2 Health Check and Monitoring

```pseudocode
STRUCT HealthCheckManager {
    health_checks: Arc<RwLock<HashMap<ComponentId, Box<dyn HealthCheck>>>>,
    check_interval: Duration,
    failure_thresholds: HashMap<ComponentId, u32>,
    notification_channels: Vec<NotificationChannel>
}

TRAIT HealthCheck {
    ASYNC FUNCTION check_health(&self) -> HealthResult
    FUNCTION component_id(&self) -> ComponentId
    FUNCTION timeout(&self) -> Duration
}

IMPL HealthCheckManager {
    ASYNC FUNCTION run_health_checks(&self) -> Result<()> {
        LOOP {
            tokio::time::sleep(self.check_interval).await
            
            health_checks = self.health_checks.read().await
            futures = health_checks.values().map(|check| {
                timeout(check.timeout(), check.check_health())
            }).collect::<Vec<_>>()
            
            results = join_all(futures).await
            
            FOR (component_id, result) IN health_checks.keys().zip(results) {
                MATCH result {
                    Ok(Ok(HealthResult::Healthy)) => {
                        self.record_success(*component_id).await
                    },
                    Ok(Ok(HealthResult::Unhealthy(reason))) => {
                        self.handle_unhealthy(*component_id, reason).await?
                    },
                    Ok(Err(e)) | Err(e) => {
                        self.handle_check_failure(*component_id, e).await?
                    }
                }
            }
        }
    }
}
```

### 5.4 State Persistence & Recovery

> **Implementation Guide**: Event sourcing patterns connect with the module organization defined in [Implementation Configuration](implementation-config.md#module-organization-structure). See the `events/` module structure for concrete implementations.

#### 5.4.1 Event Sourcing for State Management

```pseudocode
// Event sourcing pattern for agent state persistence
STRUCT EventStore {
    storage: Arc<dyn EventStorage>,
    event_serializer: EventSerializer,
    snapshot_store: SnapshotStore,
    event_cache: Arc<RwLock<LruCache<EventId, Event>>>
}

TRAIT Event {
    FUNCTION event_type(&self) -> &str
    FUNCTION aggregate_id(&self) -> &str
    FUNCTION event_version(&self) -> u64
    FUNCTION timestamp(&self) -> DateTime<Utc>
    FUNCTION apply_to_state(&self, state: &mut AgentState) -> Result<()>
}

STRUCT AgentStateManager {
    event_store: EventStore,
    current_states: Arc<RwLock<HashMap<AgentId, AgentState>>>,
    snapshot_interval: u64,
    state_validators: Vec<Box<dyn StateValidator>>
}

IMPL AgentStateManager {
    #[tracing::instrument(skip(self, event))]
    ASYNC FUNCTION persist_event(&self, event: Box<dyn Event>) -> Result<()> {
        // Validate event before persistence
        FOR validator IN &self.state_validators {
            validator.validate_event(&*event)?
        }
        
        // Store event
        event_id = self.event_store.append_event(event.clone()).await?
        
        // Update in-memory state
        current_states = self.current_states.write().await
        IF LET Some(state) = current_states.get_mut(event.aggregate_id()) {
            event.apply_to_state(state)?
            state.last_event_id = event_id
            state.version += 1
        }
        
        // Check if snapshot needed
        IF state.version % self.snapshot_interval == 0 {
            self.create_snapshot(event.aggregate_id().to_string()).await?
        }
        
        Ok(())
    }
    
    #[tracing::instrument(skip(self))]
    ASYNC FUNCTION restore_state(&self, agent_id: &str) -> Result<AgentState> {
        // Try to load latest snapshot first
        IF LET Some(snapshot) = self.event_store.load_latest_snapshot(agent_id).await? {
            state = snapshot.state
            last_event_id = snapshot.last_event_id
        } ELSE {
            state = AgentState::default()
            last_event_id = None
        }
        
        // Apply events since snapshot
        events = self.event_store.load_events_since(agent_id, last_event_id).await?
        
        FOR event IN events {
            event.apply_to_state(&mut state)?
            state.version += 1
        }
        
        // Cache restored state
        self.current_states.write().await.insert(agent_id.to_string(), state.clone())
        
        Ok(state)
    }
    
    ASYNC FUNCTION create_snapshot(&self, agent_id: String) -> Result<()> {
        current_states = self.current_states.read().await
        IF LET Some(state) = current_states.get(&agent_id) {
            snapshot = StateSnapshot {
                agent_id: agent_id.clone(),
                state: state.clone(),
                last_event_id: state.last_event_id,
                timestamp: Utc::now()
            }
            
            self.event_store.save_snapshot(snapshot).await?
            tracing::info!(agent_id = %agent_id, version = state.version, "State snapshot created")
        }
        
        Ok(())
    }
}
```

#### 5.4.2 Distributed State Coordination

```pseudocode
// CQRS pattern for read/write separation
STRUCT CommandHandler {
    event_store: EventStore,
    command_validators: Vec<Box<dyn CommandValidator>>,
    state_manager: AgentStateManager
}

STRUCT QueryHandler {
    read_models: HashMap<String, Box<dyn ReadModel>>,
    query_cache: Arc<RwLock<LruCache<String, QueryResult>>>
}

TRAIT Command {
    FUNCTION command_type(&self) -> &str
    FUNCTION target_aggregate(&self) -> &str
    FUNCTION validate(&self, current_state: &AgentState) -> Result<()>
    FUNCTION to_events(&self, current_state: &AgentState) -> Result<Vec<Box<dyn Event>>>
}

IMPL CommandHandler {
    #[tracing::instrument(skip(self, command))]
    ASYNC FUNCTION handle_command(&self, command: Box<dyn Command>) -> Result<CommandResult> {
        // Load current state
        current_state = self.state_manager.restore_state(command.target_aggregate()).await?
        
        // Validate command
        command.validate(&current_state)?
        FOR validator IN &self.command_validators {
            validator.validate_command(&*command, &current_state)?
        }
        
        // Generate events
        events = command.to_events(&current_state)?
        
        // Persist events atomically
        FOR event IN events {
            self.state_manager.persist_event(event).await?
        }
        
        CommandResult {
            command_id: command.command_id(),
            events_generated: events.len(),
            new_state_version: current_state.version + events.len() as u64
        }
    }
}

// Saga pattern for distributed transactions
STRUCT SagaOrchestrator {
    saga_store: SagaStore,
    compensation_handlers: HashMap<String, Box<dyn CompensationHandler>>,
    timeout_manager: TimeoutManager
}

STRUCT Saga {
    saga_id: String,
    saga_type: String,
    steps: Vec<SagaStep>,
    current_step: usize,
    state: SagaState,
    compensation_data: HashMap<String, Value>
}

ENUM SagaState {
    Running,
    Compensating,
    Completed,
    Failed,
    Aborted
}

IMPL SagaOrchestrator {
    #[tracing::instrument(skip(self, saga))]
    ASYNC FUNCTION execute_saga(&self, mut saga: Saga) -> Result<SagaResult> {
        WHILE saga.current_step < saga.steps.len() && saga.state == SagaState::Running {
            step = &saga.steps[saga.current_step]
            
            // Execute step with timeout
            step_result = tokio::time::timeout(
                step.timeout,
                self.execute_saga_step(&mut saga, step)
            ).await
            
            MATCH step_result {
                Ok(Ok(())) => {
                    saga.current_step += 1
                    self.saga_store.save_saga(&saga).await?
                },
                Ok(Err(step_error)) => {
                    tracing::error!(saga_id = %saga.saga_id, step = saga.current_step, error = %step_error, "Saga step failed")
                    saga.state = SagaState::Compensating
                    self.compensate_saga(&mut saga).await?
                    BREAK
                },
                Err(_timeout) => {
                    tracing::error!(saga_id = %saga.saga_id, step = saga.current_step, "Saga step timed out")
                    saga.state = SagaState::Compensating
                    self.compensate_saga(&mut saga).await?
                    BREAK
                }
            }
        }
        
        IF saga.current_step >= saga.steps.len() {
            saga.state = SagaState::Completed
        }
        
        self.saga_store.save_saga(&saga).await?
        
        SagaResult {
            saga_id: saga.saga_id,
            final_state: saga.state,
            completed_steps: saga.current_step
        }
    }
    
    ASYNC FUNCTION compensate_saga(&self, saga: &mut Saga) -> Result<()> {
        // Execute compensation in reverse order
        FOR step_index IN (0..saga.current_step).rev() {
            step = &saga.steps[step_index]
            
            IF LET Some(handler) = self.compensation_handlers.get(&step.step_type) {
                compensation_data = saga.compensation_data.get(&step.step_id).cloned()
                
                compensation_result = handler.compensate(
                    &step.step_id,
                    compensation_data
                ).await
                
                IF compensation_result.is_err() {
                    tracing::error!(
                        saga_id = %saga.saga_id, 
                        step = step_index, 
                        "Compensation failed"
                    )
                    // Continue with remaining compensations
                }
            }
        }
        
        saga.state = SagaState::Aborted
        Ok(())
    }
}
```

### 5.5 Async Message Flow Patterns

#### 5.5.1 Stream-Based Message Processing

```pseudocode
// Tokio streams for message processing with backpressure
STRUCT MessageStream {
    inner: Pin<Box<dyn Stream<Item = Result<Message, MessageError>>>>,
    backpressure_config: BackpressureConfig,
    metrics: StreamMetrics
}

STRUCT MessageProcessor {
    input_streams: Vec<MessageStream>,
    processing_pipeline: ProcessingPipeline,
    output_sinks: Vec<MessageSink>,
    error_handler: ErrorHandler
}

IMPL MessageProcessor {
    #[tracing::instrument(skip(self))]
    ASYNC FUNCTION process_messages(&mut self) -> Result<()> {
        // Merge all input streams
        merged_stream = futures::stream::select_all(self.input_streams.iter_mut())
        
        // Process with backpressure handling
        merged_stream
            .map(|message_result| async move {
                MATCH message_result {
                    Ok(message) => {
                        self.process_single_message(message)
                            .instrument(tracing::info_span!(
                                "message_processing", 
                                message_id = %message.id,
                                message_type = %message.message_type
                            ))
                            .await
                    },
                    Err(error) => {
                        self.error_handler.handle_stream_error(error).await
                    }
                }
            })
            .buffer_unordered(CONCURRENT_MESSAGE_LIMIT)
            .try_for_each(|_| async { Ok(()) })
            .await
    }
    
    #[tracing::instrument(skip(self, message))]
    ASYNC FUNCTION process_single_message(&self, message: Message) -> Result<()> {
        // Apply processing pipeline stages
        processed_message = self.processing_pipeline.process(message).await?
        
        // Route to appropriate sinks
        FOR sink IN &self.output_sinks {
            IF sink.accepts_message_type(&processed_message.message_type) {
                // Handle sink backpressure
                MATCH sink.try_send(processed_message.clone()).await {
                    Ok(()) => continue,
                    Err(SinkError::Full) => {
                        // Apply backpressure strategy
                        MATCH self.backpressure_config.strategy {
                            BackpressureStrategy::Block => {
                                sink.send(processed_message.clone()).await?
                            },
                            BackpressureStrategy::Drop => {
                                self.metrics.record_dropped_message(&processed_message.message_type)
                                continue
                            },
                            BackpressureStrategy::Buffer => {
                                self.buffer_message_for_sink(sink.id(), processed_message.clone()).await?
                            }
                        }
                    },
                    Err(e) => return Err(e.into())
                }
            }
        }
        
        Ok(())
    }
}
```

#### 5.5.2 Future Composition for Message Flows

```pseudocode
// Complex message flows with proper error handling
STRUCT MessageFlow {
    flow_id: String,
    flow_type: String,
    stages: Vec<FlowStage>,
    error_policy: ErrorPolicy,
    timeout_config: TimeoutConfig
}

ENUM FlowStage {
    Sequential(Vec<MessageOperation>),
    Parallel(Vec<MessageOperation>),
    Conditional(Condition, Box<FlowStage>, Option<Box<FlowStage>>),
    Loop(LoopCondition, Box<FlowStage>),
    ErrorHandler(ErrorHandler)
}

STRUCT MessageFlowExecutor {
    flow_registry: HashMap<String, MessageFlow>,
    operation_handlers: HashMap<String, Box<dyn OperationHandler>>,
    metrics: FlowMetrics
}

IMPL MessageFlowExecutor {
    #[tracing::instrument(skip(self, message))]
    ASYNC FUNCTION execute_flow(
        &self, 
        flow_id: &str, 
        message: Message
    ) -> Result<FlowResult> {
        flow = self.flow_registry.get(flow_id)
            .ok_or(FlowError::UnknownFlow(flow_id.to_string()))?
        
        flow_context = FlowContext {
            message,
            variables: HashMap::new(),
            state: FlowState::Running
        }
        
        // Execute with overall timeout
        result = tokio::time::timeout(
            flow.timeout_config.total_timeout,
            self.execute_stages(&flow.stages, flow_context)
        ).await
        
        MATCH result {
            Ok(Ok(flow_result)) => {
                self.metrics.record_flow_success(flow_id)
                Ok(flow_result)
            },
            Ok(Err(flow_error)) => {
                self.handle_flow_error(flow, flow_error).await
            },
            Err(_timeout) => {
                self.metrics.record_flow_timeout(flow_id)
                Err(FlowError::Timeout)
            }
        }
    }
    
    #[tracing::instrument(skip(self, stages, context))]
    ASYNC FUNCTION execute_stages(
        &self,
        stages: &[FlowStage],
        mut context: FlowContext
    ) -> Result<FlowResult> {
        FOR stage IN stages {
            context = self.execute_stage(stage, context).await?
            
            IF context.state != FlowState::Running {
                BREAK
            }
        }
        
        FlowResult {
            final_message: context.message,
            variables: context.variables,
            state: context.state
        }
    }
    
    ASYNC FUNCTION execute_stage(
        &self,
        stage: &FlowStage,
        context: FlowContext
    ) -> Result<FlowContext> {
        MATCH stage {
            FlowStage::Sequential(operations) => {
                self.execute_sequential_operations(operations, context).await
            },
            FlowStage::Parallel(operations) => {
                self.execute_parallel_operations(operations, context).await
            },
            FlowStage::Conditional(condition, then_stage, else_stage) => {
                IF condition.evaluate(&context) {
                    self.execute_stage(then_stage, context).await
                } ELSE IF LET Some(else_stage) = else_stage {
                    self.execute_stage(else_stage, context).await
                } ELSE {
                    Ok(context)
                }
            },
            // ... other stage types
        }
    }
    
    ASYNC FUNCTION execute_parallel_operations(
        &self,
        operations: &[MessageOperation],
        context: FlowContext
    ) -> Result<FlowContext> {
        // Clone context for each operation
        operation_futures = operations.iter().map(|op| {
            operation_context = context.clone()
            self.execute_operation(op, operation_context)
        }).collect::<Vec<_>>()
        
        // Execute all operations in parallel
        results = try_join_all(operation_futures).await?
        
        // Merge results back into single context
        merged_context = self.merge_operation_results(context, results)
        
        Ok(merged_context)
    }
}
```

## 6. Implementation Guidelines

> **Cross-Reference**: These implementation guidelines work in conjunction with the [Implementation Configuration](implementation-config.md) document. Refer to the configuration settings and module organization for concrete implementation details.

### 6.1 Error Handling Strategy

```pseudocode
ENUM SystemError {
    Runtime(RuntimeError),
    Supervision(SupervisionError),
    Configuration(ConfigError),
    Resource(ResourceError),
    Network(NetworkError),
    Persistence(PersistenceError)
}

IMPL SystemError {
    FUNCTION severity(&self) -> ErrorSeverity {
        MATCH self {
            SystemError::Runtime(_) => ErrorSeverity::Critical,
            SystemError::Supervision(_) => ErrorSeverity::High,
            SystemError::Configuration(_) => ErrorSeverity::Medium,
            SystemError::Resource(_) => ErrorSeverity::Medium,
            SystemError::Network(_) => ErrorSeverity::Low,
            SystemError::Persistence(_) => ErrorSeverity::High
        }
    }
    
    FUNCTION recovery_strategy(&self) -> RecoveryStrategy {
        MATCH self {
            SystemError::Runtime(_) => RecoveryStrategy::Restart,
            SystemError::Supervision(_) => RecoveryStrategy::Escalate,
            SystemError::Configuration(_) => RecoveryStrategy::Reload,
            SystemError::Resource(_) => RecoveryStrategy::Retry,
            SystemError::Network(_) => RecoveryStrategy::CircuitBreaker,
            SystemError::Persistence(_) => RecoveryStrategy::Failover
        }
    }
}
```

### 6.2 Testing Framework

```pseudocode
STRUCT SystemTestHarness {
    mock_runtime: MockRuntime,
    test_supervision_tree: TestSupervisionTree,
    test_event_bus: TestEventBus,
    assertion_framework: AssertionFramework
}

IMPL SystemTestHarness {
    ASYNC FUNCTION test_component_failure_recovery<C: Component>(&self, component: C) -> TestResult {
        // Inject failure
        self.mock_runtime.inject_failure(component.id(), FailureType::Crash).await
        
        // Verify supervision response
        recovery_event = self.test_event_bus.wait_for_event(EventType::ComponentRecovery, TIMEOUT_DURATION).await?
        
        // Assert component was restarted
        ASSERT!(recovery_event.component_id == component.id())
        ASSERT!(recovery_event.action == RecoveryAction::Restart)
        
        // Verify component is healthy after restart
        health_status = component.health_check().await?
        ASSERT!(health_status == HealthStatus::Healthy)
        
        RETURN TestResult::Passed
    }
}
```

### 6.3 Critical Anti-Patterns to Avoid

#### 6.3.1 Uncontrolled Agent Spawning
```pseudocode
// ❌ BAD: Unlimited spawning without resource bounds
ASYNC FUNCTION handle_task_badly(task: Task) {
    FOR subtask IN task.decompose() {
        spawn_agent(subtask) // No limits! Can exhaust resources
    }
}

// ✅ GOOD: Resource-bounded spawning with limits
STRUCT SpawnController {
    max_agents: usize,
    active: Arc<AtomicUsize>,
    
    ASYNC FUNCTION spawn_bounded(&self, role: AgentRole) -> Result<Agent> {
        IF self.active.load(Ordering::SeqCst) >= self.max_agents {
            RETURN Err("Agent limit reached")
        }
        // Spawn with cleanup on drop
        RETURN Ok(BoundedAgent::new(role, self.active.clone()))
    }
}
```

#### 6.3.2 Context Overflow
```pseudocode
// ❌ BAD: Accumulating unlimited context memory
STRUCT NaiveAgent {
    context: Vec<Message>, // Grows forever, causing memory issues
}

// ✅ GOOD: Windowed context with periodic summarization
STRUCT SmartAgent {
    recent_context: VecDeque<Message>,
    context_summary: Summary,
    max_context_size: usize,
    
    FUNCTION add_context(&mut self, msg: Message) {
        self.recent_context.push_back(msg)
        IF self.recent_context.len() > self.max_context_size {
            self.summarize_old_context()
        }
    }
}
```

#### 6.3.3 Synchronous Tool Blocking
```pseudocode
// ❌ BAD: Blocking tool calls that freeze the runtime
IMPL Tool FOR WebSearch {
    ASYNC FUNCTION execute(&self, query: Value) -> Result<Value> {
        results = reqwest::blocking::get(url)? // Blocks entire thread!
        RETURN Ok(results.into())
    }
}

// ✅ GOOD: Truly async tools with timeouts
IMPL Tool FOR AsyncWebSearch {
    ASYNC FUNCTION execute(&self, query: Value) -> Result<Value> {
        client = reqwest::Client::new()
        
        RETURN tokio::time::timeout(
            Duration::from_secs(30),
            client.get(url).send()
        ).await??
    }
}
```

#### 6.3.4 Monolithic Supervisor
```pseudocode
// ❌ BAD: Single supervisor managing all agents directly
// This creates a bottleneck and single point of failure

// ✅ GOOD: Hierarchical supervisors with domain-specific delegation
// Distribute supervision responsibility across multiple levels
```

#### 6.3.5 Static Role Assignment
```pseudocode
// ❌ BAD: Fixed teams for all projects regardless of needs
// Wastes resources and limits flexibility

// ✅ GOOD: Dynamic team composition based on task analysis
// Spawn only the agents needed for each specific project
```

## 7. Extension Mechanisms

> **Implementation Context**: Extension mechanisms are implemented within the module structure defined in [Implementation Configuration](implementation-config.md#module-organization-structure). See the `async_patterns/middleware.rs` module for concrete implementations.

### 7.1 Middleware Pattern

```pseudocode
TRAIT AgentMiddleware: Send + Sync {
    ASYNC FUNCTION before_process(&self, msg: &Message) -> Result<()>
    ASYNC FUNCTION after_process(&self, msg: &Message, result: &Value) -> Result<()>
}

STRUCT Agent {
    middleware: Vec<Box<dyn AgentMiddleware>>,
    core_processor: AgentProcessor,
    
    ASYNC FUNCTION process(&self, msg: Message) -> Result<Value> {
        // Execute before hooks
        FOR mw IN &self.middleware {
            mw.before_process(&msg).await?
        }
        
        // Core processing
        result = self.core_processor.process(msg).await?
        
        // Execute after hooks
        FOR mw IN &self.middleware {
            mw.after_process(&msg, &result).await?
        }
        
        RETURN Ok(result)
    }
}

// Example middleware implementations
STRUCT LoggingMiddleware { logger: Logger }
STRUCT MetricsMiddleware { metrics: MetricsCollector }
STRUCT AuthMiddleware { auth_service: AuthService }
```

### 7.2 Event Emitter Pattern

```pseudocode
ENUM SystemEvent {
    AgentSpawned(AgentId),
    TaskCompleted(TaskId, Value),
    ToolCalled(AgentId, ToolId),
    Error(AgentId, String),
    ContextSummarized(AgentId, Summary),
    SupervisionDecision(NodeId, SupervisionAction)
}

STRUCT EventBus {
    subscribers: HashMap<TypeId, Vec<Box<dyn EventHandler>>>,
    event_history: CircularBuffer<SystemEvent>,
    
    FUNCTION emit(&self, event: SystemEvent) {
        // Store in history
        self.event_history.push(event.clone())
        
        // Notify subscribers
        IF LET Some(handlers) = self.subscribers.get(&event.type_id()) {
            FOR handler IN handlers {
                handler.handle(event.clone())
            }
        }
    }
    
    FUNCTION subscribe<H: EventHandler>(&mut self, event_type: TypeId, handler: H) {
        self.subscribers
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(Box::new(handler))
    }
}
```

### 7.3 Custom Routing Strategies

```pseudocode
// Extension hook for custom routing logic
TRAIT RoutingStrategy {
    FUNCTION select_recipient(&self, msg: &Message, agents: &[AgentId]) -> AgentId
    FUNCTION priority(&self) -> RoutingPriority
}

// Built-in routing strategies
STRUCT LoadBalancedRouting {
    agent_loads: Arc<RwLock<HashMap<AgentId, f64>>>
}

STRUCT CapabilityBasedRouting {
    agent_capabilities: HashMap<AgentId, Vec<Capability>>
}

STRUCT PriorityRouting {
    priority_queue: BinaryHeap<(Priority, AgentId)>
}

// Allow custom routing strategy registration
IMPL MessageBus {
    FUNCTION register_routing_strategy(&mut self, name: String, strategy: Box<dyn RoutingStrategy>) {
        self.routing_strategies.insert(name, strategy)
    }
}
```

---

## Navigation

- **Previous**: [System Architecture Overview](system-architecture.md)
- **Next**: [Implementation Configuration](implementation-config.md)
- **Related**: 
  - [Implementation Configuration](implementation-config.md) - Agent configuration and module organization
  - [Integration Patterns](./integration-patterns.md) - Error handling and event patterns
  - [Transport Protocols](../transport/) - Communication layer specifications
  - [Data Management](../data-management/) - Data handling and persistence
  - [Security Framework](../security/) - Security protocols and implementation

---

*System Integration Patterns & Implementation - Part of the Mister Smith AI Agent Framework*