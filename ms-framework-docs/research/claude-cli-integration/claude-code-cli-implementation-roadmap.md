# Claude Code CLI Implementation Roadmap

## Technical Implementation Strategy for Mister Smith Framework

### Implementation Overview

This roadmap provides a systematic approach to integrating Claude Code CLI capabilities into the Mister Smith multi-agent framework. The implementation is structured in phases that build upon each other while maintaining system stability.

---

## Phase 1: Core CLI Integration Foundation

### Objective

Establish basic Claude CLI process management and NATS integration.

### Technical Deliverables

#### 1.1 Claude CLI Controller Implementation

**Location**: `src/claude_cli/controller.rs`

```rust
pub struct ClaudeCliController {
    config: ClaudeCliConfig,
    active_sessions: Arc<RwLock<HashMap<AgentId, ClaudeSession>>>,
    agent_pool: AgentPool,
    nats_client: async_nats::Client,
    resource_manager: ResourceManager,
}

// Core functionality:
// - spawn_agent() - Create new Claude CLI instances
// - terminate_agent() - Graceful shutdown with cleanup
// - get_agent_status() - Session monitoring
// - list_active_agents() - Pool management
```

#### 1.2 Basic Hook Bridge Service

**Location**: `src/claude_cli/hook_bridge.rs`

```rust
pub struct HookBridge {
    nats_client: async_nats::Client,
    hook_configs: Vec<HookConfig>,
    timeout_manager: TimeoutManager,
}

// Core functionality:
// - process_hook_input() - Parse Claude CLI hook JSON
// - determine_nats_subject() - Route to appropriate NATS subjects
// - handle_hook_response() - Process framework responses
```

#### 1.3 Configuration Management

**Location**: `config/claude-cli.toml`

```toml
[claude_cli]
max_concurrent_agents = 25
default_model = "claude-3-5-sonnet-20241022"
api_timeout = 300
hook_timeout = 60
output_format = "stream-json"
```

#### 1.4 Hook Bridge Scripts

**Location**: `scripts/nats-hook-bridge`

```bash
#!/bin/bash
# Bridge Claude CLI hooks to NATS subjects
HOOK_TYPE="$1"
INPUT=$(cat)
AGENT_ID=$(echo "$INPUT" | jq -r '.session_info.agent_id // "unknown"')
SUBJECT="agent.${AGENT_ID}.${HOOK_TYPE}"
echo "$INPUT" | nats pub "$SUBJECT" --stdin
```

### Framework Modifications

#### Update System Architecture

**File**: `ms-framework-docs/core-architecture/system-architecture.md`

Add Claude CLI Controller as new component in architecture diagram and component descriptions.

#### Update Transport Layer

**File**: `ms-framework-docs/transport/nats-transport.md`

Hook message formats already added. Verify integration with existing NATS subject taxonomy.

### Success Criteria

- Claude CLI instances can be spawned and terminated
- Basic hook events are published to NATS subjects
- Configuration system is functional
- Resource limits are enforced

---

## Phase 2: Hook System Integration

### Objective

Complete hook system integration with comprehensive error handling and timeout management.

### Technical Deliverables

#### 2.1 Enhanced Hook Bridge

**Location**: `src/claude_cli/hook_bridge.rs`

```rust
impl HookBridge {
    // Enhanced hook processing with decision control
    pub async fn process_hook_with_decision(&self, hook_input: HookInput) -> Result<HookResponse, HookError> {
        // Parse and validate hook input
        let event = self.parse_and_validate_hook(hook_input)?;
        
        // Publish to NATS and wait for framework response
        let response = self.publish_and_await_response(&event).await?;
        
        // Convert framework response to Claude CLI hook response
        self.convert_to_hook_response(response)
    }
    
    // Timeout management for hook execution
    async fn publish_and_await_response(&self, event: &HookEvent) -> Result<FrameworkResponse, HookError> {
        let subject = self.determine_nats_subject(event);
        let response_subject = format!("{}.response", subject);
        
        // Subscribe to response subject
        let mut response_sub = self.nats_client.subscribe(&response_subject).await?;
        
        // Publish hook event
        self.nats_client.publish(&subject, serde_json::to_vec(event)?).await?;
        
        // Wait for response with timeout
        tokio::time::timeout(
            Duration::from_secs(self.config.hook_timeout),
            response_sub.next()
        ).await??
        .map(|msg| serde_json::from_slice(&msg.payload))
        .transpose()?
        .ok_or(HookError::NoResponse)
    }
}
```

#### 2.2 Hook Configuration Management

**Location**: `.claude/hooks.json`

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
  ]
}
```

#### 2.3 Error Handling and Recovery

**Location**: `src/claude_cli/error_handling.rs`

```rust
pub enum HookError {
    ParseError(serde_json::Error),
    NatsError(async_nats::Error),
    TimeoutError,
    ValidationError(String),
    NoResponse,
}

