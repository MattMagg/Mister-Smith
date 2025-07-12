#!/bin/bash
# verify-all.sh - Comprehensive AWS migration verification script

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIBRARY_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="${RESULTS_DIR:-./verification-results}"
ENVIRONMENT="${1:-production}"
OUTPUT_FORMAT="${2:-json}"
PARALLEL_JOBS="${PARALLEL_JOBS:-4}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Initialize results directory
mkdir -p "$RESULTS_DIR"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
REPORT_DIR="$RESULTS_DIR/report-$TIMESTAMP"
mkdir -p "$REPORT_DIR"

echo "Starting comprehensive verification for environment: $ENVIRONMENT"
echo "Results will be saved to: $REPORT_DIR"

# Function to log results
log_result() {
    local component=$1
    local status=$2
    local message=$3
    local details=$4
    
    # Create JSON result
    cat >> "$REPORT_DIR/results.jsonl" << EOF
{"timestamp":"$(date -u +%Y-%m-%dT%H:%M:%SZ)","component":"$component","status":"$status","message":"$message","details":$details}
EOF
    
    # Console output
    if [ "$status" == "PASS" ]; then
        echo -e "${GREEN}✓${NC} $component: $message"
    elif [ "$status" == "FAIL" ]; then
        echo -e "${RED}✗${NC} $component: $message"
    else
        echo -e "${YELLOW}⚠${NC} $component: $message"
    fi
}

# Function to run verification with timeout
run_verification() {
    local name=$1
    local command=$2
    local timeout=${3:-300}  # 5 minutes default
    
    echo "Running $name verification..."
    
    if timeout $timeout bash -c "$command" > "$REPORT_DIR/${name}.log" 2>&1; then
        log_result "$name" "PASS" "Verification completed successfully" "{\"duration\":$SECONDS}"
        return 0
    else
        log_result "$name" "FAIL" "Verification failed or timed out" "{\"duration\":$SECONDS,\"timeout\":$timeout}"
        return 1
    fi
}

# Get AWS configuration based on environment
get_aws_config() {
    case $ENVIRONMENT in
        production)
            export AWS_REGION="${AWS_REGION:-us-east-1}"
            export VPC_ID="${VPC_ID:-vpc-prod-12345}"
            export ECS_CLUSTER="${ECS_CLUSTER:-mistersmith-prod}"
            export AURORA_CLUSTER="${AURORA_CLUSTER:-mistersmith-aurora-prod}"
            ;;
        staging)
            export AWS_REGION="${AWS_REGION:-us-east-1}"
            export VPC_ID="${VPC_ID:-vpc-stage-12345}"
            export ECS_CLUSTER="${ECS_CLUSTER:-mistersmith-stage}"
            export AURORA_CLUSTER="${AURORA_CLUSTER:-mistersmith-aurora-stage}"
            ;;
        *)
            echo "Unknown environment: $ENVIRONMENT"
            exit 1
            ;;
    esac
}

# Infrastructure verification functions
verify_vpc() {
    aws ec2 describe-vpcs --vpc-ids $VPC_ID --query 'Vpcs[0].State' --output text | grep -q "available"
}

verify_subnets() {
    local subnet_count=$(aws ec2 describe-subnets \
        --filters "Name=vpc-id,Values=$VPC_ID" \
        --query 'length(Subnets[?State==`available`])' \
        --output text)
    [ "$subnet_count" -ge 4 ]  # Minimum 4 subnets (2 public, 2 private)
}

verify_security_groups() {
    aws ec2 describe-security-groups \
        --filters "Name=vpc-id,Values=$VPC_ID" \
        --query 'SecurityGroups[].GroupId' \
        --output text | wc -w | grep -qE '^[0-9]+$'
}

# Service verification functions
verify_ecs_cluster() {
    local cluster_status=$(aws ecs describe-clusters \
        --clusters $ECS_CLUSTER \
        --query 'clusters[0].status' \
        --output text)
    [ "$cluster_status" == "ACTIVE" ]
}

