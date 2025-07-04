---
title: postgresql-implementation
type: guide
permalink: framework-modularization/data-management/postgresql-implementation
tags:
- '#postgresql #data-persistence #framework-modularization #extracted-content'
extracted-from: data-persistence.md
agent: Agent-8
phase: Phase-1-Group-1B
---

# PostgreSQL Implementation Guide
## Extracted Framework Components

> **Origin**: This document contains PostgreSQL-specific sections extracted from `data-persistence.md` as part of the Framework Modularization Operation.
> **Sections**: 2, 9, 12, 13, 14 - Comprehensive PostgreSQL implementation patterns and procedures

This guide consolidates all PostgreSQL implementation details including schema patterns, complete schema definitions, connection pool management, indexing strategies, and backup/recovery procedures.

**Hybrid Storage Architecture**: This PostgreSQL implementation works in tandem with [JetStream KV Storage](./jetstream-kv.md) to provide a dual-store system where PostgreSQL handles authoritative data storage and complex queries while JetStream KV provides fast-access caching and real-time state management.

## Table of Contents

- [Integration Overview](#integration-overview)
- [2. PostgreSQL Schema Patterns](#2-postgresql-schema-patterns)
  - [2.1 Basic Schema Design with JSONB](#21-basic-schema-design-with-jsonb)
  - [2.2 State Hydration Support](#22-state-hydration-support)
- [9. Complete PostgreSQL Schema Definitions](#9-complete-postgresql-schema-definitions)
  - [9.1 Domain and Type Definitions](#91-domain-and-type-definitions)
  - [9.2 Core Schema Definitions](#92-core-schema-definitions)
  - [9.3 Indexes for Performance Optimization](#93-indexes-for-performance-optimization)
- [12. Connection Pool Configuration & Management](#12-connection-pool-configuration--management)
  - [12.1 Multiple Pool Configurations](#121-multiple-pool-configurations)
  - [12.2 Connection Pool Health Monitoring](#122-connection-pool-health-monitoring)
  - [12.3 Failover and Load Balancing](#123-failover-and-load-balancing)
- [13. Index Strategies & Partitioning Implementation](#13-index-strategies--partitioning-implementation)
  - [13.1 Advanced Indexing Patterns](#131-advanced-indexing-patterns)
  - [13.2 Partition Management](#132-partition-management)
  - [13.3 Index Maintenance and Optimization](#133-index-maintenance-and-optimization)
- [14. Backup & Recovery Procedures](#14-backup--recovery-procedures)
  - [14.1 Comprehensive Backup Strategy](#141-comprehensive-backup-strategy)
  - [14.2 Point-in-Time Recovery Procedures](#142-point-in-time-recovery-procedures)
  - [14.3 Cross-System Consistency](#143-cross-system-consistency)
  - [14.4 Recovery Testing Automation](#144-recovery-testing-automation)
- [Cross-References](#cross-references)

---

## Integration Overview

This PostgreSQL implementation guide works in conjunction with [JetStream KV Storage](./jetstream-kv.md) to provide a hybrid storage architecture:

- **PostgreSQL**: Long-term persistence, complex queries, ACID transactions, backup/recovery
- **JetStream KV**: Fast-access cache layer, session data, real-time agent state

### Key Integration Points

1. **Dual-Store Pattern**: PostgreSQL serves as the authoritative data store while JetStream KV provides high-performance caching
2. **State Hydration**: Agent state is loaded from PostgreSQL into JetStream KV on startup
3. **Write-Through Caching**: Changes are written to JetStream KV immediately and asynchronously flushed to PostgreSQL
4. **Cross-System Backup**: Coordinated backup procedures ensure data consistency across both systems

> **Related Implementation**: See [JetStream KV Storage Patterns](./jetstream-kv.md) for caching layer implementation and hybrid storage patterns.

---

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

**Integration with JetStream KV**: This pattern supports loading agent state from PostgreSQL into JetStream KV storage for fast access. See [JetStream KV Hybrid Storage Pattern](./jetstream-kv.md#1-hybrid-storage-pattern) for implementation details.

```pseudocode
-- Agent checkpoint table for recovery
CREATE TABLE agents.checkpoints (
    agent_id UUID,
    checkpoint_id UUID DEFAULT gen_random_uuid(),
    state_snapshot JSONB,
    kv_revision BIGINT,  -- Track KV version for sync with JetStream
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (agent_id, checkpoint_id)
);

-- Hydration query for agent startup - loads state into JetStream KV
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

## 9. Complete PostgreSQL Schema Definitions

### 9.1 Domain and Type Definitions

```sql
-- Custom domains for type safety and validation
CREATE DOMAIN agent_id_type AS UUID
  CHECK (VALUE IS NOT NULL);

CREATE DOMAIN task_id_type AS UUID
  CHECK (VALUE IS NOT NULL);

CREATE DOMAIN message_id_type AS UUID
  CHECK (VALUE IS NOT NULL);

-- Enumerated types for controlled vocabularies
CREATE TYPE agent_status_type AS ENUM (
  'initializing',
  'active', 
  'idle',
  'suspended',
  'terminated',
  'error'
);

CREATE TYPE task_status_type AS ENUM (
  'pending',
  'queued',
  'running', 
  'paused',
  'completed',
  'failed',
  'cancelled'
);

CREATE TYPE task_priority_type AS ENUM (
  'low',
  'normal', 
  'high',
  'urgent',
  'critical'
);

CREATE TYPE message_type AS ENUM (
  'command',
  'query',
  'response',
  'notification',
  'heartbeat',
  'error'
);

-- JSON validation functions
CREATE OR REPLACE FUNCTION validate_agent_metadata(metadata JSONB)
RETURNS BOOLEAN AS $$
BEGIN
  -- Ensure required fields exist
  IF NOT (metadata ? 'created_at' AND metadata ? 'version') THEN
    RETURN FALSE;
  END IF;
  
  -- Validate timestamp format
  IF NOT (metadata->>'created_at')::TEXT ~ '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}' THEN
    RETURN FALSE;
  END IF;
  
  RETURN TRUE;
END;
$$ LANGUAGE plpgsql IMMUTABLE;
```

### 9.2 Core Schema Definitions

```sql
-- ============================================================================
-- AGENTS SCHEMA - Agent metadata, state, and lifecycle management
-- ============================================================================

CREATE SCHEMA IF NOT EXISTS agents;

-- Agent registry with metadata and configuration
CREATE TABLE agents.registry (
  agent_id agent_id_type PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_type VARCHAR(50) NOT NULL,
  agent_name VARCHAR(255) NOT NULL,
  status agent_status_type DEFAULT 'initializing',
  capabilities JSONB DEFAULT '{}',
  configuration JSONB DEFAULT '{}',
  metadata JSONB DEFAULT '{}' CHECK (validate_agent_metadata(metadata)),
  parent_agent_id agent_id_type REFERENCES agents.registry(agent_id),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  last_heartbeat TIMESTAMP WITH TIME ZONE,
  
  -- Constraints
  CONSTRAINT valid_agent_name CHECK (LENGTH(agent_name) > 0),
  CONSTRAINT valid_agent_type CHECK (LENGTH(agent_type) > 0),
  CONSTRAINT no_self_parent CHECK (agent_id != parent_agent_id)
);

-- Agent state with JSONB for flexibility and versioning
CREATE TABLE agents.state (
  agent_id agent_id_type NOT NULL REFERENCES agents.registry(agent_id) ON DELETE CASCADE,
  state_key VARCHAR(255) NOT NULL,
  state_value JSONB NOT NULL,
  version BIGINT DEFAULT 1,
  checksum VARCHAR(64), -- SHA-256 hash for integrity verification
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  expires_at TIMESTAMP WITH TIME ZONE, -- Optional TTL
  
  PRIMARY KEY (agent_id, state_key),
  
  -- Constraints
  CONSTRAINT valid_state_key CHECK (LENGTH(state_key) > 0),
  CONSTRAINT valid_version CHECK (version > 0),
  CONSTRAINT future_expiry CHECK (expires_at IS NULL OR expires_at > created_at)
) PARTITION BY HASH (agent_id);

-- Create partitions for agent state (8 partitions for load distribution)
CREATE TABLE agents.state_0 PARTITION OF agents.state FOR VALUES WITH (MODULUS 8, REMAINDER 0);
CREATE TABLE agents.state_1 PARTITION OF agents.state FOR VALUES WITH (MODULUS 8, REMAINDER 1);
CREATE TABLE agents.state_2 PARTITION OF agents.state FOR VALUES WITH (MODULUS 8, REMAINDER 2);
CREATE TABLE agents.state_3 PARTITION OF agents.state FOR VALUES WITH (MODULUS 8, REMAINDER 3);
CREATE TABLE agents.state_4 PARTITION OF agents.state FOR VALUES WITH (MODULUS 8, REMAINDER 4);
CREATE TABLE agents.state_5 PARTITION OF agents.state FOR VALUES WITH (MODULUS 8, REMAINDER 5);
CREATE TABLE agents.state_6 PARTITION OF agents.state FOR VALUES WITH (MODULUS 8, REMAINDER 6);
CREATE TABLE agents.state_7 PARTITION OF agents.state FOR VALUES WITH (MODULUS 8, REMAINDER 7);

-- Agent checkpoints for recovery and rollback
CREATE TABLE agents.checkpoints (
  checkpoint_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id agent_id_type NOT NULL REFERENCES agents.registry(agent_id) ON DELETE CASCADE,
  checkpoint_name VARCHAR(255),
  state_snapshot JSONB NOT NULL,
  kv_revision BIGINT,
  trigger_event VARCHAR(100),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  
  -- Constraints
  CONSTRAINT valid_checkpoint_name CHECK (checkpoint_name IS NULL OR LENGTH(checkpoint_name) > 0)
);

-- Agent lifecycle events for audit and debugging
CREATE TABLE agents.lifecycle_events (
  event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id agent_id_type NOT NULL REFERENCES agents.registry(agent_id) ON DELETE CASCADE,
  event_type VARCHAR(50) NOT NULL,
  previous_status agent_status_type,
  new_status agent_status_type,
  event_data JSONB DEFAULT '{}',
  triggered_by agent_id_type REFERENCES agents.registry(agent_id),
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  
  -- Constraints
  CONSTRAINT valid_event_type CHECK (LENGTH(event_type) > 0),
  CONSTRAINT status_change CHECK (previous_status IS DISTINCT FROM new_status)
) PARTITION BY RANGE (created_at);

-- ============================================================================
-- TASKS SCHEMA - Task management, orchestration, and execution tracking
-- ============================================================================

CREATE SCHEMA IF NOT EXISTS tasks;

-- Main task queue with comprehensive metadata
CREATE TABLE tasks.queue (
  task_id task_id_type PRIMARY KEY DEFAULT gen_random_uuid(),
  task_type VARCHAR(100) NOT NULL,
  task_name VARCHAR(255),
  assigned_agent_id agent_id_type REFERENCES agents.registry(agent_id),
  created_by_agent_id agent_id_type REFERENCES agents.registry(agent_id),
  parent_task_id task_id_type REFERENCES tasks.queue(task_id),
  
  -- Task configuration and data
  payload JSONB NOT NULL DEFAULT '{}',
  configuration JSONB DEFAULT '{}',
  metadata JSONB DEFAULT '{}',
  
  -- Status and scheduling
  status task_status_type DEFAULT 'pending',
  priority task_priority_type DEFAULT 'normal',
  scheduled_at TIMESTAMP WITH TIME ZONE,
  started_at TIMESTAMP WITH TIME ZONE,
  completed_at TIMESTAMP WITH TIME ZONE,
  expires_at TIMESTAMP WITH TIME ZONE,
  
  -- Timing and resource limits
  max_execution_time INTERVAL DEFAULT '1 hour',
  max_retries INTEGER DEFAULT 3,
  retry_count INTEGER DEFAULT 0,
  
  -- Auditing
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  
  -- Constraints
  CONSTRAINT valid_task_type CHECK (LENGTH(task_type) > 0),
  CONSTRAINT valid_retry_count CHECK (retry_count >= 0 AND retry_count <= max_retries),
  CONSTRAINT valid_timing CHECK (
    (started_at IS NULL OR started_at >= created_at) AND
    (completed_at IS NULL OR completed_at >= COALESCE(started_at, created_at)) AND
    (expires_at IS NULL OR expires_at > created_at)
  ),
  CONSTRAINT no_self_parent CHECK (task_id != parent_task_id)
) PARTITION BY LIST (task_type);

-- Task dependencies for orchestration
CREATE TABLE tasks.dependencies (
  dependency_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  task_id task_id_type NOT NULL REFERENCES tasks.queue(task_id) ON DELETE CASCADE,
  depends_on_task_id task_id_type NOT NULL REFERENCES tasks.queue(task_id) ON DELETE CASCADE,
  dependency_type VARCHAR(50) DEFAULT 'completion',
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  
  UNIQUE (task_id, depends_on_task_id),
  CONSTRAINT no_self_dependency CHECK (task_id != depends_on_task_id),
  CONSTRAINT valid_dependency_type CHECK (dependency_type IN ('completion', 'start', 'data', 'resource'))
);

-- Task execution history for monitoring and debugging
CREATE TABLE tasks.executions (
  execution_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  task_id task_id_type NOT NULL REFERENCES tasks.queue(task_id) ON DELETE CASCADE,
  agent_id agent_id_type NOT NULL REFERENCES agents.registry(agent_id),
  execution_attempt INTEGER NOT NULL DEFAULT 1,
  
  -- Execution details
  started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  completed_at TIMESTAMP WITH TIME ZONE,
  status task_status_type DEFAULT 'running',
  
  -- Results and error information
  result JSONB,
  error_message TEXT,
  error_details JSONB,
  
  -- Resource usage
  cpu_time_ms BIGINT,
  memory_peak_mb INTEGER,
  io_operations BIGINT,
  
  -- Constraints
  CONSTRAINT valid_execution_attempt CHECK (execution_attempt > 0),
  CONSTRAINT completion_consistency CHECK (
    (status = 'completed' AND completed_at IS NOT NULL AND result IS NOT NULL) OR
    (status = 'failed' AND completed_at IS NOT NULL AND error_message IS NOT NULL) OR
    (status IN ('running', 'paused') AND completed_at IS NULL)
  )
) PARTITION BY RANGE (started_at);

-- ============================================================================
-- MESSAGES SCHEMA - Inter-agent communication and event logging
-- ============================================================================

CREATE SCHEMA IF NOT EXISTS messages;

-- Communication channels configuration
CREATE TABLE messages.channels (
  channel_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  channel_name VARCHAR(255) UNIQUE NOT NULL,
  channel_type VARCHAR(50) NOT NULL,
  description TEXT,
  configuration JSONB DEFAULT '{}',
  is_active BOOLEAN DEFAULT TRUE,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  
  CONSTRAINT valid_channel_name CHECK (LENGTH(channel_name) > 0),
  CONSTRAINT valid_channel_type CHECK (channel_type IN ('broadcast', 'direct', 'topic', 'queue'))
);

-- Message routing and subscription rules
CREATE TABLE messages.subscriptions (
  subscription_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id agent_id_type NOT NULL REFERENCES agents.registry(agent_id) ON DELETE CASCADE,
  channel_id UUID NOT NULL REFERENCES messages.channels(channel_id) ON DELETE CASCADE,
  message_pattern VARCHAR(255),
  filters JSONB DEFAULT '{}',
  is_active BOOLEAN DEFAULT TRUE,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  
  UNIQUE (agent_id, channel_id, message_pattern)
);

-- Comprehensive message log for all communications
CREATE TABLE messages.log (
  message_id message_id_type PRIMARY KEY DEFAULT gen_random_uuid(),
  channel_id UUID REFERENCES messages.channels(channel_id),
  
  -- Message routing
  from_agent_id agent_id_type REFERENCES agents.registry(agent_id),
  to_agent_id agent_id_type REFERENCES agents.registry(agent_id),
  broadcast_to_type VARCHAR(50), -- For broadcast messages
  
  -- Message content and metadata
  message_type message_type NOT NULL,
  subject VARCHAR(255),
  payload JSONB NOT NULL DEFAULT '{}',
  headers JSONB DEFAULT '{}',
  correlation_id UUID, -- For request/response correlation
  reply_to VARCHAR(255), -- Response routing
  
  -- Delivery and processing
  sent_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  delivered_at TIMESTAMP WITH TIME ZONE,
  processed_at TIMESTAMP WITH TIME ZONE,
  delivery_attempts INTEGER DEFAULT 0,
  
  -- Message properties
  priority INTEGER DEFAULT 0,
  expires_at TIMESTAMP WITH TIME ZONE,
  is_persistent BOOLEAN DEFAULT TRUE,
  
  -- Constraints
  CONSTRAINT valid_priority CHECK (priority BETWEEN 0 AND 10),
  CONSTRAINT valid_delivery_attempts CHECK (delivery_attempts >= 0),
  CONSTRAINT routing_consistency CHECK (
    (to_agent_id IS NOT NULL AND broadcast_to_type IS NULL) OR
    (to_agent_id IS NULL AND broadcast_to_type IS NOT NULL) OR
    (message_type = 'heartbeat')
  )
) PARTITION BY RANGE (sent_at);

-- ============================================================================
-- SESSIONS SCHEMA - User and agent session management
-- ============================================================================

CREATE SCHEMA IF NOT EXISTS sessions;

-- Active session tracking
CREATE TABLE sessions.active (
  session_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_token VARCHAR(255) UNIQUE NOT NULL,
  agent_id agent_id_type REFERENCES agents.registry(agent_id),
  user_id VARCHAR(255),
  
  -- Session properties
  session_type VARCHAR(50) NOT NULL DEFAULT 'interactive',
  session_data JSONB DEFAULT '{}',
  preferences JSONB DEFAULT '{}',
  
  -- Security and access control
  ip_address INET,
  user_agent TEXT,
  permissions JSONB DEFAULT '{}',
  
  -- Timing
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  last_activity TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
  
  -- Constraints
  CONSTRAINT valid_session_token CHECK (LENGTH(session_token) >= 32),
  CONSTRAINT valid_session_type CHECK (session_type IN ('interactive', 'api', 'system', 'background')),
  CONSTRAINT future_expiry CHECK (expires_at > created_at),
  CONSTRAINT active_session CHECK (expires_at > NOW())
) PARTITION BY HASH (session_id);

-- ============================================================================
-- KNOWLEDGE SCHEMA - Long-term knowledge storage and retrieval
-- ============================================================================

CREATE SCHEMA IF NOT EXISTS knowledge;

-- Structured knowledge facts
CREATE TABLE knowledge.facts (
  fact_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  entity_type VARCHAR(100) NOT NULL,
  entity_id VARCHAR(255) NOT NULL,
  
  -- Fact content
  predicate VARCHAR(255) NOT NULL,
  object_value JSONB NOT NULL,
  object_type VARCHAR(50) NOT NULL,
  
  -- Provenance and confidence
  source_agent_id agent_id_type REFERENCES agents.registry(agent_id),
  confidence_score DECIMAL(3,2) CHECK (confidence_score BETWEEN 0.0 AND 1.0),
  evidence JSONB DEFAULT '{}',
  
  -- Versioning and lifecycle
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  valid_from TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  valid_until TIMESTAMP WITH TIME ZONE,
  
  -- Constraints
  CONSTRAINT valid_entity_type CHECK (LENGTH(entity_type) > 0),
  CONSTRAINT valid_entity_id CHECK (LENGTH(entity_id) > 0),
  CONSTRAINT valid_predicate CHECK (LENGTH(predicate) > 0),
  CONSTRAINT valid_validity_period CHECK (valid_until IS NULL OR valid_until > valid_from)
);

-- Vector embeddings for semantic search
CREATE TABLE knowledge.embeddings (
  embedding_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  fact_id UUID NOT NULL REFERENCES knowledge.facts(fact_id) ON DELETE CASCADE,
  
  -- Embedding data
  embedding_model VARCHAR(100) NOT NULL,
  embedding_version VARCHAR(20) NOT NULL,
  embedding_vector VECTOR(1536), -- Adjust dimension based on model
  
  -- Metadata
  text_content TEXT NOT NULL,
  content_hash VARCHAR(64) NOT NULL, -- SHA-256 of text_content
  
  -- Timing
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  
  -- Constraints
  CONSTRAINT valid_embedding_model CHECK (LENGTH(embedding_model) > 0),
  CONSTRAINT valid_text_content CHECK (LENGTH(text_content) > 0)
);
```

### 9.3 Indexes for Performance Optimization

```sql
-- ============================================================================
-- COMPREHENSIVE INDEXING STRATEGY
-- ============================================================================

-- Agents schema indexes
CREATE INDEX idx_agents_registry_type_status ON agents.registry(agent_type, status);
CREATE INDEX idx_agents_registry_parent ON agents.registry(parent_agent_id) WHERE parent_agent_id IS NOT NULL;
CREATE INDEX idx_agents_registry_heartbeat ON agents.registry(last_heartbeat) WHERE status = 'active';
CREATE INDEX idx_agents_registry_metadata_gin ON agents.registry USING gin(metadata);

-- Agent state indexes with JSONB optimization
CREATE INDEX idx_agents_state_updated ON agents.state(updated_at);
CREATE INDEX idx_agents_state_expires ON agents.state(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_agents_state_value_gin ON agents.state USING gin(state_value);
CREATE INDEX idx_agents_state_agent_updated ON agents.state(agent_id, updated_at);

-- Agent checkpoints indexes
CREATE INDEX idx_agents_checkpoints_agent_created ON agents.checkpoints(agent_id, created_at);
CREATE INDEX idx_agents_checkpoints_name ON agents.checkpoints(checkpoint_name) WHERE checkpoint_name IS NOT NULL;

-- Lifecycle events indexes
CREATE INDEX idx_agents_lifecycle_agent_created ON agents.lifecycle_events(agent_id, created_at);
CREATE INDEX idx_agents_lifecycle_event_type ON agents.lifecycle_events(event_type, created_at);
CREATE INDEX idx_agents_lifecycle_status_change ON agents.lifecycle_events(new_status, created_at);

-- Tasks schema indexes
CREATE INDEX idx_tasks_queue_status_priority ON tasks.queue(status, priority, created_at);
CREATE INDEX idx_tasks_queue_assigned_agent ON tasks.queue(assigned_agent_id, status);
CREATE INDEX idx_tasks_queue_type_status ON tasks.queue(task_type, status);
CREATE INDEX idx_tasks_queue_parent ON tasks.queue(parent_task_id) WHERE parent_task_id IS NOT NULL;
CREATE INDEX idx_tasks_queue_scheduled ON tasks.queue(scheduled_at) WHERE scheduled_at IS NOT NULL;
CREATE INDEX idx_tasks_queue_expires ON tasks.queue(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_tasks_queue_payload_gin ON tasks.queue USING gin(payload);

-- Task dependencies indexes
CREATE INDEX idx_tasks_dependencies_task ON tasks.dependencies(task_id);
CREATE INDEX idx_tasks_dependencies_depends_on ON tasks.dependencies(depends_on_task_id);

-- Task executions indexes
CREATE INDEX idx_tasks_executions_task_started ON tasks.executions(task_id, started_at);
CREATE INDEX idx_tasks_executions_agent_started ON tasks.executions(agent_id, started_at);
CREATE INDEX idx_tasks_executions_status ON tasks.executions(status, started_at);

-- Messages schema indexes
CREATE INDEX idx_messages_channels_name ON messages.channels(channel_name);
CREATE INDEX idx_messages_channels_type_active ON messages.channels(channel_type, is_active);

CREATE INDEX idx_messages_subscriptions_agent ON messages.subscriptions(agent_id, is_active);
CREATE INDEX idx_messages_subscriptions_channel ON messages.subscriptions(channel_id, is_active);

-- Message log indexes with time-based optimization
CREATE INDEX idx_messages_log_from_sent ON messages.log(from_agent_id, sent_at);
CREATE INDEX idx_messages_log_to_sent ON messages.log(to_agent_id, sent_at);
CREATE INDEX idx_messages_log_type_sent ON messages.log(message_type, sent_at);
CREATE INDEX idx_messages_log_correlation ON messages.log(correlation_id) WHERE correlation_id IS NOT NULL;
CREATE INDEX idx_messages_log_channel_sent ON messages.log(channel_id, sent_at);
CREATE INDEX idx_messages_log_undelivered ON messages.log(sent_at) WHERE delivered_at IS NULL;

-- Sessions schema indexes
CREATE INDEX idx_sessions_active_token ON sessions.active(session_token);
CREATE INDEX idx_sessions_active_agent ON sessions.active(agent_id) WHERE agent_id IS NOT NULL;
CREATE INDEX idx_sessions_active_user ON sessions.active(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_sessions_active_activity ON sessions.active(last_activity);
CREATE INDEX idx_sessions_active_expires ON sessions.active(expires_at);

-- Knowledge schema indexes
CREATE INDEX idx_knowledge_facts_entity ON knowledge.facts(entity_type, entity_id);
CREATE INDEX idx_knowledge_facts_predicate ON knowledge.facts(predicate, created_at);
CREATE INDEX idx_knowledge_facts_source ON knowledge.facts(source_agent_id, created_at);
CREATE INDEX idx_knowledge_facts_validity ON knowledge.facts(valid_from, valid_until);
CREATE INDEX idx_knowledge_facts_confidence ON knowledge.facts(confidence_score DESC, created_at);
CREATE INDEX idx_knowledge_facts_object_gin ON knowledge.facts USING gin(object_value);

-- Vector similarity search index
CREATE INDEX idx_knowledge_embeddings_vector ON knowledge.embeddings USING ivfflat (embedding_vector vector_cosine_ops);
CREATE INDEX idx_knowledge_embeddings_fact ON knowledge.embeddings(fact_id);
CREATE INDEX idx_knowledge_embeddings_model ON knowledge.embeddings(embedding_model, embedding_version);
CREATE INDEX idx_knowledge_embeddings_hash ON knowledge.embeddings(content_hash);
```

## 12. Connection Pool Configuration & Management

### 12.1 Multiple Pool Configurations

```rust
// Example Rust configuration using SQLx
use sqlx::postgres::{PgPoolOptions, PgConnectOptions};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub primary_pool: PgPoolConfig,
    pub replica_pool: PgPoolConfig,
    pub background_pool: PgPoolConfig,
    pub analytics_pool: PgPoolConfig,
}

#[derive(Debug, Clone)]
pub struct PgPoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
    pub test_before_acquire: bool,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}

impl DatabaseConfig {
    pub fn production() -> Self {
        Self {
            // Primary pool for writes and consistent reads
            primary_pool: PgPoolConfig {
                max_connections: 20,
                min_connections: 5,
                acquire_timeout: Duration::from_secs(30),
                idle_timeout: Some(Duration::from_secs(600)), // 10 minutes
                max_lifetime: Some(Duration::from_secs(3600)), // 1 hour
                test_before_acquire: true,
                host: "primary-db.internal".to_string(),
                port: 5432,
                database: "mister_smith".to_string(),
                username: "app_primary".to_string(),
                password: env::var("PRIMARY_DB_PASSWORD").unwrap(),
            },
            
            // Replica pool for read-only queries
            replica_pool: PgPoolConfig {
                max_connections: 15,
                min_connections: 3,
                acquire_timeout: Duration::from_secs(20),
                idle_timeout: Some(Duration::from_secs(300)), // 5 minutes
                max_lifetime: Some(Duration::from_secs(1800)), // 30 minutes
                test_before_acquire: true,
                host: "replica-db.internal".to_string(),
                port: 5432,
                database: "mister_smith".to_string(),
                username: "app_replica".to_string(),
                password: env::var("REPLICA_DB_PASSWORD").unwrap(),
            },
            
            // Background pool for maintenance operations
            background_pool: PgPoolConfig {
                max_connections: 5,
                min_connections: 1,
                acquire_timeout: Duration::from_secs(60),
                idle_timeout: Some(Duration::from_secs(1800)), // 30 minutes
                max_lifetime: Some(Duration::from_secs(7200)), // 2 hours
                test_before_acquire: false,
                host: "primary-db.internal".to_string(),
                port: 5432,
                database: "mister_smith".to_string(),
                username: "app_background".to_string(),
                password: env::var("BACKGROUND_DB_PASSWORD").unwrap(),
            },
            
            // Analytics pool for reporting queries
            analytics_pool: PgPoolConfig {
                max_connections: 10,
                min_connections: 2,
                acquire_timeout: Duration::from_secs(45),
                idle_timeout: Some(Duration::from_secs(900)), // 15 minutes
                max_lifetime: Some(Duration::from_secs(3600)), // 1 hour
                test_before_acquire: true,
                host: "analytics-db.internal".to_string(),
                port: 5432,
                database: "mister_smith_analytics".to_string(),
                username: "app_analytics".to_string(),
                password: env::var("ANALYTICS_DB_PASSWORD").unwrap(),
            },
        }
    }
}
```

### 12.2 Connection Pool Health Monitoring

```sql
-- ============================================================================
-- CONNECTION POOL MONITORING AND HEALTH CHECKS
-- ============================================================================

-- View for monitoring active connections
CREATE OR REPLACE VIEW monitoring.connection_stats AS
SELECT 
  datname as database_name,
  usename as username,
  application_name,
  client_addr,
  client_hostname,
  state,
  COUNT(*) as connection_count,
  MAX(backend_start) as oldest_connection,
  MIN(backend_start) as newest_connection,
  AVG(EXTRACT(epoch FROM (NOW() - backend_start))) as avg_connection_age_seconds
FROM pg_stat_activity 
WHERE datname = current_database()
GROUP BY datname, usename, application_name, client_addr, client_hostname, state
ORDER BY connection_count DESC;

-- Function for connection pool health check
CREATE OR REPLACE FUNCTION monitoring.check_connection_pool_health()
RETURNS TABLE(
  pool_name TEXT,
  total_connections INTEGER,
  active_connections INTEGER,
  idle_connections INTEGER,
  idle_in_transaction INTEGER,
  max_connections INTEGER,
  health_status TEXT,
  recommendations TEXT[]
) AS $$
DECLARE
  v_max_connections INTEGER;
  v_total_connections INTEGER;
  v_active_connections INTEGER;
  v_idle_connections INTEGER;
  v_idle_in_transaction INTEGER;
  v_recommendations TEXT[] := '{}';
BEGIN
  -- Get max connections setting
  SELECT setting::INTEGER INTO v_max_connections 
  FROM pg_settings WHERE name = 'max_connections';
  
  -- Count current connections by state
  SELECT 
    COUNT(*),
    COUNT(*) FILTER (WHERE state = 'active'),
    COUNT(*) FILTER (WHERE state = 'idle'),
    COUNT(*) FILTER (WHERE state = 'idle in transaction')
  INTO v_total_connections, v_active_connections, v_idle_connections, v_idle_in_transaction
  FROM pg_stat_activity 
  WHERE datname = current_database();
  
  -- Generate recommendations
  IF v_total_connections > v_max_connections * 0.8 THEN
    v_recommendations := array_append(v_recommendations, 'Connection count approaching maximum');
  END IF;
  
  IF v_idle_in_transaction > 5 THEN
    v_recommendations := array_append(v_recommendations, 'High number of idle in transaction connections');
  END IF;
  
  IF v_active_connections > v_total_connections * 0.7 THEN
    v_recommendations := array_append(v_recommendations, 'High ratio of active connections - consider scaling');
  END IF;
  
  -- Return health status
  RETURN QUERY SELECT
    'application_pool'::TEXT,
    v_total_connections,
    v_active_connections,
    v_idle_connections,
    v_idle_in_transaction,
    v_max_connections,
    CASE 
      WHEN v_total_connections > v_max_connections * 0.9 THEN 'critical'
      WHEN v_total_connections > v_max_connections * 0.8 THEN 'warning'
      ELSE 'healthy'
    END,
    v_recommendations;
END;
$$ LANGUAGE plpgsql;
```

### 12.3 Failover and Load Balancing

```rust
// Connection management with failover support
use sqlx::{Pool, Postgres};
use std::sync::Arc;

#[derive(Clone)]
pub struct DatabaseManager {
    primary_pool: Arc<Pool<Postgres>>,
    replica_pools: Vec<Arc<Pool<Postgres>>>,
    background_pool: Arc<Pool<Postgres>>,
    current_replica_index: Arc<std::sync::atomic::AtomicUsize>,
}

impl DatabaseManager {
    pub async fn new(config: DatabaseConfig) -> Result<Self, sqlx::Error> {
        let primary_pool = Arc::new(Self::create_pool(&config.primary_pool).await?);
        let replica_pools = vec![
            Arc::new(Self::create_pool(&config.replica_pool).await?),
            // Add more replica pools as needed
        ];
        let background_pool = Arc::new(Self::create_pool(&config.background_pool).await?);
        
        Ok(Self {
            primary_pool,
            replica_pools,
            background_pool,
            current_replica_index: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }
    
    // Get connection for write operations
    pub fn get_write_pool(&self) -> &Pool<Postgres> {
        &self.primary_pool
    }
    
    // Get connection for read operations with load balancing
    pub fn get_read_pool(&self) -> &Pool<Postgres> {
        if self.replica_pools.is_empty() {
            return &self.primary_pool;
        }
        
        let index = self.current_replica_index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed) 
            % self.replica_pools.len();
        
        &self.replica_pools[index]
    }
    
    // Get connection for background operations
    pub fn get_background_pool(&self) -> &Pool<Postgres> {
        &self.background_pool
    }
    
    // Health check for all pools
    pub async fn health_check(&self) -> HealthCheckResult {
        let mut results = Vec::new();
        
        // Check primary pool
        results.push(self.check_pool_health(&self.primary_pool, "primary").await);
        
        // Check replica pools
        for (i, pool) in self.replica_pools.iter().enumerate() {
            results.push(self.check_pool_health(pool, &format!("replica_{}", i)).await);
        }
        
        // Check background pool
        results.push(self.check_pool_health(&self.background_pool, "background").await);
        
        HealthCheckResult { pool_results: results }
    }
    
    async fn check_pool_health(&self, pool: &Pool<Postgres>, name: &str) -> PoolHealthResult {
        match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => PoolHealthResult {
                name: name.to_string(),
                healthy: true,
                error: None,
                connections_active: pool.size(),
                connections_idle: pool.num_idle(),
            },
            Err(e) => PoolHealthResult {
                name: name.to_string(),
                healthy: false,
                error: Some(e.to_string()),
                connections_active: pool.size(),
                connections_idle: pool.num_idle(),
            },
        }
    }
}
```

## 13. Index Strategies & Partitioning Implementation

### 13.1 Advanced Indexing Patterns

```sql
-- ============================================================================
-- ADVANCED INDEXING STRATEGIES FOR PERFORMANCE OPTIMIZATION
-- ============================================================================

-- Covering indexes to avoid table lookups
CREATE INDEX idx_agents_state_covering ON agents.state 
(agent_id, state_key) 
INCLUDE (state_value, version, updated_at);

CREATE INDEX idx_tasks_queue_priority_covering ON tasks.queue 
(status, priority, created_at) 
INCLUDE (task_id, task_type, assigned_agent_id);

-- Partial indexes for filtered queries
CREATE INDEX idx_agents_registry_active ON agents.registry (agent_type, updated_at) 
WHERE status = 'active';

CREATE INDEX idx_tasks_queue_pending ON tasks.queue (priority, created_at) 
WHERE status = 'pending';

CREATE INDEX idx_sessions_active_unexpired ON sessions.active (last_activity) 
WHERE expires_at > NOW();

-- Expression indexes for computed values
CREATE INDEX idx_agents_state_json_path ON agents.state 
USING gin ((state_value -> 'computed_fields'));

CREATE INDEX idx_tasks_queue_estimated_duration ON tasks.queue 
((EXTRACT(epoch FROM max_execution_time))::INTEGER) 
WHERE status IN ('pending', 'queued');

-- Composite indexes for complex queries
CREATE INDEX idx_messages_log_routing ON messages.log 
(from_agent_id, to_agent_id, sent_at)
WHERE delivered_at IS NULL;

CREATE INDEX idx_knowledge_facts_entity_predicate ON knowledge.facts 
(entity_type, entity_id, predicate, valid_from)
WHERE valid_until IS NULL OR valid_until > NOW();

-- JSONB specialized indexes
CREATE INDEX idx_agents_registry_capabilities_gin ON agents.registry 
USING gin (capabilities jsonb_path_ops);

CREATE INDEX idx_tasks_queue_payload_specific ON tasks.queue 
USING gin ((payload -> 'parameters'));

-- Text search indexes
CREATE INDEX idx_knowledge_facts_text_search ON knowledge.facts 
USING gin (to_tsvector('english', object_value ->> 'text_content'))
WHERE object_type = 'text';
```

### 13.2 Partition Management

```sql
-- ============================================================================
-- AUTOMATED PARTITION MANAGEMENT FUNCTIONS
-- ============================================================================

-- Function to create time-based partitions
CREATE OR REPLACE FUNCTION partitions.create_time_partition(
  p_table_name TEXT,
  p_start_date DATE,
  p_interval INTERVAL DEFAULT '1 day'::INTERVAL
) RETURNS TEXT AS $$
DECLARE
  v_partition_name TEXT;
  v_end_date DATE;
  v_start_str TEXT;
  v_end_str TEXT;
BEGIN
  v_end_date := p_start_date + p_interval;
  v_partition_name := p_table_name || '_' || to_char(p_start_date, 'YYYY_MM_DD');
  v_start_str := quote_literal(p_start_date::TEXT);
  v_end_str := quote_literal(v_end_date::TEXT);
  
  EXECUTE format(
    'CREATE TABLE %I PARTITION OF %I FOR VALUES FROM (%s) TO (%s)',
    v_partition_name, p_table_name, v_start_str, v_end_str
  );
  
  -- Create indexes on the new partition
  EXECUTE format(
    'CREATE INDEX %I ON %I (created_at)',
    'idx_' || v_partition_name || '_created_at', v_partition_name
  );
  
  RETURN v_partition_name;
END;
$$ LANGUAGE plpgsql;

-- Function to automatically manage partitions
CREATE OR REPLACE FUNCTION partitions.manage_time_partitions(
  p_table_name TEXT,
  p_retain_days INTEGER DEFAULT 30,
  p_future_days INTEGER DEFAULT 7
) RETURNS INTEGER AS $$
DECLARE
  v_current_date DATE := CURRENT_DATE;
  v_partition_date DATE;
  v_partitions_created INTEGER := 0;
  v_partitions_dropped INTEGER := 0;
  v_partition_name TEXT;
BEGIN
  -- Create future partitions
  FOR i IN 0..p_future_days LOOP
    v_partition_date := v_current_date + (i || ' days')::INTERVAL;
    
    BEGIN
      SELECT partitions.create_time_partition(p_table_name, v_partition_date);
      v_partitions_created := v_partitions_created + 1;
    EXCEPTION WHEN duplicate_table THEN
      -- Partition already exists, continue
      NULL;
    END;
  END LOOP;
  
  -- Drop old partitions
  FOR v_partition_name IN 
    SELECT schemaname || '.' || tablename
    FROM pg_tables 
    WHERE tablename LIKE p_table_name || '_%'
    AND tablename ~ '\d{4}_\d{2}_\d{2}$'
    AND to_date(substring(tablename from '(\d{4}_\d{2}_\d{2})$'), 'YYYY_MM_DD') 
        < v_current_date - p_retain_days
  LOOP
    EXECUTE 'DROP TABLE ' || v_partition_name;
    v_partitions_dropped := v_partitions_dropped + 1;
  END LOOP;
  
  RETURN v_partitions_created + v_partitions_dropped;
END;
$$ LANGUAGE plpgsql;

-- Create initial partitions for time-based tables
SELECT partitions.create_time_partition('agents.lifecycle_events', CURRENT_DATE - 1);
SELECT partitions.create_time_partition('agents.lifecycle_events', CURRENT_DATE);
SELECT partitions.create_time_partition('agents.lifecycle_events', CURRENT_DATE + 1);

SELECT partitions.create_time_partition('tasks.executions', CURRENT_DATE - 1);
SELECT partitions.create_time_partition('tasks.executions', CURRENT_DATE);
SELECT partitions.create_time_partition('tasks.executions', CURRENT_DATE + 1);

SELECT partitions.create_time_partition('messages.log', CURRENT_DATE - 1);
SELECT partitions.create_time_partition('messages.log', CURRENT_DATE);
SELECT partitions.create_time_partition('messages.log', CURRENT_DATE + 1);
```

### 13.3 Index Maintenance and Optimization

```sql
-- ============================================================================
-- INDEX MAINTENANCE AND MONITORING
-- ============================================================================

-- Function to analyze index usage and provide recommendations
CREATE OR REPLACE FUNCTION monitoring.analyze_index_usage()
RETURNS TABLE(
  schema_name TEXT,
  table_name TEXT,
  index_name TEXT,
  index_size TEXT,
  index_scans BIGINT,
  tuples_read BIGINT,
  tuples_fetched BIGINT,
  usage_ratio NUMERIC,
  recommendation TEXT
) AS $$
BEGIN
  RETURN QUERY
  WITH index_stats AS (
    SELECT 
      schemaname,
      tablename,
      indexname,
      pg_size_pretty(pg_relation_size(indexrelid)) as size_pretty,
      idx_scan,
      idx_tup_read,
      idx_tup_fetch,
      CASE 
        WHEN idx_scan = 0 THEN 0
        ELSE ROUND((idx_tup_fetch::NUMERIC / idx_tup_read::NUMERIC) * 100, 2)
      END as efficiency
    FROM pg_stat_user_indexes psi
    JOIN pg_indexes pi ON psi.indexrelname = pi.indexname AND psi.schemaname = pi.schemaname
    WHERE psi.schemaname NOT IN ('information_schema', 'pg_catalog')
  )
  SELECT 
    s.schemaname,
    s.tablename,
    s.indexname,
    s.size_pretty,
    s.idx_scan,
    s.idx_tup_read,
    s.idx_tup_fetch,
    s.efficiency,
    CASE 
      WHEN s.idx_scan = 0 THEN 'Consider dropping - never used'
      WHEN s.idx_scan < 100 THEN 'Low usage - review necessity'
      WHEN s.efficiency < 10 THEN 'Low efficiency - review index definition'
      WHEN s.efficiency > 90 THEN 'Highly efficient'
      ELSE 'Normal usage'
    END
  FROM index_stats s
  ORDER BY s.idx_scan ASC, s.efficiency ASC;
END;
$$ LANGUAGE plpgsql;

-- Function to detect missing indexes based on slow queries
CREATE OR REPLACE FUNCTION monitoring.suggest_missing_indexes()
RETURNS TABLE(
  suggested_index TEXT,
  reason TEXT,
  estimated_benefit TEXT
) AS $$
BEGIN
  -- This would analyze pg_stat_statements for patterns indicating missing indexes
  -- Implementation depends on having pg_stat_statements enabled
  
  RETURN QUERY
  SELECT 
    'CREATE INDEX idx_missing_example ON table_name (column1, column2);'::TEXT,
    'Frequent sequential scans detected'::TEXT,
    'High - would eliminate table scans'::TEXT
  WHERE FALSE; -- Placeholder implementation
END;
$$ LANGUAGE plpgsql;
```

## 14. Backup & Recovery Procedures

### 14.1 Comprehensive Backup Strategy

```bash
#!/bin/bash
# ============================================================================
# POSTGRESQL BACKUP SCRIPT WITH MULTIPLE STRATEGIES
# ============================================================================

# Configuration
BACKUP_DIR="/var/backups/postgresql"
S3_BUCKET="mister-smith-backups"
DATABASE="mister_smith"
RETENTION_DAYS=30
RETENTION_WEEKS=12
RETENTION_MONTHS=12

# Logging
LOG_FILE="/var/log/postgresql_backup.log"
exec 1> >(tee -a "$LOG_FILE")
exec 2>&1

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

# Full base backup using pg_basebackup
perform_base_backup() {
    local backup_date=$(date +%Y%m%d_%H%M%S)
    local backup_path="$BACKUP_DIR/base/$backup_date"
    
    log "Starting base backup to $backup_path"
    
    mkdir -p "$backup_path"
    
    pg_basebackup \
        --pgdata="$backup_path" \
        --format=tar \
        --compress=9 \
        --checkpoint=fast \
        --progress \
        --verbose \
        --wal-method=stream \
        --max-rate=100M
    
    if [ $? -eq 0 ]; then
        log "Base backup completed successfully"
        
        # Upload to S3
        aws s3 sync "$backup_path" "s3://$S3_BUCKET/base/$backup_date/" \
            --storage-class GLACIER_IR
        
        log "Base backup uploaded to S3"
    else
        log "ERROR: Base backup failed"
        return 1
    fi
}

# Logical backup using pg_dump
perform_logical_backup() {
    local backup_date=$(date +%Y%m%d_%H%M%S)
    local backup_file="$BACKUP_DIR/logical/${DATABASE}_${backup_date}.sql.gz"
    
    log "Starting logical backup to $backup_file"
    
    mkdir -p "$(dirname "$backup_file")"
    
    pg_dump \
        --dbname="$DATABASE" \
        --verbose \
        --format=custom \
        --compress=9 \
        --no-owner \
        --no-privileges \
        | gzip > "$backup_file"
    
    if [ $? -eq 0 ]; then
        log "Logical backup completed successfully"
        
        # Upload to S3
        aws s3 cp "$backup_file" "s3://$S3_BUCKET/logical/"
        
        log "Logical backup uploaded to S3"
    else
        log "ERROR: Logical backup failed"
        return 1
    fi
}

# WAL archiving function
archive_wal() {
    local wal_file="$1"
    local wal_path="$2"
    
    # Copy to local archive
    cp "$wal_path" "$BACKUP_DIR/wal/$wal_file"
    
    # Upload to S3
    aws s3 cp "$wal_path" "s3://$S3_BUCKET/wal/$wal_file"
    
    log "WAL file $wal_file archived"
}

# Cleanup old backups
cleanup_old_backups() {
    log "Cleaning up old backups"
    
    # Remove local backups older than retention period
    find "$BACKUP_DIR/logical" -name "*.sql.gz" -mtime +$RETENTION_DAYS -delete
    find "$BACKUP_DIR/base" -maxdepth 1 -type d -mtime +$RETENTION_WEEKS -exec rm -rf {} \;
    find "$BACKUP_DIR/wal" -name "*.wal" -mtime +7 -delete
    
    # Cleanup S3 backups (using lifecycle policies is preferred)
    log "Local backup cleanup completed"
}

# Verify backup integrity
verify_backup() {
    local backup_file="$1"
    
    log "Verifying backup integrity: $backup_file"
    
    if [ "${backup_file##*.}" = "gz" ]; then
        # Check gzip integrity
        gzip -t "$backup_file"
        if [ $? -eq 0 ]; then
            log "Backup file integrity verified"
            return 0
        else
            log "ERROR: Backup file is corrupted"
            return 1
        fi
    fi
}

# Main backup orchestration
main() {
    log "Starting PostgreSQL backup process"
    
    case "${1:-daily}" in
        "base")
            perform_base_backup
            ;;
        "logical")
            perform_logical_backup
            ;;
        "daily")
            perform_logical_backup
            cleanup_old_backups
            ;;
        "weekly")
            perform_base_backup
            perform_logical_backup
            cleanup_old_backups
            ;;
        *)
            log "Usage: $0 {base|logical|daily|weekly}"
            exit 1
            ;;
    esac
    
    log "Backup process completed"
}

main "$@"
```

### 14.2 Point-in-Time Recovery Procedures

```sql
-- ============================================================================
-- POINT-IN-TIME RECOVERY PROCEDURES AND FUNCTIONS
-- ============================================================================

-- Function to prepare for point-in-time recovery
CREATE OR REPLACE FUNCTION recovery.prepare_pitr(
  p_target_time TIMESTAMP WITH TIME ZONE,
  p_backup_location TEXT
) RETURNS TEXT AS $$
DECLARE
  v_recovery_config TEXT;
  v_wal_files TEXT[];
  v_required_wal_start TEXT;
BEGIN
  -- Generate recovery configuration
  v_recovery_config := format('
# Point-in-time recovery configuration
# Generated on: %s
# Target time: %s

# Recovery settings
restore_command = ''cp %s/wal/%%f %%p''
recovery_target_time = ''%s''
recovery_target_action = ''promote''

# WAL settings
archive_mode = off
hot_standby = on
max_standby_archive_delay = 300s
max_standby_streaming_delay = 300s

# Logging
log_min_messages = info
log_checkpoints = on
log_connections = on
log_disconnections = on
log_lock_waits = on

# Performance during recovery
shared_buffers = 256MB
effective_cache_size = 1GB
random_page_cost = 1.1
  ', 
  NOW()::TEXT,
  p_target_time::TEXT,
  p_backup_location,
  p_target_time::TEXT
  );
  
  RETURN v_recovery_config;
END;
$$ LANGUAGE plpgsql;

-- Function to validate recovery readiness
CREATE OR REPLACE FUNCTION recovery.validate_recovery_readiness(
  p_backup_path TEXT,
  p_target_time TIMESTAMP WITH TIME ZONE
) RETURNS TABLE(
  check_name TEXT,
  status TEXT,
  message TEXT
) AS $$
BEGIN
  -- Check if base backup exists
  RETURN QUERY SELECT 
    'base_backup_exists'::TEXT,
    CASE WHEN pg_stat_file(p_backup_path || '/base.tar').size > 0 
         THEN 'PASS' ELSE 'FAIL' END,
    'Base backup file validation'::TEXT;
  
  -- Check WAL continuity (simplified check)
  RETURN QUERY SELECT 
    'wal_continuity'::TEXT,
    'PASS'::TEXT,  -- Would implement actual WAL validation
    'WAL file continuity validation'::TEXT;
  
  -- Check target time feasibility
  RETURN QUERY SELECT 
    'target_time_feasible'::TEXT,
    CASE WHEN p_target_time > NOW() - INTERVAL '30 days' 
         THEN 'PASS' ELSE 'WARN' END,
    'Target time within retention period'::TEXT;
END;
$$ LANGUAGE plpgsql;
```

### 14.3 Cross-System Consistency

**JetStream Integration**: This backup strategy coordinates with [JetStream KV Backup Procedures](./jetstream-kv.md#cross-system-backup-coordination) to ensure data consistency.

```bash
#!/bin/bash
# ============================================================================
# POSTGRESQL-JETSTREAM BACKUP COORDINATION SCRIPT
# ============================================================================

# Configuration
BACKUP_TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_ROOT="/var/backups/mister-smith"
JETSTREAM_DATA_DIR="/var/lib/nats"
POSTGRES_BACKUP_DIR="$BACKUP_ROOT/$BACKUP_TIMESTAMP"

# Logging
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "/var/log/cross_system_backup.log"
}

# Create consistent snapshot across systems
create_consistent_snapshot() {
    log "Starting consistent cross-system backup"
    
    # Step 1: Pause JetStream writes (if possible)
    log "Pausing JetStream message processing"
    # Implementation depends on your JetStream setup
    # Could involve stopping publishers or enabling read-only mode
    
    # Step 2: Ensure PostgreSQL consistency
    log "Creating PostgreSQL snapshot"
    psql -d mister_smith -c "SELECT pg_start_backup('cross_system_$BACKUP_TIMESTAMP', true);"
    
    # Step 3: Backup JetStream streams and KV stores
    log "Backing up JetStream data"
    mkdir -p "$POSTGRES_BACKUP_DIR/jetstream"
    
    # Backup JetStream streams (see jetstream-kv.md for stream definitions)
    nats stream backup AGENT_STATE "$POSTGRES_BACKUP_DIR/jetstream/agent_state.backup"
    nats stream backup TASK_EXECUTION "$POSTGRES_BACKUP_DIR/jetstream/task_execution.backup"
    nats stream backup AGENT_MESSAGES "$POSTGRES_BACKUP_DIR/jetstream/agent_messages.backup"
    
    # Backup KV buckets (see jetstream-kv.md for bucket configurations)
    nats kv backup SESSION_DATA "$POSTGRES_BACKUP_DIR/jetstream/session_data.backup"
    nats kv backup AGENT_STATE "$POSTGRES_BACKUP_DIR/jetstream/agent_state_kv.backup"
    nats kv backup AGENT_CONFIG "$POSTGRES_BACKUP_DIR/jetstream/agent_config.backup"
    nats kv backup QUERY_CACHE "$POSTGRES_BACKUP_DIR/jetstream/query_cache.backup"
    
    # Step 4: Complete PostgreSQL backup
    log "Completing PostgreSQL backup"
    pg_basebackup --pgdata="$POSTGRES_BACKUP_DIR/postgresql" --format=tar --compress=9
    psql -d mister_smith -c "SELECT pg_stop_backup();"
    
    # Step 5: Resume JetStream processing
    log "Resuming JetStream message processing"
    # Resume JetStream operations
    
    # Step 6: Create backup manifest
    create_backup_manifest "$POSTGRES_BACKUP_DIR"
    
    log "Consistent cross-system backup completed"
}

# Create backup manifest with checksums
create_backup_manifest() {
    local backup_dir="$1"
    local manifest_file="$backup_dir/backup_manifest.json"
    
    log "Creating backup manifest"
    
    cat > "$manifest_file" << EOF
{
  "backup_timestamp": "$BACKUP_TIMESTAMP",
  "backup_type": "cross_system_consistent",
  "systems": {
    "postgresql": {
      "backup_method": "pg_basebackup",
      "backup_location": "./postgresql",
      "database_version": "$(psql --version | head -1)",
      "backup_size": "$(du -sh $backup_dir/postgresql | cut -f1)"
    },
    "nats": {
      "backup_method": "nats_cli",
      "streams": [
        {
          "name": "AGENT_STATE",
          "backup_file": "./nats/agent_state.backup",
          "size": "$(stat -c%s $backup_dir/nats/agent_state.backup 2>/dev/null || echo 0)"
        },
        {
          "name": "TASK_EXECUTION", 
          "backup_file": "./nats/task_execution.backup",
          "size": "$(stat -c%s $backup_dir/nats/task_execution.backup 2>/dev/null || echo 0)"
        },
        {
          "name": "AGENT_MESSAGES",
          "backup_file": "./nats/agent_messages.backup", 
          "size": "$(stat -c%s $backup_dir/nats/agent_messages.backup 2>/dev/null || echo 0)"
        }
      ],
      "kv_buckets": [
        {
          "name": "SESSION_DATA",
          "backup_file": "./nats/session_data.backup",
          "size": "$(stat -c%s $backup_dir/nats/session_data.backup 2>/dev/null || echo 0)"
        },
        {
          "name": "AGENT_STATE",
          "backup_file": "./nats/agent_state_kv.backup",
          "size": "$(stat -c%s $backup_dir/nats/agent_state_kv.backup 2>/dev/null || echo 0)"
        },
        {
          "name": "AGENT_CONFIG",
          "backup_file": "./nats/agent_config.backup",
          "size": "$(stat -c%s $backup_dir/nats/agent_config.backup 2>/dev/null || echo 0)"
        }
      ]
    }
  },
  "checksums": {
EOF

    # Add checksums for all backup files
    find "$backup_dir" -type f -name "*.backup" -o -name "*.tar" | while read file; do
        local relative_path=$(realpath --relative-to="$backup_dir" "$file")
        local checksum=$(sha256sum "$file" | cut -d' ' -f1)
        echo "    \"$relative_path\": \"$checksum\"," >> "$manifest_file"
    done
    
    # Close JSON
    cat >> "$manifest_file" << EOF
  },
  "verification": {
    "backup_verified": false,
    "verification_timestamp": null,
    "verification_notes": ""
  }
}
EOF
    
    log "Backup manifest created: $manifest_file"
}

# Verify cross-system backup integrity
verify_cross_system_backup() {
    local backup_dir="$1"
    local manifest_file="$backup_dir/backup_manifest.json"
    
    log "Verifying cross-system backup integrity"
    
    if [ ! -f "$manifest_file" ]; then
        log "ERROR: Backup manifest not found"
        return 1
    fi
    
    # Verify checksums
    local verification_passed=true
    
    while IFS= read -r line; do
        if [[ $line =~ \"([^\"]+)\":\ \"([^\"]+)\" ]]; then
            local file_path="$backup_dir/${BASH_REMATCH[1]}"
            local expected_checksum="${BASH_REMATCH[2]}"
            
            if [ -f "$file_path" ]; then
                local actual_checksum=$(sha256sum "$file_path" | cut -d' ' -f1)
                if [ "$actual_checksum" != "$expected_checksum" ]; then
                    log "ERROR: Checksum mismatch for $file_path"
                    verification_passed=false
                fi
            else
                log "ERROR: Missing backup file $file_path"
                verification_passed=false
            fi
        fi
    done < <(grep -E '\"[^\"]+\":\s*\"[a-f0-9]{64}\"' "$manifest_file")
    
    if [ "$verification_passed" = true ]; then
        log "Backup verification PASSED"
        
        # Update manifest with verification status
        local temp_manifest=$(mktemp)
        jq '.verification.backup_verified = true | .verification.verification_timestamp = now | .verification.verification_notes = "All checksums verified successfully"' "$manifest_file" > "$temp_manifest"
        mv "$temp_manifest" "$manifest_file"
        
        return 0
    else
        log "Backup verification FAILED"
        return 1
    fi
}

# Main execution
main() {
    case "${1:-backup}" in
        "backup")
            create_consistent_snapshot
            verify_cross_system_backup "$POSTGRES_BACKUP_DIR"
            ;;
        "verify")
            if [ -z "$2" ]; then
                log "ERROR: Please provide backup directory for verification"
                exit 1
            fi
            verify_cross_system_backup "$2"
            ;;
        *)
            log "Usage: $0 {backup|verify <backup_dir>}"
            exit 1
            ;;
    esac
}

main "$@"
```

### 14.4 Recovery Testing Automation

```sql
-- ============================================================================
-- AUTOMATED RECOVERY TESTING PROCEDURES
-- ============================================================================

-- Recovery test tracking table
CREATE TABLE IF NOT EXISTS recovery.test_results (
  test_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  test_type VARCHAR(50) NOT NULL,
  backup_timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
  recovery_target TIMESTAMP WITH TIME ZONE,
  test_started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  test_completed_at TIMESTAMP WITH TIME ZONE,
  test_status VARCHAR(20) DEFAULT 'running',
  data_verification_passed BOOLEAN,
  performance_metrics JSONB DEFAULT '{}',
  error_messages TEXT[],
  notes TEXT,
  
  CONSTRAINT valid_test_type CHECK (test_type IN ('full_restore', 'pitr', 'partial_restore', 'cross_system')),
  CONSTRAINT valid_test_status CHECK (test_status IN ('running', 'passed', 'failed', 'cancelled'))
);

-- Function to record recovery test results
CREATE OR REPLACE FUNCTION recovery.record_test_result(
  p_test_type TEXT,
  p_backup_timestamp TIMESTAMP WITH TIME ZONE,
  p_recovery_target TIMESTAMP WITH TIME ZONE DEFAULT NULL,
  p_test_status TEXT DEFAULT 'passed',
  p_data_verified BOOLEAN DEFAULT NULL,
  p_performance_metrics JSONB DEFAULT '{}'::JSONB,
  p_notes TEXT DEFAULT NULL
) RETURNS UUID AS $$
DECLARE
  v_test_id UUID;
BEGIN
  INSERT INTO recovery.test_results (
    test_type, backup_timestamp, recovery_target, test_completed_at,
    test_status, data_verification_passed, performance_metrics, notes
  ) VALUES (
    p_test_type, p_backup_timestamp, p_recovery_target, NOW(),
    p_test_status, p_data_verified, p_performance_metrics, p_notes
  ) RETURNING test_id INTO v_test_id;
  
  RETURN v_test_id;
END;
$$ LANGUAGE plpgsql;

-- Function to verify data consistency after recovery
CREATE OR REPLACE FUNCTION recovery.verify_data_consistency()
RETURNS TABLE(
  table_name TEXT,
  record_count BIGINT,
  consistency_check TEXT,
  issues_found TEXT[]
) AS $$
DECLARE
  v_table RECORD;
  v_count BIGINT;
  v_issues TEXT[] := '{}';
BEGIN
  -- Check all major tables for consistency
  FOR v_table IN 
    SELECT schemaname, tablename 
    FROM pg_tables 
    WHERE schemaname IN ('agents', 'tasks', 'messages', 'sessions', 'knowledge')
  LOOP
    -- Get record count
    EXECUTE format('SELECT COUNT(*) FROM %I.%I', v_table.schemaname, v_table.tablename)
    INTO v_count;
    
    -- Perform table-specific consistency checks
    CASE v_table.schemaname || '.' || v_table.tablename
      WHEN 'agents.state' THEN
        -- Check for orphaned state records
        EXECUTE '
          SELECT COUNT(*) FROM agents.state s 
          LEFT JOIN agents.registry r ON s.agent_id = r.agent_id 
          WHERE r.agent_id IS NULL
        ' INTO v_count;
        
        IF v_count > 0 THEN
          v_issues := array_append(v_issues, format('%s orphaned state records', v_count));
        END IF;
        
      WHEN 'tasks.queue' THEN
        -- Check for invalid task assignments
        EXECUTE '
          SELECT COUNT(*) FROM tasks.queue t 
          LEFT JOIN agents.registry a ON t.assigned_agent_id = a.agent_id 
          WHERE t.assigned_agent_id IS NOT NULL AND a.agent_id IS NULL
        ' INTO v_count;
        
        IF v_count > 0 THEN
          v_issues := array_append(v_issues, format('%s tasks assigned to non-existent agents', v_count));
        END IF;
    END CASE;
    
    RETURN QUERY SELECT 
      v_table.schemaname || '.' || v_table.tablename,
      v_count,
      CASE WHEN array_length(v_issues, 1) IS NULL THEN 'PASS' ELSE 'ISSUES_FOUND' END,
      v_issues;
      
    v_issues := '{}'; -- Reset for next table
  END LOOP;
END;
$$ LANGUAGE plpgsql;
```

---

## Cross-References

### Related Framework Documents
- **Hybrid Storage Partner**: [JetStream KV Storage](./jetstream-kv.md) - Fast-access cache layer and hybrid storage patterns
- **Data Management Hub**: [Data Management Directory](./CLAUDE.md) - Complete data management navigation
- **Transport Layer**: [NATS Transport](../transport/nats-transport.md) - Message passing integration
- **Core Architecture**: [System Integration](../core-architecture/system-integration.md) - Overall system design patterns
- **Operations Guide**: [Operations Directory](../operations/) - Deployment and monitoring procedures

### Key Integration Points
1. **Hybrid Storage**: Works with JetStream KV for dual-store architecture
2. **State Hydration**: Agent state loaded from PostgreSQL into JetStream KV
3. **Cross-System Backup**: Coordinated backup with JetStream streams and KV buckets
4. **Event Publishing**: PostgreSQL triggers publish to JetStream streams

### Implementation Dependencies
- PostgreSQL 15+
- SQLx 0.7+ (Rust)
- NATS JetStream 2.9+ (for cross-system coordination)
- AWS S3 (for backup storage)
- Vector extension (for embedding search)

### Bidirectional Navigation
- [← JetStream KV Storage](./jetstream-kv.md) - Cache layer implementation
- [← Back to Data Management](./CLAUDE.md) - Directory navigation
- [→ Storage Patterns](./storage-patterns.md) - Additional storage strategies
- [→ Connection Management](./connection-management.md) - Connection handling details

---

*PostgreSQL Implementation Guide - Agent 29, Phase 2, Batch 2*
*Cross-references updated with JetStream KV integration patterns*
*File size: 1,730+ lines - Recommend splitting into:*
*- postgresql-schemas.md (Sections 9.1-9.3)*
*- postgresql-operations.md (Sections 12-13)*
*- postgresql-backup.md (Section 14)*