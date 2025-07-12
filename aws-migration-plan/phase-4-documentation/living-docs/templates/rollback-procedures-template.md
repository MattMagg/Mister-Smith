# Rollback Procedures - [Component Name]
> **Criticality**: [LOW|MEDIUM|HIGH|CRITICAL]  
> **Time to Execute**: ~[X] minutes  
> **Recovery Point Objective (RPO)**: [X] minutes  
> **Recovery Time Objective (RTO)**: [X] minutes  
> **Last Tested**: [Date]  
> **Owner**: [Team/Person]  

## 🚨 Quick Reference

### Emergency Contacts
| Role | Name | Phone | Email | Slack |
|------|------|-------|-------|-------|
| Incident Commander | [Name] | [Phone] | [Email] | @[handle] |
| Tech Lead | [Name] | [Phone] | [Email] | @[handle] |
| Database Admin | [Name] | [Phone] | [Email] | @[handle] |
| AWS Support | - | [Number] | - | - |

### Critical Resources
- **Runbook Location**: `s3://[bucket]/runbooks/[component]/`
- **Backup Location**: `s3://[bucket]/backups/[component]/`
- **Logs Location**: CloudWatch Log Group: `/aws/[service]/[component]`
- **Monitoring Dashboard**: [CloudWatch Dashboard URL]

## 📋 Rollback Decision Criteria

### Automatic Triggers (any of these)
1. 🔴 Application health checks failing for >5 minutes
2. 🔴 Error rate exceeds 5% for >2 minutes
3. 🔴 Response time degradation >50% for >3 minutes
4. 🔴 Critical functionality unavailable
5. 🔴 Data corruption or integrity issues detected
6. 🔴 Security breach or vulnerability exposed

### Manual Triggers
- Product owner decision based on user impact
- Critical bug discovered post-deployment
- Performance degradation affecting user experience
- Integration failures with dependent systems

## 🚀 Rollback Procedure

### 🔴 Phase 1: Immediate Actions (0-2 minutes)

#### 1.1 Initiate Incident Response
```bash
# Create incident channel
./scripts/create-incident.sh "[Component] Rollback $(date +%Y%m%d-%H%M%S)"

# Send notifications
export INCIDENT_ID=$(cat /tmp/incident-id)
aws sns publish --topic-arn $SNS_TOPIC_ARN \
  --message "Initiating rollback for [Component]. Incident: $INCIDENT_ID"
```

#### 1.2 Stop New Traffic
```bash
# Remove from load balancer
aws elbv2 modify-target-group-attributes \
  --target-group-arn $TARGET_GROUP_ARN \
  --attributes Key=deregistration_delay.timeout_seconds,Value=0

# Drain connections
aws elbv2 deregister-targets \
  --target-group-arn $TARGET_GROUP_ARN \
  --targets Id=$INSTANCE_ID
```

#### 1.3 Capture Current State
```bash
# Save current configuration
aws ecs describe-services \
  --cluster $CLUSTER_NAME \
  --services $SERVICE_NAME > /tmp/rollback_state_$(date +%s).json

# Capture metrics snapshot
aws cloudwatch get-metric-statistics \
  --namespace AWS/ECS \
  --metric-name CPUUtilization \
  --dimensions Name=ServiceName,Value=$SERVICE_NAME \
  --start-time $(date -u -d '10 minutes ago' +%Y-%m-%dT%H:%M:%S) \
  --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
  --period 60 \
  --statistics Average > /tmp/metrics_pre_rollback.json
```

### 🔵 Phase 2: Execute Rollback (2-10 minutes)

#### 2.1 Database Rollback (if needed)
```bash
# Check if database changes were made
if [ -f "/tmp/deployment_${DEPLOYMENT_ID}/db_migrated" ]; then
    echo "Rolling back database changes..."
    
    # Execute rollback script
    aws rds modify-db-cluster \
      --db-cluster-identifier $DB_CLUSTER_ID \
      --backup-retention-period 7 \
      --preferred-backup-window "03:00-04:00"
    
    # Run rollback migrations
    docker run --rm \
      -e DATABASE_URL=$DATABASE_URL \
      migration-tool:latest \
      rollback --to-version $PREVIOUS_VERSION
fi
```

#### 2.2 Application Rollback
```bash
# For ECS/Fargate
echo "Rolling back ECS service to previous task definition..."
aws ecs update-service \
  --cluster $CLUSTER_NAME \
  --service $SERVICE_NAME \
  --task-definition $PREVIOUS_TASK_DEF \
  --force-new-deployment

# For EC2/Auto Scaling
echo "Rolling back Auto Scaling Group..."
aws autoscaling update-auto-scaling-group \
  --auto-scaling-group-name $ASG_NAME \
  --launch-configuration-name $PREVIOUS_LAUNCH_CONFIG

# Monitor rollback progress
watch -n 5 "aws ecs describe-services \
  --cluster $CLUSTER_NAME \
  --services $SERVICE_NAME \
  --query 'services[0].deployments' \
  --output table"
```

#### 2.3 Configuration Rollback
```bash
# Revert Parameter Store values
aws ssm put-parameter \
  --name "/app/${ENVIRONMENT}/${COMPONENT}/config" \
  --value file://configs/previous-config.json \
  --type SecureString \
  --overwrite

# Revert feature flags
aws appconfig create-deployment \
  --application-id $APP_CONFIG_APP_ID \
  --environment-id $ENVIRONMENT_ID \
  --deployment-strategy-id $SAFE_DEPLOY_STRATEGY \
  --configuration-profile-id $CONFIG_PROFILE_ID \
  --configuration-version $PREVIOUS_VERSION
```

