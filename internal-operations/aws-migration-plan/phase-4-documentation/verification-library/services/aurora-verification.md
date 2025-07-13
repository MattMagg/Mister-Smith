# Aurora Database Verification Commands

## Overview
Commands for verifying Aurora PostgreSQL cluster health, performance, and configuration.

## Prerequisites
- AWS CLI with RDS permissions
- PostgreSQL client (psql)
- jq for JSON parsing
- Performance Insights enabled (recommended)

## Commands

### 1. Cluster Status Verification
```bash
# Get Aurora cluster status
aws rds describe-db-clusters \
  --db-cluster-identifier mistersmith-aurora-cluster \
  --query 'DBClusters[0].[DBClusterIdentifier,Status,Engine,EngineVersion,MultiAZ,DBClusterMembers[*].DBInstanceIdentifier]' \
  --output table

# Check cluster endpoints
aws rds describe-db-clusters \
  --db-cluster-identifier mistersmith-aurora-cluster \
  --query 'DBClusters[0].[Endpoint,ReaderEndpoint,CustomEndpoints[*].Endpoint]' \
  --output table

# Verify cluster configuration
function verify_aurora_config() {
    local cluster_id=$1
    aws rds describe-db-clusters \
      --db-cluster-identifier $cluster_id \
      --query 'DBClusters[0]' > /tmp/cluster-config.json
    
    echo "=== Aurora Cluster Configuration ==="
    jq -r '
      "Cluster: \(.DBClusterIdentifier)",
      "Status: \(.Status)",
      "Engine: \(.Engine) \(.EngineVersion)",
      "Storage Encrypted: \(.StorageEncrypted)",
      "Backup Retention: \(.BackupRetentionPeriod) days",
      "Multi-AZ: \(.MultiAZ)",
      "Deletion Protection: \(.DeletionProtection)"
    ' /tmp/cluster-config.json
}
```

### 2. Instance Health Checks
```bash
# List all cluster instances
aws rds describe-db-instances \
  --filters "Name=db-cluster-id,Values=mistersmith-aurora-cluster" \
  --query 'DBInstances[].[DBInstanceIdentifier,DBInstanceStatus,DBInstanceClass,AvailabilityZone,PromotionTier]' \
  --output table

# Check instance metrics
function check_instance_metrics() {
    local instance_id=$1
    local metric=$2  # CPUUtilization, DatabaseConnections, ReadLatency, WriteLatency
    
    aws cloudwatch get-metric-statistics \
      --namespace AWS/RDS \
      --metric-name $metric \
      --dimensions Name=DBInstanceIdentifier,Value=$instance_id \
      --start-time $(date -u -d '30 minutes ago' +%Y-%m-%dT%H:%M:%S) \
      --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
      --period 300 \
      --statistics Average,Maximum \
      --output json | jq '.Datapoints | sort_by(.Timestamp) | .[-6:]'
}

# Check all instances in cluster
function check_cluster_instances() {
    local cluster_id=$1
    aws rds describe-db-instances \
      --filters "Name=db-cluster-id,Values=$cluster_id" \
      --query 'DBInstances[].DBInstanceIdentifier' \
      --output text | tr '\t' '\n' | while read instance; do
        echo "Checking instance: $instance"
        check_instance_metrics $instance "CPUUtilization"
        check_instance_metrics $instance "DatabaseConnections"
    done
}
```

### 3. Database Connection Tests
```bash
# Test writer endpoint connectivity
function test_writer_connection() {
    local cluster_endpoint=$1
    local db_name=$2
    local master_username=$3
    
    PGPASSWORD=$(aws secretsmanager get-secret-value \
      --secret-id mistersmith/aurora/master \
      --query 'SecretString' \
      --output text | jq -r '.password')
    
    psql -h $cluster_endpoint -U $master_username -d $db_name -c "SELECT version();" || echo "Connection failed"
}

# Test reader endpoint connectivity
function test_reader_connection() {
    local reader_endpoint=$1
    local db_name=$2
    local username=$3
    
    psql -h $reader_endpoint -U $username -d $db_name \
      -c "SELECT pg_is_in_recovery();" || echo "Reader connection failed"
}

# Test connection pooling
function test_connection_pool() {
    local endpoint=$1
    local max_connections=50
    
    echo "Testing connection pool with $max_connections connections..."
    for i in $(seq 1 $max_connections); do
        (psql -h $endpoint -U app_user -d mistersmith -c "SELECT 1;" &) 2>/dev/null
    done
    wait
    echo "Connection pool test completed"
}
```

