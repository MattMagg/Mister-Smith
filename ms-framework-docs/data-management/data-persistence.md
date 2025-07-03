---
title: revised-data-persistence
type: note
permalink: revision-swarm/data-management/revised-data-persistence
tags:
- '#revised-document #data-persistence #foundation-focus'
---

# Data Persistence & Memory Management Architecture
## Foundation Patterns Guide

> **Canonical Reference**: See `tech-framework.md` for authoritative technology stack specifications

## Executive Summary

This document defines foundational data persistence and memory management patterns using PostgreSQL 15 with SQLx 0.7 as the primary data layer, complemented by JetStream KV for distributed state management. The architecture implements a dual-store approach with short-term state in NATS JetStream KV and long-term state in PostgreSQL, achieving high throughput while maintaining durability. Focus is on teachable patterns and basic architectural principles.

## 1. Basic Storage Architecture

### 1.1 Storage Pattern Overview

```pseudocode
DEFINE StorageLayer ENUM {
    MEMORY_CACHE,     // In-process cache
    DISTRIBUTED_KV,   // JetStream KV (short-term)
    RELATIONAL_DB,    // PostgreSQL (long-term)
    VECTOR_STORE      // Optional: for semantic search
}

INTERFACE DataStorage {
    FUNCTION store(key: String, value: Object) -> Result
    FUNCTION retrieve(key: String) -> Result<Object>
    FUNCTION remove(key: String) -> Result
    FUNCTION search(query: String) -> Result<List<Object>> // For vector stores
}
```

### 1.2 Data Categories & Two-Tier Architecture

```pseudocode
DEFINE DataType ENUM {
    SESSION_DATA,     // Temporary user sessions (KV)
    AGENT_STATE,      // Agent runtime state (KV → SQL)
    TASK_INFO,        // Task metadata (SQL)
    MESSAGE_LOG,      // Communication history (SQL)
    KNOWLEDGE_BASE    // Long-term facts (Vector + SQL)
}

CLASS DataRouter {
    FUNCTION selectStorage(dataType: DataType) -> StorageLayer {
        SWITCH dataType {
            CASE SESSION_DATA:
                RETURN DISTRIBUTED_KV  // Fast, TTL-based
            CASE AGENT_STATE:
                RETURN DISTRIBUTED_KV  // Primary, flushed to SQL
            CASE TASK_INFO:
                RETURN RELATIONAL_DB
            CASE MESSAGE_LOG:
                RETURN RELATIONAL_DB
            CASE KNOWLEDGE_BASE:
                RETURN VECTOR_STORE   // With SQL metadata
        }
    }
}
```

### 1.3 Hybrid Storage Pattern

```pseudocode
-- Dual-store implementation for agent state
CLASS HybridStateManager {
    PRIVATE kv_store: JetStreamKV
    PRIVATE sql_store: PostgresDB
    PRIVATE flush_threshold: Integer = 50
    PRIVATE dirty_keys: Set<String>
    
    FUNCTION writeState(key: String, value: Object) -> Result {
        -- Write to fast KV first
        kv_result = kv_store.put(key, value, TTL.minutes(30))
        dirty_keys.add(key)
        
        -- Trigger flush if threshold reached
        IF dirty_keys.size() >= flush_threshold THEN
            asyncFlushToSQL()
        END IF
        
        RETURN kv_result
    }
    
    FUNCTION readState(key: String) -> Result<Object> {
        -- Try fast KV first
        kv_result = kv_store.get(key)
        IF kv_result.exists() THEN
            RETURN kv_result
        END IF
        
        -- Fallback to SQL (lazy hydration)
        sql_result = sql_store.query("SELECT value FROM agent_state WHERE key = ?", key)
        IF sql_result.exists() THEN
            -- Populate KV for next access
            kv_store.put(key, sql_result.value)
            RETURN sql_result
        END IF
        
        RETURN NotFound()
    }
}
```

## 2. PostgreSQL Schema Patterns

### 2.1 Basic Schema Design with JSONB

```pseudocode
-- Core schema organization
CREATE SCHEMA agents;
CREATE SCHEMA tasks;
CREATE SCHEMA messages;

-- Enhanced agent state with JSONB
CREATE TABLE agents.state (
    agent_id UUID NOT NULL,
    key TEXT NOT NULL,
    value JSONB NOT NULL,  -- Flexible structure
    version BIGINT DEFAULT 1,
    updated_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (agent_id, key)
);

-- JSONB indexing for performance
CREATE INDEX idx_state_value_gin ON agents.state USING gin(value);
CREATE INDEX idx_state_key_btree ON agents.state(agent_id, key);
CREATE INDEX idx_state_updated ON agents.state(updated_at);

-- Task tracking with metadata
CREATE TABLE tasks.queue (
    task_id UUID PRIMARY KEY,
    task_type VARCHAR(50),
    payload JSONB,
    metadata JSONB DEFAULT '{}',  -- TTL, priority, etc.
    status VARCHAR(20) DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP  -- Optional TTL
);
```

