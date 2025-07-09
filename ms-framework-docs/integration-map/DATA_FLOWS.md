# MS-1 Data Flow Documentation

## Overview
This document describes the data flow patterns between components in the MS-1 architecture, illustrating how information moves through the system for various operations.

## 1. Discovery Sharing Flow

### Flow Diagram
```
Agent Process
    ↓ (1) Generate Discovery
Claude Executor
    ↓ (2) Format as JSON
MCP Client Call (share_discovery)
    ↓ (3) HTTP POST /mcp
MCP Server
    ↓ (4) Route to Handler
share_discovery_handler
    ↓ (5) Store in Discovery Store
    ├─→ (6a) Publish to NATS
    │    ↓
    │  NATS Subject: discoveries.{agent_id}.{type}
    │    ↓
    │  All Subscribers
    └─→ (6b) Collaboration Bridge
         ├─→ Context Manager (share context)
         ├─→ Live Orchestrator (if Question type)
         └─→ SSE Broadcaster
              ↓
            UI Clients (real-time updates)
```

### Data Transformations

1. **Agent Discovery** (Claude Output):
```json
{
  "discovery": "Unusual pattern in authentication logs",
  "confidence": 0.85,
  "type": "pattern"
}
```

2. **MCP Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "call_tool",
  "params": {
    "name": "share_discovery",
    "arguments": {
      "agent_id": "agent-123",
      "agent_role": "Analyzer",
      "discovery_type": "pattern",
      "content": "Unusual pattern in authentication logs",
      "confidence": 0.85
    }
  }
}
```

3. **NATS Message**:
```json
{
  "agent_id": "agent-123",
  "agent_role": "Analyzer",
  "discovery_type": "Pattern",
  "content": "Unusual pattern in authentication logs",
  "confidence": 0.85,
  "timestamp": "2025-07-09T15:30:00.123Z",
  "related_to": null
}
```

4. **SSE Event to UI**:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/updated",
  "params": {
    "uri": "discovery://agents/agent-123/discoveries/disc_abc123"
  }
}
```

## 2. Task Execution Flow

### Flow Diagram
```
Task Request (API/UI)
    ↓ (1) POST /api/v1/tasks
REST API Handler
    ↓ (2) Create Task Record
Task Manager
    ↓ (3) Store in PostgreSQL
    ↓ (4) Find Suitable Agent
Agent Discovery (NATS)
    ↓ (5) Query: agents.discovery.query
Agent Pool
    ↓ (6) Response: matching agents
Task Manager
    ↓ (7) Assign Task
    ↓ (8) Publish: tasks.{task_id}.assignment
Selected Agent
    ↓ (9) Receive & Execute
Claude Executor
    ↓ (10) Process with Claude CLI
Agent
    ↓ (11) Publish Progress
    ├─→ tasks.{task_id}.progress
    └─→ (12) Complete & Publish Result
         ↓
       tasks.{task_id}.result
         ↓
    Task Manager
         ↓ (13) Update PostgreSQL
         └─→ (14) Notify Subscribers
```

### Key Data Points

1. **Task Creation**:
```json
{
  "description": "Analyze system logs",
  "priority": 7,
  "required_capabilities": ["log_analysis"],
  "parameters": {
    "source": "/var/log/system.log",
    "time_range": "24h"
  }
}
```

2. **Agent Assignment**:
```json
{
  "task_id": "task_456",
  "assigned_to": "agent-123",
  "assigned_at": "2025-07-09T15:30:00Z"
}
```

3. **Task Result**:
```json
{
  "task_id": "task_456",
  "status": "completed",
  "result": {
    "findings": ["anomaly_1", "pattern_2"],
    "summary": "2 issues found"
  }
}
```

## 3. Real-Time Monitoring Flow

### Flow Diagram
```
System Components
    ↓ (Continuous)
Metrics Collection
    ├─→ Agent Metrics
    │    ↓ agents.{id}.metrics.performance
    ├─→ NATS Metrics
    │    ↓ Internal stats
    └─→ System Metrics
         ↓ OS/Process stats
    
Metrics Aggregator
    ↓ (Every 5s)
API Endpoint (/metrics/system)
    ↓
UI Components
    ├─→ WebSocket Client (bidirectional)
    └─→ React Query (polling)
         ↓
    UI State Updates
         ↓
    Visual Components
```

### Metric Flow Example

1. **Agent Publishes Metrics**:
```bash
Subject: agents.agent-123.metrics.performance
```

2. **Aggregated System Metrics**:
```json
{
  "timestamp": "2025-07-09T15:30:00Z",
  "agents": {
    "total": 5,
    "idle": 2,
    "busy": 3
  },
  "nats": {
    "connections": 5,
    "messages_per_sec": 125
  },
  "system": {
    "cpu_percent": 45.2,
    "memory_mb": 1024
  }
}
```

3. **UI Update via WebSocket**:
```json
{
  "type": "metrics_update",
  "payload": {
    "component": "system",
    "metrics": { /* aggregated data */ }
  }
}
```

## 4. Agent Lifecycle Flow

