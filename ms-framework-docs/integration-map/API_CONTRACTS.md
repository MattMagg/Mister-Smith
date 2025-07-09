# MS-1 API Contracts

## MCP (Model Context Protocol) API

### Base Endpoint
- **URL**: `http://localhost:8080/mcp`
- **Protocol**: JSON-RPC 2.0
- **Method**: POST

### Request Structure
```typescript
interface JsonRpcRequest {
  jsonrpc: "2.0";
  method: string;
  params?: any;
  id?: string | number | null;
}
```

### Response Structure
```typescript
interface JsonRpcResponse {
  jsonrpc: "2.0";
  result?: any;
  error?: JsonRpcError;
  id?: string | number | null;
}

interface JsonRpcError {
  code: number;
  message: string;
  data?: any;
}
```

### Available Methods

#### 1. list_tools
Lists all available MCP tools.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "list_tools",
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [
      {
        "name": "share_discovery",
        "description": "Share a discovery with other agents via NATS",
        "inputSchema": {
          "type": "object",
          "properties": {
            "agent_id": {"type": "string"},
            "agent_role": {"type": "string"},
            "discovery_type": {"type": "string"},
            "content": {"type": "string"},
            "confidence": {"type": "number"},
            "related_to": {"type": "string"}
          },
          "required": ["agent_id", "agent_role", "discovery_type", "content", "confidence"]
        }
      },
      {
        "name": "subscribe_discoveries",
        "description": "Subscribe to discoveries with filters via SSE",
        "inputSchema": {
          "type": "object",
          "properties": {
            "discovery_types": {"type": "array", "items": {"type": "string"}},
            "agent_roles": {"type": "array", "items": {"type": "string"}},
            "min_confidence": {"type": "number"},
            "subscriber_agent_id": {"type": "string"}
          },
          "required": ["subscriber_agent_id"]
        }
      }
    ]
  },
  "id": 1
}
```

#### 2. call_tool
Executes a specific tool with provided arguments.

**Request:**
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
      "content": "Unusual login pattern detected",
      "confidence": 0.85,
      "related_to": null
    }
  },
  "id": 2
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": "Discovery shared successfully: disc_abc123",
    "metadata": {
      "discovery_id": "disc_abc123",
      "nats_subject": "discoveries.agent-123.patterns",
      "timestamp": "2025-07-09T15:30:00Z",
      "broadcast_status": "published",
      "collaboration_enabled": true
    }
  },
  "id": 2
}
```

### Error Codes
- `-32600`: Invalid Request - JSON-RPC version must be 2.0
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error
- `-32700`: Parse error

## REST API

### Base URL
`/api/v1`

### Authentication
All endpoints require authentication via Bearer token:
```
Authorization: Bearer <token>
```

### Common Response Structures

#### Paginated Response
```typescript
interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}
```

#### Error Response
```typescript
interface ErrorResponse {
  error: string;
  message: string;
  details?: Record<string, any>;
}
```

### Agent Management

#### List Agents
`GET /api/v1/agents`

