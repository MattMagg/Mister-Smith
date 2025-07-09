# MS-1 Integration Map

## Overview
This document provides a comprehensive map of all integration points between components in the MS-1 (MisterSmith) architecture, including API contracts, message schemas, and data flows.

## 1. MCP ↔ NATS Integration

### Components
- **MCP Server** (`src/mcp_server/server.rs`)
- **NATS Client Manager** (`src/mcp_server/nats_config.rs`)
- **Message Router** (`src/message/router.rs`)

### Data Flow
```
MCP JSON-RPC Request
    ↓
MCP Server (HTTP/8080)
    ↓
Handler Resolution
    ↓
NATS Client Manager
    ↓
NATS Publish/Subscribe
    ↓
Agent Message Handlers
```

### NATS Subject Hierarchy
```yaml
agents:
  - agents.{agent_id}.commands.{command}
  - agents.{agent_id}.status.{status_type}
  - agents.{agent_id}.heartbeat
  - agents.{agent_id}.capabilities
  - agents.{agent_id}.metrics.{metric_type}
  - agents.{agent_id}.events.{event_type}

tasks:
  - tasks.{task_id}.assignment
  - tasks.{task_id}.queue.{priority}
  - tasks.{task_id}.progress
  - tasks.{task_id}.result

system:
  - system.control.{operation}
  - system.discovery.{discovery_type}
  - system.health.{component}
  - system.config.{section}.{key}

discovery:
  - discoveries.{agent_id}.{type}
  - agents.discovery.announce
  - agents.discovery.query
  - agents.discovery.response
  - agents.discovery.departing
```

### Message Schemas

#### Discovery Message
```json
{
  "agent_id": "string",
  "agent_role": "string",
  "discovery_type": "Pattern|Anomaly|Connection|Solution|Question|Insight",
  "content": "string",
  "confidence": 0.0-1.0,
  "timestamp": "ISO 8601",
  "related_to": "optional<string>"
}
```

#### Agent Command
```json
{
  "jsonrpc": "2.0",
  "method": "agent.command",
  "params": {
    "agent_id": "string",
    "command": "string",
    "arguments": {}
  },
  "id": "string|number"
}
```

## 2. UI ↔ API Integration

### API Client Configuration
- **Base URL**: `/api/v1`
- **Authentication**: Bearer token stored in localStorage
- **Error Handling**: 401 redirects to `/login`
- **Content Type**: `application/json`

### REST API Endpoints
```yaml
Discoveries:
  - GET    /api/v1/discoveries
  - GET    /api/v1/discoveries/{id}
  - POST   /api/v1/discoveries/filter

Agents:
  - GET    /api/v1/agents
  - GET    /api/v1/agents/{id}
  - GET    /api/v1/agents/{id}/discoveries
  - GET    /api/v1/agents/{id}/metrics
  - POST   /api/v1/agents
  - DELETE /api/v1/agents/{id}

Tasks:
  - POST   /api/v1/tasks
  - GET    /api/v1/tasks/{id}

System:
  - GET    /api/v1/health
  - GET    /api/v1/metrics/system
  - GET    /api/v1/metrics/nats
  - GET    /api/v1/metrics/history
  - GET    /api/v1/info

Debug:
  - GET    /api/v1/debug/logs
  - POST   /api/v1/debug/test-discovery
  - GET    /api/v1/debug/trace/{id}
```

### Real-Time Communication

#### SSE (Server-Sent Events)
- **Endpoint**: `/discoveries/stream?agent_id={id}`
- **Protocol**: EventSource API
- **Keep-Alive**: 30 second intervals
- **Event Types**:
  - `connected`: Initial connection confirmation
  - `discovery`: New discovery notification

#### SSE Message Format
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/updated",
  "params": {
    "uri": "discovery://agents/{agent_id}/discoveries/{discovery_id}"
  }
}
```

#### WebSocket
- **URL**: `ws://localhost:8080/ws`
- **Reconnection**: Exponential backoff (max 30s)
- **Message Types**:
  ```typescript
  type WSCommand = 'subscribe' | 'unsubscribe' | 'filter' | 'command'
  ```

### UI State Management
- **Query Client**: React Query for caching and synchronization
- **WebSocket Client**: RxJS Observable pattern
- **SSE Client**: Native EventSource with reconnection

## 3. Claude ↔ NATS Integration

### Claude Executor Pattern
- **Execution Model**: Single-shot subprocess execution
- **Command**: `claude --print --output-format json`
- **Communication**: Process stdin/stdout
- **Timeout**: Configurable (default 30s)

### Claude Response Schema
```json
{
  "type": "response_type",
  "subtype": "optional<string>",
  "is_error": false,
  "result": "optional<string>",
  "session_id": "string",
  "total_cost_usd": 0.0,
  "usage": {
    "input_tokens": 0,
    "output_tokens": 0,
    "cache_creation_input_tokens": 0,
    "cache_read_input_tokens": 0
  },
  "duration_ms": 0
}
```

### Integration Flow
```
Agent Task Request
    ↓
NATS Message (agent.command)
    ↓
Agent Message Handler
    ↓
Claude Executor (spawn)
    ↓
JSON Response
    ↓
NATS Publish (task.result)
```

## 4. Database ↔ Services Integration

### Connection Pool Configuration
```yaml
PostgreSQL:
  url: "postgresql://localhost/mistersmith"
  max_connections: 30
  min_connections: 5
  connect_timeout: 10s
  idle_timeout: 300s
  max_lifetime: 3600s
```

