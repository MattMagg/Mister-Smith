# MisterSmith AWS Migration Rollback Procedures
## Sub-Phase 3.2: Comprehensive Rollback Strategies

### Overview
This document provides detailed rollback procedures for each component of the MisterSmith AWS migration, ensuring system stability and data integrity during any rollback scenario.

---

## 1. Rollback Triggers and Thresholds

### 1.1 Automatic Triggers
```yaml
health_check_failures:
  threshold: 3 consecutive failures
  interval: 30 seconds
  action: initiate_component_rollback

performance_degradation:
  response_time: >5 seconds for 5 minutes
  error_rate: >5% for 2 minutes
  cpu_usage: >90% for 10 minutes
  memory_usage: >95% for 5 minutes
  action: evaluate_and_rollback

connectivity_issues:
  database_timeout: >30 seconds
  nats_disconnection: >60 seconds
  service_unreachable: >3 attempts
  action: immediate_rollback
```

### 1.2 Manual Rollback Commands
```bash
# Global rollback command
./rollback-all.sh --environment production --confirm

# Component-specific rollback
./rollback-component.sh --service orchestrator --version previous
./rollback-component.sh --service webui --version previous
./rollback-component.sh --database postgres --snapshot latest
```

---

## 2. State Preservation Procedures

### 2.1 Pre-Migration Snapshots
```bash
#!/bin/bash
# snapshot-state.sh

# Configuration backup
aws s3 cp ~/.mistersmith/config.json s3://mistersmith-backups/pre-migration/config-$(date +%Y%m%d-%H%M%S).json

# Database snapshot
pg_dump -h localhost -U mistersmith -d mistersmith > mistersmith-backup-$(date +%Y%m%d-%H%M%S).sql
aws s3 cp mistersmith-backup-*.sql s3://mistersmith-backups/pre-migration/

# Container images
docker save mistersmith/orchestrator:current | gzip > orchestrator-backup.tar.gz
docker save mistersmith/webui:current | gzip > webui-backup.tar.gz
docker save mistersmith/nats:current | gzip > nats-backup.tar.gz

# NATS message state
nats stream backup MisterSmith-Tasks ./nats-backup/
nats stream backup MisterSmith-Results ./nats-backup/
tar -czf nats-state-$(date +%Y%m%d-%H%M%S).tar.gz ./nats-backup/
aws s3 cp nats-state-*.tar.gz s3://mistersmith-backups/pre-migration/
```

### 2.2 Configuration Versioning
```json
{
  "version": "1.0.0-pre-migration",
  "timestamp": "2025-01-15T10:00:00Z",
  "checksum": "sha256:abc123...",
  "components": {
    "orchestrator": {
      "image": "mistersmith/orchestrator:v1.0.0",
      "config_hash": "sha256:def456..."
    },
    "webui": {
      "image": "mistersmith/webui:v1.0.0",
      "config_hash": "sha256:ghi789..."
    }
  }
}
```

---

## 3. Component-Specific Rollback Procedures

### 3.1 Orchestrator Service Rollback
```bash
#!/bin/bash
# rollback-orchestrator.sh

echo "Starting Orchestrator rollback..."

# Stop current service
docker stop mistersmith-orchestrator || true
docker rm mistersmith-orchestrator || true

# Restore previous version
docker load < orchestrator-backup.tar.gz

# Restore configuration
aws s3 cp s3://mistersmith-backups/pre-migration/orchestrator-config-latest.json ~/.mistersmith/orchestrator/config.json

# Start previous version with health check
docker run -d \
  --name mistersmith-orchestrator \
  --restart unless-stopped \
  -v ~/.mistersmith/orchestrator:/app/config \
  -p 50051:50051 \
  --health-cmd="grpc_health_probe -addr=:50051" \
  --health-interval=30s \
  --health-retries=3 \
  --health-timeout=10s \
  mistersmith/orchestrator:previous

# Verify rollback
for i in {1..10}; do
  if docker exec mistersmith-orchestrator grpc_health_probe -addr=:50051; then
    echo "Orchestrator rollback successful"
    break
  fi
  sleep 5
done
```

