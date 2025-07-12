# VPC Verification Commands

## Overview
Commands for verifying VPC configuration, subnets, routing tables, and network connectivity.

## Prerequisites
- AWS CLI configured with appropriate credentials
- jq installed for JSON parsing
- Network utilities (ping, traceroute, nc)

## Commands

### 1. Verify VPC Configuration
```bash
# Get VPC details
aws ec2 describe-vpcs \
  --vpc-ids vpc-12345 \
  --query 'Vpcs[0].[VpcId,CidrBlock,State,Tags[?Key==`Name`].Value|[0]]' \
  --output table

# Expected output:
# -----------------------------------------------
# |              DescribeVpcs                   |
# +-------------+-----------------+-------------+
# | vpc-12345   | 10.0.0.0/16    | available   |
# | MisterSmith-Prod-VPC                        |
# +-------------+-----------------+-------------+
```

### 2. Verify Subnets
```bash
# List all subnets in VPC
aws ec2 describe-subnets \
  --filters "Name=vpc-id,Values=vpc-12345" \
  --query 'Subnets[].[SubnetId,CidrBlock,AvailabilityZone,Tags[?Key==`Name`].Value|[0],State]' \
  --output table

# Verify subnet connectivity
function verify_subnet_connectivity() {
    local subnet_id=$1
    local instance_id=$(aws ec2 describe-instances \
      --filters "Name=subnet-id,Values=$subnet_id" \
      "Name=instance-state-name,Values=running" \
      --query 'Reservations[0].Instances[0].InstanceId' \
      --output text)
    
    if [ "$instance_id" != "None" ]; then
        echo "Testing connectivity in subnet $subnet_id via instance $instance_id"
        aws ssm start-session \
          --target $instance_id \
          --document-name AWS-RunShellScript \
          --parameters 'commands=["curl -I https://www.amazonaws.com"]'
    fi
}
```

### 3. Verify Route Tables
```bash
# Get route tables for VPC
aws ec2 describe-route-tables \
  --filters "Name=vpc-id,Values=vpc-12345" \
  --query 'RouteTables[].[RouteTableId,Tags[?Key==`Name`].Value|[0]]' \
  --output table

# Verify specific routes
function verify_routes() {
    local route_table_id=$1
    aws ec2 describe-route-tables \
      --route-table-ids $route_table_id \
      --query 'RouteTables[0].Routes[].[DestinationCidrBlock,GatewayId,NatGatewayId,State]' \
      --output table
}

# Check for black holes
aws ec2 describe-route-tables \
  --filters "Name=vpc-id,Values=vpc-12345" \
  --query 'RouteTables[].Routes[?State==`blackhole`].[RouteTableId,DestinationCidrBlock]' \
  --output table
```

### 4. Verify Internet Gateway
```bash
# Check IGW attachment
aws ec2 describe-internet-gateways \
  --filters "Name=attachment.vpc-id,Values=vpc-12345" \
  --query 'InternetGateways[].[InternetGatewayId,Attachments[0].State]' \
  --output table

# Test internet connectivity from public subnet
function test_internet_connectivity() {
    local public_subnet_id=$1
    local test_instance=$(aws ec2 run-instances \
      --image-id ami-12345 \
      --instance-type t3.micro \
      --subnet-id $public_subnet_id \
      --associate-public-ip-address \
      --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=connectivity-test}]' \
      --query 'Instances[0].InstanceId' \
      --output text)
    
    echo "Waiting for instance to be running..."
    aws ec2 wait instance-running --instance-ids $test_instance
    
    # Test connectivity
    aws ssm start-session \
      --target $test_instance \
      --document-name AWS-RunShellScript \
      --parameters 'commands=["curl -s -o /dev/null -w \"%{http_code}\" https://www.google.com"]'
    
    # Cleanup
    aws ec2 terminate-instances --instance-ids $test_instance
}
```

