# Aurora Serverless v2 Resource Sizing

## Database Requirements Analysis

Based on MisterSmith's data patterns and agent coordination needs, we calculate Aurora Serverless v2 requirements.

## Data Volume Estimates

### 1. Agent State Storage
```sql
-- Agent state table
CREATE TABLE agent_states (
    agent_id UUID PRIMARY KEY,
    state JSONB NOT NULL,
    version INTEGER NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Size calculation:
-- Average state size: 1 KB
-- Updates per agent per hour: 60
-- Retention: 30 days
-- Total for 1000 agents: 43.2 GB/month
```

### 2. Discovery Events
```sql
-- Discovery events table
CREATE TABLE discovery_events (
    event_id UUID PRIMARY KEY,
    agent_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    confidence DECIMAL(3,2),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    INDEX idx_agent_time (agent_id, created_at)
);

-- Size calculation:
-- Average event size: 2 KB
-- Events per agent per hour: 10
-- Retention: 90 days
-- Total for 1000 agents: 43.2 GB/month
```

### 3. Task Execution History
```sql
-- Task history table
CREATE TABLE task_history (
    task_id UUID PRIMARY KEY,
    agent_id UUID NOT NULL,
    task_type VARCHAR(100),
    input JSONB,
    output JSONB,
    duration_ms INTEGER,
    status VARCHAR(20),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    INDEX idx_agent_task (agent_id, created_at)
);

-- Size calculation:
-- Average record size: 5 KB
-- Tasks per agent per day: 100
-- Retention: 7 days
-- Total for 1000 agents: 3.5 GB
```

### 4. Metrics and Telemetry
```sql
-- Metrics table (partitioned by day)
CREATE TABLE metrics (
    metric_id UUID,
    agent_id UUID,
    metric_name VARCHAR(100),
    value DECIMAL(20,5),
    tags JSONB,
    timestamp TIMESTAMPTZ,
    PRIMARY KEY (agent_id, timestamp, metric_id)
) PARTITION BY RANGE (timestamp);

-- Size calculation:
-- Average metric size: 500 bytes
-- Metrics per agent per minute: 10
-- Retention: 30 days
-- Total for 1000 agents: 216 GB/month
```

## ACU (Aurora Capacity Unit) Sizing

### ACU Capacity Planning
```yaml
Each ACU provides:
- Memory: ~2 GB
- CPU: ~2 vCPU equivalent
- Network: ~90 MB/s
- Connections: ~30
```

### Environment Configurations

#### Development Environment
```yaml
aurora_dev:
  min_acu: 0.5
  max_acu: 2
  scale_points:
    - 0.5  # Idle
    - 1    # Light load
    - 2    # Testing
  
  expected_load:
    agents: 1-10
    connections: 1-30
    storage: 10 GB
    iops: 500
```

#### Staging Environment
```yaml
aurora_staging:
  min_acu: 1
  max_acu: 8
  scale_points:
    - 1    # Baseline
    - 2    # Normal
    - 4    # Load testing
    - 8    # Stress testing
    
  expected_load:
    agents: 10-100
    connections: 30-240
    storage: 50 GB
    iops: 2000
```

#### Production Environment
```yaml
aurora_production:
  min_acu: 2
  max_acu: 32
  scale_points:
    - 2    # Night/weekend
    - 4    # Low traffic
    - 8    # Normal traffic
    - 16   # Peak traffic
    - 32   # Extreme peak
    
  expected_load:
    agents: 100-1000
    connections: 60-960
    storage: 500 GB
    iops: 10000
```

## Connection Pool Sizing

### Per-Service Connection Requirements
```yaml
connection_pools:
  supervisor:
    min_connections: 5
    max_connections: 20
    services: 2  # Primary + standby
    total: 40
    
  worker:
    min_connections: 2
    max_connections: 10
    services: 20  # Scaled out
    total: 200
    
  gateway:
    min_connections: 10
    max_connections: 30
    services: 5   # Behind ALB
    total: 150
    
  total_connections: 390
  required_acu: 13  # 390 / 30 per ACU
```

### RDS Proxy Configuration
```yaml
rds_proxy:
  engine_family: POSTGRESQL
  auth:
    auth_scheme: SECRETS
    secret_arn: ${aurora_secret_arn}
    
  target:
    db_cluster_identifier: mistersmith-aurora
    
  connection_pool_config:
    max_connections_percent: 80
    max_idle_connections_percent: 50
    connection_borrow_timeout: 120
    session_pinning_filters: []
    
  benefits:
    - Connection multiplexing
    - Automatic failover
    - IAM authentication
    - Reduced connection overhead
```

## Storage and IOPS Calculations

