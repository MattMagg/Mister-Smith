#!/bin/bash
# Infrastructure Validation Script for MisterSmith AWS Migration

set -euo pipefail

RESULTS_FILE="infrastructure-validation-$(date +%Y%m%d-%H%M%S).log"
PASS_COUNT=0
FAIL_COUNT=0

echo "🏗️  Infrastructure Validation Starting..." | tee -a "$RESULTS_FILE"
echo "=======================================" | tee -a "$RESULTS_FILE"

# Function to check test results
check_result() {
    local test_name=$1
    local expected=$2
    local actual=$3
    
    if [[ "$actual" == "$expected" ]]; then
        echo "✅ $test_name: PASS" | tee -a "$RESULTS_FILE"
        ((PASS_COUNT++))
    else
        echo "❌ $test_name: FAIL (Expected: $expected, Actual: $actual)" | tee -a "$RESULTS_FILE"
        ((FAIL_COUNT++))
    fi
}

# 1. VPC Validation
echo -e "\n📍 Checking VPC Status..." | tee -a "$RESULTS_FILE"
VPC_STATE=$(aws ec2 describe-vpcs --filters "Name=tag:Name,Values=MisterSmith-VPC" --query 'Vpcs[0].State' --output text 2>/dev/null || echo "error")
check_result "VPC State" "available" "$VPC_STATE"

# Check subnets
SUBNET_COUNT=$(aws ec2 describe-subnets --filters "Name=vpc-id,Values=$VPC_ID" --query 'length(Subnets)' --output text 2>/dev/null || echo "0")
check_result "Subnet Count" "4" "$SUBNET_COUNT"

# 2. Aurora Cluster Validation
echo -e "\n📍 Checking Aurora Cluster..." | tee -a "$RESULTS_FILE"
AURORA_STATUS=$(aws rds describe-db-clusters --db-cluster-identifier mistersmith-aurora --query 'DBClusters[0].Status' --output text 2>/dev/null || echo "error")
check_result "Aurora Status" "available" "$AURORA_STATUS"

# 3. ECS Services Validation
echo -e "\n📍 Checking ECS Services..." | tee -a "$RESULTS_FILE"
ECS_STATUS=$(aws ecs describe-services --cluster MisterSmith-Cluster --services mistersmith-service --query 'services[0].status' --output text 2>/dev/null || echo "error")
check_result "ECS Service Status" "ACTIVE" "$ECS_STATUS"

# Get running vs desired count
RUNNING=$(aws ecs describe-services --cluster MisterSmith-Cluster --services mistersmith-service --query 'services[0].runningCount' --output text 2>/dev/null || echo "0")
DESIRED=$(aws ecs describe-services --cluster MisterSmith-Cluster --services mistersmith-service --query 'services[0].desiredCount' --output text 2>/dev/null || echo "0")
check_result "ECS Task Count Match" "$DESIRED" "$RUNNING"

# 4. Security Groups Validation
echo -e "\n📍 Checking Security Groups..." | tee -a "$RESULTS_FILE"
SG_COUNT=$(aws ec2 describe-security-groups --filters "Name=group-name,Values=MisterSmith-*" --query 'length(SecurityGroups)' --output text 2>/dev/null || echo "0")
if [[ $SG_COUNT -ge 3 ]]; then
    check_result "Security Groups" "3+" "$SG_COUNT"
else
    check_result "Security Groups" "3+" "$SG_COUNT"
fi

# 5. IAM Roles Validation
echo -e "\n📍 Checking IAM Roles..." | tee -a "$RESULTS_FILE"
TASK_ROLE=$(aws iam get-role --role-name MisterSmithTaskRole --query 'Role.RoleName' --output text 2>/dev/null || echo "error")
check_result "Task Role" "MisterSmithTaskRole" "$TASK_ROLE"

EXEC_ROLE=$(aws iam get-role --role-name MisterSmithExecutionRole --query 'Role.RoleName' --output text 2>/dev/null || echo "error")
check_result "Execution Role" "MisterSmithExecutionRole" "$EXEC_ROLE"

# Summary
echo -e "\n📊 Infrastructure Validation Summary" | tee -a "$RESULTS_FILE"
echo "====================================" | tee -a "$RESULTS_FILE"
echo "✅ Passed: $PASS_COUNT" | tee -a "$RESULTS_FILE"
echo "❌ Failed: $FAIL_COUNT" | tee -a "$RESULTS_FILE"
echo "📁 Results saved to: $RESULTS_FILE" | tee -a "$RESULTS_FILE"

# Exit with appropriate code
if [[ $FAIL_COUNT -eq 0 ]]; then
    echo -e "\n🎉 All infrastructure checks PASSED!" | tee -a "$RESULTS_FILE"
    exit 0
else
    echo -e "\n⚠️  Some infrastructure checks FAILED. Review the results." | tee -a "$RESULTS_FILE"
    exit 1
fi