# Security Audit Commands

## Overview
Comprehensive security verification commands for AWS migration components.

## Prerequisites
- AWS CLI with security audit permissions
- IAM Access Analyzer enabled
- AWS Config (recommended)
- CloudTrail logging enabled

## Commands

### 1. IAM Role and Policy Verification
```bash
# List all roles with mistersmith prefix
aws iam list-roles \
  --query 'Roles[?contains(RoleName, `mistersmith`)].[RoleName,CreateDate,AssumeRolePolicyDocument.Statement[0].Principal]' \
  --output table

# Verify role trust relationships
function verify_role_trust() {
    local role_name=$1
    aws iam get-role \
      --role-name $role_name \
      --query 'Role.AssumeRolePolicyDocument' | jq '.Statement[] | {
        Effect: .Effect,
        Principal: .Principal,
        Condition: .Condition
      }'
}

# Check for overly permissive policies
function audit_role_policies() {
    local role_name=$1
    
    # Get attached policies
    aws iam list-attached-role-policies \
      --role-name $role_name \
      --query 'AttachedPolicies[].PolicyArn' \
      --output text | while read policy_arn; do
        
        echo "Checking policy: $policy_arn"
        
        # Get policy version
        version=$(aws iam get-policy \
          --policy-arn $policy_arn \
          --query 'Policy.DefaultVersionId' \
          --output text)
        
        # Check for wildcards
        aws iam get-policy-version \
          --policy-arn $policy_arn \
          --version-id $version \
          --query 'PolicyVersion.Document' | jq '
          .Statement[] | select(.Effect == "Allow") | 
          select(.Action[] | contains("*") or .Resource[] | contains("*")) |
          {Action: .Action, Resource: .Resource}'
    done
}

# Find unused roles
function find_unused_roles() {
    local days_threshold=${1:-90}
    local cutoff_date=$(date -d "$days_threshold days ago" +%Y-%m-%d)
    
    aws iam list-roles \
      --query "Roles[?contains(RoleName, 'mistersmith')].[RoleName,RoleLastUsed.LastUsedDate]" \
      --output json | jq -r --arg cutoff "$cutoff_date" '
      .[] | select(.[1] == null or .[1] < $cutoff) | 
      "Role: \(.[0]) - Last used: \(.[1] // "Never")"'
}
```

### 2. Security Group Audit
```bash
# Find overly permissive security groups
function audit_security_groups() {
    local vpc_id=$1
    
    # Check for 0.0.0.0/0 ingress rules
    aws ec2 describe-security-groups \
      --filters "Name=vpc-id,Values=$vpc_id" \
      --query 'SecurityGroups[?IpPermissions[?IpRanges[?CidrIp==`0.0.0.0/0`]]].[GroupId,GroupName,IpPermissions[?IpRanges[?CidrIp==`0.0.0.0/0`]].[IpProtocol,FromPort,ToPort]]' \
      --output json | jq -r '.[] | 
      "Security Group: \(.[0]) (\(.[1]))\nOpen to Internet: \(.[2] | map("Protocol: \(.[0]), Ports: \(.[1])-\(.[2])") | join(", "))\n"'
}

# Check for unused security groups
function find_unused_security_groups() {
    local vpc_id=$1
    
    # Get all security groups
    all_sgs=$(aws ec2 describe-security-groups \
      --filters "Name=vpc-id,Values=$vpc_id" \
      --query 'SecurityGroups[].GroupId' \
      --output text)
    
    # Get used security groups
    used_sgs=$(
        # From EC2 instances
        aws ec2 describe-instances \
          --filters "Name=vpc-id,Values=$vpc_id" \
          --query 'Reservations[].Instances[].SecurityGroups[].GroupId' \
          --output text
        
        # From RDS instances
        aws rds describe-db-instances \
          --query 'DBInstances[].VpcSecurityGroups[].VpcSecurityGroupId' \
          --output text
        
        # From Load Balancers
        aws elbv2 describe-load-balancers \
          --query "LoadBalancers[?VpcId=='$vpc_id'].SecurityGroups[]" \
          --output text
    )
    
    # Find unused
    for sg in $all_sgs; do
        if ! echo "$used_sgs" | grep -q "$sg"; then
            aws ec2 describe-security-groups \
              --group-ids $sg \
              --query 'SecurityGroups[0].[GroupId,GroupName,Description]' \
              --output text
        fi
    done
}

# Verify security group dependencies
function check_sg_dependencies() {
    local sg_id=$1
    
    # Check for references in other security groups
    aws ec2 describe-security-groups \
      --query "SecurityGroups[?IpPermissions[?UserIdGroupPairs[?GroupId=='$sg_id']]].GroupId" \
      --output text
}
```

