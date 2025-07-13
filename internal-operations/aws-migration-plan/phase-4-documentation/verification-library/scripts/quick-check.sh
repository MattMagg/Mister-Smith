#!/bin/bash
# quick-check.sh - Fast health check for critical services

set -euo pipefail

# Configuration
SERVICES="${1:-ecs,aurora,sqs}"
ENVIRONMENT="${ENVIRONMENT:-production}"
TIMEOUT="${TIMEOUT:-30}"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Quick check results
RESULTS=()
FAILED=0

# Helper function for timed checks
run_check() {
    local name=$1
    local command=$2
    local start_time=$(date +%s)
    
    if timeout $TIMEOUT bash -c "$command" &>/dev/null; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        echo -e "${GREEN}✓${NC} $name (${duration}s)"
        RESULTS+=("$name:PASS:${duration}s")
    else
        echo -e "${RED}✗${NC} $name (timeout or failed)"
        RESULTS+=("$name:FAIL:timeout")
        ((FAILED++))
    fi
}

echo "=== MisterSmith Quick Health Check ==="
echo "Environment: $ENVIRONMENT"
echo "Services: $SERVICES"
echo "Timeout: ${TIMEOUT}s per check"
echo ""

# Parse services
IFS=',' read -ra SERVICE_ARRAY <<< "$SERVICES"

# ECS checks
if [[ " ${SERVICE_ARRAY[@]} " =~ " ecs " ]]; then
    echo "Checking ECS..."
    
    # Cluster status
    run_check "ECS Cluster" "aws ecs describe-clusters --clusters mistersmith-$ENVIRONMENT --query 'clusters[0].status' --output text | grep -q ACTIVE"
    
    # Service status
    run_check "ECS Services" "aws ecs list-services --cluster mistersmith-$ENVIRONMENT --query 'length(serviceArns)' --output text | grep -qE '^[1-9][0-9]*$'"
    
    # Running tasks
    run_check "ECS Tasks" "aws ecs list-tasks --cluster mistersmith-$ENVIRONMENT --desired-status RUNNING --query 'length(taskArns)' --output text | grep -qE '^[1-9][0-9]*$'"
fi

# Aurora checks
if [[ " ${SERVICE_ARRAY[@]} " =~ " aurora " ]]; then
    echo -e "\nChecking Aurora..."
    
    # Cluster status
    run_check "Aurora Cluster" "aws rds describe-db-clusters --db-cluster-identifier mistersmith-aurora-$ENVIRONMENT --query 'DBClusters[0].Status' --output text | grep -q available"
    
    # Instance status
    run_check "Aurora Instances" "aws rds describe-db-instances --filters Name=db-cluster-id,Values=mistersmith-aurora-$ENVIRONMENT --query 'DBInstances[?DBInstanceStatus==\`available\`] | length(@)' --output text | grep -qE '^[1-9][0-9]*$'"
fi

# SQS checks
if [[ " ${SERVICE_ARRAY[@]} " =~ " sqs " ]]; then
    echo -e "\nChecking SQS..."
    
    # Queue existence
    run_check "SQS Queues" "aws sqs list-queues --queue-name-prefix mistersmith-$ENVIRONMENT --query 'length(QueueUrls)' --output text | grep -qE '^[1-9][0-9]*$'"
    
    # Check for high message count (warning only)
    QUEUE_URL=$(aws sqs list-queues --queue-name-prefix "mistersmith-$ENVIRONMENT" --query 'QueueUrls[0]' --output text 2>/dev/null || echo "")
    if [ ! -z "$QUEUE_URL" ]; then
        MSG_COUNT=$(aws sqs get-queue-attributes --queue-url "$QUEUE_URL" --attribute-names ApproximateNumberOfMessages --query 'Attributes.ApproximateNumberOfMessages' --output text 2>/dev/null || echo "0")
        if [ "$MSG_COUNT" -gt 1000 ]; then
            echo -e "${YELLOW}⚠${NC}  Queue backlog: $MSG_COUNT messages"
        fi
    fi
fi

