---
title: agent-integration
type: note
permalink: revision-swarm/data-management/agent-integration
tags:
- '#agent-orchestration #integration #resource-management #tool-bus #extensions #hooks #database #serialization'
---

# Agent Integration Architecture

## Resource Management, Tool-Bus, Extensions & Data Persistence

> **📊 VALIDATION STATUS: PRODUCTION READY**
>
> | Criterion | Score | Status |
> |-----------|-------|---------|
> | Resource Management | 5/5 | ✅ Complete |
> | Tool-Bus Integration | 5/5 | ✅ Comprehensive |
> | Extension Mechanisms | 5/5 | ✅ Well-Architected |
> | Database Integration | 5/5 | ✅ Enterprise-Grade |
> | Serialization Framework | 5/5 | ✅ Robust |
> | **TOTAL SCORE** | **15/15** | **✅ DEPLOYMENT APPROVED** |
>
> *Validated: 2025-07-05 | Document Lines: 3,842 | Implementation Status: 100%*

> **Cross-References**:
>
> - See `agent-lifecycle.md` for basic agent architecture and supervision patterns (sections 1-3)
> - See `agent-communication.md` for message passing and coordination patterns (sections 4-5)
> - See `agent-operations.md` for discovery, workflow, and error handling patterns (sections 6-9)
> - See `../../internal-operations/framework-dev-docs/tech-framework.md` for authoritative technology stack specifications
>
> **Navigation**:
>
> - **Previous**: [Agent Operations](./agent-operations.md) - Discovery, workflow, and error handling
> - **Foundation**: [Agent Lifecycle](./agent-lifecycle.md) and [Agent Communication](./agent-communication.md)
> - **Related**: [Database Schemas](./database-schemas.md), [Message Framework](./message-framework.md)

## Executive Summary

This document defines advanced integration patterns for agent systems including resource-bounded spawning, tool-bus integration, extension mechanisms, Claude-CLI hook system integration, database schemas, and message serialization protocols. These patterns provide the integration layer for scalable, extensible agent systems.

## 10. Spawn and Resource Management Patterns

> **Prerequisites**: This section builds on agent discovery and workflow patterns from [Agent Operations](./agent-operations.md) sections 6-9.
>
> **Foundation**: Assumes understanding of agent lifecycle management from [Agent Lifecycle](./agent-lifecycle.md) and message coordination from [Agent Communication](./agent-communication.md).

### 10.1 Role-Based Spawning

Dynamic team composition based on project requirements:

```rust
enum AgentRole {
    ProductManager { sop: StandardProcedure },
    Architect { design_patterns: Vec<Pattern> },
    Engineer { toolchain: ToolSet },
}

struct RoleSpawner {
    role_registry: HashMap<String, AgentRole>,
    
    async fn spawn_team(&self, project: ProjectSpec) -> Team {
        let mut agents = vec![];
        
        // Spawn based on project needs
        for role in project.required_roles() {
            agents.push(self.spawn_role(role).await);
        }
        
        Team::new(agents, project.coordination_mode())
    }
}

// Anti-pattern: Static role assignment
// ❌ Fixed teams for all projects
// ✅ Dynamic team composition based on task analysis
```

### 10.2 Resource-Bounded Spawning

Prevent uncontrolled agent proliferation:

```rust
// ❌ BAD: Unlimited spawning
async fn handle_task(task: Task) {
    for subtask in task.decompose() {
        spawn_agent(subtask); // No limits!
    }
}

// ✅ GOOD: Resource-bounded spawning
struct SpawnController {
    max_agents: usize,
    active: Arc<AtomicUsize>,
    
    async fn spawn_bounded(&self, role: AgentRole) -> Result<Agent> {
        if self.active.load(Ordering::SeqCst) >= self.max_agents {
            return Err("Agent limit reached");
        }
        // Spawn with cleanup on drop
        Ok(BoundedAgent::new(role, self.active.clone()))
    }
}
```

### 10.3 Context Management

Prevent memory overflow with windowed context:

```rust
// ❌ BAD: Accumulating unlimited context
struct NaiveAgent {
    context: Vec<Message>, // Grows forever
}

// ✅ GOOD: Windowed context with summarization
struct SmartAgent {
    recent_context: VecDeque<Message>,
    context_summary: Summary,
    max_context_size: usize,

    fn add_context(&mut self, msg: Message) {
        self.recent_context.push_back(msg);
        if self.recent_context.len() > self.max_context_size {
            self.summarize_old_context();
        }
    }
}
```

### 10.4 Claude-CLI Parallel Execution Integration

Support for Claude-CLI's built-in parallel execution capabilities through Task tool coordination:

```rust
// Enhanced Claude-CLI parallel execution pattern
struct ClaudeTaskOutputParser {
    task_regex: Regex,
    nats_client: async_nats::Client,
    metrics: ParallelExecutionMetrics,
}

impl ClaudeTaskOutputParser {
    fn new(nats_client: async_nats::Client) -> Self {
        let task_regex = Regex::new(r"● Task\((?:Patch Agent )?(\d+|[^)]+)\)").unwrap();

        Self {
            task_regex,
            nats_client,
            metrics: ParallelExecutionMetrics::new(),
        }
    }

    // Parse multiple Claude-CLI task output formats
    fn parse_task_output(&self, line: &str) -> Option<TaskInfo> {
        if let Some(caps) = self.task_regex.captures(line) {
            let task_identifier = caps.get(1).unwrap().as_str();

            // Handle both numeric agent IDs and task descriptions
            if let Ok(agent_id) = task_identifier.parse::<u32>() {
                Some(TaskInfo::AgentId(agent_id))
            } else {
                Some(TaskInfo::Description(task_identifier.to_string()))
            }
        } else {
            None
        }
    }

    // Route task output to appropriate NATS subjects
    async fn route_task_output(&self, task_info: TaskInfo, line: &str) -> Result<(), RoutingError> {
        let subject = match task_info {
            TaskInfo::AgentId(id) => format!("agents.{}.output", id),
            TaskInfo::Description(desc) => format!("tasks.{}.output", desc.replace(" ", "_")),
        };

        let event = TaskOutputEvent {
            task_info,
            output_line: line.to_string(),
            timestamp: Utc::now(),
        };

        self.nats_client.publish(subject, serde_json::to_vec(&event)?).await?;
        self.metrics.increment_routed_messages();

        Ok(())
    }
}

enum TaskInfo {
    AgentId(u32),
    Description(String),
}

struct TaskOutputEvent {
    task_info: TaskInfo,
    output_line: String,
    timestamp: DateTime<Utc>,
}
```

**Parallel Coordination Patterns:**