### 3.2 Web UI Rollback
```bash
#!/bin/bash
# rollback-webui.sh

echo "Starting Web UI rollback..."

# Stop current service
docker stop mistersmith-webui || true
docker rm mistersmith-webui || true

# Restore previous version
docker load < webui-backup.tar.gz

# Restore Nginx configuration
aws s3 cp s3://mistersmith-backups/pre-migration/nginx.conf /etc/nginx/sites-available/mistersmith

# Start previous version
docker run -d \
  --name mistersmith-webui \
  --restart unless-stopped \
  -p 3000:3000 \
  -e REACT_APP_API_URL=http://localhost:50051 \
  mistersmith/webui:previous

# Restart Nginx
sudo nginx -t && sudo systemctl restart nginx

# Verify UI accessibility
curl -f http://localhost:3000/health || exit 1
```

### 3.3 Database Rollback
```bash
#!/bin/bash
# rollback-database.sh

echo "Starting Database rollback..."

# Stop all connections
sudo -u postgres psql -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'mistersmith';"

# Drop current database
sudo -u postgres dropdb mistersmith || true

# Restore from backup
sudo -u postgres createdb mistersmith
sudo -u postgres psql mistersmith < mistersmith-backup-latest.sql

# Verify restoration
sudo -u postgres psql -d mistersmith -c "SELECT COUNT(*) FROM agents;"
sudo -u postgres psql -d mistersmith -c "SELECT COUNT(*) FROM tasks;"
```

### 3.4 NATS Rollback
```bash
#!/bin/bash
# rollback-nats.sh

echo "Starting NATS rollback..."

# Stop current NATS
docker stop mistersmith-nats || true
docker rm mistersmith-nats || true

# Restore configuration
aws s3 cp s3://mistersmith-backups/pre-migration/nats-server.conf /etc/nats/nats-server.conf

# Restore message streams
tar -xzf nats-state-latest.tar.gz
nats stream restore MisterSmith-Tasks < ./nats-backup/MisterSmith-Tasks.json
nats stream restore MisterSmith-Results < ./nats-backup/MisterSmith-Results.json

# Start NATS with previous configuration
docker run -d \
  --name mistersmith-nats \
  --restart unless-stopped \
  -v /etc/nats:/etc/nats \
  -p 4222:4222 \
  -p 8222:8222 \
  nats:2.10-alpine \
  -c /etc/nats/nats-server.conf

# Verify NATS connectivity
nats server check connection
```

---

## 4. Rollback Verification Procedures

### 4.1 Service Health Verification
```bash
#!/bin/bash
# verify-rollback.sh

echo "Verifying rollback completion..."

# Check all services
services=("orchestrator" "webui" "nats" "postgres")
failed=0

for service in "${services[@]}"; do
  case $service in
    orchestrator)
      if ! grpc_health_probe -addr=localhost:50051; then
        echo "❌ Orchestrator health check failed"
        ((failed++))
      else
        echo "✅ Orchestrator healthy"
      fi
      ;;
    webui)
      if ! curl -f http://localhost:3000/health; then
        echo "❌ Web UI health check failed"
        ((failed++))
      else
        echo "✅ Web UI healthy"
      fi
      ;;
    nats)
      if ! nats server check connection; then
        echo "❌ NATS connection failed"
        ((failed++))
      else
        echo "✅ NATS healthy"
      fi
      ;;
    postgres)
      if ! sudo -u postgres pg_isready; then
        echo "❌ PostgreSQL not ready"
        ((failed++))
      else
        echo "✅ PostgreSQL healthy"
      fi
      ;;
  esac
done

if [ $failed -eq 0 ]; then
  echo "✅ All services rolled back successfully"
  exit 0
else
  echo "❌ $failed services failed rollback"
  exit 1
fi
```

### 4.2 Data Integrity Verification
```sql
-- verify-data-integrity.sql
-- Run after database rollback

-- Check agent count
SELECT 'Agents', COUNT(*) FROM agents
UNION ALL
SELECT 'Tasks', COUNT(*) FROM tasks
UNION ALL
SELECT 'Discoveries', COUNT(*) FROM discoveries
UNION ALL
SELECT 'Sessions', COUNT(*) FROM hive_sessions;

-- Verify foreign key constraints
SELECT 
    tc.constraint_name, 
    tc.table_name, 
    kcu.column_name, 
    ccu.table_name AS foreign_table_name,
    ccu.column_name AS foreign_column_name 
FROM 
    information_schema.table_constraints AS tc 
    JOIN information_schema.key_column_usage AS kcu
      ON tc.constraint_name = kcu.constraint_name
      AND tc.table_schema = kcu.table_schema
    JOIN information_schema.constraint_column_usage AS ccu
      ON ccu.constraint_name = tc.constraint_name
      AND ccu.table_schema = tc.table_schema
WHERE tc.constraint_type = 'FOREIGN KEY'
  AND tc.table_schema = 'public';

-- Check for orphaned records
SELECT 'Orphaned Tasks' AS check_type, COUNT(*) AS count
FROM tasks t
LEFT JOIN agents a ON t.assigned_agent_id = a.id
WHERE t.assigned_agent_id IS NOT NULL AND a.id IS NULL;
```

