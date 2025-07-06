# Claude Code CLI Integration Plan

## Detailed Implementation Strategy for Mister Smith Framework

### Overview

This document provides a comprehensive integration plan for incorporating Claude Code CLI capabilities into the Mister Smith multi-agent framework. The integration leverages existing framework architecture while adding minimal new components.

---

## Integration Architecture

### 1. Component Integration Map

```
Mister Smith Framework (Rust)
├── NATS Server (existing messaging backbone)
├── Agent Supervisor (existing Tokio-based)
├── Claude CLI Controller (NEW COMPONENT)
│   ├── Agent Pool Manager
│   ├── Hook Bridge Service  
│   └── Output Parser Service
├── Hook Bridge Scripts (NEW COMPONENT)
│   ├── NATS Publishers
│   └── JSON Processors
└── Claude CLI Instances (1-25)
    ├── claude --output-format=stream-json
    ├── Hook Scripts (NATS integration)
    └── MCP Servers (optional)
```

### 2. Data Flow Architecture

```
User Request → Framework → Claude CLI Controller → Claude CLI Instance
                ↓                                        ↓
         NATS Subjects ← Hook Bridge ← Claude Hooks ← Task Execution
                ↓                                        ↓
         Agent Orchestrator ← Output Parser ← Task Output Stream
```

---

## Required Framework Modifications

### 1. New Components to Add

#### A. Claude CLI Controller (`core-architecture/claude-cli-integration.md`)

**Purpose**: Manage Claude CLI instance lifecycle and resource allocation

**Implementation**:

```rust
pub struct ClaudeCliController {
    config: ClaudeCliConfig,
    active_sessions: HashMap<AgentId, ClaudeSession>,
    agent_pool: AgentPool,
    nats_client: async_nats::Client,
    hook_bridge: Arc<HookBridge>,
    metrics: ControllerMetrics,
}

impl ClaudeCliController {
    pub async fn spawn_agent(&mut self, request: SpawnRequest) -> Result<AgentId, SpawnError> {
        // Resource validation
        self.validate_resources()?;
        
        // Build Claude CLI command
        let command = self.build_command(&request)?;
        
        // Spawn process with supervision
        let session = ClaudeSession::spawn(command, &self.nats_client).await?;
        
        // Register with agent pool
        let agent_id = self.agent_pool.register(session).await?;
        
        Ok(agent_id)
    }
    
    pub async fn terminate_agent(&mut self, agent_id: AgentId) -> Result<(), TerminateError> {
        // Graceful shutdown with cleanup
        if let Some(session) = self.active_sessions.remove(&agent_id) {
            session.terminate_gracefully().await?;
            self.agent_pool.unregister(agent_id).await?;
        }
        Ok(())
    }
}
```

#### B. Hook Bridge Service (`nats-transport.md` enhancement)

**Purpose**: Bridge Claude Code hooks to NATS messaging system

**Implementation**:

```rust
pub struct HookBridge {
    nats_client: async_nats::Client,
    hook_configs: Vec<HookConfig>,
    timeout_manager: TimeoutManager,
}

impl HookBridge {
    pub async fn process_hook_event(&self, hook_input: HookInput) -> Result<HookResponse, HookError> {
        // Parse hook JSON input
        let event = self.parse_hook_input(hook_input)?;
        
        // Publish to appropriate NATS subject
        let subject = self.determine_subject(&event);
        self.nats_client.publish(subject, event.to_json()).await?;
        
        // Return appropriate response
        Ok(HookResponse::Continue)
    }
    
    fn determine_subject(&self, event: &HookEvent) -> String {
        match event.hook_type {
            HookType::Startup => "control.startup".to_string(),
            HookType::PreTask => format!("agent.{}.pre", event.agent_id),
            HookType::PostTask => format!("agent.{}.post", event.agent_id),
            HookType::OnError => format!("agent.{}.error", event.agent_id),
            HookType::OnFileChange => format!("ctx.{}.file_change", event.context_id),
        }
    }
}
```

#### C. Task Output Parser (`data-management/task-output-parsing.md`)

**Purpose**: Parse Claude CLI task output and route to NATS subjects

**Implementation**:

```rust
pub struct TaskOutputParser {
    agent_id_extractor: Regex,
    output_router: OutputRouter,
    stream_processor: StreamProcessor,
}

impl TaskOutputParser {
    pub async fn process_output_stream(&self, stream: OutputStream) -> Result<(), ParseError> {
        let mut lines = stream.lines();
        
        while let Some(line) = lines.next().await {
            let line = line?;
            
            // Extract agent ID from task output
            if let Some(agent_id) = self.extract_agent_id(&line) {
                // Route to appropriate NATS subject
                let subject = format!("agents.{}.output", agent_id);
                self.output_router.publish(subject, &line).await?;
            }
        }
        
        Ok(())
    }
    
    fn extract_agent_id(&self, line: &str) -> Option<u32> {
        self.agent_id_extractor
            .captures(line)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }
}
```

### 2. Existing Components to Enhance

#### A. Agent Orchestration (`data-management/agent-orchestration.md`)

**Additions Required**:

```rust
// Add to existing AgentOrchestrator
impl AgentOrchestrator {
    pub async fn spawn_claude_cli_agent(&mut self, request: ClaudeAgentRequest) -> Result<AgentId, OrchestrationError> {
        // Delegate to Claude CLI Controller
        let agent_id = self.claude_cli_controller.spawn_agent(request).await?;
        
        // Register with existing supervision tree
        self.supervision_tree.add_agent(agent_id, AgentType::ClaudeCli).await?;
        
        Ok(agent_id)
    }
    
    pub async fn coordinate_parallel_tasks(&mut self, tasks: Vec<TaskRequest>) -> Result<Vec<TaskResult>, CoordinationError> {
        // Use Claude CLI Task tool for parallel execution
        let task_requests = tasks.into_iter()
            .map(|task| self.build_claude_task_request(task))
            .collect();
            
        // Spawn parallel Claude CLI agents
        let agent_ids = self.spawn_parallel_agents(task_requests).await?;
        
        // Coordinate execution and collect results
        self.coordinate_execution(agent_ids).await
    }
}
```

#### B. Transport Layer (`nats-transport.md`)

**Hook Integration Subjects** (already defined, just document usage):

```
control.startup               # Claude CLI initialization
agent.{id}.pre               # Pre-task hook processing
agent.{id}.post              # Post-task hook processing  
agent.{id}.error             # Error hook handling
agent.{id}.hook_response     # Hook mutation responses
ctx.{gid}.file_change        # File change notifications
```

**New Message Formats**:

```rust
#[derive(Serialize, Deserialize)]
pub struct HookEvent {
    pub hook_type: HookType,
    pub agent_id: String,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub session_info: SessionInfo,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct TaskOutputEvent {
    pub agent_id: u32,
    pub output_line: String,
    pub task_status: TaskStatus,
    pub timestamp: DateTime<Utc>,
}
```

---

## Configuration Management

### 1. Claude CLI Configuration

**File**: `config/claude-cli.toml`

```toml
[claude_cli]
max_concurrent_agents = 25
default_model = "claude-3-5-sonnet-20241022"
api_timeout = 300
retry_attempts = 3
hook_timeout = 60
output_buffer_size = 8192

[claude_cli.spawn_limits]
rate_limit_per_minute = 10
memory_limit_mb = 200
cpu_limit_percent = 50

[claude_cli.hooks]
config_path = ".claude/hooks.json"
enable_nats_bridge = true
hook_execution_timeout = 30

[claude_cli.mcp]
enable_mcp_integration = true
mcp_config_path = ".claude/mcp.json"
```

### 2. Hook Configuration

**File**: `.claude/hooks.json`

```json
{
  "PreToolUse": [
    {
      "matcher": "Task",
      "hooks": [
        {
          "type": "command",
          "command": "nats-hook-bridge pre_task",
          "timeout": 30
        }
      ]
    }
  ],
  "PostToolUse": [
    {
      "matcher": ".*",
      "hooks": [
        {
          "type": "command",
          "command": "nats-hook-bridge post_task",
          "timeout": 30
        }
      ]
    }
  ],
  "Notification": [
    {
      "hooks": [
        {
          "type": "command",
          "command": "nats-hook-bridge notification",
          "timeout": 10
        }
      ]
    }
  ],
  "Stop": [
    {
      "hooks": [
        {
          "type": "command",
          "command": "nats-hook-bridge stop",
          "timeout": 30
        }
      ]
    }
  ],
  "SubagentStop": [
    {
      "hooks": [
        {
          "type": "command",
          "command": "nats-hook-bridge subagent_stop",
          "timeout": 30
        }
      ]
    }
  ]
}
```