### 4. Replication Health
```bash
# Check replication lag
function check_replication_lag() {
    local reader_instance=$1
    
    aws cloudwatch get-metric-statistics \
      --namespace AWS/RDS \
      --metric-name AuroraReplicaLag \
      --dimensions Name=DBInstanceIdentifier,Value=$reader_instance \
      --start-time $(date -u -d '10 minutes ago' +%Y-%m-%dT%H:%M:%S) \
      --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
      --period 60 \
      --statistics Average,Maximum \
      --output json | jq '.Datapoints | sort_by(.Timestamp) | .[-1]'
}

# Monitor replication status via SQL
function check_replication_sql() {
    local reader_endpoint=$1
    psql -h $reader_endpoint -U monitor_user -d postgres -t -c "
    SELECT 
        pg_is_in_recovery() as is_replica,
        pg_last_xact_replay_timestamp() as last_replay,
        EXTRACT(EPOCH FROM (now() - pg_last_xact_replay_timestamp()))::int as lag_seconds
    ;"
}
```

### 5. Performance Insights
```bash
# Get top SQL statements
function get_top_sql() {
    local instance_id=$1
    local period_minutes=${2:-60}
    
    end_time=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    start_time=$(date -u -d "$period_minutes minutes ago" +%Y-%m-%dT%H:%M:%SZ)
    
    aws pi get-resource-metrics \
      --service-type RDS \
      --identifier "arn:aws:rds:region:account:db:$instance_id" \
      --start-time $start_time \
      --end-time $end_time \
      --period-in-seconds 300 \
      --metric-queries '[
        {
          "Metric": "db.SQL.Innodb_rows_read.avg",
          "GroupBy": {"Group": "db.SQL_TOKENIZED"}
        }
      ]' \
      --query 'MetricList[0].DataPoints[*].[Timestamp,Value]' \
      --output table
}

# Check database load
function check_db_load() {
    local instance_id=$1
    
    aws cloudwatch get-metric-statistics \
      --namespace AWS/RDS \
      --metric-name DBLoad \
      --dimensions Name=DBInstanceIdentifier,Value=$instance_id \
      --start-time $(date -u -d '1 hour ago' +%Y-%m-%dT%H:%M:%S) \
      --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
      --period 300 \
      --statistics Average \
      --output json | jq '.Datapoints | sort_by(.Timestamp)'
}
```

### 6. Backup Verification
```bash
# List recent backups
aws rds describe-db-cluster-snapshots \
  --db-cluster-identifier mistersmith-aurora-cluster \
  --query 'DBClusterSnapshots[].[DBClusterSnapshotIdentifier,SnapshotCreateTime,Status,SnapshotType]' \
  --output table | head -20

# Verify automated backup window
aws rds describe-db-clusters \
  --db-cluster-identifier mistersmith-aurora-cluster \
  --query 'DBClusters[0].[PreferredBackupWindow,BackupRetentionPeriod,LatestRestorableTime]' \
  --output table

# Test point-in-time recovery capability
function verify_pitr() {
    local cluster_id=$1
    local latest_restorable=$(aws rds describe-db-clusters \
      --db-cluster-identifier $cluster_id \
      --query 'DBClusters[0].LatestRestorableTime' \
      --output text)
    
    echo "Latest restorable time: $latest_restorable"
    echo "Current time: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    
    # Calculate recovery point objective (RPO)
    current_epoch=$(date +%s)
    restorable_epoch=$(date -d "$latest_restorable" +%s)
    rpo_seconds=$((current_epoch - restorable_epoch))
    echo "Recovery Point Objective (RPO): $rpo_seconds seconds"
}
```