### 3. Encryption Verification
```bash
# Check S3 bucket encryption
function verify_s3_encryption() {
    aws s3api list-buckets --query 'Buckets[?contains(Name, `mistersmith`)].Name' --output text | while read bucket; do
        echo "Checking bucket: $bucket"
        
        # Check default encryption
        aws s3api get-bucket-encryption \
          --bucket $bucket 2>/dev/null || echo "  No default encryption"
        
        # Check bucket policy for HTTPS enforcement
        aws s3api get-bucket-policy \
          --bucket $bucket 2>/dev/null | jq -r '.Policy' | jq '.Statement[] | 
          select(.Effect == "Deny" and .Condition.Bool."aws:SecureTransport" == "false")'
    done
}

# Check EBS volume encryption
function verify_ebs_encryption() {
    aws ec2 describe-volumes \
      --filters "Name=tag:Project,Values=MisterSmith" \
      --query 'Volumes[].[VolumeId,Encrypted,State,Attachments[0].InstanceId]' \
      --output table
    
    # Count unencrypted volumes
    unencrypted=$(aws ec2 describe-volumes \
      --filters "Name=tag:Project,Values=MisterSmith" "Name=encrypted,Values=false" \
      --query 'length(Volumes)')
    
    echo "Unencrypted volumes: $unencrypted"
}

# Check RDS encryption
function verify_rds_encryption() {
    # Aurora clusters
    aws rds describe-db-clusters \
      --query 'DBClusters[?contains(DBClusterIdentifier, `mistersmith`)].[DBClusterIdentifier,StorageEncrypted,KmsKeyId]' \
      --output table
    
    # Regular RDS instances
    aws rds describe-db-instances \
      --query 'DBInstances[?contains(DBInstanceIdentifier, `mistersmith`)].[DBInstanceIdentifier,StorageEncrypted,KmsKeyId]' \
      --output table
}

# Check Secrets Manager encryption
function verify_secrets_encryption() {
    aws secretsmanager list-secrets \
      --filters Key=name,Values=mistersmith \
      --query 'SecretList[].[Name,KmsKeyId]' \
      --output table
}
```

### 4. Network Security Audit
```bash
# Check VPC Flow Logs
function verify_flow_logs() {
    local vpc_id=$1
    
    aws ec2 describe-flow-logs \
      --filter "Name=resource-id,Values=$vpc_id" \
      --query 'FlowLogs[].[FlowLogId,FlowLogStatus,LogDestinationType,TrafficType]' \
      --output table
}

# Verify Network ACLs
function audit_network_acls() {
    local vpc_id=$1
    
    # Check for overly permissive rules
    aws ec2 describe-network-acls \
      --filters "Name=vpc-id,Values=$vpc_id" \
      --query 'NetworkAcls[].Entries[?RuleAction==`allow` && CidrBlock==`0.0.0.0/0`].[NetworkAclId,RuleNumber,Protocol,PortRange]' \
      --output table
}

# Check for public IPs
function find_public_resources() {
    # EC2 instances with public IPs
    echo "=== EC2 Instances with Public IPs ==="
    aws ec2 describe-instances \
      --filters "Name=instance-state-name,Values=running" \
      --query 'Reservations[].Instances[?PublicIpAddress!=`null`].[InstanceId,PublicIpAddress,Tags[?Key==`Name`].Value|[0]]' \
      --output table
    
    # RDS instances with public access
    echo "=== RDS Instances with Public Access ==="
    aws rds describe-db-instances \
      --query 'DBInstances[?PubliclyAccessible==`true`].[DBInstanceIdentifier,Endpoint.Address]' \
      --output table
}
```

### 5. Access Key and Credential Audit
```bash
# Check IAM access key age
function audit_access_keys() {
    local max_age_days=${1:-90}
    
    aws iam list-users --query 'Users[].UserName' --output text | while read user; do
        aws iam list-access-keys --user-name $user --query "AccessKeyMetadata[].[UserName,AccessKeyId,CreateDate,Status]" --output json | \
        jq -r --arg max_days "$max_age_days" '.[] | 
        select((now - (.[2] | fromdateiso8601)) / 86400 > ($max_days | tonumber)) |
        "User: \(.[0]), Key: \(.[1]), Age: \((now - (.[2] | fromdateiso8601)) / 86400 | floor) days, Status: \(.[3])"'
    done
}

# Check for root account usage
function check_root_usage() {
    aws iam get-account-summary --query 'SummaryMap.AccountMFAEnabled' --output text
    
    # Check CloudTrail for root usage
    aws cloudtrail lookup-events \
      --lookup-attributes AttributeKey=UserName,AttributeValue=root \
      --start-time $(date -u -d '7 days ago' +%Y-%m-%dT%H:%M:%SZ) \
      --query 'Events[].[EventTime,EventName,UserAgent]' \
      --output table
}

# Find inactive users
function find_inactive_users() {
    local days_threshold=${1:-90}
    
    aws iam list-users --query 'Users[].UserName' --output text | while read user; do
        last_used=$(aws iam get-user --user-name $user --query 'User.PasswordLastUsed' --output text)
        if [ "$last_used" == "None" ]; then
            echo "User $user has never logged in"
        else
            days_ago=$(( ($(date +%s) - $(date -d "$last_used" +%s)) / 86400 ))
            if [ $days_ago -gt $days_threshold ]; then
                echo "User $user inactive for $days_ago days"
            fi
        fi
    done
}
```