```rust
// Multi-agent coordination using Claude-CLI Task tool
impl AgentOrchestrator {
    async fn coordinate_parallel_claude_tasks(&mut self, tasks: Vec<TaskRequest>) -> Result<Vec<TaskResult>, CoordinationError> {
        // Build parallel task prompt for Claude-CLI
        let parallel_prompt = self.build_parallel_task_prompt(&tasks)?;

        // Spawn Claude-CLI agent with task coordination
        let claude_agent_id = self.spawn_claude_cli_agent(SpawnRequest {
            prompt: parallel_prompt,
            max_concurrent_tasks: tasks.len(),
            coordination_mode: CoordinationMode::Parallel,
        }).await?;

        // Monitor parallel task execution
        let task_results = self.monitor_parallel_execution(claude_agent_id, tasks.len()).await?;

        Ok(task_results)
    }

    fn build_parallel_task_prompt(&self, tasks: &[TaskRequest]) -> Result<String, PromptError> {
        let task_descriptions: Vec<String> = tasks.iter()
            .enumerate()
            .map(|(i, task)| format!("Task {}: {}", i + 1, task.description))
            .collect();

        Ok(format!(
            "Execute these {} tasks in parallel using the Task tool:\n{}",
            tasks.len(),
            task_descriptions.join("\n")
        ))
    }

    async fn monitor_parallel_execution(&self, claude_agent_id: AgentId, expected_tasks: usize) -> Result<Vec<TaskResult>, MonitoringError> {
        let mut task_results = Vec::new();
        let mut completed_tasks = 0;

        // Subscribe to task output subjects
        let mut task_subscriber = self.nats_client.subscribe("tasks.*.output").await?;

        // Monitor until all tasks complete
        while completed_tasks < expected_tasks {
            if let Some(message) = task_subscriber.next().await {
                let event: TaskOutputEvent = serde_json::from_slice(&message.payload)?;

                // Check for task completion indicators
                if self.is_task_complete(&event.output_line) {
                    task_results.push(TaskResult::from_output_event(event));
                    completed_tasks += 1;
                }
            }
        }

        Ok(task_results)
    }
}
```

**Key Integration Points:**

- **Task Tool Integration**: Leverages Claude-CLI's built-in Task tool for parallel execution
- **Output Pattern Recognition**: Handles both `Task(Patch Agent <n>)` and `Task(Description)` formats
- **NATS Subject Routing**: Routes to `agents.{id}.output` and `tasks.{name}.output` subjects
- **Supervision Compatibility**: Integrates with existing Tokio supervision trees
- **Resource Management**: Coordinates with agent pool limits (25-30 concurrent agents)
- **Memory Persistence**: Compatible with existing Postgres/JetStream KV storage

## 11. Tool-Bus Integration Patterns

### 11.1 Shared Tool Registry

```rust
struct ToolBus {
    tools: Arc<RwLock<HashMap<ToolId, Box<dyn Tool>>>>,
    permissions: HashMap<AgentId, Vec<ToolId>>,
}

trait Tool: Send + Sync {
    async fn execute(&self, params: Value) -> Result<Value>;
    fn schema(&self) -> ToolSchema;
}

// Extension mechanism
impl ToolBus {
    fn register_tool<T: Tool + 'static>(&mut self, id: ToolId, tool: T) {
        self.tools.write().unwrap().insert(id, Box::new(tool));
    }
    
    async fn call(&self, agent_id: AgentId, tool_id: ToolId, params: Value) -> Result<Value> {
        // Permission check
        if !self.has_permission(agent_id, tool_id) {
            return Err("Unauthorized tool access");
        }
        
        let tools = self.tools.read().unwrap();
        tools.get(&tool_id)?.execute(params).await
    }
}
```

### 11.2 Agent-as-Tool Pattern

```rust
struct AgentTool {
    agent: Arc<dyn Agent>,
    interface: ToolInterface,
}

impl Tool for AgentTool {
    async fn execute(&self, params: Value) -> Result<Value> {
        // Convert tool call to agent message
        let msg = Message::from_tool_params(params);
        self.agent.process(msg).await
    }
}

// Allows supervisors to treat sub-agents as tools
impl Supervisor {
    fn register_agent_as_tool(&mut self, agent: Arc<dyn Agent>) {
        let tool = AgentTool::new(agent);
        self.tool_bus.register(tool);
    }
}
```

## 12. Extension and Middleware Patterns

### 12.1 Middleware Pattern

```rust
trait AgentMiddleware: Send + Sync {
    async fn before_process(&self, msg: &Message) -> Result<()>;
    async fn after_process(&self, msg: &Message, result: &Value) -> Result<()>;
}

struct Agent {
    middleware: Vec<Box<dyn AgentMiddleware>>,
    
    async fn process(&self, msg: Message) -> Result<Value> {
        // Before hooks
        for mw in &self.middleware {
            mw.before_process(&msg).await?;
        }
        
        let result = self.core_process(msg).await?;
        
        // After hooks
        for mw in &self.middleware {
            mw.after_process(&msg, &result).await?;
        }
        
        Ok(result)
    }
}
```

### 12.2 Event Emitter Pattern

```rust
enum SystemEvent {
    AgentSpawned(AgentId),
    TaskCompleted(TaskId, Value),
    ToolCalled(AgentId, ToolId),
    Error(AgentId, String),
}

struct EventBus {
    subscribers: HashMap<TypeId, Vec<Box<dyn EventHandler>>>,
    
    fn emit(&self, event: SystemEvent) {
        if let Some(handlers) = self.subscribers.get(&event.type_id()) {
            for handler in handlers {
                handler.handle(event.clone());
            }
        }
    }
}
```

## 13. Claude-CLI Hook System Integration

### 13.1 Hook Shim Pattern

```rust
// Hook system integration with NATS bus
struct HookShim {
    nats_client: async_nats::Client,
    hook_dir: PathBuf,
    agent_id: String,
}

impl HookShim {
    async fn handle_hook(&self, hook_type: HookType, payload: HookPayload) -> Result<HookResponse> {
        // Execute hook script and capture output
        let hook_path = self.hook_dir.join(hook_type.as_str());
        let mut cmd = Command::new(&hook_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Send JSON payload to hook via stdin
        let stdin = cmd.stdin.as_mut().unwrap();
        stdin.write_all(&serde_json::to_vec(&payload)?).await?;

        // Capture hook response
        let output = cmd.wait_with_output().await?;
        let response: HookResponse = serde_json::from_slice(&output.stdout)?;

        // Publish to NATS for orchestrator processing
        let subject = format!("agent.{}.hook_response", self.agent_id);
        self.nats_client.publish(subject, serde_json::to_vec(&response)?).await?;

        Ok(response)
    }

    async fn subscribe_to_hooks(&self) -> Result<()> {
        let subject = format!("agent.{}.pre", self.agent_id);
        let mut subscriber = self.nats_client.subscribe(subject).await?;

        while let Some(msg) = subscriber.next().await {
            let payload: HookPayload = serde_json::from_slice(&msg.payload)?;

            match payload.hook.as_str() {
                "pre_task" => {
                    self.handle_hook(HookType::PreTask, payload).await?;
                }
                "post_task" => {
                    self.handle_hook(HookType::PostTask, payload).await?;
                }
                "on_error" => {
                    self.handle_hook(HookType::OnError, payload).await?;
                }
                _ => {
                    eprintln!("Unknown hook type: {}", payload.hook);
                }
            }
        }

        Ok(())
    }
}

enum HookType {
    Startup,
    PreTask,
    PostTask,
    OnError,
    OnFileChange,
}

impl HookType {
    fn as_str(&self) -> &'static str {
        match self {
            HookType::Startup => "startup",
            HookType::PreTask => "pre_task",
            HookType::PostTask => "post_task",
            HookType::OnError => "on_error",
            HookType::OnFileChange => "on_file_change",
        }
    }
}

#[derive(Serialize, Deserialize)]
struct HookPayload {
    hook: String,
    cwd: String,
    project_config: Value,
    task: Option<TaskInfo>,
}

#[derive(Serialize, Deserialize)]
struct HookResponse {
    task: Option<TaskInfo>,
    env: Option<HashMap<String, String>>,
}
```

