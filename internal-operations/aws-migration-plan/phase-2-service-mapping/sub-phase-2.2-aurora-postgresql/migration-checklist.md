# Aurora PostgreSQL Migration Checklist

## Pre-Migration Phase

### 1. Assessment & Planning
- [ ] Analyze current PostgreSQL version and features used
- [ ] Document all database dependencies
- [ ] Review application connection strings
- [ ] Identify peak usage times
- [ ] Plan migration window (aim for low-traffic period)
- [ ] Set up rollback procedures

### 2. AWS Account Preparation
- [ ] Verify AWS account limits
- [ ] Request Aurora Serverless v2 limit increases if needed
- [ ] Ensure sufficient VPC IP addresses
- [ ] Configure AWS CLI/SDK access

### 3. Database Analysis
- [ ] Run compatibility check queries:
  ```sql
  -- Check for incompatible features
  SELECT * FROM pg_extension;
  SELECT proname FROM pg_proc WHERE probin IS NOT NULL;
  ```
- [ ] Document custom functions and procedures
- [ ] List all users and permissions
- [ ] Note connection pool settings

## Migration Phase

### 4. Infrastructure Setup
- [ ] Deploy VPC and subnets (if not existing)
- [ ] Create security groups
- [ ] Set up Secrets Manager with credentials
- [ ] Deploy Aurora Serverless v2 cluster
- [ ] Configure RDS Proxy
- [ ] Set up CloudWatch alarms

### 5. Schema Migration
- [ ] Export schema from source database:
  ```bash
  pg_dump --host=source-host --port=5432 --username=postgres \
    --dbname=supervision_db --schema-only > schema.sql
  ```
- [ ] Review and clean schema export
- [ ] Import schema to Aurora:
  ```bash
  psql --host=aurora-endpoint --port=5432 --username=postgres \
    --dbname=supervision_db < schema.sql
  ```
- [ ] Verify all objects created successfully

### 6. Data Migration

#### Option A: Direct Migration (Small Databases < 100GB)
- [ ] Create consistent backup:
  ```bash
  pg_dump --host=source-host --port=5432 --username=postgres \
    --dbname=supervision_db --data-only --format=custom > data.dump
  ```
- [ ] Restore to Aurora:
  ```bash
  pg_restore --host=aurora-endpoint --port=5432 --username=postgres \
    --dbname=supervision_db --data-only --jobs=4 data.dump
  ```

#### Option B: AWS DMS (Minimal Downtime)
- [ ] Create DMS replication instance
- [ ] Create source and target endpoints
- [ ] Create and test migration task
- [ ] Start full load and CDC
- [ ] Monitor replication lag

### 7. Validation
- [ ] Verify row counts:
  ```sql
  SELECT schemaname, tablename, n_live_tup 
  FROM pg_stat_user_tables 
  ORDER BY n_live_tup DESC;
  ```
- [ ] Test critical queries
- [ ] Verify indexes and constraints
- [ ] Check sequence values
- [ ] Validate user permissions

## Cut-Over Phase

### 8. Application Cutover
- [ ] Stop application writes to source database
- [ ] Ensure replication caught up (if using DMS)
- [ ] Update application configuration:
  ```yaml
  # Old connection
  DATABASE_URL: postgresql://user:pass@old-host:5432/supervision_db
  
  # New connection (via RDS Proxy)
  DATABASE_URL: postgresql://user:pass@proxy-endpoint:5432/supervision_db
  ```
- [ ] Restart application services
- [ ] Monitor application logs

### 9. Verification
- [ ] Test all critical application functions
- [ ] Monitor database connections
- [ ] Check query performance
- [ ] Verify data integrity
- [ ] Monitor Aurora scaling behavior

## Post-Migration Phase

### 10. Optimization
- [ ] Enable Query Performance Insights
- [ ] Review and optimize slow queries
- [ ] Adjust Aurora capacity range if needed
- [ ] Fine-tune RDS Proxy settings
- [ ] Update database statistics:
  ```sql
  ANALYZE;
  ```

### 11. Cleanup
- [ ] Document new connection endpoints
- [ ] Update runbooks and documentation
- [ ] Remove DMS resources (if used)
- [ ] Schedule old database decommission
- [ ] Update backup and DR procedures

### 12. Monitoring Setup
- [ ] Configure CloudWatch dashboards
- [ ] Set up alerting thresholds:
  - Connection count > 25
  - Capacity usage > 80%
  - Replication lag > 5 seconds
  - Failed connection attempts
- [ ] Enable Performance Insights
- [ ] Configure log exports

## Rollback Procedures

### If Issues Occur:
1. [ ] Revert application connection strings
2. [ ] Stop writes to Aurora
3. [ ] Restore from latest backup if data corruption
4. [ ] Document issues for post-mortem
5. [ ] Plan remediation and retry

## Success Metrics

- [ ] Zero data loss
- [ ] Application downtime < 30 minutes
- [ ] All queries performing within SLA
- [ ] Connection pool operating efficiently
- [ ] Auto-scaling working as expected

## Contact Information

- **DBA Team**: [Contact]
- **Application Team**: [Contact]
- **AWS Support**: [Case Number]
- **Escalation**: [Manager Contact]

## Notes Section

_Use this section to document any issues, observations, or deviations from the plan during migration_

---

**Migration Date**: _______________
**Migration Team**: _______________
**Sign-off**: _______________