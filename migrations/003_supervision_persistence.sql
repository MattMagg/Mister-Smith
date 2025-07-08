-- Migration 003: Add supervision persistence for Phase 3
-- Description: Comprehensive supervision system tables for agent orchestration

BEGIN;

-- Create custom types for supervision system
CREATE TYPE node_type AS ENUM (
    'RootSupervisor',
    'AgentSupervisor', 
    'Worker',
    'Service'
);

CREATE TYPE supervision_strategy AS ENUM (
    'OneForOne',
    'OneForAll',
    'RestForOne',
    'Escalate'
);

CREATE TYPE backoff_strategy AS ENUM (
    'Fixed',
    'Linear',
    'Exponential'
);

CREATE TYPE failure_reason AS ENUM (
    'Crash',
    'HeartbeatTimeout',
    'ResourceExhaustion',
    'UnhandledError',
    'ExternalFailure'
);

CREATE TYPE child_state AS ENUM (
    'Starting',
    'Running',
    'Failed',
    'Stopped'
);

CREATE TYPE task_type AS ENUM (
    'Research',
    'Code',
    'Analysis',
    'Communication',
    'Custom'
);

CREATE TYPE task_status AS ENUM (
    'pending',
    'assigned',
    'processing',
    'completed',
    'failed',
    'cancelled'
);

CREATE TYPE lock_type AS ENUM (
    'exclusive',
    'shared',
    'advisory'
);

CREATE TYPE supervision_event_type AS ENUM (
    'node_started',
    'node_stopped',
    'node_failed',
    'task_assigned',
    'task_completed',
    'restart_initiated',
    'escalation_triggered'
);

CREATE TYPE node_status AS ENUM (
    'active',
    'inactive',
    'failed',
    'terminated'
);

-- Core supervision nodes registry
CREATE TABLE supervision_nodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id TEXT NOT NULL UNIQUE,
    node_type node_type NOT NULL,
    node_type_detail TEXT,
    status node_status NOT NULL DEFAULT 'inactive',
    parent_node_id UUID REFERENCES supervision_nodes(id) ON DELETE CASCADE,
    agent_id UUID REFERENCES agents(id),
    configuration JSONB NOT NULL DEFAULT '{}',
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    terminated_at TIMESTAMPTZ,
    CONSTRAINT chk_node_type_detail CHECK (
        (node_type IN ('AgentSupervisor', 'Worker', 'Service') AND node_type_detail IS NOT NULL) OR
        (node_type = 'RootSupervisor' AND node_type_detail IS NULL)
    ),
    CONSTRAINT chk_agent_link CHECK (
        (node_type IN ('Worker', 'AgentSupervisor') AND agent_id IS NOT NULL) OR
        (node_type IN ('RootSupervisor', 'Service') AND agent_id IS NULL)
    )
);

-- Supervision policies per node
CREATE TABLE supervision_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id UUID NOT NULL REFERENCES supervision_nodes(id) ON DELETE CASCADE,
    supervision_strategy supervision_strategy NOT NULL,
    max_restarts INTEGER NOT NULL CHECK (max_restarts >= 0),
    time_window_seconds INTEGER NOT NULL CHECK (time_window_seconds > 0),
    backoff_strategy backoff_strategy NOT NULL,
    backoff_factor NUMERIC(4,2) DEFAULT 2.0,
    min_restart_delay_ms INTEGER NOT NULL CHECK (min_restart_delay_ms >= 0),
    max_restart_delay_ms INTEGER NOT NULL CHECK (max_restart_delay_ms >= min_restart_delay_ms),
    escalate_on_limit BOOLEAN NOT NULL DEFAULT true,
    escalate_on_error_types TEXT[] NOT NULL DEFAULT '{}',
    escalation_timeout_seconds INTEGER NOT NULL DEFAULT 10,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(node_id),
    CONSTRAINT chk_backoff_factor CHECK (
        (backoff_strategy = 'Exponential' AND backoff_factor > 1) OR
        (backoff_strategy != 'Exponential')
    )
);