### Flow Diagram
```
Agent Spawn Request
    ↓ (1) Via API or Orchestrator
Agent Pool Manager
    ↓ (2) Allocate Resources
Process Manager
    ↓ (3) Spawn Claude Process
    ↓ (4) Initialize Connection
Agent
    ↓ (5) Announce: agents.discovery.announce
    ↓ (6) Start Heartbeat
    ├─→ agents.{id}.heartbeat (every 30s)
    └─→ (7) Update Status
         ↓
    PostgreSQL (persist state)
         ↓
    Supervision Tree
         ↓ (8) Monitor Health
    Health Checks
         ↓ (9) If Degraded
    Supervision Decision
         ├─→ Restart Agent
         └─→ Notify Administrators
```

### State Transitions

```
Created → Starting → Running → Busy → Idle → Stopping → Stopped
                ↓                  ↓
              Error ←──────────── Failed
```

## 5. Discovery Subscription Flow

### Flow Diagram
```
UI Client
    ↓ (1) Subscribe Request
MCP Server (subscribe_discoveries)
    ↓ (2) Create Subscription
Discovery Store
    ↓ (3) Register Filters
    └─→ (4) Generate SSE Endpoint
         ↓
UI Client
    ↓ (5) Connect to SSE
SSE Broadcaster
    ↓ (6) Establish Stream
    ↓
NATS Subscriber
    ↓ (7) Listen: discoveries.>
    ↓ (For each discovery)
Filter Check
    ↓ (8) Match Criteria?
    └─→ Yes: Send SSE Event
         ↓
    UI Client (receives event)
```

### Subscription Lifecycle

1. **Initial Subscription**:
```json
{
  "subscriber_agent_id": "ui-client-123",
  "discovery_types": ["pattern", "anomaly"],
  "min_confidence": 0.7
}
```

2. **SSE Connection**:
```http
GET /discoveries/stream?agent_id=ui-client-123
Accept: text/event-stream
```

3. **Filtered Event Delivery**:
```
event: discovery
data: {"jsonrpc":"2.0","method":"notifications/resources/updated","params":{"uri":"discovery://..."}}
```

## 6. Database Persistence Flow

### Flow Diagram
```
Application Events
    ↓
Service Layer
    ├─→ Agent State Changes
    │    ↓
    │  DbPool (connection)
    │    ↓
    │  agents table
    ├─→ Task Updates
    │    ↓
    │  tasks table
    └─→ Discovery Archive
         ↓
       discoveries table
         ↓
    Async Write Queue
         ↓
    PostgreSQL
         ↓
    Write-Ahead Log
         ↓
    Disk Persistence
```

### Transaction Patterns

1. **Agent State Update**:
```sql
BEGIN;
UPDATE agents 
SET status = 'busy', 
    last_activity = NOW() 
WHERE id = 'agent-123';

INSERT INTO agent_events (agent_id, event_type, timestamp)
VALUES ('agent-123', 'status_change', NOW());
COMMIT;
```

2. **Task Completion**:
```sql
BEGIN;
UPDATE tasks 
SET status = 'completed',
    completed_at = NOW(),
    result = '{"summary": "..."}'::jsonb
WHERE id = 'task_456';

UPDATE agents
SET current_task_id = NULL
WHERE id = 'agent-123';
COMMIT;
```

## 7. Error Propagation Flow

### Flow Diagram
```
Error Origin (Agent/Service)
    ↓ (1) Error Occurs
Local Error Handler
    ↓ (2) Log & Categorize
    ├─→ Recoverable?
    │    ↓ Yes
    │  Retry Logic
    │    ↓ Success?
    │    └─→ Continue
    └─→ No/Failed
         ↓ (3) Publish Error Event
    agents.{id}.events.error
         ↓
    Error Aggregator
         ↓ (4) Analyze Patterns
    Supervision System
         ↓ (5) Decide Action
         ├─→ Restart Component
         ├─→ Circuit Breaker
         └─→ Alert Admin
              ↓
         Monitoring UI
```

### Error Categories

1. **Transient Errors** (retry):
   - Network timeouts
   - Temporary resource unavailability
   - Rate limits

2. **Fatal Errors** (restart):
   - Process crashes
   - Memory exhaustion
   - Corrupted state

3. **Logical Errors** (alert):
   - Invalid configuration
   - Missing dependencies
   - Authentication failures

## 8. Performance Optimization Flows

### Caching Flow
```
Request
    ↓
Cache Check (Redis planned)
    ├─→ Hit: Return Cached
    └─→ Miss: 
         ↓
    Fetch from Source
         ↓
    Update Cache
         ↓
    Return Result
```

### Batch Processing Flow
```
Multiple Requests
    ↓
Request Queue
    ↓ (Batch Window: 100ms)
Batch Processor
    ↓
Single NATS Publish
    ↓
Parallel Processing
    ↓
Batch Response
    ↓
Individual Responses
```

## Key Performance Patterns

1. **Connection Pooling**:
   - NATS: Single persistent connection
   - PostgreSQL: 30 connections (5 min idle)
   - SSE: 1000 max concurrent

2. **Message Buffering**:
   - Discovery Store: 10,000 circular buffer
   - SSE Broadcast: 1024 message buffer
   - NATS: Default client buffering

3. **Async Processing**:
   - All I/O operations non-blocking
   - Tokio runtime for concurrency
   - Parallel agent execution

4. **Circuit Breakers**:
   - Service calls: 5 failures trigger
   - Reset after 60 seconds
   - Half-open state testing

This data flow documentation provides a comprehensive view of how information moves through the MS-1 system, enabling developers to understand and optimize system behavior.