### 2.2 State Hydration Support

```pseudocode
-- Agent checkpoint table for recovery
CREATE TABLE agents.checkpoints (
    agent_id UUID,
    checkpoint_id UUID DEFAULT gen_random_uuid(),
    state_snapshot JSONB,
    kv_revision BIGINT,  -- Track KV version
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (agent_id, checkpoint_id)
);

-- Hydration query for agent startup
CREATE FUNCTION hydrate_agent_state(p_agent_id UUID) 
RETURNS TABLE(key TEXT, value JSONB) AS $$
BEGIN
    RETURN QUERY
    SELECT s.key, s.value
    FROM agents.state s
    WHERE s.agent_id = p_agent_id
    ORDER BY s.updated_at DESC;
END;
$$ LANGUAGE plpgsql;
```

## 3. JetStream KV Patterns with TTL

### 3.1 Key-Value Store Setup with TTL

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

### 3.2 State Operations with Conflict Resolution

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

## 4. Common Patterns

### 4.1 Repository Pattern with Dual Store

```pseudocode
INTERFACE Repository<T> {
    FUNCTION save(entity: T) -> Result<T>
    FUNCTION find(id: UUID) -> Result<T>
    FUNCTION update(entity: T) -> Result<T>
    FUNCTION delete(id: UUID) -> Result
}

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
        
        -- Async write to SQL (non-blocking)
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
        
        -- Fallback to SQL and hydrate KV
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

### 4.2 State Lifecycle Management

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
        
        -- Load from SQL
        rows = db.query(
            "SELECT key, value FROM agents.state WHERE agent_id = $1",
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
                        "INSERT INTO agents.state (agent_id, key, value, version) 
                         VALUES ($1, $2, $3::jsonb, $4)
                         ON CONFLICT (agent_id, key) 
                         DO UPDATE SET value = $3::jsonb, version = $4",
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

### 4.3 Enhanced Caching Pattern with TTL

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

## 5. Connection Management

### 5.1 Database Connection Pool

```pseudocode
CLASS ConnectionPool {
    PRIVATE pool: Pool
    
    FUNCTION initialize(config: DatabaseConfig) {
        pool = createPool({
            database_url: config.url,
            max_connections: 10,
            min_connections: 2,
            acquire_timeout: Duration.seconds(30),
            idle_timeout: Duration.minutes(10),
            max_lifetime: Duration.hours(2)
        })
    }
    
    FUNCTION getConnection() -> Connection {
        RETURN pool.acquire()
    }
}
```

### 5.2 Transaction Patterns with Consistency

```pseudocode
CLASS TransactionManager {
    ENUM IsolationLevel {
        READ_COMMITTED,    -- Default, good for most cases
        REPEATABLE_READ,   -- For flush operations
        SERIALIZABLE       -- For critical updates
    }
    
    FUNCTION executeInTransaction(
        operations: List<Operation>, 
        isolation: IsolationLevel = REPEATABLE_READ
    ) -> Result {
        connection = pool.getConnection()
        transaction = connection.beginTransaction()
        
        -- Set isolation level for consistency
        transaction.execute(
            "SET TRANSACTION ISOLATION LEVEL " + isolation.toString()
        )
        
        TRY {
            FOR operation IN operations {
                operation.execute(transaction)
            }
            transaction.commit()
            RETURN Success()
        } CATCH (error) {
            transaction.rollback()
            IF error.type == SERIALIZATION_FAILURE THEN
                -- Retry for serialization conflicts
                RETURN retryWithBackoff(operations, isolation)
            END IF
            RETURN Failure(error)
        }
    }
}
```

## 6. Error Handling and Conflict Resolution

### 6.1 Enhanced Error Types

```pseudocode
ENUM DataError {
    NOT_FOUND,
    DUPLICATE_KEY,
    CONNECTION_FAILED,
    SERIALIZATION_ERROR,
    VERSION_CONFLICT,    -- For optimistic locking
    TTL_EXPIRED,        -- KV entry expired
    CONSISTENCY_VIOLATION
}