-- Failure tracking with detailed information
CREATE TABLE supervision_failures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id UUID NOT NULL REFERENCES supervision_nodes(id) ON DELETE CASCADE,
    failure_reason failure_reason NOT NULL,
    failure_detail TEXT NOT NULL,
    stack_trace TEXT,
    context JSONB DEFAULT '{}',
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    restart_attempted BOOLEAN NOT NULL DEFAULT false,
    escalated_to_parent BOOLEAN NOT NULL DEFAULT false,
    recovery_action TEXT,
    correlation_id UUID,
    CONSTRAINT chk_failure_detail_length CHECK (length(failure_detail) <= 5000)
) PARTITION BY RANGE (occurred_at);

-- Restart attempt history
CREATE TABLE supervision_restarts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id UUID NOT NULL REFERENCES supervision_nodes(id) ON DELETE CASCADE,
    failure_id UUID REFERENCES supervision_failures(id) ON DELETE CASCADE,
    restart_attempt_number INTEGER NOT NULL CHECK (restart_attempt_number > 0),
    restart_delay_ms INTEGER NOT NULL CHECK (restart_delay_ms >= 0),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    success BOOLEAN,
    error_message TEXT,
    new_instance_id UUID,
    CONSTRAINT chk_restart_completion CHECK (
        (completed_at IS NOT NULL AND success IS NOT NULL) OR
        (completed_at IS NULL AND success IS NULL)
    )
);

-- Aggregated supervision metrics
CREATE TABLE supervision_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id UUID NOT NULL REFERENCES supervision_nodes(id) ON DELETE CASCADE,
    restarts_total BIGINT NOT NULL DEFAULT 0,
    failures_total BIGINT NOT NULL DEFAULT 0,
    uptime_seconds BIGINT NOT NULL DEFAULT 0,
    last_failure_time TIMESTAMPTZ,
    child_count INTEGER NOT NULL DEFAULT 0,
    successful_tasks BIGINT NOT NULL DEFAULT 0,
    failed_tasks BIGINT NOT NULL DEFAULT 0,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metrics_window INTERVAL NOT NULL DEFAULT '1 hour'::INTERVAL,
    UNIQUE(node_id, calculated_at, metrics_window)
);

-- Current state of child processes
CREATE TABLE supervision_child_states (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id UUID NOT NULL REFERENCES supervision_nodes(id) ON DELETE CASCADE,
    child_id TEXT NOT NULL,
    child_node_id UUID REFERENCES supervision_nodes(id),
    instance_id UUID REFERENCES agent_instances(id),
    state child_state NOT NULL,
    state_data JSONB NOT NULL DEFAULT '{}',
    last_heartbeat TIMESTAMPTZ,
    error_count INTEGER NOT NULL DEFAULT 0,
    restart_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(node_id, child_id)
);

-- Task type definitions and routing rules
CREATE TABLE task_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_type task_type NOT NULL,
    custom_type_name TEXT,
    description TEXT,
    routing_rules JSONB NOT NULL DEFAULT '{}',
    required_capabilities TEXT[] NOT NULL DEFAULT '{}',
    priority_weight INTEGER NOT NULL DEFAULT 5 CHECK (priority_weight BETWEEN 1 AND 10),
    timeout_seconds INTEGER DEFAULT 300,
    retry_policy JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_custom_type CHECK (
        (task_type = 'Custom' AND custom_type_name IS NOT NULL) OR
        (task_type != 'Custom' AND custom_type_name IS NULL)
    ),
    UNIQUE(task_type, custom_type_name)
);

-- Task assignments and routing history
CREATE TABLE task_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id UUID NOT NULL UNIQUE,
    task_type task_type NOT NULL,
    custom_type_name TEXT,
    task_payload JSONB NOT NULL,
    priority INTEGER NOT NULL CHECK (priority BETWEEN 1 AND 255),
    assigned_to_node_id UUID REFERENCES supervision_nodes(id),
    assigned_by_node_id UUID REFERENCES supervision_nodes(id),
    status task_status NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assigned_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    deadline_at TIMESTAMPTZ,
    result_payload JSONB,
    error_message TEXT,
    message_id UUID REFERENCES messages(id),
    correlation_id UUID,
    CONSTRAINT chk_custom_task_type CHECK (
        (task_type = 'Custom' AND custom_type_name IS NOT NULL) OR
        (task_type != 'Custom')
    ),
    CONSTRAINT chk_assignment_timeline CHECK (
        (assigned_at IS NULL OR assigned_at >= created_at) AND
        (started_at IS NULL OR assigned_at IS NULL OR started_at >= assigned_at) AND
        (completed_at IS NULL OR started_at IS NULL OR completed_at >= started_at)
    )
) PARTITION BY RANGE (created_at);