# SNS checks
if [[ " ${SERVICE_ARRAY[@]} " =~ " sns " ]]; then
    echo -e "\nChecking SNS..."
    
    # Topic existence
    run_check "SNS Topics" "aws sns list-topics --query 'Topics[?contains(TopicArn, \`mistersmith-$ENVIRONMENT\`)] | length(@)' --output text | grep -qE '^[1-9][0-9]*$'"
fi

# ALB checks
if [[ " ${SERVICE_ARRAY[@]} " =~ " alb " ]]; then
    echo -e "\nChecking Load Balancers..."
    
    # ALB status
    ALB_ARN=$(aws elbv2 describe-load-balancers --names "mistersmith-$ENVIRONMENT-alb" --query 'LoadBalancers[0].LoadBalancerArn' --output text 2>/dev/null || echo "")
    if [ ! -z "$ALB_ARN" ] && [ "$ALB_ARN" != "None" ]; then
        run_check "ALB Status" "aws elbv2 describe-load-balancers --load-balancer-arns $ALB_ARN --query 'LoadBalancers[0].State.Code' --output text | grep -q active"
        
        # Target health
        TG_ARN=$(aws elbv2 describe-target-groups --load-balancer-arn $ALB_ARN --query 'TargetGroups[0].TargetGroupArn' --output text 2>/dev/null || echo "")
        if [ ! -z "$TG_ARN" ] && [ "$TG_ARN" != "None" ]; then
            HEALTHY_COUNT=$(aws elbv2 describe-target-health --target-group-arn $TG_ARN --query 'TargetHealthDescriptions[?TargetHealth.State==\`healthy\`] | length(@)' --output text 2>/dev/null || echo "0")
            if [ "$HEALTHY_COUNT" -gt 0 ]; then
                echo -e "${GREEN}✓${NC} Healthy targets: $HEALTHY_COUNT"
            else
                echo -e "${RED}✗${NC} No healthy targets"
                ((FAILED++))
            fi
        fi
    fi
fi

# Secrets checks
if [[ " ${SERVICE_ARRAY[@]} " =~ " secrets " ]]; then
    echo -e "\nChecking Secrets..."
    
    # Secrets existence
    run_check "Secrets Manager" "aws secretsmanager list-secrets --filters Key=name,Values=mistersmith/$ENVIRONMENT --query 'length(SecretList)' --output text | grep -qE '^[1-9][0-9]*$'"
    
    # Parameter Store
    run_check "Parameter Store" "aws ssm describe-parameters --filters Key=Name,Values=/mistersmith/$ENVIRONMENT/ --query 'length(Parameters)' --output text | grep -qE '^[1-9][0-9]*$'"
fi

# API checks
if [[ " ${SERVICE_ARRAY[@]} " =~ " api " ]]; then
    echo -e "\nChecking API..."
    
    # API Gateway
    API_ID=$(aws apigateway get-rest-apis --query "items[?name=='mistersmith-$ENVIRONMENT'].id" --output text 2>/dev/null || echo "")
    if [ ! -z "$API_ID" ] && [ "$API_ID" != "None" ]; then
        run_check "API Gateway" "aws apigateway get-rest-api --rest-api-id $API_ID --query 'name' --output text | grep -q mistersmith"
    fi
    
    # Health endpoint (if accessible)
    API_ENDPOINT="${API_ENDPOINT:-https://api-$ENVIRONMENT.mistersmith.com/health}"
    if command -v curl &>/dev/null && [[ ! -z "$API_ENDPOINT" ]]; then
        run_check "API Health Endpoint" "curl -sf -m 5 $API_ENDPOINT -o /dev/null"
    fi
fi

# Summary
echo ""
echo "=== Summary ==="
echo "Total checks: ${#RESULTS[@]}"
echo "Failed: $FAILED"

# Generate timestamp
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

# Save results if requested
if [ "${SAVE_RESULTS:-false}" == "true" ]; then
    RESULT_FILE="quick-check-$TIMESTAMP.txt"
    printf '%s\n' "${RESULTS[@]}" > "$RESULT_FILE"
    echo "Results saved to: $RESULT_FILE"
fi

# Exit code based on failures
if [ $FAILED -gt 0 ]; then
    echo -e "\n${RED}Health check failed with $FAILED errors${NC}"
    exit 1
else
    echo -e "\n${GREEN}All health checks passed${NC}"
    exit 0
fi