### 3. Hook Bridge Scripts

**File**: `scripts/nats-hook-bridge`

```bash
#!/bin/bash
# NATS Hook Bridge Script for Claude Code CLI Integration

HOOK_TYPE="$1"
NATS_URL="${NATS_URL:-nats://localhost:4222}"

# Read JSON input from stdin
INPUT=$(cat)

# Extract agent ID from session info
AGENT_ID=$(echo "$INPUT" | jq -r '.session_info.agent_id // "unknown"')

# Determine NATS subject
case "$HOOK_TYPE" in
    "pre_task")
        SUBJECT="agent.${AGENT_ID}.pre"
        ;;
    "post_task")
        SUBJECT="agent.${AGENT_ID}.post"
        ;;
    "notification")
        SUBJECT="control.notification"
        ;;
    "stop")
        SUBJECT="agent.${AGENT_ID}.stop"
        ;;
    "subagent_stop")
        SUBJECT="agent.${AGENT_ID}.subagent_stop"
        ;;
    *)
        SUBJECT="control.unknown_hook"
        ;;
esac

# Publish to NATS
echo "$INPUT" | nats pub "$SUBJECT" --stdin

# Return success (continue execution)
exit 0
```

---

## Implementation Roadmap

### Phase 1: Core CLI Integration (Weeks 1-2)

**Priority**: Critical
**Dependencies**: None

**Deliverables**:

1. Claude CLI Controller implementation
2. Basic hook bridge for NATS integration
3. Task output parsing and routing
4. Configuration management

**Framework Files to Create/Modify**:

- Create: `core-architecture/claude-cli-integration.md`
- Modify: `core-architecture/system-architecture.md`
- Modify: `nats-transport.md`
- Create: `config/claude-cli.toml`

### Phase 2: Hook System Integration (Weeks 3-4)

**Priority**: High
**Dependencies**: Phase 1

**Deliverables**:

1. Complete hook bridge implementation
2. JSON message format standardization
3. Hook configuration management
4. Error handling and timeout management

**Framework Files to Modify**:

- Enhance: `nats-transport.md`
- Modify: `observability-monitoring-framework.md`
- Create: `scripts/nats-hook-bridge`

### Phase 3: Parallel Execution Enhancement (Weeks 5-6)

**Priority**: High
**Dependencies**: Phase 1, 2

**Deliverables**:

1. Multi-agent coordination patterns
2. Resource pool management
3. Load balancing and work distribution
4. Performance optimization

**Framework Files to Modify**:

- Enhance: `data-management/agent-orchestration.md`
- Modify: `deployment-architecture-specifications.md`
- Enhance: `observability-monitoring-framework.md`

### Phase 4: MCP Integration (Weeks 7-8)

**Priority**: Medium
**Dependencies**: Phase 1

**Deliverables**:

1. MCP server integration
2. Tool registry enhancement
3. Slash command workflow integration
4. Permission system integration

**Framework Files to Create/Modify**:

- Create: `integration/mcp-integration-specifications.md`
- Enhance: `data-management/tool-registry.md`
- Modify: `security-framework.md`

### Phase 5: Advanced Features (Weeks 9-10)

**Priority**: Low
**Dependencies**: All previous phases

**Deliverables**:

1. Advanced coordination patterns
2. Performance optimization
3. Enterprise features
4. Monitoring and observability enhancements

---

## Success Metrics

### 1. Performance Metrics

- **Agent Spawn Time**: < 5 seconds per agent
- **Concurrent Agents**: 25-30 agents sustained
- **Memory Usage**: < 6GB total for all agents
- **Hook Latency**: < 100ms for hook processing

### 2. Reliability Metrics

- **Agent Uptime**: > 99% availability
- **Hook Success Rate**: > 99.5% successful hook executions
- **Error Recovery**: < 30 seconds for agent restart
- **Message Delivery**: > 99.9% NATS message delivery

### 3. Integration Metrics

- **API Compatibility**: 100% Claude Code CLI feature coverage
- **Framework Compatibility**: No breaking changes to existing components
- **Configuration Simplicity**: Single configuration file for all settings

This integration plan provides a clear path to incorporate Claude Code CLI capabilities while maintaining the integrity and performance of the existing Mister Smith framework.