### 4.3 Performance Baseline Verification
```bash
#!/bin/bash
# verify-performance.sh

echo "Verifying performance baselines..."

# Test orchestrator response time
start_time=$(date +%s%N)
grpc_health_probe -addr=localhost:50051
end_time=$(date +%s%N)
response_time=$((($end_time - $start_time) / 1000000))
echo "Orchestrator response time: ${response_time}ms"

# Test database query performance
time sudo -u postgres psql -d mistersmith -c "SELECT COUNT(*) FROM tasks;" > /dev/null

# Test NATS message throughput
nats bench MisterSmith-Tasks --msgs=1000 --size=256

# Compare with baseline
if [ $response_time -lt 1000 ]; then
  echo "✅ Performance within acceptable range"
else
  echo "⚠️ Performance degraded - investigation needed"
fi
```

---

## 5. Automation Scripts

### 5.1 Master Rollback Script
```bash
#!/bin/bash
# rollback-all.sh

set -e

# Parse arguments
ENVIRONMENT=${1:-production}
CONFIRM=${2:-false}

if [ "$CONFIRM" != "--confirm" ]; then
  echo "⚠️  This will rollback ALL MisterSmith services to previous version"
  echo "Usage: $0 [environment] --confirm"
  exit 1
fi

echo "🔄 Starting complete system rollback for $ENVIRONMENT..."

# Create rollback log
ROLLBACK_LOG="/var/log/mistersmith/rollback-$(date +%Y%m%d-%H%M%S).log"
mkdir -p /var/log/mistersmith

# Execute rollback in correct order
{
  echo "[$(date)] Starting rollback procedure"
  
  # 1. Stop all services
  echo "[$(date)] Stopping all services..."
  ./scripts/stop-all-services.sh
  
  # 2. Rollback database
  echo "[$(date)] Rolling back database..."
  ./scripts/rollback-database.sh
  
  # 3. Rollback NATS
  echo "[$(date)] Rolling back NATS..."
  ./scripts/rollback-nats.sh
  
  # 4. Rollback orchestrator
  echo "[$(date)] Rolling back orchestrator..."
  ./scripts/rollback-orchestrator.sh
  
  # 5. Rollback Web UI
  echo "[$(date)] Rolling back Web UI..."
  ./scripts/rollback-webui.sh
  
  # 6. Verify rollback
  echo "[$(date)] Verifying rollback..."
  ./scripts/verify-rollback.sh
  
  echo "[$(date)] Rollback completed"
} 2>&1 | tee $ROLLBACK_LOG

# Send notification
if [ $? -eq 0 ]; then
  echo "✅ Rollback completed successfully"
  # Send success notification (implement your notification method)
else
  echo "❌ Rollback failed - check $ROLLBACK_LOG"
  # Send failure alert
fi
```

### 5.2 AWS-Specific Rollback
```bash
#!/bin/bash
# rollback-aws-migration.sh

echo "Rolling back AWS migration..."

# Revert Route 53 DNS
aws route53 change-resource-record-sets \
  --hosted-zone-id Z1234567890ABC \
  --change-batch '{
    "Changes": [{
      "Action": "UPSERT",
      "ResourceRecordSet": {
        "Name": "mistersmith.example.com",
        "Type": "A",
        "TTL": 300,
        "ResourceRecords": [{"Value": "ORIGINAL_IP"}]
      }
    }]
  }'

# Stop ECS services
aws ecs update-service \
  --cluster mistersmith-cluster \
  --service mistersmith-orchestrator \
  --desired-count 0

aws ecs update-service \
  --cluster mistersmith-cluster \
  --service mistersmith-webui \
  --desired-count 0

# Restore RDS from snapshot
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier mistersmith-db-rollback \
  --db-snapshot-identifier mistersmith-pre-migration-snapshot

# Update local services to use original configuration
./scripts/restore-local-config.sh
```

