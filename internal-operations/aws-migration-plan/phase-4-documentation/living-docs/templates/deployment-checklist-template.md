# Deployment Checklist - [Component Name]
> **Version**: 1.0.0  
> **Last Updated**: [Date]  
> **Owner**: [Team/Person]  
> **Criticality**: [LOW|MEDIUM|HIGH|CRITICAL]  
> **Estimated Duration**: [Time]  

## 📦 Pre-Deployment Verification

### Code Quality
- [ ] All unit tests passing (coverage >80%)
- [ ] Integration tests completed successfully
- [ ] No critical SonarQube issues
- [ ] Code review approved by 2+ reviewers
- [ ] Security scan completed (no high/critical vulnerabilities)

### Documentation
- [ ] API documentation updated
- [ ] README.md reflects current version
- [ ] CHANGELOG.md updated with release notes
- [ ] Runbooks updated for new features
- [ ] Architecture diagrams current

### Dependencies
- [ ] All dependencies version-locked
- [ ] No deprecated dependencies
- [ ] License compliance verified
- [ ] Dependency vulnerabilities scanned

## 🏗️ Infrastructure Readiness

### AWS Resources
- [ ] VPC and subnets configured
- [ ] Security groups reviewed and approved
- [ ] IAM roles/policies created with least privilege
- [ ] RDS/DynamoDB provisioned and configured
- [ ] S3 buckets created with proper policies
- [ ] CloudFront distributions configured (if applicable)

### Networking
- [ ] Load balancer health checks defined
- [ ] Target groups configured
- [ ] Route 53 DNS records prepared
- [ ] SSL/TLS certificates valid and installed
- [ ] Network ACLs reviewed

### Monitoring & Logging
- [ ] CloudWatch log groups created
- [ ] CloudWatch alarms configured
- [ ] X-Ray tracing enabled
- [ ] Custom metrics defined
- [ ] Log retention policies set
- [ ] Dashboard created

## 🚀 Application Deployment

### Build & Artifacts
- [ ] Build pipeline successful
- [ ] Docker images scanned and tagged
- [ ] Artifacts uploaded to S3/ECR
- [ ] Version tags applied correctly
- [ ] Build artifacts backed up

### Configuration
- [ ] Environment variables documented
- [ ] Secrets stored in AWS Secrets Manager
- [ ] Parameter Store values configured
- [ ] Feature flags set appropriately
- [ ] Connection strings verified

### Database
- [ ] Database migrations tested
- [ ] Rollback scripts prepared
- [ ] Database backups verified
- [ ] Connection pooling configured
- [ ] Read replicas ready (if applicable)

## 🎯 Deployment Execution

### Deployment Steps
- [ ] Deployment plan communicated to stakeholders
- [ ] Maintenance window scheduled (if required)
- [ ] Blue-green deployment strategy confirmed
- [ ] Canary deployment percentage set
- [ ] Auto-scaling policies configured

### During Deployment
- [ ] Monitor deployment progress
- [ ] Watch error rates and latency
- [ ] Verify health checks passing
- [ ] Check auto-scaling behavior
- [ ] Monitor resource utilization

## ✅ Post-Deployment Validation

### Functional Validation
- [ ] Smoke tests executed successfully
- [ ] Critical user journeys tested
- [ ] API endpoints responding correctly
- [ ] Background jobs processing
- [ ] Integrations verified

### Performance Validation  
- [ ] Response times within SLA
- [ ] Throughput meets requirements
- [ ] No memory leaks detected
- [ ] CPU utilization normal
- [ ] Database query performance acceptable

### Security Validation
- [ ] SSL/TLS properly configured
- [ ] Authentication working correctly
- [ ] Authorization rules enforced
- [ ] No sensitive data in logs
- [ ] Security headers present

## 📋 Sign-offs

### Required Approvals
- [ ] Development Lead: _________________ Date: _______
- [ ] QA Lead: _________________________ Date: _______
- [ ] Security Lead: ___________________ Date: _______
- [ ] Operations Lead: _________________ Date: _______
- [ ] Product Owner: ___________________ Date: _______

### Stakeholder Notifications
- [ ] Customer Success team notified
- [ ] Support team briefed on changes
- [ ] Documentation team updated
- [ ] Marketing team informed (if applicable)

## 🔄 Rollback Criteria

### Automatic Rollback Triggers
- Error rate exceeds 5% for 5 minutes
- Response time degrades by >50%
- Health checks failing on >25% of instances
- Critical functionality unavailable
- Data corruption detected

### Rollback Decision Authority
- Primary: [Name/Role]
- Secondary: [Name/Role]
- Emergency: On-call engineer

## 📄 Notes & Comments

### Deployment Notes
```
[Add any specific notes, warnings, or special instructions here]
```

### Known Issues
```
[Document any known issues or limitations]
```

### Post-Deployment Tasks
```
[List any tasks that need to be completed after deployment]
```

---

**Checklist Status**: [ ] DRAFT [ ] REVIEWED [ ] APPROVED [ ] COMPLETED  
**Next Review Date**: _____________  
**Template Version**: 1.0.0