### 7. Security Verification
```bash
# Check encryption status
aws rds describe-db-clusters \
  --db-cluster-identifier mistersmith-aurora-cluster \
  --query 'DBClusters[0].[StorageEncrypted,KmsKeyId]' \
  --output table

# Verify network security
function verify_network_security() {
    local cluster_id=$1
    
    # Get security groups
    aws rds describe-db-clusters \
      --db-cluster-identifier $cluster_id \
      --query 'DBClusters[0].VpcSecurityGroups[*].VpcSecurityGroupId' \
      --output text | tr '\t' '\n' | while read sg_id; do
        echo "Security Group: $sg_id"
        aws ec2 describe-security-groups \
          --group-ids $sg_id \
          --query 'SecurityGroups[0].IpPermissions[].[IpProtocol,FromPort,ToPort,IpRanges[0].CidrIp]' \
          --output table
    done
}

# Check SSL/TLS enforcement
function verify_ssl_enforcement() {
    local cluster_id=$1
    
    # Check parameter group
    param_group=$(aws rds describe-db-clusters \
      --db-cluster-identifier $cluster_id \
      --query 'DBClusters[0].DBClusterParameterGroup' \
      --output text)
    
    aws rds describe-db-cluster-parameters \
      --db-cluster-parameter-group-name $param_group \
      --query 'Parameters[?ParameterName==`rds.force_ssl`].[ParameterName,ParameterValue]' \
      --output table
}
```

### 8. Query Performance Analysis
```bash
#!/bin/bash
# aurora-query-analysis.sh - Analyze query performance

function analyze_slow_queries() {
    local endpoint=$1
    local threshold_ms=${2:-1000}
    
    psql -h $endpoint -U dba_user -d mistersmith << EOF
    -- Top 10 slowest queries
    SELECT 
        query,
        calls,
        total_time::bigint as total_ms,
        mean_time::bigint as avg_ms,
        max_time::bigint as max_ms
    FROM pg_stat_statements
    WHERE mean_time > $threshold_ms
    ORDER BY mean_time DESC
    LIMIT 10;
EOF
}

# Check for blocking queries
function check_blocking_queries() {
    local endpoint=$1
    
    psql -h $endpoint -U dba_user -d mistersmith << EOF
    SELECT 
        pid,
        usename,
        application_name,
        client_addr,
        backend_start,
        state,
        wait_event,
        query
    FROM pg_stat_activity
    WHERE wait_event IS NOT NULL
    AND state != 'idle'
    ORDER BY backend_start;
EOF
}

# Analyze table statistics
function analyze_table_stats() {
    local endpoint=$1
    
    psql -h $endpoint -U dba_user -d mistersmith << EOF
    SELECT 
        schemaname,
        tablename,
        n_live_tup as live_rows,
        n_dead_tup as dead_rows,
        last_vacuum,
        last_analyze
    FROM pg_stat_user_tables
    WHERE n_dead_tup > 1000
    ORDER BY n_dead_tup DESC;
EOF
}
```

