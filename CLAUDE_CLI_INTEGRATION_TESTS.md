# Claude Code CLI Integration Tests Documentation

## Overview

This document provides comprehensive guidance for the Claude Code CLI integration within the MisterSmith Framework. The integration enables orchestration of multiple Claude CLI instances as autonomous agents, with robust process management, error handling, and concurrent operations support.

## Integration Architecture

### Core Components

1. **ProcessManager** (`src/runtime/process.rs`)
   - Manages individual Claude CLI process lifecycles
   - Handles stdin/stdout communication
   - Implements state management (NotStarted, Starting, Running, Paused, Stopping, Terminated, Error)
   - Provides health monitoring and graceful shutdown

2. **ClaudeCommand** (`src/runtime/command.rs`)
   - Builds Claude CLI commands with proper arguments
   - Validates configuration before execution
   - Supports all Claude CLI flags and options

3. **AgentPool** (`src/agent/pool.rs`)
   - Manages concurrent agent limits (25-30 agents recommended)
   - Implements resource allocation via semaphores
   - Tracks metrics and pool status

## Test Coverage Summary

### 1. Basic Claude CLI Integration Tests (`claude_cli.rs`)

- **Process Spawning**: Tests basic Claude CLI process creation with various configurations
- **Command Building**: Validates command construction with MCP, hooks, and tool configurations
- **State Management**: Tests all valid state transitions (Running → Paused → Running → Terminated)
- **I/O Handling**: Validates input sending and output reading with JSON format
- **Graceful Shutdown**: Tests exit command and timeout-based force kill

### 2. Agent Lifecycle Tests (`agent_lifecycle.rs`)

- **Pool Management**: Tests resource limits and concurrent agent tracking
- **Crash Recovery**: Validates recovery from unexpected terminations
- **Work Simulation**: Tests full lifecycle with actual command execution
- **Multiple Agent Coordination**: Tests interaction between multiple agents

### 3. Concurrent Agent Tests (`concurrent_agents.rs`)

- **Concurrent Spawning**: Tests simultaneous agent creation
- **Resource Contention**: Validates behavior when pool capacity is exceeded
- **Load Balancing**: Tests work distribution across agents
- **Performance Monitoring**: Tracks metrics across multiple agents

### 4. Error Scenario Tests (`error_scenarios.rs`)

- **Invalid Configurations**: Tests handling of invalid models and tools
- **Process Failures**: Tests spawn failures and I/O errors
- **State Conflicts**: Tests invalid state transitions
- **Resource Exhaustion**: Tests pool capacity limits

## Claude-Specific Considerations

### 1. Model Selection
```rust
// Supported models (as of 2025-07-07)
const CLAUDE_MODELS: &[&str] = &[
    "claude-opus-4-20250514",      // Most capable
    "claude-sonnet-4-20250514",    // Balanced performance
    "claude-3-7-sonnet-20250219",  // Claude 3.7
    "claude-3-5-haiku-20241022",   // Fast, lightweight
];
```

### 2. Output Format Requirements
- Always use `--output-format json` for structured parsing
- Alternative: `--output-format stream-json` for real-time processing
- Never use `--output-format text` for programmatic integration

### 3. Tool Permissions
```rust
// MCP tools follow specific naming convention
let mcp_tools = vec![
    "mcp__filesystem__read_file",
    "mcp__filesystem__list_directory",
    "mcp__github__create_issue",
];

// Standard tools
let standard_tools = vec![
    "Read",
    "Write",
    "Bash",
    "WebSearch",
];
```

### 4. Session Management
- Use `--max-turns` to limit conversation length
- Implement timeout handling for long-running operations
- Store session IDs for resumption if needed

### 5. Process Lifecycle
```
NotStarted → Starting → Running ↔ Paused
                           ↓
                      Stopping → Terminated
```

## Best Practices

### 1. Resource Management
```rust
// Recommended pool configuration
let pool = AgentPool::new(25); // Max 25-30 agents

// Always clean up on shutdown
pool.shutdown_all().await?;
```

### 2. Error Handling
```rust
// Always handle spawn failures
match process_manager.spawn(&config).await {
    Ok(_) => info!("Claude CLI spawned successfully"),
    Err(ProcessError::SpawnFailed(e)) => {
        error!("Failed to spawn Claude CLI: {}", e);
        // Implement retry logic or fallback
    }
    Err(e) => error!("Unexpected error: {}", e),
}
```

### 3. Input/Output Patterns
```rust
// Send input with error handling
if let Err(e) = process_manager.send_input(&prompt).await {
    match e {
        ProcessError::ProcessPaused => {
            // Resume and retry
            process_manager.resume().await?;
            process_manager.send_input(&prompt).await?;
        }
        ProcessError::NoStdin => {
            // Process terminated, need to respawn
        }
        _ => return Err(e.into()),
    }
}

// Read output with timeout
let output = timeout(
    Duration::from_secs(30),
    process_manager.read_output_line()
).await??;
```