impl HookBridge {
    async fn handle_hook_error(&self, error: HookError, event: &HookEvent) -> HookResponse {
        // Log error for observability
        self.metrics.increment_hook_errors();
        
        match error {
            HookError::TimeoutError => {
                // Allow execution to continue on timeout
                HookResponse::Continue
            },
            HookError::ValidationError(_) => {
                // Block execution on validation errors
                HookResponse::Block("Validation failed".to_string())
            },
            _ => {
                // Default to continue for other errors
                HookResponse::Continue
            }
        }
    }
}
```

### Framework Modifications

#### Update Observability Framework

**File**: `ms-framework-docs/observability/observability-monitoring-framework.md`

Add hook execution metrics:

- Hook processing latency
- Hook success/failure rates
- Hook timeout occurrences
- NATS message delivery metrics

### Success Criteria

- All 5 hook types (startup, pre_task, post_task, on_error, on_file_change) are functional
- Hook responses can control Claude CLI execution (approve/block/continue)
- Error handling prevents system failures
- Timeout management prevents hanging hooks

---

## Phase 3: Parallel Execution Enhancement

### Objective

Implement robust parallel task coordination and resource management for 25-30 concurrent agents.

### Technical Deliverables

#### 3.1 Task Output Parser

**Location**: `src/claude_cli/task_output_parser.rs`

```rust
pub struct TaskOutputParser {
    task_regex: Regex,
    nats_client: async_nats::Client,
    metrics: ParsingMetrics,
}

impl TaskOutputParser {
    pub async fn process_output_stream(&self, mut stream: Lines<BufReader<ChildStdout>>) -> Result<(), ParseError> {
        while let Some(line) = stream.next_line().await? {
            if let Some(task_info) = self.extract_task_info(&line) {
                self.route_task_output(task_info, &line).await?;
            } else {
                self.route_general_output(&line).await?;
            }
        }
        Ok(())
    }
    
    fn extract_task_info(&self, line: &str) -> Option<TaskInfo> {
        // Handle multiple output formats:
        // "● Task(Patch Agent 1)" -> TaskInfo::AgentId(1)
        // "● Task(Explore backend)" -> TaskInfo::Description("Explore backend")
        self.task_regex.captures(line).map(|caps| {
            let identifier = caps.get(1).unwrap().as_str();
            if let Ok(id) = identifier.parse::<u32>() {
                TaskInfo::AgentId(id)
            } else {
                TaskInfo::Description(identifier.to_string())
            }
        })
    }
}
```

#### 3.2 Agent Pool Management

**Location**: `src/claude_cli/agent_pool.rs`

```rust
pub struct AgentPool {
    max_size: usize,
    active_agents: Arc<RwLock<HashMap<AgentId, AgentPoolEntry>>>,
    spawn_semaphore: Arc<Semaphore>,
    health_checker: HealthChecker,
    metrics: PoolMetrics,
}

impl AgentPool {
    pub async fn register_with_health_check(&self, session: ClaudeSession) -> Result<AgentId, PoolError> {
        // Acquire spawn permit (blocks if at capacity)
        let permit = self.spawn_semaphore.acquire().await?;
        
        // Register agent
        let agent_id = session.agent_id.clone();
        let entry = AgentPoolEntry {
            session,
            registered_at: Instant::now(),
            permit,
            health_status: HealthStatus::Healthy,
        };
        
        self.active_agents.write().await.insert(agent_id.clone(), entry);
        
        // Start health monitoring
        self.health_checker.start_monitoring(agent_id.clone()).await?;
        
        Ok(agent_id)
    }
    
    pub async fn cleanup_unhealthy_agents(&self) -> Result<Vec<AgentId>, PoolError> {
        let unhealthy_agents = self.health_checker.get_unhealthy_agents().await?;
        
        for agent_id in &unhealthy_agents {
            self.force_terminate_agent(agent_id).await?;
        }
        
        Ok(unhealthy_agents)
    }
}
```

#### 3.3 Resource Management

**Location**: `src/claude_cli/resource_manager.rs`

```rust
pub struct ResourceManager {
    memory_monitor: MemoryMonitor,
    cpu_monitor: CpuMonitor,
    network_monitor: NetworkMonitor,
    limits: ResourceLimits,
}

impl ResourceManager {
    pub async fn validate_spawn_request(&self, request: &SpawnRequest) -> Result<(), ResourceError> {
        // Check system resource availability
        let current_usage = self.get_current_usage().await?;
        let projected_usage = self.project_usage_with_new_agent(&current_usage, request)?;
        
        // Validate against limits
        if projected_usage.memory > self.limits.max_memory {
            return Err(ResourceError::MemoryLimitExceeded);
        }
        
        if projected_usage.cpu > self.limits.max_cpu {
            return Err(ResourceError::CpuLimitExceeded);
        }
        
        Ok(())
    }
    