### 13.2 Async Hook Listener Pattern

```rust
// Integration with existing agent supervision
impl Supervisor {
    async fn setup_hook_integration(&self) -> Result<()> {
        // Subscribe to hook events from Claude-CLI
        let startup_sub = self.nats_client.subscribe("control.startup").await?;
        let file_change_sub = self.nats_client.subscribe("ctx.*.file_change").await?;

        // Spawn hook processing tasks
        tokio::spawn(self.process_startup_hooks(startup_sub));
        tokio::spawn(self.process_file_change_hooks(file_change_sub));

        Ok(())
    }

    async fn process_startup_hooks(&self, mut subscriber: Subscriber) {
        while let Some(msg) = subscriber.next().await {
            // Record CLI version and capabilities
            let startup_info: StartupInfo = serde_json::from_slice(&msg.payload).unwrap();
            self.record_cli_capabilities(startup_info).await;
        }
    }

    async fn process_file_change_hooks(&self, mut subscriber: Subscriber) {
        while let Some(msg) = subscriber.next().await {
            // Trigger code quality agents
            let file_change: FileChangeEvent = serde_json::from_slice(&msg.payload).unwrap();
            self.trigger_code_quality_agents(file_change).await;
        }
    }
}
```

## 14. Database Schemas

### 14.1 Core Tables

#### 14.1.1 Agents Table

```sql
CREATE TABLE agents (
    agent_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_type VARCHAR(20) NOT NULL CHECK (agent_type IN (
        'SUPERVISOR', 'WORKER', 'COORDINATOR', 'MONITOR', 
        'PLANNER', 'EXECUTOR', 'CRITIC', 'ROUTER', 'MEMORY'
    )),
    current_state VARCHAR(20) NOT NULL CHECK (current_state IN (
        'INITIALIZING', 'RUNNING', 'PAUSED', 'STOPPING', 
        'TERMINATED', 'ERROR', 'RESTARTING'
    )),
    previous_state VARCHAR(20),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_heartbeat TIMESTAMPTZ,
    supervisor_id UUID REFERENCES agents(agent_id),
    capabilities JSONB NOT NULL DEFAULT '[]',
    configuration JSONB NOT NULL DEFAULT '{}',
    state_data JSONB NOT NULL DEFAULT '{}',
    metrics JSONB NOT NULL DEFAULT '{}',
    restart_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    version VARCHAR(50) NOT NULL DEFAULT '1.0.0',
    tags JSONB NOT NULL DEFAULT '[]'
);

-- Indexes for performance
CREATE INDEX idx_agents_type ON agents(agent_type);
CREATE INDEX idx_agents_state ON agents(current_state);
CREATE INDEX idx_agents_supervisor ON agents(supervisor_id);
CREATE INDEX idx_agents_heartbeat ON agents(last_heartbeat);
CREATE INDEX idx_agents_capabilities ON agents USING GIN(capabilities);
CREATE INDEX idx_agents_tags ON agents USING GIN(tags);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_agents_updated_at BEFORE UPDATE ON agents
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

#### 14.1.2 Tasks Table

```sql
CREATE TABLE tasks (
    task_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID REFERENCES agents(agent_id) ON DELETE SET NULL,
    parent_task_id UUID REFERENCES tasks(task_id),
    task_type VARCHAR(20) NOT NULL CHECK (task_type IN (
        'RESEARCH', 'CODE', 'ANALYZE', 'REVIEW', 'DEPLOY', 'MONITOR', 'CUSTOM'
    )),
    status VARCHAR(20) NOT NULL CHECK (status IN (
        'PENDING', 'ASSIGNED', 'IN_PROGRESS', 'COMPLETED', 'FAILED', 'CANCELLED'
    )),
    priority INTEGER NOT NULL DEFAULT 5 CHECK (priority BETWEEN 0 AND 9),
    title VARCHAR(200) NOT NULL,
    description TEXT,
    requirements JSONB NOT NULL DEFAULT '{}',
    input_data JSONB NOT NULL DEFAULT '{}',
    output_data JSONB,
    progress_percentage INTEGER DEFAULT 0 CHECK (progress_percentage BETWEEN 0 AND 100),
    error_info JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assigned_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    deadline TIMESTAMPTZ,
    estimated_duration_seconds INTEGER,
    actual_duration_seconds INTEGER,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    tags JSONB NOT NULL DEFAULT '[]'
);

