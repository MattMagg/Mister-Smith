# Verification Command Library

## Overview

This directory contains verification commands and expected outputs for validating the AWS migration at each phase. Commands are organized by category and phase.

## Directory Structure

```
verification/
├── README.md                    # This file
├── infrastructure/              # Infrastructure verification commands
│   ├── vpc-validation.md
│   ├── security-groups.md
│   ├── iam-roles.md
│   └── networking.md
├── application/                 # Application verification commands
│   ├── health-checks.md
│   ├── api-endpoints.md
│   ├── database-connectivity.md
│   └── integrations.md
├── performance/                 # Performance verification commands
│   ├── load-testing.md
│   ├── response-times.md
│   ├── throughput.md
│   └── resource-utilization.md
└── operational/                 # Operational readiness commands
    ├── monitoring.md
    ├── logging.md
    ├── backups.md
    └── disaster-recovery.md
```

## Command Format

Each verification document follows this structure:

```markdown
# [Verification Name]

## Purpose
Brief description of what this verification validates

## Prerequisites
- Required permissions
- Required tools
- Required environment variables

## Command
```bash
# The actual command to run
```

## Expected Output
```
# What successful output looks like
```

## Common Issues
- Issue 1: Description and resolution
- Issue 2: Description and resolution
```

## Quick Verification Commands

### Infrastructure Health Check
```bash
# Check VPC and subnet configuration
aws ec2 describe-vpcs --filters "Name=tag:Project,Values=MisterSmith" \
  --query 'Vpcs[*].[VpcId,State,CidrBlock]' --output table

# Verify security groups
aws ec2 describe-security-groups --filters "Name=tag:Project,Values=MisterSmith" \
  --query 'SecurityGroups[*].[GroupId,GroupName,Description]' --output table
```

### Application Health Check
```bash
# Check ECS services
aws ecs list-services --cluster mistersmith-cluster | \
  xargs -I {} aws ecs describe-services --cluster mistersmith-cluster --services {} \
  --query 'services[*].[serviceName,status,runningCount,desiredCount]' --output table

# Check ALB target health
aws elbv2 describe-target-health --target-group-arn $TARGET_GROUP_ARN \
  --query 'TargetHealthDescriptions[*].[Target.Id,TargetHealth.State]' --output table
```

### Database Connectivity
```bash
# Test RDS connectivity
aws rds describe-db-instances --db-instance-identifier mistersmith-db \
  --query 'DBInstances[0].[DBInstanceStatus,Endpoint.Address,Endpoint.Port]' --output table

# Check parameter groups
aws rds describe-db-parameters --db-parameter-group-name mistersmith-params \
  --query 'Parameters[?ParameterValue!=`null`].[ParameterName,ParameterValue]' --output table
```

### Monitoring Status
```bash
# List CloudWatch alarms
aws cloudwatch describe-alarms --alarm-name-prefix "MisterSmith" \
  --query 'MetricAlarms[*].[AlarmName,StateValue,MetricName]' --output table

# Check log groups
aws logs describe-log-groups --log-group-name-prefix "/aws/mistersmith" \
  --query 'logGroups[*].[logGroupName,retentionInDays,storedBytes]' --output table
```

## Phase-Specific Verifications

### Phase 1: Architecture Analysis
- Document review completeness
- Dependency mapping accuracy
- Risk assessment validation

### Phase 2: Service Mapping
- Service inventory correctness
- Integration points identified
- Data flow documentation

### Phase 3: Strategy Formulation
- Migration approach feasibility
- Timeline realism
- Resource allocation adequacy

### Phase 4: Documentation Infrastructure
- Document generation automation
- Update frequency validation
- Cross-reference integrity

### Phase 5: Swarm Orchestration
- Agent communication verification
- Task distribution efficiency
- Coordination effectiveness

### Phase 6: Infrastructure Setup
- AWS resource provisioning
- Security configuration
- Network connectivity

### Phase 7: Service Migration
- Data migration integrity
- Service functionality
- Performance benchmarks

### Phase 8: Validation & Testing
- Test coverage completeness
- Performance requirements met
- Security compliance

### Phase 9: Production Cutover
- Cutover readiness
- Rollback procedures tested
- Monitoring fully operational

## Automation Integration

These commands can be integrated into the verification scripts:

```bash
# Example: Create a verification script
cat > /tmp/verify-infrastructure.sh << 'EOF'
#!/bin/bash
set -euo pipefail

# Source verification commands
source /path/to/verification/infrastructure/vpc-validation.sh
source /path/to/verification/infrastructure/security-groups.sh

# Run verifications
verify_vpc || exit 1
verify_security_groups || exit 1

echo "Infrastructure verification complete"
EOF

chmod +x /tmp/verify-infrastructure.sh
```

## Expected Results Documentation

### Success Criteria
- All commands return 0 exit code
- Output matches expected patterns
- No error messages in logs
- Metrics within acceptable ranges

### Failure Handling
1. Capture full error output
2. Check AWS CloudTrail for API errors
3. Verify IAM permissions
4. Check service quotas
5. Review security group rules

## Continuous Improvement

### Adding New Verifications
1. Identify verification need
2. Write and test command
3. Document expected output
4. Add to appropriate category
5. Update automation scripts

### Maintaining Verifications
- Review monthly for accuracy
- Update after infrastructure changes
- Add new checks for new services
- Remove obsolete verifications
- Improve based on incident learnings

---

**Last Updated**: 2025-01-11  
**Maintained By**: MisterSmith DevOps Team  
**Review Cycle**: Monthly