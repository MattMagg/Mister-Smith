---
title: jetstream-kv-storage
type: note
permalink: framework-docs/data-management/jetstream-kv-storage
tags:
- '#jetstream #key-value #distributed-storage #caching #state-management'
---

# JetStream KV Storage Patterns & Configuration

## Executive Summary

This document specifies the JetStream Key-Value storage patterns and configuration for the Mister Smith AI Agent Framework. JetStream KV provides a distributed, persistent key-value store built on NATS JetStream, offering:

- **High-Performance Caching**: Sub-millisecond access for hot data
- **TTL-Based Expiration**: Automatic cleanup of temporary state
- **Distributed State Management**: Consistent state across agent clusters
- **Dual-Store Architecture**: Seamless integration with PostgreSQL for durability

The patterns defined here implement a hybrid storage approach where JetStream KV serves as the primary fast-access layer for agent state, session data, and caching, while PostgreSQL provides long-term persistence and complex querying capabilities.

> **Related Documentation**: 
> - See [PostgreSQL Implementation](./postgresql-implementation.md) for comprehensive database schemas and hybrid storage integration
> - See [Data Management Directory](./CLAUDE.md) for complete data management architecture
> - See [NATS Transport](../transport/nats-transport.md) for NATS messaging patterns
> - See [Transport Core](../transport/transport-core.md) for transport layer specifications

## Table of Contents

