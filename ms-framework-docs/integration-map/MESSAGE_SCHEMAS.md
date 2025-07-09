# MS-1 Message Schemas

## NATS Message Patterns

### Discovery Messages

#### Discovery Shared Event
**Subject Pattern**: `discoveries.{agent_id}.{discovery_type}`

```json
{
  "agent_id": "agent-123",
  "agent_role": "Analyzer",
  "discovery_type": "Pattern",
  "content": "Identified recurring authentication failures from IP range 192.168.1.0/24",
  "confidence": 0.85,
  "timestamp": "2025-07-09T15:30:00.123Z",
  "related_to": "disc_xyz789"
}
```

**Discovery Types**:
- `Pattern`: Recurring behavior or structure
- `Anomaly`: Deviation from normal behavior
- `Connection`: Relationship between entities
- `Solution`: Proposed resolution
- `Question`: Query requiring investigation
- `Insight`: High-level understanding

### Agent Communication

#### Agent Command
**Subject**: `agents.{agent_id}.commands.{command_type}`

```json
{
  "command_id": "cmd_abc123",
  "command_type": "execute",
  "issued_by": "orchestrator",
  "parameters": {
    "task": "analyze_logs",
    "target": "/var/log/system.log",
    "time_range": {
      "start": "2025-07-09T00:00:00Z",
      "end": "2025-07-09T23:59:59Z"
    }
  },
  "priority": "high",
  "timeout_ms": 30000
}
```

#### Agent Status Update
**Subject**: `agents.{agent_id}.status.update`

```json
{
  "agent_id": "agent-123",
  "previous_state": "idle",
  "new_state": "busy",
  "transition_reason": "task_assigned",
  "timestamp": "2025-07-09T15:30:00Z",
  "metadata": {
    "task_id": "task_456",
    "estimated_duration_ms": 5000
  }
}
```

#### Agent Heartbeat
**Subject**: `agents.{agent_id}.heartbeat`

```json
{
  "agent_id": "agent-123",
  "timestamp": "2025-07-09T15:30:00Z",
  "status": "healthy",
  "metrics": {
    "cpu_usage": 15.5,
    "memory_mb": 128,
    "active_tasks": 1,
    "completed_tasks": 47
  }
}
```

#### Agent Capabilities
**Subject**: `agents.{agent_id}.capabilities`

```json
{
  "agent_id": "agent-123",
  "capabilities": [
    {
      "name": "log_analysis",
      "version": "1.0",
      "supported_formats": ["syslog", "json", "plain"],
      "max_file_size_mb": 100
    },
    {
      "name": "pattern_detection",
      "version": "2.1",
      "algorithms": ["frequency", "anomaly", "correlation"]
    }
  ],
  "constraints": {
    "max_concurrent_tasks": 3,
    "timeout_seconds": 300
  }
}
```

### Task Management

#### Task Assignment
**Subject**: `tasks.{task_id}.assignment`

```json
{
  "task_id": "task_456",
  "assigned_to": "agent-123",
  "assigned_at": "2025-07-09T15:30:00Z",
  "task_definition": {
    "type": "analyze",
    "description": "Analyze authentication logs for anomalies",
    "parameters": {
      "source": "/var/log/auth.log",
      "patterns": ["failed_login", "privilege_escalation"]
    },
    "priority": 7,
    "deadline": "2025-07-09T16:00:00Z"
  }
}
```

#### Task Progress
**Subject**: `tasks.{task_id}.progress`

```json
{
  "task_id": "task_456",
  "agent_id": "agent-123",
  "status": "in_progress",
  "progress_percentage": 45,
  "current_step": "scanning_logs",
  "steps_completed": ["initialization", "file_loading"],
  "estimated_completion": "2025-07-09T15:45:00Z",
  "intermediate_results": {
    "lines_processed": 15000,
    "patterns_found": 3
  }
}
```

#### Task Result
**Subject**: `tasks.{task_id}.result`

```json
{
  "task_id": "task_456",
  "agent_id": "agent-123",
  "status": "completed",
  "completed_at": "2025-07-09T15:45:00Z",
  "duration_ms": 15000,
  "result": {
    "summary": "Found 3 suspicious authentication patterns",
    "findings": [
      {
        "type": "brute_force_attempt",
        "severity": "high",
        "source_ip": "192.168.1.100",
        "attempt_count": 47,
        "time_window": "5 minutes"
      }
    ],
    "recommendations": [
      "Block IP 192.168.1.100",
      "Enable rate limiting on authentication endpoint"
    ]
  },
  "metrics": {
    "lines_analyzed": 50000,
    "patterns_matched": 3,
    "false_positives": 0
  }
}
```

### System Control

#### System Control Command
**Subject**: `system.control.{operation}`

```json
{
  "operation": "scale_agents",
  "parameters": {
    "agent_type": "analyzer",
    "target_count": 5,
    "scaling_strategy": "gradual"
  },
  "issued_by": "admin",
  "reason": "increased load detected"
}
```

#### Health Check Request/Response
**Subject**: `system.health.{component}`

Request:
```json
{
  "component": "agent_pool",
  "check_type": "deep",
  "include_metrics": true
}
```