    pub async fn monitor_resource_usage(&self) -> Result<(), ResourceError> {
        // Continuous monitoring loop
        loop {
            let usage = self.get_current_usage().await?;
            
            // Check for resource pressure
            if usage.memory > self.limits.warning_threshold_memory {
                self.trigger_memory_pressure_response().await?;
            }
            
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}
```

### Framework Modifications

#### Update Agent Orchestration

**File**: `ms-framework-docs/data-management/agent-orchestration.md`

Enhanced parallel coordination patterns already added.

#### Update Deployment Architecture

**File**: `ms-framework-docs/deployment/deployment-architecture-specifications.md`

Add scaling patterns for 25-30 concurrent Claude CLI agents:

- Resource allocation strategies
- Load balancing patterns
- Performance optimization guidelines

### Success Criteria

- 25-30 concurrent Claude CLI agents can be sustained
- Task output is correctly parsed and routed
- Resource limits prevent system overload
- Health monitoring detects and recovers from failures

---

## Phase 4: MCP Integration

### Objective

Integrate Model Context Protocol servers for extended tool capabilities.

### Technical Deliverables

#### 4.1 MCP Server Integration

**Location**: `src/claude_cli/mcp_integration.rs`

```rust
pub struct McpIntegration {
    mcp_config: McpConfig,
    server_registry: McpServerRegistry,
    tool_mapper: ToolMapper,
}

impl McpIntegration {
    pub async fn register_mcp_server(&mut self, server_config: McpServerConfig) -> Result<ServerId, McpError> {
        // Start MCP server process
        let server = McpServer::start(server_config).await?;
        
        // Register tools with framework tool registry
        let tools = server.list_tools().await?;
        for tool in tools {
            self.tool_mapper.register_mcp_tool(tool, server.id()).await?;
        }
        
        // Add to server registry
        let server_id = self.server_registry.register(server).await?;
        
        Ok(server_id)
    }
    
    pub async fn handle_mcp_tool_call(&self, tool_name: &str, args: serde_json::Value) -> Result<ToolResult, McpError> {
        // Parse MCP tool name: "mcp__server__tool"
        let (server_name, tool_name) = self.parse_mcp_tool_name(tool_name)?;
        
        // Route to appropriate MCP server
        let server = self.server_registry.get_server(&server_name)?;
        server.call_tool(tool_name, args).await
    }
}
```

#### 4.2 Tool Registry Enhancement

**Location**: `src/tools/tool_registry.rs`

```rust
impl ToolRegistry {
    pub async fn register_mcp_tools(&mut self, server_id: ServerId, tools: Vec<McpTool>) -> Result<(), RegistryError> {
        for tool in tools {
            let tool_id = format!("mcp__{}__{}", server_id, tool.name);
            let registry_entry = ToolRegistryEntry {
                id: tool_id,
                name: tool.name,
                description: tool.description,
                provider: ToolProvider::Mcp(server_id),
                schema: tool.input_schema,
            };
            
            self.tools.insert(registry_entry.id.clone(), registry_entry);
        }
        
        Ok(())
    }
}
```

### Framework Modifications

#### Create MCP Integration Specifications

**File**: `ms-framework-docs/integration/mcp-integration-specifications.md`

New document defining:

- MCP server lifecycle management
- Tool naming conventions
- Permission system integration
- Error handling patterns

### Success Criteria

- MCP servers can be registered and managed
- MCP tools are available through framework tool registry
- Tool calls are correctly routed to MCP servers
- Permission system controls MCP tool access

---

## Phase 5: Advanced Features and Optimization

### Objective

Implement advanced coordination patterns and performance optimizations.

### Technical Deliverables

#### 5.1 Advanced Coordination Patterns

- Hierarchical task decomposition
- Dynamic load balancing
- Adaptive resource allocation
- Circuit breaker patterns

#### 5.2 Performance Optimization

- Connection pooling for Anthropic API
- Shared memory for common data
- Optimized NATS message routing
- Garbage collection coordination

#### 5.3 Enterprise Features

- Audit logging for all operations
- Role-based access control
- Multi-tenant isolation
- Backup and recovery procedures

### Success Criteria

- System can handle peak loads efficiently
- Advanced coordination patterns are functional
- Enterprise features meet security requirements
- Performance metrics meet target thresholds

---

## Implementation Dependencies

### External Dependencies

- Claude Code CLI binary in system PATH
- Anthropic API key configuration
- NATS server running and accessible
- Rust toolchain with required crates

### Framework Dependencies

- Existing NATS messaging infrastructure
- Tokio supervision tree patterns
- Agent lifecycle management
- Configuration management system

### Resource Requirements

- 8-16GB system memory
- 4-8 CPU cores
- Stable internet connectivity
- 1-2GB storage for logs and configurations

This roadmap provides a systematic approach to Claude Code CLI integration while maintaining system stability and performance throughout the implementation process.