-- Distributed coordination and locking
CREATE TABLE supervision_coordination (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    coordination_key TEXT NOT NULL,
    owner_node_id UUID REFERENCES supervision_nodes(id) ON DELETE CASCADE,
    lock_type lock_type NOT NULL,
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    heartbeat_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB DEFAULT '{}',
    release_on_owner_failure BOOLEAN NOT NULL DEFAULT true,
    CONSTRAINT chk_expiry CHECK (expires_at > acquired_at),
    UNIQUE(coordination_key, lock_type)
);

-- Supervision events log
CREATE TABLE supervision_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type supervision_event_type NOT NULL,
    source_node_id UUID REFERENCES supervision_nodes(id),
    target_node_id UUID REFERENCES supervision_nodes(id),
    task_id UUID,
    failure_id UUID REFERENCES supervision_failures(id),
    event_data JSONB NOT NULL DEFAULT '{}',
    severity TEXT CHECK (severity IN ('debug', 'info', 'warning', 'error', 'critical')),
    correlation_id UUID,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (occurred_at);

-- Create indexes for performance
CREATE INDEX idx_nodes_parent ON supervision_nodes(parent_node_id) 
    WHERE parent_node_id IS NOT NULL;
CREATE INDEX idx_nodes_status ON supervision_nodes(status) 
    WHERE status = 'active';
CREATE INDEX idx_nodes_type ON supervision_nodes(node_type, node_type_detail);
CREATE INDEX idx_nodes_agent ON supervision_nodes(agent_id) 
    WHERE agent_id IS NOT NULL;

CREATE INDEX idx_policies_node ON supervision_policies(node_id);

CREATE INDEX idx_failures_node_time ON supervision_failures(node_id, occurred_at DESC);
CREATE INDEX idx_failures_reason ON supervision_failures(failure_reason, occurred_at);
CREATE INDEX idx_failures_correlation ON supervision_failures(correlation_id) 
    WHERE correlation_id IS NOT NULL;

CREATE INDEX idx_restarts_node ON supervision_restarts(node_id, started_at DESC);
CREATE INDEX idx_restarts_failure ON supervision_restarts(failure_id) 
    WHERE failure_id IS NOT NULL;
CREATE INDEX idx_restarts_pending ON supervision_restarts(node_id) 
    WHERE completed_at IS NULL;

CREATE INDEX idx_metrics_node_time ON supervision_metrics(node_id, calculated_at DESC);
CREATE INDEX idx_metrics_window ON supervision_metrics(metrics_window, calculated_at DESC);

CREATE INDEX idx_child_states_node ON supervision_child_states(node_id);
CREATE INDEX idx_child_states_active ON supervision_child_states(node_id, state) 
    WHERE state IN ('Starting', 'Running');
CREATE INDEX idx_child_states_instance ON supervision_child_states(instance_id) 
    WHERE instance_id IS NOT NULL;

CREATE INDEX idx_task_defs_type ON task_definitions(task_type, custom_type_name);
CREATE INDEX idx_task_assignments_pending ON task_assignments(status, priority DESC) 
    WHERE status = 'pending';
CREATE INDEX idx_task_assignments_node ON task_assignments(assigned_to_node_id, status);
CREATE INDEX idx_task_assignments_deadline ON task_assignments(deadline_at) 
    WHERE deadline_at IS NOT NULL AND status IN ('pending', 'assigned', 'processing');

CREATE INDEX idx_coordination_active ON supervision_coordination(coordination_key, expires_at) 
    WHERE expires_at > NOW();
CREATE INDEX idx_coordination_owner ON supervision_coordination(owner_node_id);

CREATE INDEX idx_events_type_time ON supervision_events(event_type, occurred_at DESC);
CREATE INDEX idx_events_source ON supervision_events(source_node_id, occurred_at DESC);
CREATE INDEX idx_events_correlation ON supervision_events(correlation_id) 
    WHERE correlation_id IS NOT NULL;

-- Helper function: Calculate restart delay based on policy
CREATE OR REPLACE FUNCTION calculate_restart_delay(
    policy_id UUID,
    attempt_number INTEGER
) RETURNS INTEGER AS $$
DECLARE
    policy RECORD;
    delay_ms INTEGER;