### 5. Verify NAT Gateways
```bash
# List NAT gateways
aws ec2 describe-nat-gateways \
  --filter "Name=vpc-id,Values=vpc-12345" \
  --query 'NatGateways[].[NatGatewayId,State,SubnetId,PublicIp]' \
  --output table

# Check NAT gateway metrics
function check_nat_gateway_health() {
    local nat_gateway_id=$1
    aws cloudwatch get-metric-statistics \
      --namespace AWS/EC2 \
      --metric-name ActiveConnectionCount \
      --dimensions Name=NatGatewayId,Value=$nat_gateway_id \
      --start-time $(date -u -d '5 minutes ago' +%Y-%m-%dT%H:%M:%S) \
      --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
      --period 300 \
      --statistics Average \
      --query 'Datapoints[0].Average' \
      --output text
}
```

### 6. Verify VPC Endpoints
```bash
# List VPC endpoints
aws ec2 describe-vpc-endpoints \
  --filters "Name=vpc-id,Values=vpc-12345" \
  --query 'VpcEndpoints[].[VpcEndpointId,ServiceName,State,VpcEndpointType]' \
  --output table

# Test S3 endpoint connectivity
function test_s3_endpoint() {
    local vpc_endpoint_id=$1
    # Run from instance in VPC
    aws s3 ls --endpoint-url https://s3.amazonaws.com --region us-east-1
}
```

### 7. Verify Network ACLs
```bash
# List Network ACLs
aws ec2 describe-network-acls \
  --filters "Name=vpc-id,Values=vpc-12345" \
  --query 'NetworkAcls[].[NetworkAclId,IsDefault,Tags[?Key==`Name`].Value|[0]]' \
  --output table

# Check NACL rules
function verify_nacl_rules() {
    local nacl_id=$1
    echo "Ingress Rules:"
    aws ec2 describe-network-acls \
      --network-acl-ids $nacl_id \
      --query 'NetworkAcls[0].Entries[?Egress==`false`].[RuleNumber,Protocol,RuleAction,CidrBlock,PortRange]' \
      --output table
    
    echo "Egress Rules:"
    aws ec2 describe-network-acls \
      --network-acl-ids $nacl_id \
      --query 'NetworkAcls[0].Entries[?Egress==`true`].[RuleNumber,Protocol,RuleAction,CidrBlock,PortRange]' \
      --output table
}
```

### 8. Verify VPC Flow Logs
```bash
# Check if flow logs are enabled
aws ec2 describe-flow-logs \
  --filter "Name=resource-id,Values=vpc-12345" \
  --query 'FlowLogs[].[FlowLogId,FlowLogStatus,LogDestinationType,LogGroupName]' \
  --output table

# Analyze flow logs for issues
function analyze_flow_logs() {
    local log_group=$1
    aws logs filter-log-events \
      --log-group-name $log_group \
      --start-time $(date -d '1 hour ago' +%s)000 \
      --filter-pattern '[version, account, eni, source, destination, srcport, destport, protocol, packets, bytes, windowstart, windowend, action="REJECT", flowlogstatus]' \
      --query 'events[0:10].[message]' \
      --output table
}
```

