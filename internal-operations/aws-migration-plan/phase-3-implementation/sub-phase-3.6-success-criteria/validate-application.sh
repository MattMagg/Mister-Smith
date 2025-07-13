#!/bin/bash
# Application Functionality Validation Script for MisterSmith AWS Migration

set -euo pipefail

RESULTS_FILE="application-validation-$(date +%Y%m%d-%H%M%S).log"
PASS_COUNT=0
FAIL_COUNT=0
ALB_ENDPOINT="${ALB_ENDPOINT:-http://alb.mistersmith.com}"

echo "🚀 Application Validation Starting..." | tee -a "$RESULTS_FILE"
echo "====================================" | tee -a "$RESULTS_FILE"

# Function to check HTTP response
check_http_response() {
    local test_name=$1
    local url=$2
    local expected_code=$3
    
    response_code=$(curl -s -o /dev/null -w "%{http_code}" "$url" || echo "000")
    
    if [[ "$response_code" == "$expected_code" ]]; then
        echo "✅ $test_name: PASS (HTTP $response_code)" | tee -a "$RESULTS_FILE"
        ((PASS_COUNT++))
        return 0
    else
        echo "❌ $test_name: FAIL (Expected: $expected_code, Actual: $response_code)" | tee -a "$RESULTS_FILE"
        ((FAIL_COUNT++))
        return 1
    fi
}

# Function to check JSON response
check_json_response() {
    local test_name=$1
    local url=$2
    local json_path=$3
    local expected=$4
    
    actual=$(curl -s "$url" | jq -r "$json_path" 2>/dev/null || echo "error")
    
    if [[ "$actual" == "$expected" ]]; then
        echo "✅ $test_name: PASS" | tee -a "$RESULTS_FILE"
        ((PASS_COUNT++))
    else
        echo "❌ $test_name: FAIL (Expected: $expected, Actual: $actual)" | tee -a "$RESULTS_FILE"
        ((FAIL_COUNT++))
    fi
}

# 1. NATS to AWS Translation Test
echo -e "\n📍 Testing NATS to AWS Translation..." | tee -a "$RESULTS_FILE"
TRANSLATION_RESPONSE=$(curl -s -X POST "$ALB_ENDPOINT/api/test-translation" \
    -H "Content-Type: application/json" \
    -d '{"protocol":"nats","message":"test","destination":"aws"}' || echo '{"status":"error"}')

translation_status=$(echo "$TRANSLATION_RESPONSE" | jq -r '.status' 2>/dev/null || echo "error")
check_result() {
    if [[ "$2" == "$3" ]]; then
        echo "✅ $1: PASS" | tee -a "$RESULTS_FILE"
        ((PASS_COUNT++))
    else
        echo "❌ $1: FAIL (Expected: $2, Actual: $3)" | tee -a "$RESULTS_FILE"
        ((FAIL_COUNT++))
    fi
}
check_result "Translation Status" "success" "$translation_status"

# Check latency if successful
if [[ "$translation_status" == "success" ]]; then
    latency=$(echo "$TRANSLATION_RESPONSE" | jq -r '.latency' 2>/dev/null || echo "999")
    if [[ $latency -lt 100 ]]; then
        echo "✅ Translation Latency: PASS (${latency}ms < 100ms)" | tee -a "$RESULTS_FILE"
        ((PASS_COUNT++))
    else
        echo "❌ Translation Latency: FAIL (${latency}ms >= 100ms)" | tee -a "$RESULTS_FILE"
        ((FAIL_COUNT++))
    fi
fi

# 2. Agent Lifecycle Test
echo -e "\n📍 Testing Agent Lifecycle..." | tee -a "$RESULTS_FILE"

# Create agent
CREATE_RESPONSE=$(curl -s -X POST "$ALB_ENDPOINT/api/agents" \
    -H "Content-Type: application/json" \
    -d '{"name":"test-agent","type":"worker"}' || echo '{"id":"error"}')
    