### 6. Compliance Verification
```bash
# Check AWS Config compliance
function check_config_compliance() {
    # Get non-compliant resources
    aws configservice describe-compliance-by-resource \
      --compliance-types NON_COMPLIANT \
      --query 'ComplianceByResources[].[ResourceType,ResourceId,Compliance.ComplianceType]' \
      --output table
}

# Check specific compliance rules
function verify_compliance_rules() {
    local rules=(
        "encrypted-volumes"
        "iam-password-policy"
        "s3-bucket-public-read-prohibited"
        "s3-bucket-public-write-prohibited"
        "rds-encryption-enabled"
        "restricted-ssh"
    )
    
    for rule in "${rules[@]}"; do
        echo "Checking rule: $rule"
        aws configservice describe-compliance-by-config-rule \
          --config-rule-names $rule \
          --query 'ComplianceByConfigRules[0].[ConfigRuleName,Compliance.ComplianceType,Compliance.ComplianceContributorCount]' \
          --output table 2>/dev/null || echo "  Rule not found"
    done
}
```

### 7. CloudTrail Audit
```bash
# Verify CloudTrail is enabled
function verify_cloudtrail() {
    aws cloudtrail describe-trails \
      --query 'trailList[].[Name,S3BucketName,IsMultiRegionTrail,LogFileValidationEnabled]' \
      --output table
    
    # Check trail status
    aws cloudtrail describe-trails --query 'trailList[].Name' --output text | while read trail; do
        echo "Trail: $trail"
        aws cloudtrail get-trail-status --name $trail \
          --query '[IsLogging,LatestDeliveryTime]' \
          --output table
    done
}

# Search for security events
function search_security_events() {
    local hours_ago=${1:-24}
    
    # Failed authentication attempts
    echo "=== Failed Authentication Attempts ==="
    aws cloudtrail lookup-events \
      --lookup-attributes AttributeKey=EventName,AttributeValue=ConsoleLogin \
      --start-time $(date -u -d "$hours_ago hours ago" +%Y-%m-%dT%H:%M:%SZ) \
      --query 'Events[?ErrorCode!=`null`].[EventTime,Username,ErrorCode,UserAgent]' \
      --output table
    
    # Unauthorized API calls
    echo "=== Unauthorized API Calls ==="
    aws cloudtrail lookup-events \
      --start-time $(date -u -d "$hours_ago hours ago" +%Y-%m-%dT%H:%M:%SZ) \
      --query 'Events[?ErrorCode==`UnauthorizedOperation` || ErrorCode==`AccessDenied`].[EventTime,EventName,UserIdentity.UserName,ErrorCode]' \
      --output table | head -20
}
```

### 8. Container Security Audit
```bash
# Check ECS task execution roles
function audit_ecs_task_roles() {
    local cluster=$1
    
    # Get unique task definitions
    aws ecs list-services --cluster $cluster --query 'serviceArns' --output text | while read service; do
        task_def=$(aws ecs describe-services \
          --cluster $cluster \
          --services $service \
          --query 'services[0].taskDefinition' \
          --output text)
        
        # Get execution and task roles
        aws ecs describe-task-definition \
          --task-definition $task_def \
          --query 'taskDefinition.[family,executionRoleArn,taskRoleArn]' \
          --output table
    done
}

# Scan container images for vulnerabilities
function scan_ecr_images() {
    # List repositories
    aws ecr describe-repositories \
      --query 'repositories[?contains(repositoryName, `mistersmith`)].repositoryName' \
      --output text | while read repo; do
        
        echo "Repository: $repo"
        
        # Get latest image
        latest_tag=$(aws ecr describe-images \
          --repository-name $repo \
          --query 'imageDetails[?imageTags!=`null`] | sort_by(@, &imagePushedAt)[-1].imageTags[0]' \
          --output text)
        
        if [ ! -z "$latest_tag" ]; then
            # Start scan
            aws ecr start-image-scan \
              --repository-name $repo \
              --image-id imageTag=$latest_tag
            
            # Get scan results
            aws ecr describe-image-scan-findings \
              --repository-name $repo \
              --image-id imageTag=$latest_tag \
              --query 'imageScanFindings.findingSeverityCounts' \
              --output json
        fi
    done
}
```