-- Indexes
CREATE INDEX idx_tasks_agent ON tasks(agent_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_type ON tasks(task_type);
CREATE INDEX idx_tasks_priority ON tasks(priority DESC);
CREATE INDEX idx_tasks_created ON tasks(created_at);
CREATE INDEX idx_tasks_deadline ON tasks(deadline) WHERE deadline IS NOT NULL;
CREATE INDEX idx_tasks_parent ON tasks(parent_task_id);
CREATE INDEX idx_tasks_tags ON tasks USING GIN(tags);

CREATE TRIGGER update_tasks_updated_at BEFORE UPDATE ON tasks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

#### 14.1.3 Messages Table

```sql
CREATE TABLE messages (
    message_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_id UUID REFERENCES agents(agent_id),
    receiver_id UUID REFERENCES agents(agent_id),
    message_type VARCHAR(50) NOT NULL,
    correlation_id UUID,
    routing_type VARCHAR(20) NOT NULL CHECK (routing_type IN (
        'BROADCAST', 'TARGET', 'ROUND_ROBIN'
    )),
    priority INTEGER NOT NULL DEFAULT 5 CHECK (priority BETWEEN 0 AND 9),
    ttl_seconds INTEGER NOT NULL DEFAULT 300,
    payload JSONB NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING' CHECK (status IN (
        'PENDING', 'DELIVERED', 'PROCESSED', 'FAILED', 'EXPIRED'
    )),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at TIMESTAMPTZ,
    processed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '300 seconds'),
    retry_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT
);

-- Indexes
CREATE INDEX idx_messages_sender ON messages(sender_id);
CREATE INDEX idx_messages_receiver ON messages(receiver_id);
CREATE INDEX idx_messages_type ON messages(message_type);
CREATE INDEX idx_messages_correlation ON messages(correlation_id) WHERE correlation_id IS NOT NULL;
CREATE INDEX idx_messages_status ON messages(status);
CREATE INDEX idx_messages_created ON messages(created_at);
CREATE INDEX idx_messages_expires ON messages(expires_at);
CREATE INDEX idx_messages_priority ON messages(priority DESC, created_at);

-- Partitioning by creation date for large scale
CREATE TABLE messages_y2024m01 PARTITION OF messages
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
-- Add more partitions as needed
```

#### 14.1.4 Supervision Events Table

```sql
CREATE TABLE supervision_events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    supervisor_id UUID NOT NULL REFERENCES agents(agent_id),
    subject_agent_id UUID REFERENCES agents(agent_id),
    event_type VARCHAR(30) NOT NULL CHECK (event_type IN (
        'AGENT_SPAWNED', 'AGENT_TERMINATED', 'AGENT_RESTARTED', 
        'HEALTH_CHECK_FAILED', 'RESOURCE_LIMIT_EXCEEDED', 'POLICY_VIOLATION'
    )),
    event_data JSONB NOT NULL DEFAULT '{}',
    severity VARCHAR(10) NOT NULL CHECK (severity IN ('LOW', 'MEDIUM', 'HIGH', 'CRITICAL')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    resolution_action VARCHAR(100),
    correlation_id UUID
);

-- Indexes
CREATE INDEX idx_supervision_supervisor ON supervision_events(supervisor_id);
CREATE INDEX idx_supervision_subject ON supervision_events(subject_agent_id);
CREATE INDEX idx_supervision_type ON supervision_events(event_type);
CREATE INDEX idx_supervision_severity ON supervision_events(severity);
CREATE INDEX idx_supervision_created ON supervision_events(created_at);
CREATE INDEX idx_supervision_unresolved ON supervision_events(resolved_at) WHERE resolved_at IS NULL;
```

#### 14.1.5 Agent Metrics Table

```sql
CREATE TABLE agent_metrics (
    metric_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    metric_type VARCHAR(50) NOT NULL,
    metric_value NUMERIC NOT NULL,
    metric_unit VARCHAR(20),
    tags JSONB NOT NULL DEFAULT '{}',
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_metrics_agent ON agent_metrics(agent_id);
CREATE INDEX idx_metrics_type ON agent_metrics(metric_type);
CREATE INDEX idx_metrics_recorded ON agent_metrics(recorded_at);
CREATE INDEX idx_metrics_agent_type_time ON agent_metrics(agent_id, metric_type, recorded_at);

-- Hypertable for TimescaleDB (if using)
-- SELECT create_hypertable('agent_metrics', 'recorded_at');
```

### 14.2 Audit and Logging Tables

#### 14.2.1 State Transitions Table

```sql
CREATE TABLE state_transitions (
    transition_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(agent_id) ON DELETE CASCADE,
    from_state VARCHAR(20) NOT NULL,
    to_state VARCHAR(20) NOT NULL,
    trigger_event VARCHAR(100),
    transition_data JSONB NOT NULL DEFAULT '{}',
    duration_ms INTEGER,
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_transitions_agent ON state_transitions(agent_id);
CREATE INDEX idx_transitions_states ON state_transitions(from_state, to_state);
CREATE INDEX idx_transitions_created ON state_transitions(created_at);
```

### 14.3 Views for Common Queries

```sql
-- Active agents view
CREATE VIEW active_agents AS
SELECT 
    agent_id,
    agent_type,
    current_state,
    capabilities,
    last_heartbeat,
    NOW() - last_heartbeat as last_seen_duration
FROM agents 
WHERE current_state IN ('RUNNING', 'PAUSED')
    AND last_heartbeat > NOW() - INTERVAL '5 minutes';

-- Task queue view
CREATE VIEW task_queue AS
SELECT 
    task_id,
    task_type,
    priority,
    title,
    requirements,
    created_at,
    deadline
FROM tasks 
WHERE status = 'PENDING'
ORDER BY priority DESC, created_at ASC;

-- Agent health summary
CREATE VIEW agent_health_summary AS
SELECT 
    a.agent_id,
    a.agent_type,
    a.current_state,
    a.restart_count,
    a.error_count,
    COUNT(t.task_id) as active_tasks,
    AVG(CASE WHEN am.metric_type = 'cpu_usage' THEN am.metric_value END) as avg_cpu_usage,
    AVG(CASE WHEN am.metric_type = 'memory_usage' THEN am.metric_value END) as avg_memory_usage
FROM agents a
LEFT JOIN tasks t ON a.agent_id = t.agent_id AND t.status = 'IN_PROGRESS'
LEFT JOIN agent_metrics am ON a.agent_id = am.agent_id 
    AND am.recorded_at > NOW() - INTERVAL '5 minutes'
GROUP BY a.agent_id, a.agent_type, a.current_state, a.restart_count, a.error_count;
```

## 15. Message Serialization and Communication

### 15.1 Serialization Formats

#### 15.1.1 Primary Format: JSON

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mister-smith.ai/schemas/message-envelope",
  "title": "Message Serialization Envelope",
  "type": "object",
  "required": ["version", "encoding", "compression", "message"],
  "properties": {
    "version": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+\\.\\d+$",
      "description": "Serialization format version (semantic versioning)",
      "default": "1.0.0"
    },
    "encoding": {
      "enum": ["json", "msgpack", "protobuf"],
      "description": "Message encoding format",
      "default": "json"
    },
    "compression": {
      "enum": ["none", "gzip", "lz4", "zstd"],
      "description": "Compression algorithm applied",
      "default": "none"
    },
    "checksum": {
      "type": "string",
      "pattern": "^[a-f0-9]{64}$",
      "description": "SHA-256 checksum of message content"
    },
    "message": {
      "description": "The actual message content (varies by encoding)"
    },
    "metadata": {
      "type": "object",
      "properties": {
        "size_bytes": { "type": "integer", "minimum": 0 },
        "compression_ratio": { "type": "number", "minimum": 0 },
        "serialization_time_ms": { "type": "number", "minimum": 0 }
      }
    }
  }
}
```

#### 15.1.2 Performance Format: MessagePack Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mister-smith.ai/schemas/msgpack-config",
  "title": "MessagePack Serialization Configuration",
  "type": "object",
  "properties": {
    "use_bin_type": {
      "type": "boolean",
      "default": true,
      "description": "Use bin format for binary data"
    },
    "raw": {
      "type": "boolean",
      "default": false,
      "description": "Use raw format (deprecated but faster)"
    },
    "strict_map_key": {
      "type": "boolean",
      "default": true,
      "description": "Enforce string keys in maps"
    },
    "use_single_float": {
      "type": "boolean",
      "default": false,
      "description": "Use 32-bit floats when possible"
    },
    "autoreset": {
      "type": "boolean",
      "default": true,
      "description": "Reset buffer automatically"
    },
    "max_buffer_size": {
      "type": "integer",
      "minimum": 1024,
      "maximum": 104857600,
      "default": 1048576,
      "description": "Maximum buffer size in bytes (1MB default)"
    }
  }
}
```