### 4. Health Monitoring
```rust
// Implement periodic health checks
tokio::spawn(async move {
    loop {
        sleep(Duration::from_secs(30)).await;
        if !process_manager.is_healthy().await.unwrap_or(false) {
            warn!("Agent unhealthy, initiating recovery");
            // Implement recovery logic
        }
    }
});
```

## Troubleshooting Guide

### Common Issues and Solutions

1. **Claude CLI Not Found**
   - Ensure `claude` is in PATH
   - Verify installation: `npm install -g @anthropic-ai/claude-code`
   - Check Node.js version (requires 18+)

2. **Process Spawn Failures**
   - Check system resource limits
   - Verify no conflicting Claude instances
   - Review system logs for permission issues

3. **Output Parsing Errors**
   - Ensure `--output-format json` is set
   - Handle partial JSON in streaming mode
   - Implement robust JSON parsing with error recovery

4. **Timeout Issues**
   - Increase timeout values for complex prompts
   - Implement retry logic with exponential backoff
   - Consider using streaming output for long responses

5. **Resource Exhaustion**
   - Monitor system memory and CPU
   - Implement proper cleanup in error paths
   - Use smaller agent pools on resource-constrained systems

## Performance Tuning

### 1. Optimal Pool Size
```rust
// Calculate based on system resources
let cpu_cores = num_cpus::get();
let recommended_pool_size = (cpu_cores * 2).min(30);
```

### 2. Timeout Configuration
```rust
// Adjust based on workload
let config = AgentConfig {
    timeout_seconds: match complexity {
        Low => 30,
        Medium => 60,
        High => 120,
        VeryHigh => 300,
    },
    // ...
};
```

### 3. Memory Management
- Each Claude CLI instance uses ~200-500MB RAM
- Implement memory monitoring
- Set system limits: `ulimit -m 512000` (per process)

### 4. Concurrent Operations
```rust
// Use semaphores for rate limiting
let rate_limiter = Arc::new(Semaphore::new(5)); // Max 5 concurrent operations

// In spawn logic
let _permit = rate_limiter.acquire().await?;
```

## Integration Patterns

### 1. Hook Bridge Integration
```rust
// Claude hooks map directly to NATS subjects
const HOOK_MAPPINGS: &[(&str, &str)] = &[
    ("startup", "control.startup"),
    ("pre_task", "agent.{id}.pre"),
    ("post_task", "agent.{id}.post"),
    ("on_error", "agent.{id}.error"),
    ("on_file_change", "ctx.{id}.file_change"),
];
```

### 2. MCP Server Configuration
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "node",
      "args": ["mcp-server-filesystem.js"],
      "env": {
        "ALLOWED_PATHS": "/workspace"
      }
    }
  }
}
```

### 3. Distributed Agent Coordination
```rust
// Use NATS for inter-agent communication
let subject = format!("agent.{}.task", target_agent_id);
nats_client.publish(&subject, task_data).await?;
```

## Security Considerations

1. **Tool Restrictions**
   - Always explicitly list allowed tools
   - Never use `--dangerously-skip-permissions`
   - Implement tool allowlisting at framework level

2. **Process Isolation**
   - Run agents with minimal privileges
   - Use separate working directories per agent
   - Implement filesystem access controls

3. **Input Validation**
   - Sanitize all user inputs before sending to Claude
   - Implement prompt size limits
   - Validate tool parameters

## Monitoring and Observability

### Metrics to Track
- Agent spawn/termination rates
- Average response times
- Error rates by type
- Resource utilization per agent
- Pool saturation metrics

### Logging Best Practices
```rust
use tracing::{info, warn, error, debug};

// Structured logging with context
info!(
    agent_id = %agent_id,
    model = %config.model,
    duration_ms = elapsed.as_millis(),
    "Task completed successfully"
);
```

## Future Enhancements

1. **Advanced Features**
   - Session persistence and recovery
   - Dynamic model selection based on task complexity
   - Automatic retry with fallback models
   - Cost optimization through model routing

2. **Integration Improvements**
   - Native Claude API integration alongside CLI
   - WebSocket support for real-time communication
   - Enhanced MCP server management
   - Distributed tracing support

## Conclusion

The Claude Code CLI integration in MisterSmith provides a robust foundation for multi-agent orchestration. By following these guidelines and best practices, developers can build reliable, scalable systems that leverage Claude's capabilities effectively.

For additional support or to report issues, please refer to the main MisterSmith documentation or open an issue in the project repository.