### Storage Growth Projection
```python
def calculate_storage_growth(agents, months):
    # Base storage
    base_storage_gb = 10
    
    # Per agent per month
    agent_state_gb = 0.0432 * agents
    discovery_events_gb = 0.0432 * agents
    task_history_gb = 0.105 * agents  # 7-day rotation
    metrics_gb = 0.216 * agents
    
    # Total monthly growth
    monthly_growth = agent_state_gb + discovery_events_gb + \
                    task_history_gb + metrics_gb
    
    # Calculate total
    total_storage = base_storage_gb + (monthly_growth * months)
    
    return {
        'initial': base_storage_gb,
        'monthly_growth': monthly_growth,
        'total_after_months': total_storage,
        'recommended_provision': total_storage * 1.5  # 50% headroom
    }

# Example: 1000 agents for 12 months
# Result: 10 GB initial, 407 GB/month growth, 4.9 TB total, 7.3 TB recommended
```

### IOPS Requirements
```yaml
iops_baseline:
  read_write_ratio: 70:30
  
  operations:
    agent_state_update: 1 write + 2 reads
    discovery_event: 1 write + 5 reads
    task_execution: 2 writes + 10 reads
    metric_insert: 1 write (batched)
    
  per_agent_per_second:
    writes: 0.5
    reads: 2.0
    total: 2.5
    
  cluster_totals:
    100_agents: 250 IOPS
    500_agents: 1,250 IOPS
    1000_agents: 2,500 IOPS
    
  recommended_provision:
    baseline: 3,000 IOPS
    burst: 10,000 IOPS
```

## Backup and Recovery

### Backup Configuration
```yaml
backup_policy:
  automated_backups:
    retention_period: 7 days
    backup_window: "03:00-04:00"  # UTC
    
  point_in_time_recovery:
    enabled: true
    retention: 7 days
    
  snapshots:
    frequency: daily
    retention:
      daily: 7
      weekly: 4
      monthly: 3
    
  cross_region:
    enabled: true
    target_region: us-east-1  # DR region
```

### Recovery Objectives
```yaml
recovery_targets:
  rpo: 5 minutes    # Recovery Point Objective
  rto: 30 minutes   # Recovery Time Objective
  
  strategies:
    - Automated backups for point-in-time recovery
    - Cross-region snapshots for disaster recovery
    - Aurora Global Database for <1 minute RTO
```

## Performance Optimization

### 1. Query Optimization
```sql
-- Partitioning for time-series data
CREATE TABLE metrics_2024_01 PARTITION OF metrics
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');

-- Indexes for common queries
CREATE INDEX CONCURRENTLY idx_agent_state_updated 
    ON agent_states(updated_at) WHERE state->>'status' = 'active';

CREATE INDEX CONCURRENTLY idx_discovery_confidence 
    ON discovery_events(confidence DESC) 
    WHERE confidence > 0.8;
```

### 2. Connection Optimization
```yaml
connection_parameters:
  # PgBouncer-style pooling via RDS Proxy
  pool_mode: transaction
  
  # Connection lifecycle
  idle_in_transaction_timeout: 60s
  statement_timeout: 30s
  
  # Performance tuning
  shared_buffers: 75%_of_memory
  effective_cache_size: 90%_of_memory
  work_mem: 256MB
  maintenance_work_mem: 1GB
```

### 3. Monitoring Queries
```sql
-- Monitor long-running queries
SELECT pid, now() - query_start AS duration, query
FROM pg_stat_activity
WHERE state != 'idle' 
  AND now() - query_start > interval '1 minute'
ORDER BY duration DESC;

-- Check connection usage
SELECT 
    datname,
    count(*) as connections,
    count(*) FILTER (WHERE state = 'active') as active,
    count(*) FILTER (WHERE state = 'idle') as idle
FROM pg_stat_activity
GROUP BY datname;
```

## Cost Estimation

### Aurora Serverless v2 Pricing
```yaml
cost_breakdown:
  acu_hour: $0.12
  
  monthly_costs:
    development:
      avg_acu: 1
      hours: 730
      cost: $87.60
      
    staging:
      avg_acu: 3
      hours: 730
      cost: $262.80
      
    production:
      avg_acu: 12  # With scaling
      hours: 730
      cost: $1,051.20
      
  storage_costs:
    rate: $0.10 per GB-month
    backup: $0.021 per GB-month
    
  data_transfer:
    same_az: $0
    cross_az: $0.01 per GB
    internet: $0.09 per GB
```

### Total Monthly Estimate
```yaml
production_total:
  compute: $1,051.20
  storage_500gb: $50.00
  backups: $10.50
  rds_proxy: $50.00
  data_transfer: $100.00
  total: $1,261.70
  
with_reserved_discount:
  1_year_commitment: $883.19 (30% savings)
  3_year_commitment: $693.94 (45% savings)
```