CLASS DataResult<T> {
    value: T?
    error: DataError?
    metadata: ResultMetadata?  -- Version, timestamp, etc.
    
    FUNCTION isSuccess() -> Boolean
    FUNCTION getValue() -> T
    FUNCTION getError() -> DataError
    FUNCTION requiresRetry() -> Boolean {
        RETURN error IN [VERSION_CONFLICT, SERIALIZATION_ERROR]
    }
}
```

### 6.2 Conflict Resolution Strategies

```pseudocode
CLASS ConflictResolver {
    ENUM Strategy {
        LAST_WRITE_WINS,
        VECTOR_CLOCK,
        CRDT_MERGE,
        CUSTOM_MERGE
    }
    
    FUNCTION resolve(
        current: StateEntry, 
        incoming: StateEntry, 
        strategy: Strategy
    ) -> StateEntry {
        SWITCH strategy {
            CASE LAST_WRITE_WINS:
                RETURN (incoming.timestamp > current.timestamp) ? 
                       incoming : current
                       
            CASE VECTOR_CLOCK:
                IF vectorClockDominates(incoming.clock, current.clock) THEN
                    RETURN incoming
                ELSE IF vectorClockDominates(current.clock, incoming.clock) THEN
                    RETURN current
                ELSE
                    -- Concurrent, need merge
                    RETURN mergeStates(current, incoming)
                END IF
                
            CASE CRDT_MERGE:
                RETURN StateEntry {
                    value: crdtMerge(current.value, incoming.value),
                    timestamp: max(current.timestamp, incoming.timestamp),
                    version: max(current.version, incoming.version) + 1
                }
        }
    }
}
```

### 6.3 Retry Logic with Backoff

```pseudocode
CLASS RetryHandler {
    FUNCTION withRetry(
        operation: Function, 
        maxAttempts: Integer = 3,
        backoffBase: Duration = 100ms
    ) -> Result {
        attempts = 0
        
        WHILE attempts < maxAttempts {
            result = operation()
            
            IF result.isSuccess() THEN
                RETURN result
            END IF
            
            IF NOT result.requiresRetry() THEN
                RETURN result  -- Non-retryable error
            END IF
            
            attempts += 1
            IF attempts < maxAttempts THEN
                backoff = backoffBase * (2 ^ attempts)  -- Exponential
                sleep(min(backoff, Duration.seconds(5)))  -- Cap at 5s
            END IF
        }
        
        RETURN Failure("Max retry attempts reached")
    }
}
```

## 7. Basic Monitoring

### 7.1 Consistency Metrics

```pseudocode
CLASS ConsistencyMonitor {
    PRIVATE metrics: MetricsCollector
    
    FUNCTION trackConsistencyWindow(agent_id: String) {
        -- Measure time between KV write and SQL flush
        kv_time = getLastKVWrite(agent_id)
        sql_time = getLastSQLWrite(agent_id)
        lag = sql_time - kv_time
        
        metrics.recordGauge("consistency_lag_ms", lag, {"agent": agent_id})
        
        IF lag > Duration.millis(200) THEN
            metrics.incrementCounter("consistency_violations")
            triggerRepairJob(agent_id)
        END IF
    }
    
    FUNCTION getMemoryStats() -> MemoryStats {
        RETURN MemoryStats {
            kv_entries: countKVEntries(),
            sql_rows: countSQLRows(),
            dirty_entries: countDirtyEntries(),
            avg_flush_time: metrics.getAverage("flush_duration_ms"),
            consistency_window_p95: metrics.getPercentile("consistency_lag_ms", 95)
        }
    }
}
```

### 7.2 Health Checks

```pseudocode
CLASS HealthChecker {
    FUNCTION checkDatabase() -> HealthStatus {
        TRY {
            start = now()
            db.execute("SELECT 1")
            latency = now() - start
            
            RETURN HealthStatus {
                status: (latency < 10ms) ? HEALTHY : DEGRADED,
                latency: latency,
                details: "Database responding"
            }
        } CATCH (error) {
            RETURN HealthStatus.UNHEALTHY
        }
    }
    
    FUNCTION checkJetStream() -> HealthStatus {
        TRY {
            jetstream.ping()
            -- Check KV bucket status
            bucket_info = jetstream.getBucketInfo("AGENT_STATE")
            
            RETURN HealthStatus {
                status: HEALTHY,
                details: {
                    entries: bucket_info.entry_count,
                    bytes: bucket_info.bytes,
                    ttl_config: bucket_info.ttl
                }
            }
        } CATCH (error) {
            RETURN HealthStatus.UNHEALTHY
        }
    }
    
    FUNCTION checkConsistency() -> HealthStatus {
        stats = ConsistencyMonitor.getMemoryStats()
        IF stats.consistency_window_p95 > 200 THEN
            RETURN HealthStatus.DEGRADED
        ELSE
            RETURN HealthStatus.HEALTHY
        END IF
    }
}
```

## Summary

This document provides foundational data persistence patterns focusing on:

1. **Dual-store architecture** - Fast KV for working state, durable SQL for long-term
2. **PostgreSQL patterns** - JSONB for flexibility, proper indexing, hydration support
3. **JetStream KV usage** - TTL-based expiry, optimistic concurrency
4. **State lifecycle** - Hydration on startup, periodic flushes, consistency tracking
5. **Conflict resolution** - Multiple strategies from simple LWW to CRDT merging
6. **Connection management** - Pooling, transactions with appropriate isolation
7. **Error handling** - Retries, conflict detection, consistency violations
8. **Monitoring** - Health checks, consistency windows, performance metrics

These patterns form the foundation for high-performance data management in distributed agent systems, balancing speed with durability while maintaining eventual consistency within tight time bounds.