### 9. Comprehensive Aurora Health Report
```bash
#!/bin/bash
# aurora-health-report.sh - Generate comprehensive Aurora health report

function generate_aurora_report() {
    local cluster_id=$1
    local report_dir="aurora-health-$(date +%Y%m%d-%H%M%S)"
    
    mkdir -p $report_dir
    
    echo "Generating Aurora health report for: $cluster_id"
    
    # Cluster overview
    aws rds describe-db-clusters \
      --db-cluster-identifier $cluster_id \
      --output json > $report_dir/cluster-config.json
    
    # Instance details
    aws rds describe-db-instances \
      --filters "Name=db-cluster-id,Values=$cluster_id" \
      --output json > $report_dir/instances.json
    
    # Recent events
    aws rds describe-events \
      --source-identifier $cluster_id \
      --source-type db-cluster \
      --duration 1440 \
      --output json > $report_dir/events.json
    
    # Performance metrics
    instances=$(jq -r '.DBInstances[].DBInstanceIdentifier' $report_dir/instances.json)
    for instance in $instances; do
        echo "Collecting metrics for $instance..."
        
        # CPU metrics
        aws cloudwatch get-metric-statistics \
          --namespace AWS/RDS \
          --metric-name CPUUtilization \
          --dimensions Name=DBInstanceIdentifier,Value=$instance \
          --start-time $(date -u -d '24 hours ago' +%Y-%m-%dT%H:%M:%S) \
          --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
          --period 3600 \
          --statistics Average,Maximum \
          --output json > $report_dir/${instance}-cpu.json
        
        # Connection metrics
        aws cloudwatch get-metric-statistics \
          --namespace AWS/RDS \
          --metric-name DatabaseConnections \
          --dimensions Name=DBInstanceIdentifier,Value=$instance \
          --start-time $(date -u -d '24 hours ago' +%Y-%m-%dT%H:%M:%S) \
          --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
          --period 3600 \
          --statistics Average,Maximum \
          --output json > $report_dir/${instance}-connections.json
    done
    
    # Generate summary
    cat > $report_dir/summary.md << EOF
# Aurora Health Report
Date: $(date)
Cluster: $cluster_id

## Cluster Status
$(jq -r '.DBClusters[0] | "- Status: \(.Status)\n- Engine: \(.Engine) \(.EngineVersion)\n- Multi-AZ: \(.MultiAZ)\n- Storage Encrypted: \(.StorageEncrypted)"' $report_dir/cluster-config.json)

## Instance Summary
$(jq -r '.DBInstances[] | "### \(.DBInstanceIdentifier)\n- Status: \(.DBInstanceStatus)\n- Class: \(.DBInstanceClass)\n- Zone: \(.AvailabilityZone)"' $report_dir/instances.json)

## Recent Events
$(jq -r '.Events[0:5][] | "- [\(.Date)] \(.Message)"' $report_dir/events.json)

## Performance Summary
See individual metric files for details.
EOF
    
    echo "Report generated in: $report_dir/"
}

# Run report generation
generate_aurora_report "mistersmith-aurora-cluster"
```

### 10. Automated Health Monitoring
```bash
#!/bin/bash
# aurora-monitor.sh - Continuous Aurora monitoring

function monitor_aurora() {
    local cluster_id=$1
    local interval=${2:-300}  # 5 minutes default
    
    while true; do
        clear
        echo "=== Aurora Cluster Monitor: $cluster_id ==="
        echo "Time: $(date)"
        echo ""
        
        # Cluster status
        aws rds describe-db-clusters \
          --db-cluster-identifier $cluster_id \
          --query 'DBClusters[0].[Status,DBClusterMembers[*].DBInstanceIdentifier]' \
          --output text
        
        echo ""
        echo "Instance Status:"
        aws rds describe-db-instances \
          --filters "Name=db-cluster-id,Values=$cluster_id" \
          --query 'DBInstances[].[DBInstanceIdentifier,DBInstanceStatus]' \
          --output text | while read instance status; do
            echo "  $instance: $status"
            
            # Get key metrics
            cpu=$(aws cloudwatch get-metric-statistics \
              --namespace AWS/RDS \
              --metric-name CPUUtilization \
              --dimensions Name=DBInstanceIdentifier,Value=$instance \
              --start-time $(date -u -d '5 minutes ago' +%Y-%m-%dT%H:%M:%S) \
              --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
              --period 300 \
              --statistics Average \
              --query 'Datapoints[0].Average' \
              --output text)
            
            connections=$(aws cloudwatch get-metric-statistics \
              --namespace AWS/RDS \
              --metric-name DatabaseConnections \
              --dimensions Name=DBInstanceIdentifier,Value=$instance \
              --start-time $(date -u -d '5 minutes ago' +%Y-%m-%dT%H:%M:%S) \
              --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
              --period 300 \
              --statistics Average \
              --query 'Datapoints[0].Average' \
              --output text)
            
            printf "    CPU: %.1f%% | Connections: %.0f\n" $cpu $connections
        done
        
        echo ""
        echo "Press Ctrl+C to exit. Refreshing in $interval seconds..."
        sleep $interval
    done
}

# Start monitoring
monitor_aurora "mistersmith-aurora-cluster"
```

## Error Recovery

### Connection Failures
```bash
# Test connectivity from different sources
for endpoint in writer-endpoint reader-endpoint; do
    echo "Testing $endpoint..."
    nc -zv $endpoint 5432 || echo "Port 5432 not accessible"
done
```

### Performance Issues
```bash
# Kill long-running queries
psql -h cluster-endpoint -U dba_user -d postgres -c "
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE state = 'active'
AND query_start < now() - interval '30 minutes'
AND query NOT LIKE '%pg_stat_activity%';"
```