BEGIN
    SELECT * INTO policy FROM supervision_policies WHERE id = policy_id;
    
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Policy not found: %', policy_id;
    END IF;
    
    CASE policy.backoff_strategy
        WHEN 'Fixed' THEN
            delay_ms := policy.min_restart_delay_ms;
        WHEN 'Linear' THEN
            delay_ms := policy.min_restart_delay_ms + (1000 * attempt_number);
        WHEN 'Exponential' THEN
            delay_ms := policy.min_restart_delay_ms * POWER(policy.backoff_factor, attempt_number - 1);
    END CASE;
    
    RETURN LEAST(delay_ms, policy.max_restart_delay_ms);
END;
$$ LANGUAGE plpgsql;

-- Helper function: Check if restart should be allowed
CREATE OR REPLACE FUNCTION should_restart(
    node_uuid UUID
) RETURNS BOOLEAN AS $$
DECLARE
    policy RECORD;
    recent_failures INTEGER;
BEGIN
    SELECT p.* INTO policy 
    FROM supervision_policies p
    JOIN supervision_nodes n ON n.id = p.node_id
    WHERE n.id = node_uuid;
    
    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;
    
    SELECT COUNT(*) INTO recent_failures
    FROM supervision_failures
    WHERE node_id = node_uuid
      AND occurred_at > NOW() - (policy.time_window_seconds * INTERVAL '1 second');
    
    RETURN recent_failures < policy.max_restarts;
END;
$$ LANGUAGE plpgsql;

-- Helper function: Get supervision tree hierarchy
CREATE OR REPLACE FUNCTION get_supervision_tree(
    root_node_id UUID DEFAULT NULL
) RETURNS TABLE(
    node_id UUID,
    node_name TEXT,
    node_type node_type,
    parent_id UUID,
    level INTEGER,
    path TEXT[]
) AS $$
WITH RECURSIVE tree AS (
    SELECT 
        n.id as node_id,
        n.node_id as node_name,
        n.node_type,
        n.parent_node_id as parent_id,
        0 as level,
        ARRAY[n.node_id] as path
    FROM supervision_nodes n
    WHERE (root_node_id IS NULL AND n.parent_node_id IS NULL)
       OR (root_node_id IS NOT NULL AND n.id = root_node_id)
    
    UNION ALL
    
    SELECT 
        n.id,
        n.node_id,
        n.node_type,
        n.parent_node_id,
        t.level + 1,
        t.path || n.node_id
    FROM supervision_nodes n
    JOIN tree t ON n.parent_node_id = t.node_id
)
SELECT * FROM tree ORDER BY path;
$$ LANGUAGE plpgsql;

-- Helper function: Assign task to best available worker
CREATE OR REPLACE FUNCTION assign_task_to_worker(
    task_uuid UUID,
    supervisor_uuid UUID
) RETURNS UUID AS $$
DECLARE
    selected_worker_id UUID;
    task_record RECORD;
BEGIN
    SELECT * INTO task_record FROM task_assignments WHERE id = task_uuid;
    
    IF NOT FOUND OR task_record.status != 'pending' THEN
        RAISE EXCEPTION 'Task not found or not pending: %', task_uuid;
    END IF;
    
    SELECT cs.node_id INTO selected_worker_id
    FROM supervision_child_states cs
    JOIN supervision_nodes n ON n.id = cs.node_id
    LEFT JOIN (
        SELECT assigned_to_node_id, COUNT(*) as active_tasks
        FROM task_assignments
        WHERE status IN ('assigned', 'processing')
        GROUP BY assigned_to_node_id
    ) workload ON workload.assigned_to_node_id = cs.node_id
    WHERE cs.node_id IN (
        SELECT id FROM supervision_nodes 
        WHERE parent_node_id = supervisor_uuid
          AND node_type = 'Worker'
          AND status = 'active'
    )
    AND cs.state = 'Running'
    ORDER BY 
        COALESCE(workload.active_tasks, 0) ASC,
        cs.error_count ASC,
        RANDOM()
    LIMIT 1;
    
    IF selected_worker_id IS NULL THEN
        RETURN NULL;
    END IF;
    
    UPDATE task_assignments
    SET assigned_to_node_id = selected_worker_id,
        assigned_by_node_id = supervisor_uuid,
        assigned_at = NOW(),
        status = 'assigned'
    WHERE id = task_uuid;
    
    RETURN selected_worker_id;
END;
$$ LANGUAGE plpgsql;

