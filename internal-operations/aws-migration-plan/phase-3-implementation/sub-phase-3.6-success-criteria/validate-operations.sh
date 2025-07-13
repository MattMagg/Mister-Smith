#!/bin/bash
# Operations Validation Script for MisterSmith AWS Migration

set -euo pipefail

RESULTS_FILE="operations-validation-$(date +%Y%m%d-%H%M%S).log"
PASS_COUNT=0
FAIL_COUNT=0

echo "🛠️  Operations Validation Starting..." | tee -a "$RESULTS_FILE"
echo "====================================" | tee -a "$RESULTS_FILE"

# Function to check operational component
check_operation() {
    local test_name=$1
    local command=$2
    local expected=$3
    
    actual=$(eval "$command" 2>/dev/null || echo "error")
    
    if [[ "$actual" == "$expected" ]] || [[ "$actual" =~ $expected ]]; then
        echo "✅ $test_name: PASS" | tee -a "$RESULTS_FILE"
        ((PASS_COUNT++))
        return 0
    else
        echo "❌ $test_name: FAIL (Expected: $expected, Actual: $actual)" | tee -a "$RESULTS_FILE"
        ((FAIL_COUNT++))
        return 1
    fi
}

# 1. Monitoring Dashboards
echo -e "\n📍 Checking Monitoring Dashboards..." | tee -a "$RESULTS_FILE"
alarm_count=$(aws cloudwatch describe-alarms --state-value OK --query 'length(MetricAlarms)' --output text 2>/dev/null || echo "0")
if [[ $alarm_count -ge 20 ]]; then
    echo "✅ CloudWatch Alarms: PASS ($alarm_count alarms configured)" | tee -a "$RESULTS_FILE"
    ((PASS_COUNT++))
else
    echo "❌ CloudWatch Alarms: FAIL (Only $alarm_count alarms found)" | tee -a "$RESULTS_FILE"
    ((FAIL_COUNT++))
fi

# 2. Log Aggregation
echo -e "\n📍 Checking Log Aggregation..." | tee -a "$RESULTS_FILE"
log_group_count=$(aws logs describe-log-groups --log-group-name-prefix /mistersmith/ --query 'length(logGroups)' --output text 2>/dev/null || echo "0")
check_operation "Log Groups Created" "echo $log_group_count" "[5-9]|[1-9][0-9]+"

# 3. Alerting System
echo -e "\n📍 Testing Alerting System..." | tee -a "$RESULTS_FILE"
sns_topic=$(aws sns list-topics --query "Topics[?contains(TopicArn, 'MisterSmith')].TopicArn | [0]" --output text 2>/dev/null || echo "none")
if [[ "$sns_topic" != "none" && "$sns_topic" != "null" ]]; then
    echo "✅ SNS Topic: PASS (Topic: $sns_topic)" | tee -a "$RESULTS_FILE"
    ((PASS_COUNT++))
else
    echo "❌ SNS Topic: FAIL (No MisterSmith topic found)" | tee -a "$RESULTS_FILE"
    ((FAIL_COUNT++))
fi

# 4. Backup Procedures
echo -e "\n📍 Verifying Backup Procedures..." | tee -a "$RESULTS_FILE"
backup_count=$(aws rds describe-db-cluster-snapshots --db-cluster-identifier mistersmith-aurora --query 'length(DBClusterSnapshots)' --output text 2>/dev/null || echo "0")
if [[ $backup_count -ge 1 ]]; then
    echo "✅ Database Backups: PASS ($backup_count snapshots found)" | tee -a "$RESULTS_FILE"
    ((PASS_COUNT++))
else
    echo "❌ Database Backups: FAIL (No snapshots found)" | tee -a "$RESULTS_FILE"
    ((FAIL_COUNT++))
fi

# 5. Rollback Readiness
echo -e "\n📍 Checking Rollback Readiness..." | tee -a "$RESULTS_FILE"
# Check for rollback scripts
if [[ -f "/Users/mac-main/Mister-Smith/MisterSmith/aws-migration-plan/rollback/rollback-plan.sh" ]]; then
    echo "✅ Rollback Plan: PASS (Script exists)" | tee -a "$RESULTS_FILE"
    ((PASS_COUNT++))
else
    echo "⚠️  Rollback Plan: WARNING (Script not found)" | tee -a "$RESULTS_FILE"
fi

# Summary
echo -e "\n📊 Operations Validation Summary" | tee -a "$RESULTS_FILE"
echo "===================================" | tee -a "$RESULTS_FILE"
echo "✅ Passed: $PASS_COUNT" | tee -a "$RESULTS_FILE"
echo "❌ Failed: $FAIL_COUNT" | tee -a "$RESULTS_FILE"
echo "📁 Results saved to: $RESULTS_FILE" | tee -a "$RESULTS_FILE"

# Exit with appropriate code
if [[ $FAIL_COUNT -eq 0 ]]; then
    echo -e "\n🎉 All operations checks PASSED!" | tee -a "$RESULTS_FILE"
    exit 0
else
    echo -e "\n⚠️  Some operations checks FAILED. Review the results." | tee -a "$RESULTS_FILE"
    exit 1
fi