Response:
```json
{
  "component": "agent_pool",
  "status": "healthy",
  "timestamp": "2025-07-09T15:30:00Z",
  "details": {
    "total_agents": 5,
    "healthy_agents": 5,
    "idle_agents": 2,
    "busy_agents": 3
  },
  "metrics": {
    "average_task_duration_ms": 4500,
    "task_success_rate": 0.98,
    "agent_utilization": 0.6
  }
}
```

### Discovery Patterns

#### Agent Discovery Announcement
**Subject**: `agents.discovery.announce`

```json
{
  "agent_id": "agent-789",
  "agent_role": "Monitor",
  "capabilities": ["metrics_collection", "alerting"],
  "endpoint": "http://agent-789:8080",
  "joined_at": "2025-07-09T15:30:00Z",
  "metadata": {
    "version": "1.2.0",
    "region": "us-west-2"
  }
}
```

#### Discovery Query
**Subject**: `agents.discovery.query`

```json
{
  "query_id": "query_123",
  "requester": "orchestrator",
  "criteria": {
    "capabilities": ["log_analysis"],
    "status": "idle",
    "min_version": "1.0.0"
  },
  "response_subject": "agents.discovery.response.query_123"
}
```

#### Discovery Response
**Subject**: `agents.discovery.response.{query_id}`

```json
{
  "query_id": "query_123",
  "matching_agents": [
    {
      "agent_id": "agent-123",
      "score": 0.95,
      "status": "idle",
      "load": 0.2
    },
    {
      "agent_id": "agent-456",
      "score": 0.88,
      "status": "idle",
      "load": 0.4
    }
  ],
  "total_matches": 2
}
```

### Supervision Events

#### Lifecycle Event
**Subject**: `supervision.lifecycle.{agent_id}`

```json
{
  "event_id": "evt_123",
  "agent_id": "agent-123",
  "event_type": "state_change",
  "previous_state": "Starting",
  "new_state": "Running",
  "timestamp": "2025-07-09T15:30:00Z",
  "metadata": {
    "trigger": "initialization_complete",
    "supervisor_id": "sup_456",
    "related_task_id": null,
    "error_details": null
  }
}
```

#### Health Degradation Event
**Subject**: `supervision.health.{agent_id}`

```json
{
  "event_id": "evt_789",
  "agent_id": "agent-123",
  "severity": "warning",
  "health_score": 0.7,
  "issues": [
    {
      "type": "high_memory_usage",
      "value": 85,
      "threshold": 80,
      "duration_seconds": 300
    }
  ],
  "recommended_actions": ["increase_memory_limit", "restart_agent"]
}
```

### Metric Events

#### Agent Metrics
**Subject**: `agents.{agent_id}.metrics.performance`

```json
{
  "agent_id": "agent-123",
  "timestamp": "2025-07-09T15:30:00Z",
  "interval_seconds": 60,
  "metrics": {
    "cpu": {
      "usage_percent": 25.5,
      "user_time_ms": 15000,
      "system_time_ms": 5000
    },
    "memory": {
      "used_mb": 256,
      "total_mb": 512,
      "gc_count": 12,
      "gc_time_ms": 450
    },
    "tasks": {
      "completed": 5,
      "failed": 0,
      "average_duration_ms": 3200,
      "queue_depth": 2
    }
  }
}
```

### Error Events

#### Agent Error
**Subject**: `agents.{agent_id}.events.error`

```json
{
  "error_id": "err_456",
  "agent_id": "agent-123",
  "timestamp": "2025-07-09T15:30:00Z",
  "severity": "error",
  "error_type": "task_execution_failed",
  "message": "Failed to analyze log file",
  "details": {
    "task_id": "task_789",
    "exception": "FileNotFoundException",
    "stack_trace": "...",
    "recovery_attempted": true,
    "recovery_successful": false
  }
}
```

## Message Envelope Format

All messages can be wrapped in a standard envelope for routing and tracking:

```json
{
  "envelope": {
    "id": "msg_abc123",
    "source": "agent-123",
    "destination": "*",
    "timestamp": "2025-07-09T15:30:00.123Z",
    "correlation_id": "corr_xyz",
    "reply_to": "agent.123.responses",
    "ttl_ms": 60000
  },
  "headers": {
    "content-type": "application/json",
    "encoding": "utf-8",
    "priority": "normal"
  },
  "payload": {
    // Actual message content
  }
}
```

## Compression and Encoding

For large payloads, messages support compression:

```json
{
  "envelope": {
    "id": "msg_large_123",
    "compressed": true,
    "compression": "gzip",
    "original_size": 524288,
    "compressed_size": 65536
  },
  "payload": "H4sIAAAAAAAAA..." // Base64 encoded compressed data
}
```

## Message Versioning

All message types include version information for backward compatibility:

```json
{
  "version": "1.0",
  "schema": "discovery_v1",
  "payload": {
    // Message content
  }
}
```

## Queue Groups

For load balancing, certain subjects use queue groups:

- Agent commands: Queue group `agents.commands`
- Task assignments: Queue group `tasks.assignments`
- System health checks: Queue group `system.health`

This ensures only one subscriber in the group receives each message.