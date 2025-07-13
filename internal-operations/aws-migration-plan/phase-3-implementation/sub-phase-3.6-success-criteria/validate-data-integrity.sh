#!/bin/bash
# Data Integrity Validation Script for MisterSmith AWS Migration

set -euo pipefail

RESULTS_FILE="data-integrity-validation-$(date +%Y%m%d-%H%M%S).log"
PASS_COUNT=0
FAIL_COUNT=0
ALB_ENDPOINT="${ALB_ENDPOINT:-http://alb.mistersmith.com}"

echo "🔒 Data Integrity Validation Starting..." | tee -a "$RESULTS_FILE"
echo "=======================================" | tee -a "$RESULTS_FILE"

# Function to validate data consistency
check_data_integrity() {
    local test_name=$1
    local expected=$2
    local actual=$3
    
    if [[ "$actual" == "$expected" ]]; then
        echo "✅ $test_name: PASS" | tee -a "$RESULTS_FILE"
        ((PASS_COUNT++))
        return 0
    else
        echo "❌ $test_name: FAIL (Expected: $expected, Actual: $actual)" | tee -a "$RESULTS_FILE"
        ((FAIL_COUNT++))
        return 1
    fi
}

# 1. Message Delivery Test
echo -e "\n📍 Testing Message Delivery Guarantee..." | tee -a "$RESULTS_FILE"
echo "Sending 1000 test messages..." | tee -a "$RESULTS_FILE"

messages_sent=0
messages_received=0

# Send messages
for i in $(seq 1 1000); do
    response=$(curl -s -X POST "$ALB_ENDPOINT/api/messages" \
        -H "Content-Type: application/json" \
        -d "{\"id\":\"msg-$i\",\"content\":\"test-$i\"}" || echo '{"status":"error"}')
    
    if [[ $(echo "$response" | jq -r '.status' 2>/dev/null) == "success" ]]; then
        ((messages_sent++))
    fi
done

# Verify receipt
sleep 2  # Allow processing time
received_response=$(curl -s "$ALB_ENDPOINT/api/messages/count" || echo '{"count":0}')
messages_received=$(echo "$received_response" | jq -r '.count' 2>/dev/null || echo "0")

check_data_integrity "Message Delivery (1000 msgs)" "$messages_sent" "$messages_received"

# 2. Database Migration Validation
echo -e "\n📍 Validating Database Migration..." | tee -a "$RESULTS_FILE"

# Check table count
table_count=$(aws rds-data execute-statement \
    --resource-arn "$AURORA_CLUSTER_ARN" \
    --secret-arn "$DB_SECRET_ARN" \
    --sql "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'mistersmith'" \
    --query 'records[0][0].longValue' --output text 2>/dev/null || echo "0")

check_data_integrity "Database Tables Migrated" "15" "$table_count"

# 3. State Synchronization Test
echo -e "\n📍 Testing State Synchronization..." | tee -a "$RESULTS_FILE"

# Create state on primary
state_id="state-$(date +%s)"
create_response=$(curl -s -X POST "$ALB_ENDPOINT/api/state" \
    -H "Content-Type: application/json" \
    -d "{\"id\":\"$state_id\",\"data\":\"test-sync\"}" || echo '{"status":"error"}')

sleep 1  # Allow sync time

# Check state on replica
replica_response=$(curl -s "$ALB_ENDPOINT/api/state/$state_id?source=replica" || echo '{"data":"error"}')
replica_data=$(echo "$replica_response" | jq -r '.data' 2>/dev/null || echo "error")

check_data_integrity "State Synchronization" "test-sync" "$replica_data"

# 4. Configuration Integrity
echo -e "\n📍 Validating Configuration Integrity..." | tee -a "$RESULTS_FILE"

# Check environment configs
env_configs=$(curl -s "$ALB_ENDPOINT/api/config/validate" || echo '{"valid":0,"total":0}')
valid_configs=$(echo "$env_configs" | jq -r '.valid' 2>/dev/null || echo "0")
total_configs=$(echo "$env_configs" | jq -r '.total' 2>/dev/null || echo "0")

check_data_integrity "Configuration Files Valid" "$total_configs" "$valid_configs"

# 5. Security Access Control
echo -e "\n📍 Testing Security & Access Control..." | tee -a "$RESULTS_FILE"

# Test unauthorized access
unauth_response=$(curl -s -o /dev/null -w "%{http_code}" "$ALB_ENDPOINT/api/admin/users" || echo "000")
check_data_integrity "Unauthorized Access Blocked" "403" "$unauth_response"

# Test encryption status
encryption_status=$(curl -s "$ALB_ENDPOINT/api/security/encryption-status" || echo '{"transit":"false","rest":"false"}')
transit_encrypted=$(echo "$encryption_status" | jq -r '.transit' 2>/dev/null || echo "false")
rest_encrypted=$(echo "$encryption_status" | jq -r '.rest' 2>/dev/null || echo "false")

check_data_integrity "Encryption in Transit" "true" "$transit_encrypted"
check_data_integrity "Encryption at Rest" "true" "$rest_encrypted"

# Summary
echo -e "\n📊 Data Integrity Validation Summary" | tee -a "$RESULTS_FILE"
echo "=====================================" | tee -a "$RESULTS_FILE"
echo "✅ Passed: $PASS_COUNT" | tee -a "$RESULTS_FILE"
echo "❌ Failed: $FAIL_COUNT" | tee -a "$RESULTS_FILE"
echo "📁 Results saved to: $RESULTS_FILE" | tee -a "$RESULTS_FILE"

# Exit with appropriate code
if [[ $FAIL_COUNT -eq 0 ]]; then
    echo -e "\n🎉 All data integrity checks PASSED!" | tee -a "$RESULTS_FILE"
    exit 0
else
    echo -e "\n⚠️  Some data integrity checks FAILED. Review the results." | tee -a "$RESULTS_FILE"
    exit 1
fi