### 9. Comprehensive Security Report
```bash
#!/bin/bash
# security-audit-report.sh - Generate comprehensive security audit report

function generate_security_report() {
    local report_dir="security-audit-$(date +%Y%m%d-%H%M%S)"
    mkdir -p $report_dir
    
    echo "Generating security audit report..."
    
    # IAM audit
    echo "Auditing IAM..." > $report_dir/iam-audit.txt
    aws iam get-account-summary >> $report_dir/iam-audit.txt
    find_unused_roles 90 >> $report_dir/iam-audit.txt
    audit_access_keys 90 >> $report_dir/iam-audit.txt
    
    # Network security
    echo "Auditing network security..." > $report_dir/network-audit.txt
    audit_security_groups $VPC_ID >> $report_dir/network-audit.txt
    find_public_resources >> $report_dir/network-audit.txt
    
    # Encryption audit
    echo "Auditing encryption..." > $report_dir/encryption-audit.txt
    verify_s3_encryption >> $report_dir/encryption-audit.txt
    verify_ebs_encryption >> $report_dir/encryption-audit.txt
    verify_rds_encryption >> $report_dir/encryption-audit.txt
    
    # Generate summary
    cat > $report_dir/summary.md << EOF
# Security Audit Report
Date: $(date)

## Key Findings

### IAM
$(grep -c "unused" $report_dir/iam-audit.txt || echo "0") unused roles found
$(grep -c "Age:" $report_dir/iam-audit.txt || echo "0") access keys older than 90 days

### Network Security
$(grep -c "0.0.0.0/0" $report_dir/network-audit.txt || echo "0") security groups open to internet
$(grep -c "Public" $report_dir/network-audit.txt || echo "0") resources with public IPs

### Encryption
$(grep -c "No default encryption" $report_dir/encryption-audit.txt || echo "0") unencrypted S3 buckets
$(grep -c "false" $report_dir/encryption-audit.txt || echo "0") unencrypted volumes

## Recommendations
1. Review and remove unused IAM roles and access keys
2. Restrict security groups to specific IP ranges
3. Enable encryption for all data at rest
4. Enable MFA for all IAM users
5. Implement least privilege access policies

## Detailed Results
See individual audit files in: $report_dir/
EOF
    
    echo "Security audit complete. Report available in: $report_dir/"
    cat $report_dir/summary.md
}

# Run the audit
generate_security_report
```

### 10. Automated Security Monitoring
```bash
#!/bin/bash
# security-monitor.sh - Continuous security monitoring

function monitor_security_events() {
    while true; do
        clear
        echo "=== Security Monitor - $(date) ==="
        
        # Check for recent failed logins
        echo "Recent Failed Logins:"
        aws cloudtrail lookup-events \
          --lookup-attributes AttributeKey=EventName,AttributeValue=ConsoleLogin \
          --start-time $(date -u -d '1 hour ago' +%Y-%m-%dT%H:%M:%SZ) \
          --query 'Events[?ErrorCode!=`null`].[EventTime,Username]' \
          --output table | head -5
        
        # Check for security group changes
        echo -e "\nRecent Security Group Changes:"
        aws cloudtrail lookup-events \
          --lookup-attributes AttributeKey=EventName,AttributeValue=AuthorizeSecurityGroupIngress \
          --start-time $(date -u -d '1 hour ago' +%Y-%m-%dT%H:%M:%SZ) \
          --query 'Events[].[EventTime,UserIdentity.UserName,RequestParameters.groupId]' \
          --output table | head -5
        
        # Check for new public resources
        echo -e "\nPublic Resources:"
        aws ec2 describe-instances \
          --filters "Name=instance-state-name,Values=running" \
          --query 'length(Reservations[].Instances[?PublicIpAddress!=`null`])' \
          --output text | xargs echo "Public EC2 instances:"
        
        sleep 300  # 5 minutes
    done
}

# Start monitoring
monitor_security_events
```

## Security Best Practices

1. **Enable AWS Config** for continuous compliance monitoring
2. **Use AWS Security Hub** for centralized security findings
3. **Enable GuardDuty** for threat detection
4. **Implement SCPs** (Service Control Policies) for organization-wide controls
5. **Regular security audits** - at least monthly
6. **Automate remediation** where possible
7. **Use IAM Access Analyzer** to validate policies