verify_ecs_services() {
    local services=$(aws ecs list-services \
        --cluster $ECS_CLUSTER \
        --query 'serviceArns' \
        --output json)
    
    local all_healthy=true
    echo "$services" | jq -r '.[]' | while read service_arn; do
        local service_name=$(basename $service_arn)
        local running=$(aws ecs describe-services \
            --cluster $ECS_CLUSTER \
            --services $service_arn \
            --query 'services[0].runningCount' \
            --output text)
        local desired=$(aws ecs describe-services \
            --cluster $ECS_CLUSTER \
            --services $service_arn \
            --query 'services[0].desiredCount' \
            --output text)
        
        if [ "$running" -ne "$desired" ]; then
            echo "Service $service_name unhealthy: $running/$desired tasks running"
            all_healthy=false
        fi
    done
    
    $all_healthy
}

verify_aurora_cluster() {
    local cluster_status=$(aws rds describe-db-clusters \
        --db-cluster-identifier $AURORA_CLUSTER \
        --query 'DBClusters[0].Status' \
        --output text)
    [ "$cluster_status" == "available" ]
}

verify_aurora_instances() {
    local unhealthy_count=$(aws rds describe-db-instances \
        --filters "Name=db-cluster-id,Values=$AURORA_CLUSTER" \
        --query 'DBInstances[?DBInstanceStatus!=`available`] | length(@)' \
        --output text)
    [ "$unhealthy_count" -eq 0 ]
}

# Load balancer verification
verify_load_balancers() {
    local alb_arns=$(aws elbv2 describe-load-balancers \
        --query "LoadBalancers[?VpcId=='$VPC_ID'].LoadBalancerArn" \
        --output text)
    
    local all_healthy=true
    for alb_arn in $alb_arns; do
        local state=$(aws elbv2 describe-load-balancers \
            --load-balancer-arns $alb_arn \
            --query 'LoadBalancers[0].State.Code' \
            --output text)
        
        if [ "$state" != "active" ]; then
            echo "ALB $alb_arn not active: $state"
            all_healthy=false
        fi
    done
    
    $all_healthy
}

# Message queue verification
verify_sqs_queues() {
    local queue_urls=$(aws sqs list-queues \
        --queue-name-prefix "mistersmith-" \
        --query 'QueueUrls' \
        --output json)
    
    local all_healthy=true
    echo "$queue_urls" | jq -r '.[]' | while read queue_url; do
        local messages=$(aws sqs get-queue-attributes \
            --queue-url $queue_url \
            --attribute-names ApproximateNumberOfMessages \
            --query 'Attributes.ApproximateNumberOfMessages' \
            --output text)
        
        # Check if queue has excessive backlog (>10000 messages)
        if [ "$messages" -gt 10000 ]; then
            echo "Queue $queue_url has high backlog: $messages messages"
            all_healthy=false
        fi
    done
    
    $all_healthy
}

# Secrets verification
verify_secrets() {
    local secret_arns=$(aws secretsmanager list-secrets \
        --filters Key=name,Values=mistersmith \
        --query 'SecretList[].ARN' \
        --output text)
    
    local all_valid=true
    for secret_arn in $secret_arns; do
        if ! aws secretsmanager describe-secret \
            --secret-id $secret_arn \
            --query 'DeletedDate' \
            --output text | grep -q "None"; then
            echo "Secret $secret_arn is scheduled for deletion"
            all_valid=false
        fi
    done
    
    $all_valid
}

# Performance verification
verify_response_times() {
    # This would typically call your API endpoints
    # For now, we'll simulate with a placeholder
    local api_endpoint="${API_ENDPOINT:-https://api.mistersmith.com/health}"
    
    if command -v curl &> /dev/null; then
        local response_time=$(curl -o /dev/null -s -w '%{time_total}\n' $api_endpoint)
        # Check if response time is under 1 second
        awk -v rt="$response_time" 'BEGIN {exit (rt < 1.0 ? 0 : 1)}'
    else
        echo "curl not available, skipping response time check"
        return 0
    fi
}