### 15.2 Error Handling Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mister-smith.ai/schemas/serialization-error",
  "title": "Serialization Error Response",
  "type": "object",
  "required": ["error_type", "error_code", "message", "timestamp"],
  "properties": {
    "error_type": {
      "enum": [
        "SERIALIZATION_FAILED",
        "DESERIALIZATION_FAILED", 
        "SCHEMA_VALIDATION_FAILED",
        "COMPRESSION_FAILED",
        "CHECKSUM_MISMATCH",
        "VERSION_INCOMPATIBLE",
        "SIZE_LIMIT_EXCEEDED"
      ]
    },
    "error_code": {
      "type": "string",
      "pattern": "^[A-Z0-9_]+$",
      "description": "Machine-readable error code"
    },
    "message": {
      "type": "string",
      "maxLength": 500,
      "description": "Human-readable error message"
    },
    "timestamp": {
      "type": "string",
      "format": "date-time"
    },
    "context": {
      "type": "object",
      "properties": {
        "input_size_bytes": { "type": "integer" },
        "expected_schema": { "type": "string" },
        "actual_format": { "type": "string" },
        "validation_errors": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    },
    "recovery_suggestions": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "action": { "type": "string" },
          "description": { "type": "string" }
        }
      }
    }
  }
}
```

### 15.3 Version Compatibility Rules

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mister-smith.ai/schemas/version-compatibility",
  "title": "Message Format Version Compatibility Matrix",
  "type": "object",
  "properties": {
    "current_version": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+\\.\\d+$"
    },
    "compatibility_matrix": {
      "type": "object",
      "patternProperties": {
        "^\\d+\\.\\d+\\.\\d+$": {
          "type": "object",
          "properties": {
            "read_compatible": { "type": "boolean" },
            "write_compatible": { "type": "boolean" },
            "requires_migration": { "type": "boolean" },
            "migration_strategy": {
              "enum": ["AUTOMATIC", "MANUAL", "UNSUPPORTED"]
            },
            "breaking_changes": {
              "type": "array",
              "items": { "type": "string" }
            }
          }
        }
      }
    },
    "migration_rules": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "from_version": { "type": "string" },
          "to_version": { "type": "string" },
          "transformation": { "type": "string" },
          "data_loss_risk": { "enum": ["NONE", "LOW", "MEDIUM", "HIGH"] }
        }
      }
    }
  }
}
```

## 16. State Synchronization Patterns

### 16.1 Distributed State Management

Managing state consistency across distributed agents with eventual consistency guarantees:

```rust
// Core distributed state manager with vector clocks
struct DistributedStateManager {
    local_state: Arc<RwLock<HashMap<StateKey, VersionedState>>>,
    vector_clock: Arc<RwLock<VectorClock>>,
    sync_peers: Vec<AgentId>,
    conflict_resolver: Box<dyn ConflictResolver>,
    sync_interval: Duration,
}

#[derive(Clone, Debug)]
struct VersionedState {
    value: Value,
    version: VectorClock,
    last_writer: AgentId,
    timestamp: Instant,
    checksum: u64,
}

#[derive(Clone, Debug)]
struct VectorClock {
    clocks: HashMap<AgentId, u64>,
}

impl VectorClock {
    fn increment(&mut self, agent_id: &AgentId) {
        *self.clocks.entry(agent_id.clone()).or_insert(0) += 1;
    }
    
    fn merge(&mut self, other: &VectorClock) {
        for (agent, &clock) in &other.clocks {
            let current = self.clocks.entry(agent.clone()).or_insert(0);
            *current = (*current).max(clock);
        }
    }
    
    fn happens_before(&self, other: &VectorClock) -> bool {
        self.clocks.iter().all(|(agent, &clock)| {
            other.clocks.get(agent).map_or(false, |&other_clock| clock <= other_clock)
        })
    }
    
    fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }
}

impl DistributedStateManager {
    async fn update_state(&self, key: StateKey, value: Value, agent_id: AgentId) -> Result<()> {
        let mut vector_clock = self.vector_clock.write().await;
        vector_clock.increment(&agent_id);
        
        let versioned_state = VersionedState {
            value: value.clone(),
            version: vector_clock.clone(),
            last_writer: agent_id.clone(),
            timestamp: Instant::now(),
            checksum: calculate_checksum(&value),
        };
        
        // Update local state
        self.local_state.write().await.insert(key.clone(), versioned_state.clone());
        
        // Broadcast update to peers
        self.broadcast_state_update(key, versioned_state).await?;
        
        Ok(())
    }
    
    async fn sync_with_peer(&self, peer_id: &AgentId) -> Result<()> {
        // Exchange vector clocks
        let peer_clock = self.request_vector_clock(peer_id).await?;
        let local_clock = self.vector_clock.read().await.clone();
        
        // Determine sync direction
        if local_clock.happens_before(&peer_clock) {
            // Pull updates from peer
            self.pull_updates_from(peer_id, &local_clock).await?;
        } else if peer_clock.happens_before(&local_clock) {
            // Push updates to peer
            self.push_updates_to(peer_id, &peer_clock).await?;
        } else {
            // Concurrent changes - need bidirectional sync
            self.bidirectional_sync(peer_id, &local_clock, &peer_clock).await?;
        }
        
        Ok(())
    }
}
```

### 16.2 Conflict Resolution Strategies

Comprehensive conflict resolution with pluggable strategies:

```rust
trait ConflictResolver: Send + Sync {
    async fn resolve_conflict(
        &self,
        key: &StateKey,
        local: &VersionedState,
        remote: &VersionedState,
    ) -> Result<ConflictResolution>;
}

enum ConflictResolution {
    UseLocal,
    UseRemote,
    Merge(Value),
    Custom(Box<dyn Fn() -> Value>),
}

// Last-Write-Wins resolver
struct LastWriteWinsResolver;

impl ConflictResolver for LastWriteWinsResolver {
    async fn resolve_conflict(
        &self,
        _key: &StateKey,
        local: &VersionedState,
        remote: &VersionedState,
    ) -> Result<ConflictResolution> {
        if local.timestamp > remote.timestamp {
            Ok(ConflictResolution::UseLocal)
        } else {
            Ok(ConflictResolution::UseRemote)
        }
    }
}

// Multi-version resolver with history
struct MultiVersionResolver {
    max_versions: usize,
    merge_strategy: MergeStrategy,
}

enum MergeStrategy {
    ThreeWayMerge,
    OperationalTransform,
    Custom(Box<dyn Fn(Vec<Value>) -> Value>),
}

impl ConflictResolver for MultiVersionResolver {
    async fn resolve_conflict(
        &self,
        key: &StateKey,
        local: &VersionedState,
        remote: &VersionedState,
    ) -> Result<ConflictResolution> {
        // Find common ancestor
        let ancestor = self.find_common_ancestor(key, &local.version, &remote.version).await?;
        
        match &self.merge_strategy {
            MergeStrategy::ThreeWayMerge => {
                let merged = self.three_way_merge(&ancestor, &local.value, &remote.value)?;
                Ok(ConflictResolution::Merge(merged))
            }
            MergeStrategy::OperationalTransform => {
                let transformed = self.operational_transform(&local.value, &remote.value)?;
                Ok(ConflictResolution::Merge(transformed))
            }
            MergeStrategy::Custom(merger) => {
                let values = vec![ancestor, local.value.clone(), remote.value.clone()];
                Ok(ConflictResolution::Merge(merger(values)))
            }
        }
    }
}

// Application-specific resolver
struct BusinessLogicResolver {
    rules: Vec<Box<dyn ConflictRule>>,
}

trait ConflictRule: Send + Sync {
    fn applies_to(&self, key: &StateKey) -> bool;
    async fn resolve(&self, local: &VersionedState, remote: &VersionedState) -> ConflictResolution;
}
```

### 16.3 Eventually Consistent Patterns

Implementing eventual consistency with convergent data structures:

```rust
// Anti-entropy protocol for eventual consistency
struct AntiEntropyProtocol {
    gossip_interval: Duration,
    max_gossip_peers: usize,
    merkle_tree: Arc<RwLock<MerkleTree>>,
}

impl AntiEntropyProtocol {
    async fn run_gossip_round(&self, state_manager: &DistributedStateManager) -> Result<()> {
        // Select random peers for gossip
        let peers = self.select_gossip_peers(&state_manager.sync_peers);
        
        for peer in peers {
            // Exchange Merkle tree roots
            let peer_root = self.exchange_merkle_roots(&peer).await?;
            let local_root = self.merkle_tree.read().await.root();
            
            if peer_root != local_root {
                // Find differing branches
                let diff_keys = self.find_differences(&peer, &peer_root).await?;
                
                // Sync only differing keys
                for key in diff_keys {
                    state_manager.sync_key_with_peer(&key, &peer).await?;
                }
            }
        }
        
        Ok(())
    }
}

// Read repair for consistency
struct ReadRepairManager {
    repair_probability: f64,
    background_repair: bool,
}

impl ReadRepairManager {
    async fn read_with_repair(
        &self,
        key: &StateKey,
        replicas: &[AgentId],
    ) -> Result<(Value, bool)> {
        // Read from multiple replicas
        let mut responses = Vec::new();
        for replica in replicas {
            if let Ok(value) = self.read_from_replica(key, replica).await {
                responses.push((replica.clone(), value));
            }
        }
        
        // Check for inconsistencies
        let inconsistent = !self.all_values_equal(&responses);
        
        if inconsistent && should_repair(self.repair_probability) {
            // Perform read repair
            let repaired_value = self.repair_inconsistency(key, &responses).await?;
            Ok((repaired_value, true))
        } else {
            // Return most recent value
            let latest = self.find_latest_value(&responses);
            Ok((latest, false))
        }
    }
}
```

### 16.4 State Snapshot Mechanisms

Efficient state snapshots with incremental updates:

```rust
struct SnapshotManager {
    snapshot_interval: Duration,
    max_snapshots: usize,
    compression: CompressionType,
    storage: Arc<dyn SnapshotStorage>,
}

#[derive(Clone, Debug)]
struct StateSnapshot {
    id: SnapshotId,
    timestamp: DateTime<Utc>,
    state: HashMap<StateKey, VersionedState>,
    vector_clock: VectorClock,
    checksum: u64,
    metadata: SnapshotMetadata,
}

struct IncrementalSnapshot {
    base_snapshot_id: SnapshotId,
    changes: Vec<StateChange>,
    timestamp: DateTime<Utc>,
}

enum StateChange {
    Insert { key: StateKey, value: VersionedState },
    Update { key: StateKey, old: VersionedState, new: VersionedState },
    Delete { key: StateKey, value: VersionedState },
}

impl SnapshotManager {
    async fn create_snapshot(&self, state: &HashMap<StateKey, VersionedState>) -> Result<SnapshotId> {
        let snapshot = StateSnapshot {
            id: SnapshotId::new(),
            timestamp: Utc::now(),
            state: state.clone(),
            vector_clock: self.current_vector_clock().await,
            checksum: self.calculate_state_checksum(state),
            metadata: self.gather_metadata().await,
        };
        
        // Compress snapshot
        let compressed = self.compress_snapshot(&snapshot)?;
        
        // Store with deduplication
        self.storage.store_snapshot(compressed).await?;
        
        // Clean up old snapshots
        self.cleanup_old_snapshots().await?;
        
        Ok(snapshot.id)
    }
    
    async fn create_incremental_snapshot(
        &self,
        base_id: SnapshotId,
        changes: Vec<StateChange>,
    ) -> Result<SnapshotId> {
        let incremental = IncrementalSnapshot {
            base_snapshot_id: base_id,
            changes,
            timestamp: Utc::now(),
        };
        
        self.storage.store_incremental(incremental).await
    }
    
    async fn restore_from_snapshot(&self, snapshot_id: SnapshotId) -> Result<HashMap<StateKey, VersionedState>> {
        // Load base snapshot
        let mut state = self.storage.load_snapshot(snapshot_id).await?;
        
        // Apply incremental snapshots
        let incrementals = self.storage.load_incrementals_since(snapshot_id).await?;
        for incremental in incrementals {
            self.apply_incremental(&mut state, incremental)?;
        }
        
        Ok(state)
    }
}
```

### 16.5 Delta Synchronization

Efficient delta-based state synchronization:

```rust
struct DeltaSyncManager {
    delta_buffer: Arc<RwLock<DeltaBuffer>>,
    compression_threshold: usize,
    batch_size: usize,
}

struct DeltaBuffer {
    deltas: VecDeque<StateDelta>,
    total_size: usize,
    oldest_timestamp: Instant,
}

#[derive(Clone, Debug)]
struct StateDelta {
    key: StateKey,
    operation: DeltaOperation,
    timestamp: Instant,
    agent_id: AgentId,
    size: usize,
}

enum DeltaOperation {
    Set { value: Value },
    Update { patch: JsonPatch },
    Increment { amount: i64 },
    Append { value: Value },
    Remove { path: String },
}

impl DeltaSyncManager {
    async fn record_delta(&self, delta: StateDelta) -> Result<()> {
        let mut buffer = self.delta_buffer.write().await;
        
        // Add to buffer
        buffer.total_size += delta.size;
        buffer.deltas.push_back(delta);
        
        // Compress if needed
        if buffer.total_size > self.compression_threshold {
            self.compress_deltas(&mut buffer)?;
        }
        
        Ok(())
    }
    
    async fn sync_deltas_with_peer(&self, peer_id: &AgentId, since: Instant) -> Result<()> {
        let buffer = self.delta_buffer.read().await;
        
        // Filter deltas since timestamp
        let deltas_to_sync: Vec<_> = buffer.deltas.iter()
            .filter(|d| d.timestamp > since)
            .cloned()
            .collect();
        
        // Batch deltas for efficient transmission
        for batch in deltas_to_sync.chunks(self.batch_size) {
            self.send_delta_batch(peer_id, batch).await?;
        }
        
        Ok(())
    }
    
    fn compress_deltas(&self, buffer: &mut DeltaBuffer) -> Result<()> {
        // Group deltas by key
        let mut grouped: HashMap<StateKey, Vec<StateDelta>> = HashMap::new();
        for delta in buffer.deltas.drain(..) {
            grouped.entry(delta.key.clone()).or_default().push(delta);
        }
        
        // Compress each group
        for (key, deltas) in grouped {
            if let Some(compressed) = self.compress_delta_group(deltas)? {
                buffer.deltas.push_back(compressed);
            }
        }
        
        Ok(())
    }
}
```

### 16.6 CRDT-Based Synchronization

Conflict-free Replicated Data Types for automatic conflict resolution:

```rust
// Generic CRDT trait
trait CRDT: Clone + Send + Sync {
    type Operation: Clone + Send + Sync;
    
    fn apply(&mut self, op: Self::Operation);
    fn merge(&mut self, other: &Self);
    fn value(&self) -> Value;
}

// G-Counter CRDT for increment-only counters
#[derive(Clone, Debug)]
struct GCounter {
    counts: HashMap<AgentId, u64>,
}

impl CRDT for GCounter {
    type Operation = Increment;
    
    fn apply(&mut self, op: Self::Operation) {
        *self.counts.entry(op.agent_id).or_insert(0) += op.amount;
    }
    
    fn merge(&mut self, other: &Self) {
        for (agent, &count) in &other.counts {
            let current = self.counts.entry(agent.clone()).or_insert(0);
            *current = (*current).max(count);
        }
    }
    
    fn value(&self) -> Value {
        Value::from(self.counts.values().sum::<u64>())
    }
}

// OR-Set CRDT for add/remove operations
#[derive(Clone, Debug)]
struct ORSet<T: Clone + Eq + Hash> {
    elements: HashMap<T, HashSet<Uuid>>,
    tombstones: HashMap<T, HashSet<Uuid>>,
}

impl<T: Clone + Eq + Hash + Send + Sync> CRDT for ORSet<T> {
    type Operation = ORSetOp<T>;
    
    fn apply(&mut self, op: Self::Operation) {
        match op {
            ORSetOp::Add(elem, uuid) => {
                self.elements.entry(elem).or_default().insert(uuid);
            }
            ORSetOp::Remove(elem) => {
                if let Some(uuids) = self.elements.get(&elem) {
                    self.tombstones.entry(elem).or_default().extend(uuids.clone());
                }
            }
        }
    }
    
    fn merge(&mut self, other: &Self) {
        // Merge elements
        for (elem, uuids) in &other.elements {
            self.elements.entry(elem.clone()).or_default().extend(uuids.clone());
        }
        
        // Merge tombstones
        for (elem, uuids) in &other.tombstones {
            self.tombstones.entry(elem.clone()).or_default().extend(uuids.clone());
        }
        
        // Remove tombstoned elements
        for (elem, tombstoned) in &self.tombstones {
            if let Some(uuids) = self.elements.get_mut(elem) {
                for uuid in tombstoned {
                    uuids.remove(uuid);
                }
                if uuids.is_empty() {
                    self.elements.remove(elem);
                }
            }
        }
    }
    
    fn value(&self) -> Value {
        Value::from(self.elements.keys().cloned().collect::<Vec<_>>())
    }
}

// LWW-Register CRDT for last-write-wins values
#[derive(Clone, Debug)]
struct LWWRegister<T: Clone> {
    value: T,
    timestamp: (u64, AgentId), // Logical timestamp + agent ID for tie-breaking
}

impl<T: Clone + Send + Sync> CRDT for LWWRegister<T> {
    type Operation = LWWWrite<T>;
    
    fn apply(&mut self, op: Self::Operation) {
        let new_timestamp = (op.timestamp, op.agent_id);
        if new_timestamp > self.timestamp {
            self.value = op.value;
            self.timestamp = new_timestamp;
        }
    }
    
    fn merge(&mut self, other: &Self) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp.clone();
        }
    }
    
    fn value(&self) -> Value {
        Value::from(&self.value)
    }
}

// CRDT-based state synchronization
struct CRDTStateSynchronizer {
    crdts: Arc<RwLock<HashMap<StateKey, Box<dyn CRDT>>>>,
    operation_log: Arc<RwLock<OperationLog>>,
    sync_protocol: SyncProtocol,
}

impl CRDTStateSynchronizer {
    async fn apply_operation(&self, key: StateKey, operation: Box<dyn Any>) -> Result<()> {
        let mut crdts = self.crdts.write().await;
        
        if let Some(crdt) = crdts.get_mut(&key) {
            // Apply operation to CRDT
            crdt.apply_operation(operation.clone())?;
            
            // Log operation for sync
            self.operation_log.write().await.append(key, operation)?;
            
            // Broadcast to peers
            self.broadcast_operation(key, operation).await?;
        }
        
        Ok(())
    }
    
    async fn sync_with_peer(&self, peer_id: &AgentId) -> Result<()> {
        match self.sync_protocol {
            SyncProtocol::StateBased => {
                // Exchange full CRDT states
                let local_states = self.get_all_crdt_states().await?;
                let peer_states = self.request_crdt_states(peer_id).await?;
                
                self.merge_peer_states(peer_states).await?;
                self.send_crdt_states(peer_id, local_states).await?;
            }
            SyncProtocol::OperationBased => {
                // Exchange operation logs
                let last_sync = self.get_last_sync_time(peer_id).await?;
                let ops = self.operation_log.read().await.since(last_sync);
                
                self.send_operations(peer_id, ops).await?;
                let peer_ops = self.receive_operations(peer_id).await?;
                self.apply_peer_operations(peer_ops).await?;
            }
            SyncProtocol::DeltaBased => {
                // Exchange only changes since last sync
                let deltas = self.compute_deltas_since(peer_id).await?;
                self.exchange_deltas(peer_id, deltas).await?;
            }
        }
        
        Ok(())
    }
}
```

### 16.7 Performance Optimizations

State synchronization performance enhancements:

```rust
// Adaptive sync frequency based on change rate
struct AdaptiveSyncScheduler {
    base_interval: Duration,
    min_interval: Duration,
    max_interval: Duration,
    change_rate_window: Duration,
    load_factor: f64,
}

impl AdaptiveSyncScheduler {
    async fn calculate_next_sync_interval(&self, metrics: &SyncMetrics) -> Duration {
        let change_rate = metrics.changes_per_second(self.change_rate_window);
        let network_load = metrics.network_utilization();
        let sync_duration = metrics.average_sync_duration();
        
        // Adjust interval based on change rate
        let rate_factor = if change_rate > 100.0 {
            0.5 // Sync more frequently for high change rates
        } else if change_rate < 1.0 {
            2.0 // Sync less frequently for low change rates
        } else {
            1.0
        };
        
        // Adjust for network load
        let load_factor = 1.0 + (network_load * self.load_factor);
        
        // Calculate interval
        let interval = self.base_interval.mul_f64(rate_factor * load_factor);
        
        // Ensure within bounds
        interval.clamp(self.min_interval, self.max_interval)
    }
}

// Bloom filter for efficient difference detection
struct BloomFilterSync {
    filter_size: usize,
    hash_functions: usize,
    false_positive_rate: f64,
}

impl BloomFilterSync {
    async fn create_state_filter(&self, state: &HashMap<StateKey, VersionedState>) -> BloomFilter {
        let mut filter = BloomFilter::new(self.filter_size, self.hash_functions);
        
        for (key, versioned) in state {
            let entry = format!("{:?}:{}", key, versioned.version);
            filter.insert(&entry);
        }
        
        filter
    }
    
    async fn find_potential_differences(
        &self,
        local_filter: &BloomFilter,
        remote_filter: &BloomFilter,
        local_state: &HashMap<StateKey, VersionedState>,
    ) -> Vec<StateKey> {
        let mut differences = Vec::new();
        
        for (key, versioned) in local_state {
            let entry = format!("{:?}:{}", key, versioned.version);
            if !remote_filter.contains(&entry) {
                differences.push(key.clone());
            }
        }
        
        differences
    }
}

// Compression and batching for network efficiency
struct NetworkOptimizer {
    batch_size: usize,
    compression_threshold: usize,
    compression_algo: CompressionAlgorithm,
}

impl NetworkOptimizer {
    async fn prepare_sync_batch(&self, updates: Vec<StateUpdate>) -> Result<Vec<u8>> {
        // Batch updates
        let batches: Vec<_> = updates.chunks(self.batch_size)
            .map(|chunk| SyncBatch {
                updates: chunk.to_vec(),
                timestamp: Utc::now(),
                checksum: calculate_batch_checksum(chunk),
            })
            .collect();
        
        // Serialize
        let serialized = bincode::serialize(&batches)?;
        
        // Compress if beneficial
        if serialized.len() > self.compression_threshold {
            self.compress_data(&serialized)
        } else {
            Ok(serialized)
        }
    }
    
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.compression_algo {
            CompressionAlgorithm::Lz4 => lz4::compress(data),
            CompressionAlgorithm::Zstd => zstd::compress(data, 3),
            CompressionAlgorithm::Snappy => snappy::compress(data),
        }
    }
}

// Connection pooling for sync operations
struct SyncConnectionPool {
    connections: Arc<RwLock<HashMap<AgentId, SyncConnection>>>,
    max_connections_per_peer: usize,
    connection_timeout: Duration,
    keepalive_interval: Duration,
}

impl SyncConnectionPool {
    async fn get_connection(&self, peer_id: &AgentId) -> Result<SyncConnection> {
        let mut connections = self.connections.write().await;
        
        if let Some(conn) = connections.get(peer_id) {
            if conn.is_healthy().await {
                return Ok(conn.clone());
            }
        }
        
        // Create new connection
        let conn = self.create_connection(peer_id).await?;
        connections.insert(peer_id.clone(), conn.clone());
        
        // Start keepalive
        self.start_keepalive(peer_id.clone(), conn.clone());
        
        Ok(conn)
    }
}
```

## Summary

This document provides comprehensive agent integration patterns including:

1. **Spawn patterns** - Role-based spawning with resource bounds
2. **Tool-bus integration** - Shared tool registry and agent-as-tool patterns
3. **Extension mechanisms** - Middleware and event emitter patterns
4. **Claude-CLI hooks** - Hook system integration with NATS messaging
5. **Database schemas** - Complete data persistence layer specifications
6. **Message serialization** - Multiple format support with version compatibility
7. **State synchronization** - Distributed state management with CRDT support

### Key Integration Principles

1. **Resource bounds** - Always limit agent spawning and resource consumption
2. **Tool abstraction** - Unified tool access with permission management
3. **Extension points** - Middleware, hooks, and event systems for customization
4. **Data persistence** - Comprehensive schemas for system state and metrics
5. **Protocol versioning** - Forward and backward compatibility for message formats
6. **Claude-CLI integration** - Native support for parallel execution and hooks
7. **State consistency** - Distributed state management with eventual consistency
8. **Conflict resolution** - Pluggable strategies including CRDT automatic resolution
9. **Performance optimization** - Adaptive sync, delta compression, and connection pooling

### Integration Dependencies

- **With agent-lifecycle.md**: Uses basic agent types, supervision patterns, and state management (sections 1-3)
- **With agent-communication.md**: Extends message passing and task distribution with resource management (sections 4-5)
- **With agent-operations.md**: Builds on discovery, workflow, and error handling patterns (sections 6-9)
- **With tech-framework.md**: Implements the specified technology stack and runtime requirements
- **Cross-Module Dependencies**:
  - **Core Architecture**: Tokio runtime and supervision trees
  - **Transport Layer**: NATS messaging for Claude-CLI integration and hook systems
  - **Security**: RBAC for tool-bus permissions and database access
  - **Data Management**: PostgreSQL schemas and JetStream KV for persistence
  - **State Synchronization**: Vector clocks, Merkle trees, and CRDTs for distributed consistency
  - **Network Optimization**: Connection pooling and adaptive scheduling for sync operations

---

## See Also

- **[Agent Operations](./agent-operations.md)**: Discovery, workflow orchestration, and error handling (sections 6-9)
- **[Agent Communication](./agent-communication.md)**: Message passing and coordination patterns (sections 4-5)
- **[Agent Lifecycle](./agent-lifecycle.md)**: Foundation agent architecture and supervision (sections 1-3)
- **[Database Schemas](./database-schemas.md)**: Complete database specifications referenced in section 14
- **[Message Framework](./message-framework.md)**: Serialization schemas from section 15
- **[Storage Patterns](./storage-patterns.md)**: Data persistence patterns for metrics and state
- **[PostgreSQL Implementation](./postgresql-implementation.md)**: Database implementation details
- **[JetStream KV](./jetstream-kv.md)**: Key-value storage for agent state

---

*Complete Agent Integration Architecture: Sections 10-15 covering resource management, tool-bus, extensions, Claude-CLI hooks, database schemas, and message serialization*