agent_id=$(echo "$CREATE_RESPONSE" | jq -r '.id' 2>/dev/null || echo "error")
if [[ "$agent_id" != "error" && "$agent_id" != "null" ]]; then
    echo "✅ Agent Creation: PASS (ID: $agent_id)" | tee -a "$RESULTS_FILE"
    ((PASS_COUNT++))
    
    # Test agent execution
    EXEC_RESPONSE=$(curl -s -X POST "$ALB_ENDPOINT/api/agents/$agent_id/execute" \
        -H "Content-Type: application/json" \
        -d '{"task":"test-task"}' || echo '{"status":"error"}')
    
    exec_status=$(echo "$EXEC_RESPONSE" | jq -r '.status' 2>/dev/null || echo "error")
    check_result "Agent Execution" "success" "$exec_status"
    
    # Terminate agent
    TERM_RESPONSE=$(curl -s -X DELETE "$ALB_ENDPOINT/api/agents/$agent_id" || echo '{"status":"error"}')
    term_status=$(echo "$TERM_RESPONSE" | jq -r '.status' 2>/dev/null || echo "error")
    check_result "Agent Termination" "success" "$term_status"
else
    echo "❌ Agent Creation: FAIL" | tee -a "$RESULTS_FILE"
    ((FAIL_COUNT++))
fi

# 3. MCP Server Response Test
echo -e "\n📍 Testing MCP Server..." | tee -a "$RESULTS_FILE"
check_http_response "MCP Health Endpoint" "$ALB_ENDPOINT/mcp/health" "200"

# Check MCP server details
MCP_HEALTH=$(curl -s "$ALB_ENDPOINT/mcp/health" || echo '{"status":"error"}')
mcp_status=$(echo "$MCP_HEALTH" | jq -r '.status' 2>/dev/null || echo "error")
check_result "MCP Server Status" "healthy" "$mcp_status"

# 4. All Health Endpoints Test
echo -e "\n📍 Testing All Health Endpoints..." | tee -a "$RESULTS_FILE"
services=("core" "agents" "mcp" "bridge")
for service in "${services[@]}"; do
    check_http_response "$service Service Health" "$ALB_ENDPOINT/health/$service" "200"
done

# 5. Configuration Loading Test
echo -e "\n📍 Testing Configuration Loading..." | tee -a "$RESULTS_FILE"
CONFIG_TEST=$(curl -s "$ALB_ENDPOINT/api/config/test" || echo '{"loaded":"false"}')
config_loaded=$(echo "$CONFIG_TEST" | jq -r '.loaded' 2>/dev/null || echo "false")
check_result "Configuration Loading" "true" "$config_loaded"

# Test SSM parameter access (if AWS CLI configured)
if command -v aws &> /dev/null; then
    SSM_VALUE=$(aws ssm get-parameter --name /mistersmith/config/test --query 'Parameter.Value' --output text 2>/dev/null || echo "error")
    if [[ "$SSM_VALUE" != "error" ]]; then
        echo "✅ SSM Parameter Access: PASS" | tee -a "$RESULTS_FILE"
        ((PASS_COUNT++))
    else
        echo "⚠️  SSM Parameter Access: SKIPPED (No access)" | tee -a "$RESULTS_FILE"
    fi
fi

# Summary
echo -e "\n📊 Application Validation Summary" | tee -a "$RESULTS_FILE"
echo "===================================" | tee -a "$RESULTS_FILE"
echo "✅ Passed: $PASS_COUNT" | tee -a "$RESULTS_FILE"
echo "❌ Failed: $FAIL_COUNT" | tee -a "$RESULTS_FILE"
echo "📁 Results saved to: $RESULTS_FILE" | tee -a "$RESULTS_FILE"

# Exit with appropriate code
if [[ $FAIL_COUNT -eq 0 ]]; then
    echo -e "\n🎉 All application checks PASSED!" | tee -a "$RESULTS_FILE"
    exit 0
else
    echo -e "\n⚠️  Some application checks FAILED. Review the results." | tee -a "$RESULTS_FILE"
    exit 1
fi