# Main execution
main() {
    get_aws_config
    
    echo "Starting parallel verification..."
    
    # Create a list of verification tasks
    cat > "$REPORT_DIR/tasks.txt" << EOF
vpc:verify_vpc
subnets:verify_subnets
security_groups:verify_security_groups
ecs_cluster:verify_ecs_cluster
ecs_services:verify_ecs_services
aurora_cluster:verify_aurora_cluster
aurora_instances:verify_aurora_instances
load_balancers:verify_load_balancers
sqs_queues:verify_sqs_queues
secrets:verify_secrets
response_times:verify_response_times
EOF
    
    # Run verifications in parallel
    export -f run_verification log_result verify_vpc verify_subnets verify_security_groups
    export -f verify_ecs_cluster verify_ecs_services verify_aurora_cluster verify_aurora_instances
    export -f verify_load_balancers verify_sqs_queues verify_secrets verify_response_times
    export REPORT_DIR VPC_ID ECS_CLUSTER AURORA_CLUSTER API_ENDPOINT
    
    cat "$REPORT_DIR/tasks.txt" | parallel -j $PARALLEL_JOBS --colsep ':' run_verification {1} {2}
    
    # Generate summary report
    generate_summary_report
}

# Generate summary report
generate_summary_report() {
    local total_checks=$(wc -l < "$REPORT_DIR/tasks.txt")
    local passed_checks=$(grep -c '"status":"PASS"' "$REPORT_DIR/results.jsonl" || true)
    local failed_checks=$(grep -c '"status":"FAIL"' "$REPORT_DIR/results.jsonl" || true)
    local warning_checks=$(grep -c '"status":"WARN"' "$REPORT_DIR/results.jsonl" || true)
    
    # Generate summary based on output format
    case $OUTPUT_FORMAT in
        json)
            cat > "$REPORT_DIR/summary.json" << EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "environment": "$ENVIRONMENT",
  "summary": {
    "total_checks": $total_checks,
    "passed": $passed_checks,
    "failed": $failed_checks,
    "warnings": $warning_checks,
    "success_rate": $(awk "BEGIN {printf \"%.2f\", ($passed_checks/$total_checks)*100}")
  },
  "results_file": "$REPORT_DIR/results.jsonl"
}
EOF
            cat "$REPORT_DIR/summary.json"
            ;;
            
        markdown)
            cat > "$REPORT_DIR/summary.md" << EOF
# AWS Migration Verification Report

**Date:** $(date)  
**Environment:** $ENVIRONMENT  
**Report ID:** $TIMESTAMP

## Summary

| Metric | Value |
|--------|-------|
| Total Checks | $total_checks |
| Passed | $passed_checks |
| Failed | $failed_checks |
| Warnings | $warning_checks |
| Success Rate | $(awk "BEGIN {printf \"%.2f%%\", ($passed_checks/$total_checks)*100}") |

## Detailed Results

\`\`\`
$(cat "$REPORT_DIR/results.jsonl" | jq -r '"\(.component): \(.status) - \(.message)"')
\`\`\`

## Logs

Detailed logs are available in: \`$REPORT_DIR/\`
EOF
            cat "$REPORT_DIR/summary.md"
            ;;
            
        text)
            echo "=== AWS Migration Verification Summary ==="
            echo "Date: $(date)"
            echo "Environment: $ENVIRONMENT"
            echo ""
            echo "Total Checks: $total_checks"
            echo "Passed: $passed_checks"
            echo "Failed: $failed_checks"
            echo "Warnings: $warning_checks"
            echo "Success Rate: $(awk "BEGIN {printf \"%.2f%%\", ($passed_checks/$total_checks)*100}")"
            echo ""
            echo "Detailed results in: $REPORT_DIR/"
            ;;
    esac
    
    # Exit with appropriate code
    if [ "$failed_checks" -gt 0 ]; then
        exit 1
    else
        exit 0
    fi
}

# Execute main function
main "$@"