**Response:**
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Analyzer-1",
    "status": "idle",
    "capabilities": ["pattern_recognition", "anomaly_detection"],
    "created_at": "2025-07-09T10:00:00Z",
    "last_activity": "2025-07-09T15:30:00Z",
    "metadata": {}
  }
]
```

#### Create Agent
`POST /api/v1/agents`

**Request:**
```json
{
  "name": "Analyzer-2",
  "capabilities": ["pattern_recognition"],
  "config": {
    "timeout": 30,
    "max_retries": 3
  }
}
```

**Response:**
```json
{
  "id": "650e8400-e29b-41d4-a716-446655440001",
  "name": "Analyzer-2",
  "status": "initializing",
  "capabilities": ["pattern_recognition"],
  "created_at": "2025-07-09T15:35:00Z",
  "last_activity": "2025-07-09T15:35:00Z",
  "metadata": {}
}
```

#### Get Agent Details
`GET /api/v1/agents/{agentId}`

#### Delete Agent
`DELETE /api/v1/agents/{agentId}`

### Task Management

#### Submit Task
`POST /api/v1/tasks`

**Request:**
```json
{
  "description": "Analyze system logs for anomalies",
  "priority": 7,
  "required_capabilities": ["anomaly_detection"],
  "parameters": {
    "time_range": "last_24h",
    "severity": "high"
  }
}
```

**Response:**
```json
{
  "id": "task_123",
  "status": "pending",
  "description": "Analyze system logs for anomalies",
  "assigned_agent": null,
  "created_at": "2025-07-09T15:40:00Z",
  "completed_at": null,
  "result": null,
  "error": null
}
```

#### Get Task Status
`GET /api/v1/tasks/{taskId}`

### Discovery Management

#### Filter Discoveries
`POST /api/v1/discoveries/filter`

**Request:**
```json
{
  "type": "pattern",
  "agentId": "agent-123",
  "minConfidence": 0.7,
  "startDate": "2025-07-09T00:00:00Z",
  "endDate": "2025-07-09T23:59:59Z"
}
```

**Response:**
```json
[
  {
    "id": "disc_abc123",
    "type": "pattern",
    "content": "Unusual login pattern detected",
    "confidence": 0.85,
    "agentId": "agent-123",
    "timestamp": "2025-07-09T15:30:00Z"
  }
]
```

### System Monitoring

#### Health Check
`GET /api/v1/health`

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2025-07-09T15:45:00Z",
  "components": {
    "nats": {
      "status": "healthy",
      "message": "Connected to NATS server"
    },
    "database": {
      "status": "healthy",
      "message": "PostgreSQL connection pool active"
    },
    "discovery_store": {
      "status": "healthy",
      "message": "Store operational, 45% capacity"
    }
  }
}
```

#### System Metrics
`GET /api/v1/metrics/system`

**Response:**
```json
{
  "timestamp": "2025-07-09T15:45:00Z",
  "agent_count": 5,
  "task_count": 12,
  "memory_usage": 245.6,
  "cpu_usage": 15.3,
  "nats_connections": 5
}
```

## SSE (Server-Sent Events) API

### Discovery Stream
`GET /discoveries/stream?agent_id={agentId}`

**Connection:**
```javascript
const eventSource = new EventSource('/discoveries/stream?agent_id=agent-123');

eventSource.addEventListener('connected', (event) => {
  const data = JSON.parse(event.data);
  // { "agent_id": "agent-123", "client_id": "550e8400..." }
});

eventSource.addEventListener('discovery', (event) => {
  const notification = JSON.parse(event.data);
  // {
  //   "jsonrpc": "2.0",
  //   "method": "notifications/resources/updated",
  //   "params": {
  //     "uri": "discovery://agents/agent-456/discoveries/1234567890-Pattern-unusual-login"
  //   }
  // }
});
```

### Event Types
- `connected`: Initial connection confirmation
- `discovery`: New discovery notification
- Keep-alive comments sent every 30 seconds

## WebSocket API

### Connection
`ws://localhost:8080/ws`

### Message Format
```typescript
interface WSMessage {
  type: string;
  payload: any;
  correlationId?: string;
  timestamp: Date;
}

interface WSCommand {
  type: 'subscribe' | 'unsubscribe' | 'filter' | 'command';
  payload: any;
  correlationId: string;
}
```

### Example Commands

#### Subscribe to Discoveries
```json
{
  "type": "subscribe",
  "payload": {
    "channel": "discoveries",
    "filters": {
      "types": ["pattern", "anomaly"],
      "minConfidence": 0.7
    }
  },
  "correlationId": "sub-123"
}
```

#### Execute Command
```json
{
  "type": "command",
  "payload": {
    "agentId": "agent-123",
    "command": "analyze",
    "arguments": {
      "target": "system_logs"
    }
  },
  "correlationId": "cmd-456"
}
```

## Rate Limiting

Currently not implemented, but planned:
- **Default**: 100 requests per minute per IP
- **Authenticated**: 1000 requests per minute per token
- **Headers**:
  - `X-RateLimit-Limit`: Request limit
  - `X-RateLimit-Remaining`: Remaining requests
  - `X-RateLimit-Reset`: Reset timestamp

## Versioning

API version is included in the URL path (`/api/v1/`). Breaking changes will increment the version number.

## CORS Policy

In development mode, CORS is permissive. Production configuration:
- **Allowed Origins**: Configured via environment
- **Allowed Methods**: GET, POST, PUT, DELETE, OPTIONS
- **Allowed Headers**: Content-Type, Authorization
- **Credentials**: true