- [1. Hybrid Storage Pattern](#1-hybrid-storage-pattern)
- [2. JetStream KV Patterns with TTL](#2-jetstream-kv-patterns-with-ttl)
- [3. Repository Pattern with Dual Store](#3-repository-pattern-with-dual-store)
- [4. State Lifecycle Management](#4-state-lifecycle-management)
- [5. Enhanced Caching Pattern with TTL](#5-enhanced-caching-pattern-with-ttl)
- [6. NATS JetStream Configuration Specifications](#6-nats-jetstream-configuration-specifications)
- [Cross-References & Integration Points](#cross-references--integration-points)

## 1. Hybrid Storage Pattern

**PostgreSQL Integration**: This pattern integrates with [PostgreSQL Schema Patterns](./postgresql-implementation.md#2-postgresql-schema-patterns) for state hydration and persistence.

```pseudocode
-- Dual-store implementation for agent state
CLASS HybridStateManager {
    PRIVATE kv_store: JetStreamKV
    PRIVATE sql_store: PostgresDB  -- See postgresql-implementation.md
    PRIVATE flush_threshold: Integer = 50
    PRIVATE dirty_keys: Set<String>
    
    FUNCTION writeState(key: String, value: Object) -> Result {
        -- Write to fast KV first
        kv_result = kv_store.put(key, value, TTL.minutes(30))
        dirty_keys.add(key)
        
        -- Trigger flush if threshold reached
        IF dirty_keys.size() >= flush_threshold THEN
            asyncFlushToSQL()  -- See postgresql-implementation.md Section 2.2
        END IF
        
        RETURN kv_result
    }
    
    FUNCTION readState(key: String) -> Result<Object> {
        -- Try fast KV first
        kv_result = kv_store.get(key)
        IF kv_result.exists() THEN
            RETURN kv_result
        END IF
        
        -- Fallback to SQL (lazy hydration from PostgreSQL)
        sql_result = sql_store.query("SELECT value FROM agents.state WHERE state_key = ?", key)
        IF sql_result.exists() THEN
            -- Populate KV for next access
            kv_store.put(key, sql_result.value)
            RETURN sql_result
        END IF
        
        RETURN NotFound()
    }
}
```

## 2. JetStream KV Patterns with TTL

### 2.1 Key-Value Store Setup with TTL

```pseudocode
CLASS KVStoreManager {
    FUNCTION createBucket(name: String, ttl_minutes: Integer = 30) -> Bucket {
        config = {
            bucket: name,
            ttl: Duration.minutes(ttl_minutes),
            replicas: 3,  -- For production
            history: 1,   -- Keep only latest
            storage: FileStorage  -- Persistent
        }
        RETURN JetStream.KeyValue.Create(config)
    }
    
    FUNCTION createTieredBuckets() -> Map<String, Bucket> {
        buckets = Map()
        -- Different TTL for different data types
        buckets["session"] = createBucket("SESSION_DATA", 60)      -- 1 hour
        buckets["agent"] = createBucket("AGENT_STATE", 30)        -- 30 min
        buckets["cache"] = createBucket("QUERY_CACHE", 5)         -- 5 min
        RETURN buckets
    }
}
```

### 2.2 State Operations with Conflict Resolution

```pseudocode
CLASS StateManager {
    PRIVATE kv_bucket: Bucket
    PRIVATE conflict_strategy: ConflictStrategy
    
    FUNCTION saveState(key: String, value: Object) -> Result {
        entry = StateEntry {
            value: value,
            timestamp: now_millis(),
            version: generateVersion()
        }
        serialized = JSON.stringify(entry)
        
        -- Optimistic concurrency control
        current = kv_bucket.get(key)
        IF current.exists() THEN
            RETURN handleConflict(key, current, entry)
        ELSE
            RETURN kv_bucket.put(key, serialized)
        END IF
    }
    
    FUNCTION handleConflict(key: String, current: Entry, new: Entry) -> Result {
        SWITCH conflict_strategy {
            CASE LAST_WRITE_WINS:
                IF new.timestamp >= current.timestamp THEN
                    RETURN kv_bucket.update(key, new, current.revision)
                END IF
            CASE VECTOR_CLOCK:
                merged = mergeWithVectorClock(current.value, new.value)
                RETURN kv_bucket.update(key, merged, current.revision)
            CASE CRDT:
                merged = crdtMerge(current.value, new.value)
                RETURN kv_bucket.update(key, merged, current.revision)
        }
    }
}
```

## 3. Repository Pattern with Dual Store

```pseudocode
CLASS AgentRepository IMPLEMENTS Repository<Agent> {
    PRIVATE db: DatabaseConnection
    PRIVATE kv: KVBucket
    PRIVATE flush_trigger: FlushTrigger
    
    FUNCTION save(agent: Agent) -> Result<Agent> {
        -- Save to KV for immediate access
        kv_key = "agent:" + agent.id
        kv.put(kv_key, agent.serialize(), TTL.minutes(30))
        
        -- Schedule SQL flush
        flush_trigger.markDirty(agent.id)
        
        -- Async write to PostgreSQL (non-blocking)
        -- See postgresql-implementation.md Section 9.2 for schema details
        asyncTask {
            query = "INSERT INTO agents.registry 
                     (agent_id, agent_type, status, metadata) 
                     VALUES ($1, $2, $3, $4::jsonb)
                     ON CONFLICT (agent_id) 
                     DO UPDATE SET status = $3, metadata = $4::jsonb"
            
            db.execute(query, [
                agent.id, 
                agent.type, 
                agent.status,
                agent.metadata
            ])
        }
        
        RETURN Success(agent)
    }
    
    FUNCTION find(id: UUID) -> Result<Agent> {
        -- Try KV first (fast path)
        kv_key = "agent:" + id
        kv_result = kv.get(kv_key)
        IF kv_result.exists() THEN
            RETURN Success(Agent.deserialize(kv_result.value))
        END IF
        
        -- Fallback to PostgreSQL and hydrate KV
        query = "SELECT * FROM agents.registry WHERE agent_id = $1"
        row = db.queryOne(query, [id])
        IF row.exists() THEN
            agent = Agent.fromRow(row)
            -- Populate KV for next access
            kv.put(kv_key, agent.serialize())
            RETURN Success(agent)
        END IF
        
        RETURN NotFound()
    }
}
```

## 4. State Lifecycle Management

```pseudocode
CLASS StateLifecycleManager {
    PRIVATE kv: KVBucket
    PRIVATE db: DatabaseConnection
    PRIVATE dirty_tracker: Map<String, Set<String>>  -- agent_id -> keys
    
    ENUM LifecycleState {
        COLD,       -- No state loaded
        HYDRATING,  -- Loading from SQL
        ACTIVE,     -- In KV, operational
        FLUSHING,   -- Writing to SQL
        EXPIRED     -- TTL exceeded
    }
    
    FUNCTION hydrateAgent(agent_id: String) -> Result {
        setState(agent_id, HYDRATING)
        
        -- Load from PostgreSQL (see postgresql-implementation.md Section 2.2)
        rows = db.query(
            "SELECT state_key, state_value FROM agents.state WHERE agent_id = $1",
            [agent_id]
        )
        
        -- Batch load into KV
        FOR row IN rows {
            kv_key = agent_id + ":" + row.key
            kv.put(kv_key, row.value)
        }
        
        setState(agent_id, ACTIVE)
        RETURN Success()
    }
    
    FUNCTION flushAgent(agent_id: String) -> Result {
        setState(agent_id, FLUSHING)
        dirty_keys = dirty_tracker.get(agent_id)
        
        IF dirty_keys.empty() THEN
            RETURN Success()  -- Nothing to flush
        END IF
        
        -- Begin transaction
        tx = db.beginTransaction()
        TRY {
            FOR key IN dirty_keys {
                kv_key = agent_id + ":" + key
                value = kv.get(kv_key)
                
                IF value.exists() THEN
                    tx.execute(
                        "INSERT INTO agents.state (agent_id, state_key, state_value, version) 
                         VALUES ($1, $2, $3::jsonb, $4)
                         ON CONFLICT (agent_id, state_key) 
                         DO UPDATE SET state_value = $3::jsonb, version = $4",
                        [agent_id, key, value.data, value.revision]
                    )
                END IF
            }
            
            tx.commit()
            dirty_tracker.clear(agent_id)
            setState(agent_id, ACTIVE)
            RETURN Success()
            
        } CATCH (error) {
            tx.rollback()
            setState(agent_id, ACTIVE)  -- Revert state
            RETURN Failure(error)
        }
    }
}
```

## 5. Enhanced Caching Pattern with TTL

```pseudocode
CLASS TieredCacheRepository {
    PRIVATE repository: Repository
    PRIVATE memory_cache: Map<UUID, CacheEntry>
    PRIVATE kv_cache: KVBucket
    PRIVATE cache_ttl: Duration
    
    STRUCT CacheEntry {
        value: Entity
        timestamp: Long
        ttl: Duration
        source: CacheSource  -- MEMORY, KV, or DB
    }
    
    FUNCTION find(id: UUID) -> Result<Entity> {
        -- L1: Check memory cache
        IF memory_cache.contains(id) THEN
            entry = memory_cache.get(id)
            IF entry.isValid() THEN
                RETURN Success(entry.value)
            ELSE
                memory_cache.remove(id)  -- Expired
            END IF
        END IF
        
        -- L2: Check KV cache
        kv_result = kv_cache.get(id.toString())
        IF kv_result.exists() THEN
            entity = deserialize(kv_result.value)
            -- Promote to memory cache
            memory_cache.put(id, CacheEntry(entity, now(), ttl, KV))
            RETURN Success(entity)
        END IF
        
        -- L3: Load from repository (SQL)
        result = repository.find(id)
        IF result.isSuccess() THEN
            -- Populate both cache layers
            kv_cache.put(id.toString(), serialize(result.value))
            memory_cache.put(id, CacheEntry(result.value, now(), ttl, DB))
        END IF
        
        RETURN result
    }
    
    FUNCTION evictExpired() {
        -- Background task to clean expired entries
        FOR entry IN memory_cache.values() {
            IF NOT entry.isValid() THEN
                memory_cache.remove(entry.id)
            END IF
        }
        -- KV entries expire automatically via TTL
    }
}
```

## 6. NATS JetStream Configuration Specifications

### 6.1 Stream Definitions

```yaml
# Agent State Stream Configuration
agent_state_stream:
  name: "AGENT_STATE"
  description: "Real-time agent state updates and synchronization"
  subjects:
    - "agents.state.>"
    - "agents.lifecycle.>"
    - "agents.heartbeat.>"
  
  # Storage configuration
  storage: file
  retention: limits
  max_age: 1800  # 30 minutes in seconds
  max_msgs: 1000000
  max_bytes: 1073741824  # 1GB
  max_msg_size: 1048576   # 1MB
  
  # Replication and clustering
  replicas: 3
  placement:
    cluster: "nats-cluster"
    tags: ["agent-state"]
  
  # Advanced features
  discard: old
  duplicate_window: 300  # 5 minutes
  allow_rollup_hdrs: true
  deny_delete: false
  deny_purge: false

# Task Execution Stream Configuration  
task_execution_stream:
  name: "TASK_EXECUTION"
  description: "Task lifecycle events and execution tracking"
  subjects:
    - "tasks.created.>"
    - "tasks.started.>"
    - "tasks.completed.>"
    - "tasks.failed.>"
    - "tasks.cancelled.>"
  
  # Storage for durability
  storage: file
  retention: interest
  max_age: 86400  # 24 hours
  max_msgs: 10000000
  max_bytes: 10737418240  # 10GB
  max_msg_size: 5242880   # 5MB
  
  # Replication
  replicas: 3
  placement:
    cluster: "nats-cluster"
    tags: ["task-execution"]

# Message Communication Stream
message_communication_stream:
  name: "AGENT_MESSAGES"
  description: "Inter-agent communication and messaging"
  subjects:
    - "messages.direct.>"
    - "messages.broadcast.>"
    - "messages.notification.>"
  
  # Memory storage for low latency
  storage: memory
  retention: limits
  max_age: 300   # 5 minutes
  max_msgs: 100000
  max_bytes: 104857600  # 100MB
  max_msg_size: 262144  # 256KB
  
  # Single replica for memory efficiency
  replicas: 1
  placement:
    cluster: "nats-cluster"
    tags: ["messaging"]
```

### 6.2 Key-Value Bucket Configurations

```yaml
# Session Data Bucket
session_kv_bucket:
  bucket: "SESSION_DATA"
  description: "Temporary user and agent session storage"
  
  # TTL and storage
  ttl: 3600  # 1 hour in seconds
  storage: file
  replicas: 3
  history: 1  # Keep only latest value
  
  # Bucket-specific settings
  placement:
    cluster: "nats-cluster"
    tags: ["session-data"]
  
  # Access control
  allow_direct: true
  mirror: null

# Agent State Bucket (Hot Cache)
agent_state_kv_bucket:
  bucket: "AGENT_STATE"
  description: "Fast access agent state cache"
  
  # Shorter TTL for active state
  ttl: 1800  # 30 minutes
  storage: file
  replicas: 3
  history: 5  # Keep some history for debugging
  
  placement:
    cluster: "nats-cluster"
    tags: ["agent-state"]
  
  # Performance optimization
  allow_direct: true
  compression: s2

# Configuration Bucket (Persistent)
config_kv_bucket:
  bucket: "AGENT_CONFIG"
  description: "Agent configuration and persistent settings"
  
  # No TTL for configuration
  ttl: 0  # Never expire
  storage: file
  replicas: 3
  history: 10  # Keep configuration history
  
  placement:
    cluster: "nats-cluster"
    tags: ["configuration"]

# Query Cache Bucket (Short-lived)
cache_kv_bucket:
  bucket: "QUERY_CACHE"
  description: "Temporary query result cache"
  
  # Very short TTL
  ttl: 300   # 5 minutes
  storage: memory  # Fast access
  replicas: 1      # No need for high availability
  history: 1
  
  placement:
    cluster: "nats-cluster"
    tags: ["cache"]
```

### 6.3 Consumer Configurations

```yaml
# Durable Consumer for Task Processing
task_processor_consumer:
  stream: "TASK_EXECUTION"
  name: "task-processor"
  description: "Processes task execution events"
  
  # Delivery configuration
  deliver_policy: all
  ack_policy: explicit
  ack_wait: 30s
  max_deliver: 3
  
  # Rate limiting
  rate_limit: 1000  # messages per second
  max_ack_pending: 100
  
  # Filtering
  filter_subject: "tasks.started.>"
  
  # Replay and recovery
  replay_policy: instant
  start_sequence: 0
  start_time: null

# Ephemeral Consumer for Real-time Monitoring
monitoring_consumer:
  stream: "AGENT_STATE"
  name: null  # Ephemeral
  description: "Real-time monitoring of agent state changes"
  
  # Start from latest for monitoring
  deliver_policy: last
  ack_policy: none  # Fire and forget
  replay_policy: instant
  
  # No delivery limits for monitoring
  max_deliver: 1
  ack_wait: 5s

# Pull-based Consumer for Batch Processing
batch_processor_consumer:
  stream: "AGENT_MESSAGES"
  name: "message-batch-processor"
  description: "Batch processing of agent messages"
  
  # Pull-based configuration
  deliver_policy: all
  ack_policy: explicit
  ack_wait: 60s
  max_deliver: 5
  
  # Batch processing optimization
  max_batch: 100
  max_expires: 30s
  max_bytes: 1048576  # 1MB batches
```

### 6.4 Integration Points with PostgreSQL

```sql
-- ============================================================================
-- POSTGRESQL-NATS INTEGRATION TRIGGERS AND FUNCTIONS
-- ============================================================================

-- Function to publish agent state changes to NATS
CREATE OR REPLACE FUNCTION notify_agent_state_change()
RETURNS TRIGGER AS $$
DECLARE
  v_subject TEXT;
  v_payload JSONB;
BEGIN
  -- Construct NATS subject
  v_subject := 'agents.state.' || COALESCE(NEW.agent_id, OLD.agent_id)::TEXT;
  
  -- Prepare payload
  v_payload := jsonb_build_object(
    'operation', TG_OP,
    'agent_id', COALESCE(NEW.agent_id, OLD.agent_id),
    'state_key', COALESCE(NEW.state_key, OLD.state_key),
    'old_value', CASE WHEN TG_OP = 'DELETE' THEN OLD.state_value ELSE NULL END,
    'new_value', CASE WHEN TG_OP != 'DELETE' THEN NEW.state_value ELSE NULL END,
    'version', COALESCE(NEW.version, OLD.version),
    'timestamp', EXTRACT(epoch FROM NOW())
  );
  
  -- Publish to NATS (implementation depends on your NATS client)
  PERFORM pg_notify('nats_publish', v_subject || '|' || v_payload::TEXT);
  
  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Trigger for agent state changes
CREATE TRIGGER trigger_agent_state_nats_notify
  AFTER INSERT OR UPDATE OR DELETE ON agents.state
  FOR EACH ROW EXECUTE FUNCTION notify_agent_state_change();

-- Function to publish task lifecycle events
CREATE OR REPLACE FUNCTION notify_task_lifecycle_change()
RETURNS TRIGGER AS $$
DECLARE
  v_subject TEXT;
  v_payload JSONB;
BEGIN
  -- Construct subject based on status change
  v_subject := 'tasks.' || 
    CASE 
      WHEN TG_OP = 'INSERT' THEN 'created'
      WHEN OLD.status != NEW.status THEN 
        CASE NEW.status
          WHEN 'running' THEN 'started'
          WHEN 'completed' THEN 'completed'
          WHEN 'failed' THEN 'failed'
          WHEN 'cancelled' THEN 'cancelled'
          ELSE 'updated'
        END
      ELSE 'updated'
    END || '.' || NEW.task_id::TEXT;
  
  -- Prepare comprehensive payload
  v_payload := jsonb_build_object(
    'task_id', NEW.task_id,
    'task_type', NEW.task_type,
    'status', NEW.status,
    'previous_status', OLD.status,
    'assigned_agent_id', NEW.assigned_agent_id,
    'priority', NEW.priority,
    'timestamp', EXTRACT(epoch FROM NOW()),
    'metadata', NEW.metadata
  );
  
  -- Publish to NATS
  PERFORM pg_notify('nats_publish', v_subject || '|' || v_payload::TEXT);
  
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for task lifecycle events
CREATE TRIGGER trigger_task_lifecycle_nats_notify
  AFTER INSERT OR UPDATE ON tasks.queue
  FOR EACH ROW EXECUTE FUNCTION notify_task_lifecycle_change();
```

---

## Cross-References & Integration Points

### Related Framework Documents
- **Hybrid Storage Partner**: [PostgreSQL Implementation](./postgresql-implementation.md) - Database schemas and persistence layer
- **Data Management Hub**: [Data Management Directory](./CLAUDE.md) - Complete data management navigation
- **Transport Integration**: [NATS Transport](../transport/nats-transport.md) - JetStream transport configuration
- **Core Architecture**: [System Integration](../core-architecture/system-integration.md) - Overall system design patterns

### Key Integration Points
1. **Dual-Store Architecture**: JetStream KV as cache layer, PostgreSQL as authoritative store
2. **State Hydration**: Loading agent state from PostgreSQL into JetStream KV on startup
3. **Cross-System Backup**: Coordinated backup procedures with PostgreSQL
4. **Stream Integration**: JetStream streams work with PostgreSQL triggers for event publishing

### Implementation Dependencies
- NATS JetStream 2.9+
- PostgreSQL 15+ (for hybrid storage)
- Rust NATS client 0.24+
- SQLx 0.7+ (for PostgreSQL integration)

## Navigation

- [← PostgreSQL Implementation](./postgresql-implementation.md) - Database layer implementation
- [← Back to Data Management](./CLAUDE.md) - Directory navigation
- [→ Storage Patterns](./storage-patterns.md) - Additional storage strategies
- [→ Message Framework](./message-framework.md) - Message handling patterns

---

*JetStream KV Storage Patterns - Agent 29, Phase 2, Batch 2*
*Cross-references updated with PostgreSQL integration points*
*Part of the Mister Smith AI Agent Framework*