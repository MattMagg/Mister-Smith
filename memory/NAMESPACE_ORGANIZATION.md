# MisterSmith Memory Namespace Organization

## Overview
Memory namespaces are organized by domain to enable efficient coordination and knowledge management across the MS-1 swarm.

## Namespace Categories

### 1. Core System Namespaces
- **core-architecture**: System design, Tokio runtime, async patterns
- **supervision-trees**: Strategies (OneForOne, OneForAll, RestForOne), failure handling
- **agent-domains**: 15 specialized domains, agent types, capabilities

### 2. Infrastructure Namespaces  
- **transport-layer**: NATS messaging, HTTP/gRPC protocols, real-time SSE
- **data-management**: PostgreSQL schemas, JetStream KV, message routing
- **monitoring**: OpenTelemetry, Prometheus metrics, health checks

### 3. Operational Namespaces
- **operations**: Deployment, configuration, process management
- **security**: Authentication, authorization, JWT handling
- **testing**: Test strategies, benchmarks, integration tests
- **collaboration**: Discovery sharing, real-time coordination, MCP

## Usage Patterns

### Storing Domain Knowledge
```bash
# Store in specific namespace
mcp__claude-flow__memory_usage \
  --action store \
  --namespace "supervision-trees" \
  --key "strategies/restforone" \
  --value '{"behavior": "Restart failed and subsequent children"}'
```

### Cross-Domain Queries
```bash
# Search across namespaces
mcp__claude-flow__memory_search \
  --pattern "*/nats-*" \
  --limit 10
```

### Agent Coordination
```bash
# Agent stores discovery in collaboration namespace
mcp__claude-flow__memory_usage \
  --action store \
  --namespace "collaboration" \
  --key "discoveries/auth-pattern-2025-07-09" \
  --value '{"agent": "security-specialist", "insight": "JWT refresh strategy"}'
```

## Benefits
1. **Organization**: Clear separation of concerns
2. **Scalability**: Namespaces can grow independently  
3. **Discovery**: Agents find relevant knowledge faster
4. **Coordination**: Shared namespaces for collaboration
5. **Performance**: Reduced search scope improves speed

## Next Steps
1. Populate namespaces with existing knowledge
2. Set up cross-namespace references
3. Create namespace-specific access patterns
4. Implement auto-categorization for new memories