-- Create materialized view for hourly metrics
CREATE MATERIALIZED VIEW supervision_metrics_hourly AS
SELECT 
    n.id as node_id,
    n.node_id as node_name,
    n.node_type,
    date_trunc('hour', f.occurred_at) as hour,
    COUNT(DISTINCT f.id) as failure_count,
    COUNT(DISTINCT r.id) as restart_count,
    COUNT(DISTINCT CASE WHEN r.success THEN r.id END) as successful_restarts,
    AVG(r.restart_delay_ms) as avg_restart_delay_ms,
    COUNT(DISTINCT t.id) as tasks_processed,
    COUNT(DISTINCT CASE WHEN t.status = 'completed' THEN t.id END) as tasks_completed,
    COUNT(DISTINCT CASE WHEN t.status = 'failed' THEN t.id END) as tasks_failed
FROM supervision_nodes n
LEFT JOIN supervision_failures f ON f.node_id = n.id
LEFT JOIN supervision_restarts r ON r.node_id = n.id
LEFT JOIN task_assignments t ON t.assigned_to_node_id = n.id
GROUP BY n.id, n.node_id, n.node_type, date_trunc('hour', f.occurred_at);

CREATE INDEX idx_metrics_hourly_node_time ON supervision_metrics_hourly(node_id, hour DESC);

-- Create triggers for automatic timestamp updates
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_supervision_nodes_updated_at 
    BEFORE UPDATE ON supervision_nodes
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_supervision_policies_updated_at 
    BEFORE UPDATE ON supervision_policies
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_supervision_child_states_updated_at 
    BEFORE UPDATE ON supervision_child_states
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_task_definitions_updated_at 
    BEFORE UPDATE ON task_definitions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Sync child state from agent instance updates
CREATE OR REPLACE FUNCTION sync_child_state_from_instance()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE supervision_child_states
    SET state = CASE
        WHEN NEW.status IN ('starting') THEN 'Starting'::child_state
        WHEN NEW.status IN ('running') THEN 'Running'::child_state
        WHEN NEW.status IN ('stopped', 'stopping') THEN 'Stopped'::child_state
        WHEN NEW.status IN ('error', 'crashed') THEN 'Failed'::child_state
    END,
    last_heartbeat = NEW.last_heartbeat
    WHERE instance_id = NEW.id;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER sync_instance_to_child_state
    AFTER UPDATE OF status, last_heartbeat ON agent_instances
    FOR EACH ROW
    EXECUTE FUNCTION sync_child_state_from_instance();

-- Create initial partitions for time-based tables
CREATE TABLE supervision_failures_2025_q1 PARTITION OF supervision_failures
    FOR VALUES FROM ('2025-01-01') TO ('2025-04-01');
    
CREATE TABLE supervision_failures_2025_q2 PARTITION OF supervision_failures
    FOR VALUES FROM ('2025-04-01') TO ('2025-07-01');

CREATE TABLE task_assignments_2025_01 PARTITION OF task_assignments
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
    
CREATE TABLE task_assignments_2025_02 PARTITION OF task_assignments
    FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');

CREATE TABLE supervision_events_2025_01 PARTITION OF supervision_events
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');

-- Integrate with existing schema
ALTER TABLE agents ADD COLUMN IF NOT EXISTS supervision_node_id UUID 
    REFERENCES supervision_nodes(id);

ALTER TABLE messages ADD COLUMN IF NOT EXISTS supervision_context JSONB DEFAULT '{}';

-- Create unified view for agent-supervision status
CREATE VIEW agent_supervision_status AS
SELECT 
    a.id as agent_id,
    a.name as agent_name,
    a.agent_type,
    a.status as agent_status,
    sn.id as supervision_node_id,
    sn.node_id as supervision_node_name,
    sn.status as supervision_status,
    cs.state as child_state,
    cs.last_heartbeat,
    cs.restart_count,
    ai.id as instance_id,
    ai.status as instance_status,
    ai.host_name,
    ai.port
FROM agents a
LEFT JOIN supervision_nodes sn ON sn.agent_id = a.id
LEFT JOIN supervision_child_states cs ON cs.child_node_id = sn.id
LEFT JOIN agent_instances ai ON ai.id = cs.instance_id;

-- Insert migration record
INSERT INTO schema_versions (version, description) 
VALUES (3, 'Add supervision persistence for Phase 3');

COMMIT;