### 5.3 Emergency Fallback Script
```bash
#!/bin/bash
# emergency-fallback.sh

# This script provides immediate fallback to local Docker containers
# when AWS services are completely unavailable

echo "⚠️  EMERGENCY FALLBACK INITIATED"

# Kill all AWS connections
sudo iptables -A OUTPUT -d 52.0.0.0/8 -j DROP
sudo iptables -A OUTPUT -d 54.0.0.0/8 -j DROP

# Start local containers with emergency config
docker-compose -f docker-compose.emergency.yml up -d

# Update DNS to localhost
echo "127.0.0.1 mistersmith.local" | sudo tee -a /etc/hosts

# Verify local services
sleep 10
curl http://localhost:3000/health || exit 1

echo "✅ Emergency fallback active - running on local containers"
```

---

## 6. Rollback Decision Matrix

| Scenario | Trigger | Rollback Scope | Procedure | Recovery Time |
|----------|---------|----------------|-----------|---------------|
| Single service failure | Health check fails 3x | Service only | rollback-component.sh | 2-5 minutes |
| Database corruption | Data integrity check fails | Database + dependent services | rollback-database.sh + verify | 10-15 minutes |
| Performance degradation | Response time >5s | Affected services | rollback-component.sh with monitoring | 5-10 minutes |
| Complete AWS failure | All AWS services unreachable | Full system | emergency-fallback.sh | 5 minutes |
| Partial migration failure | 50% services migrated | Revert to original | rollback-all.sh | 15-20 minutes |

---

## 7. Post-Rollback Actions

### 7.1 Incident Report Template
```markdown
# Rollback Incident Report

**Date/Time:** [YYYY-MM-DD HH:MM:SS]
**Triggered By:** [Automatic/Manual]
**Reason:** [Detailed reason for rollback]
**Components Affected:** [List of services]
**Rollback Duration:** [Time taken]
**Data Loss:** [Yes/No - details if yes]

## Root Cause Analysis
[Detailed analysis of what caused the need for rollback]

## Actions Taken
1. [Step by step actions]
2. [Including any manual interventions]

## Lessons Learned
- [What went wrong]
- [How to prevent in future]

## Follow-up Actions
- [ ] Update runbooks
- [ ] Modify monitoring thresholds
- [ ] Schedule post-mortem meeting
```

### 7.2 Recovery Validation Checklist
- [ ] All services responding to health checks
- [ ] Database integrity verified
- [ ] Message queues operational
- [ ] No data loss confirmed
- [ ] Performance baselines met
- [ ] User access restored
- [ ] Monitoring alerts cleared
- [ ] Backup systems updated
- [ ] Documentation updated
- [ ] Team notified

---

## 8. Testing Rollback Procedures

### 8.1 Rollback Drill Schedule
- **Weekly:** Component-level rollback test (rotating services)
- **Monthly:** Full system rollback test in staging
- **Quarterly:** AWS migration rollback test
- **Annually:** Emergency fallback drill

### 8.2 Test Scenarios
```bash
#!/bin/bash
# test-rollback-scenarios.sh

# Scenario 1: Single service failure
./simulate-failure.sh --service orchestrator
./rollback-component.sh --service orchestrator
./verify-rollback.sh

# Scenario 2: Database corruption
./simulate-corruption.sh --database postgres
./rollback-database.sh
./verify-data-integrity.sh

# Scenario 3: Complete system failure
./simulate-system-failure.sh
./rollback-all.sh --confirm
./verify-rollback.sh --complete
```

---

## Implementation Status

- [x] Rollback trigger definitions
- [x] State preservation procedures
- [x] Component-specific rollback scripts
- [x] Verification procedures
- [x] Automation scripts
- [x] AWS-specific rollback procedures
- [x] Emergency fallback procedures
- [x] Decision matrix
- [x] Post-rollback actions
- [x] Testing procedures

**Next Steps:**
1. Test all rollback scripts in staging environment
2. Conduct team training on rollback procedures
3. Schedule quarterly rollback drills
4. Integrate with monitoring and alerting systems