### 9. Comprehensive VPC Health Check
```bash
#!/bin/bash
# Complete VPC verification script

function verify_vpc_health() {
    local vpc_id=$1
    local report_file="vpc-verification-$(date +%Y%m%d-%H%M%S).json"
    
    echo "Starting VPC verification for $vpc_id..."
    
    # Initialize report
    echo '{"vpc_id": "'$vpc_id'", "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'", "checks": {}}' > $report_file
    
    # Check VPC state
    vpc_state=$(aws ec2 describe-vpcs --vpc-ids $vpc_id --query 'Vpcs[0].State' --output text)
    jq '.checks.vpc_state = "'$vpc_state'"' $report_file > tmp.json && mv tmp.json $report_file
    
    # Check subnets
    subnet_count=$(aws ec2 describe-subnets --filters "Name=vpc-id,Values=$vpc_id" --query 'length(Subnets)' --output text)
    jq '.checks.subnet_count = '$subnet_count'' $report_file > tmp.json && mv tmp.json $report_file
    
    # Check route tables
    route_tables=$(aws ec2 describe-route-tables --filters "Name=vpc-id,Values=$vpc_id" --query 'length(RouteTables)' --output text)
    jq '.checks.route_table_count = '$route_tables'' $report_file > tmp.json && mv tmp.json $report_file
    
    # Check IGW
    igw_attached=$(aws ec2 describe-internet-gateways --filters "Name=attachment.vpc-id,Values=$vpc_id" --query 'length(InternetGateways)' --output text)
    jq '.checks.internet_gateway_attached = '$igw_attached'' $report_file > tmp.json && mv tmp.json $report_file
    
    # Check NAT gateways
    nat_gateways=$(aws ec2 describe-nat-gateways --filter "Name=vpc-id,Values=$vpc_id" "Name=state,Values=available" --query 'length(NatGateways)' --output text)
    jq '.checks.nat_gateways_available = '$nat_gateways'' $report_file > tmp.json && mv tmp.json $report_file
    
    # Generate summary
    echo "VPC Verification Complete. Report saved to: $report_file"
    cat $report_file | jq .
}

# Execute verification
verify_vpc_health "vpc-12345"
```

## Error Handling

### Common Errors and Solutions

1. **VPC Not Found**
   ```bash
   Error: The vpc ID 'vpc-xxxxx' does not exist
   Solution: Verify VPC ID and region
   ```

2. **Insufficient Permissions**
   ```bash
   Error: UnauthorizedOperation
   Solution: Add required IAM permissions:
   - ec2:DescribeVpcs
   - ec2:DescribeSubnets
   - ec2:DescribeRouteTables
   ```

3. **Connectivity Issues**
   ```bash
   # Test from bastion host
   ssh bastion-host 'curl -s -o /dev/null -w "%{http_code}" http://internal-service'
   ```

## Automation Script
```bash
#!/bin/bash
# vpc-verify-all.sh - Complete VPC verification

set -euo pipefail

VPC_ID=${1:-}
OUTPUT_DIR=${2:-"./verification-results"}

if [ -z "$VPC_ID" ]; then
    echo "Usage: $0 <vpc-id> [output-dir]"
    exit 1
fi

mkdir -p "$OUTPUT_DIR"

echo "Verifying VPC: $VPC_ID"
echo "Output directory: $OUTPUT_DIR"

# Run all verifications
./verify-vpc-config.sh "$VPC_ID" > "$OUTPUT_DIR/vpc-config.txt" 2>&1
./verify-subnets.sh "$VPC_ID" > "$OUTPUT_DIR/subnets.txt" 2>&1
./verify-routes.sh "$VPC_ID" > "$OUTPUT_DIR/routes.txt" 2>&1
./verify-gateways.sh "$VPC_ID" > "$OUTPUT_DIR/gateways.txt" 2>&1
./verify-endpoints.sh "$VPC_ID" > "$OUTPUT_DIR/endpoints.txt" 2>&1

# Generate summary report
cat > "$OUTPUT_DIR/summary.md" << EOF
# VPC Verification Summary
Date: $(date)
VPC ID: $VPC_ID

## Results
- VPC Configuration: $(grep -c "PASSED" "$OUTPUT_DIR/vpc-config.txt" || echo "0") passed
- Subnets: $(grep -c "PASSED" "$OUTPUT_DIR/subnets.txt" || echo "0") passed
- Routes: $(grep -c "PASSED" "$OUTPUT_DIR/routes.txt" || echo "0") passed
- Gateways: $(grep -c "PASSED" "$OUTPUT_DIR/gateways.txt" || echo "0") passed
- Endpoints: $(grep -c "PASSED" "$OUTPUT_DIR/endpoints.txt" || echo "0") passed

## Issues Found
$(grep "FAILED\|ERROR" "$OUTPUT_DIR"/*.txt | head -10 || echo "No issues found")
EOF

echo "Verification complete. Summary available at: $OUTPUT_DIR/summary.md"
```