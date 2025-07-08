# SSE Discovery Broadcaster

## Overview

The SSE (Server-Sent Events) broadcaster bridges NATS discovery messages to web clients, enabling real-time streaming of agent discoveries through standard HTTP connections.

## Architecture

```
NATS Discovery Messages
         ↓
  SSE Broadcaster
         ↓
   Axum HTTP Server
         ↓
    SSE Clients
```

## Key Components

### DiscoverySSEBroadcaster

Main struct that manages:
- SSE client connections with unique IDs
- NATS subscription to discovery topics
- Broadcasting discoveries to connected clients
- Keep-alive heartbeats (30s intervals)
- Client disconnection handling

### Message Format

Discoveries are sent as JSON-RPC notifications:

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/updated",
  "params": {
    "uri": "discovery://agents/{agent_id}/discoveries/{discovery_id}"
  }
}
```

### SSE Event Types

1. **connected** - Initial connection confirmation
   ```
   event: connected
   data: {"agent_id":"agent-123","client_id":"550e8400-e29b-41d4-a716"}
   ```

2. **discovery** - Discovery update notification
   ```
   event: discovery
   data: {"jsonrpc":"2.0","method":"notifications/resources/updated","params":{"uri":"discovery://agents/security-001/discoveries/1234567890-Anomaly-unusual-auth"}}
   ```

## Usage

### Server Setup

```rust
use mistersmith::collaboration::DiscoverySSEBroadcaster;
use std::sync::Arc;

// Connect to NATS
let nats = async_nats::connect("nats://localhost:4222").await?;

// Create broadcaster
let broadcaster = Arc::new(DiscoverySSEBroadcaster::new(nats));

// Start listening for discoveries
broadcaster.start_listening().await?;

// Create Axum router
let app = broadcaster.create_router();

// Start server
let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
axum::serve(listener, app).await?;
```

### Client Connection

#### Using curl:
```bash
curl -N "http://localhost:3000/discoveries/stream?agent_id=monitor-001"
```

#### Using JavaScript EventSource:
```javascript
const source = new EventSource('http://localhost:3000/discoveries/stream?agent_id=web-001');

source.addEventListener('connected', (e) => {
    const data = JSON.parse(e.data);
    console.log('Connected as:', data.agent_id);
});

source.addEventListener('discovery', (e) => {
    const notification = JSON.parse(e.data);
    console.log('Discovery:', notification.params.uri);
});
```

## Features

### Backpressure Handling
- Broadcast channel with 1024 buffer slots
- Automatic dropping of old messages if buffer fills
- Non-blocking send to prevent slow clients affecting others

### Connection Management
- Unique client IDs for tracking
- Automatic cleanup on disconnect (with timeout fallback)
- Connection metadata (agent_id, connected_at)

### Keep-Alive Support
- 30-second heartbeat intervals
- Prevents proxy/firewall timeouts
- Maintains persistent connections

## Example Output

```text
event: connected
data: {"agent_id":"agent-123","client_id":"550e8400-e29b-41d4-a716-446655440000"}

event: discovery
data: {"jsonrpc":"2.0","method":"notifications/resources/updated","params":{"uri":"discovery://agents/security-001/discoveries/1234567890-Anomaly-unusual-login-patte"}}

: keep-alive

event: discovery  
data: {"jsonrpc":"2.0","method":"notifications/resources/updated","params":{"uri":"discovery://agents/perf-002/discoveries/1234567895-Pattern-cpu-spike-correlate"}}
```

## Running the Examples

1. Start NATS:
   ```bash
   nats-server
   ```

2. Run the SSE server example:
   ```bash
   cargo run --example sse_discovery_stream
   ```

3. Open the HTML client:
   ```bash
   open examples/sse_client_example.html
   ```

Or connect with curl:
```bash
curl -N "http://localhost:3000/discoveries/stream?agent_id=test-001"
```