### Database Schema Integration
```sql
-- Agent State Persistence
agents:
  - id: UUID PRIMARY KEY
  - name: VARCHAR
  - status: agent_status_enum
  - capabilities: JSONB
  - created_at: TIMESTAMP
  - last_activity: TIMESTAMP

-- Task Management
tasks:
  - id: UUID PRIMARY KEY
  - agent_id: UUID REFERENCES agents
  - status: task_status_enum
  - description: TEXT
  - result: JSONB
  - created_at: TIMESTAMP
  - completed_at: TIMESTAMP

-- Discovery Storage
discoveries:
  - id: UUID PRIMARY KEY
  - agent_id: VARCHAR
  - discovery_type: VARCHAR
  - content: TEXT
  - confidence: FLOAT
  - timestamp: TIMESTAMP
  - related_to: UUID
```

### Service Integration Points
```yaml
Agent Pool:
  - Persists agent state changes
  - Loads agent configurations
  - Tracks agent lifecycle events

Task Manager:
  - Stores task assignments
  - Updates task progress
  - Archives completed tasks

Discovery Store:
  - In-memory cache with persistence
  - Circular buffer (10,000 entries)
  - Subscription management
```

## 5. Collaboration Bridge Integration

### Purpose
The Collaboration Bridge (`src/collaboration/bridge.rs`) unifies MCP handlers with the collaboration infrastructure, providing enhanced discovery sharing and agent lifecycle management.

### Components
```yaml
Discovery Broadcaster:
  - Publishes discoveries to NATS
  - Manages related discoveries
  - Tracks confidence levels

Live Orchestrator:
  - Spawns collaborative agents
  - Provides guidance
  - Manages agent coordination

Context Manager:
  - Shares working memory
  - Broadcasts context updates
  - Maintains agent context

SSE Broadcaster:
  - Streams discoveries to UI
  - Manages client connections
  - Filters by agent/type
```

### Enhanced Discovery Flow
```
MCP share_discovery
    ↓
Collaboration Bridge
    ├── Store in Discovery Store
    ├── Update Agent Activity
    ├── Broadcast via NATS
    ├── Share Context
    └── Notify Orchestrator
```

### Agent Lifecycle Management
```yaml
Registration:
  - agent_id generation
  - role assignment
  - collaboration flag

Activity Tracking:
  - last_activity timestamp
  - discovery count
  - session association

Cleanup:
  - inactive agent removal (1h)
  - resource deallocation
  - subscription cleanup
```

## 6. API Contracts

### MCP JSON-RPC Contract
```yaml
Request:
  jsonrpc: "2.0"
  method: "list_tools" | "call_tool"
  params: optional<object>
  id: optional<value>

Response:
  jsonrpc: "2.0"
  result: optional<value>
  error: optional<{code, message, data}>
  id: optional<value>

Error Codes:
  -32600: Invalid Request
  -32601: Method not found
  -32603: Internal error
```

### REST API Contract
```yaml
Pagination:
  page: number (default: 1)
  pageSize: number (default: 20)
  
Response:
  data: array
  total: number
  page: number
  pageSize: number
  totalPages: number

Error Response:
  error: string
  message: string
  details: optional<object>
```

## 7. Security & Authentication

### Token Management
- **Storage**: localStorage (client-side)
- **Format**: Bearer token
- **Header**: `Authorization: Bearer {token}`
- **Expiry**: Handled by API (401 response)

### NATS Security
- **Connection**: TLS optional
- **Authentication**: Token-based (future)
- **Subjects**: Pattern-based ACLs

### API Security
- **CORS**: Enabled (permissive in dev)
- **Rate Limiting**: Planned
- **Input Validation**: Parameter validation in handlers

## 8. Performance Considerations

### Connection Pooling
- **NATS**: Persistent connection with auto-reconnect
- **PostgreSQL**: 30 max connections, 5 min idle
- **SSE**: 1000 max concurrent clients

### Message Buffering
- **Discovery Store**: 10,000 entry circular buffer
- **SSE Broadcast**: 1024 message buffer
- **NATS**: Default client buffering

### Timeout Configuration
- **Claude Execution**: 30s default
- **API Requests**: 30s default
- **Database Queries**: 10s connection timeout
- **SSE Keep-Alive**: 30s intervals

## 9. Error Handling & Recovery

### Retry Patterns
```yaml
Claude Executor:
  - Max retries: 2
  - Backoff: exponential (100ms * 2^attempt)
  
NATS Connection:
  - Max reconnects: 10
  - Backoff: exponential with jitter
  
Database Pool:
  - Connection retry: 3 attempts
  - Delay: configurable
```

### Error Propagation
```
Service Error
    ↓
Handler Error (McpError)
    ↓
JSON-RPC Error Response
    ↓
UI Error Display
```

## 10. Monitoring & Observability

### Metrics Collection
```yaml
System Metrics:
  - CPU usage
  - Memory usage
  - Agent count
  - Task count
  - NATS connections

Performance Metrics:
  - Message throughput
  - Discovery rate
  - Task completion time
  - API response time
```

### Health Checks
```yaml
Components:
  - NATS: connection status
  - Database: SELECT 1 query
  - Discovery Store: capacity check
  - Agent Pool: active count
```

### Logging
- **Framework**: tracing (Rust)
- **Levels**: ERROR, WARN, INFO, DEBUG
- **Structured**: JSON format available
- **Correlation**: Request IDs for tracing

---

This integration map serves as the authoritative reference for understanding how all components in the MS-1 architecture communicate and integrate with each other.