### 🔶 Phase 3: Validate Recovery (10-15 minutes)

#### 3.1 Health Verification
```bash
# Check service health
for i in {1..10}; do
    echo "Health check attempt $i/10..."
    
    HEALTH=$(curl -s -o /dev/null -w "%{http_code}" \
      http://$LOAD_BALANCER_DNS/health)
    
    if [ "$HEALTH" = "200" ]; then
        echo "✅ Health check passed"
        break
    else
        echo "⚠️  Health check failed (HTTP $HEALTH), retrying..."
        sleep 30
    fi
done
```

#### 3.2 Functionality Tests
```bash
# Run smoke tests
docker run --rm \
  -e BASE_URL=http://$LOAD_BALANCER_DNS \
  -e TEST_SUITE=smoke \
  test-runner:latest

# Verify critical endpoints
for endpoint in "/api/v1/health" "/api/v1/status" "/api/v1/users"; do
    echo "Testing $endpoint..."
    curl -f -s "http://$LOAD_BALANCER_DNS$endpoint" > /dev/null && \
      echo "✅ $endpoint OK" || \
      echo "❌ $endpoint FAILED"
done
```

#### 3.3 Performance Validation
```bash
# Check response times
ab -n 100 -c 10 "http://$LOAD_BALANCER_DNS/api/v1/health" | \
  grep "Time per request" | \
  awk '{print "Average response time: " $4 "ms"}'

# Verify error rates
aws cloudwatch get-metric-statistics \
  --namespace AWS/ApplicationELB \
  --metric-name TargetResponseTime \
  --dimensions Name=LoadBalancer,Value=$LB_NAME \
  --start-time $(date -u -d '5 minutes ago' +%Y-%m-%dT%H:%M:%S) \
  --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
  --period 60 \
  --statistics Average
```

### 📢 Phase 4: Post-Rollback Actions

#### 4.1 Communication
```bash
# Update incident status
./scripts/update-incident.sh $INCIDENT_ID "RESOLVED" \
  "Rollback completed successfully. Service restored to version $PREVIOUS_VERSION"

# Notify stakeholders
aws sns publish --topic-arn $STAKEHOLDER_TOPIC_ARN \
  --subject "[RESOLVED] [Component] Rollback Complete" \
  --message "Service has been successfully rolled back to the previous version."
```

#### 4.2 Documentation
- [ ] Create incident report
- [ ] Update deployment log
- [ ] Document root cause
- [ ] Schedule post-mortem
- [ ] Update runbooks if needed

#### 4.3 Cleanup
```bash
# Archive failed deployment artifacts
aws s3 cp /tmp/rollback_state_*.json \
  s3://$BUCKET/incidents/$INCIDENT_ID/ --recursive

# Clean up temporary resources
rm -f /tmp/rollback_state_*.json
rm -f /tmp/metrics_*.json
```

## 📑 Rollback Verification Checklist

### Application State
- [ ] Previous version deployed and running
- [ ] All instances/containers healthy
- [ ] Configuration reverted
- [ ] Feature flags reset
- [ ] Background jobs processing normally

### Data Integrity
- [ ] Database state verified
- [ ] No data loss confirmed
- [ ] Cache cleared/refreshed
- [ ] Message queues draining normally

### Performance Metrics
- [ ] Response times back to baseline
- [ ] Error rates at normal levels
- [ ] Resource utilization stable
- [ ] No memory leaks

### External Dependencies
- [ ] API integrations restored
- [ ] Third-party services connected
- [ ] CDN cache purged if needed
- [ ] DNS propagation complete

## 🔧 Troubleshooting

### Common Issues

#### Issue: Rollback stuck in progress
```bash
# Force service update
aws ecs update-service \
  --cluster $CLUSTER_NAME \
  --service $SERVICE_NAME \
  --force-new-deployment
```

#### Issue: Database rollback failed
```bash
# Restore from snapshot
aws rds restore-db-cluster-from-snapshot \
  --db-cluster-identifier $DB_CLUSTER_ID-rollback \
  --snapshot-identifier $LATEST_SNAPSHOT_ID
```

#### Issue: Configuration mismatch
```bash
# Force parameter refresh
aws ssm send-command \
  --document-name "AWS-RunShellScript" \
  --targets Key=tag:Component,Values=$COMPONENT \
  --parameters 'commands=["sudo systemctl restart app"]'
```

## 📊 Metrics to Monitor Post-Rollback

| Metric | Normal Range | Alert Threshold | Dashboard Link |
|--------|--------------|-----------------|----------------|
| Response Time | <200ms | >500ms | [Link] |
| Error Rate | <0.1% | >1% | [Link] |
| CPU Usage | 40-60% | >80% | [Link] |
| Memory Usage | 50-70% | >85% | [Link] |
| Queue Depth | <100 | >1000 | [Link] |

## 📋 Post-Mortem Template

After successful rollback, create post-mortem with:
1. Timeline of events
2. Root cause analysis
3. Impact assessment
4. Lessons learned
5. Action items
6. Process improvements

---

**Document Status**: [ ] DRAFT [ ] REVIEWED [ ] APPROVED [ ] TESTED  
**Last Test Date**: _____________  
**Next Review**: _____________  
**Template Version**: 1.0.0