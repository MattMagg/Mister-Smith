#!/bin/bash
# verify-rollback.sh
# Comprehensive verification of MisterSmith rollback completion

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ORCHESTRATOR_PORT=50051
WEBUI_PORT=3000
NATS_PORT=4222
POSTGRES_PORT=5432
VERIFICATION_LOG="/var/log/mistersmith/verify-rollback-$(date +%Y%m%d-%H%M%S).log"

# Ensure log directory exists
mkdir -p /var/log/mistersmith

# Initialize counters
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNINGS=0

# Function to log results
log_result() {
    local status=$1
    local check=$2
    local details=$3
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    case $status in
        "PASS")
            PASSED_CHECKS=$((PASSED_CHECKS + 1))
            echo -e "${GREEN}✅ $check${NC}" | tee -a $VERIFICATION_LOG
            ;;
        "FAIL")
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
            echo -e "${RED}❌ $check${NC}" | tee -a $VERIFICATION_LOG
            ;;
        "WARN")
            WARNINGS=$((WARNINGS + 1))
            echo -e "${YELLOW}⚠️  $check${NC}" | tee -a $VERIFICATION_LOG
            ;;
    esac
    
    if [ -n "$details" ]; then
        echo "   $details" | tee -a $VERIFICATION_LOG
    fi
}

# Header
echo -e "${BLUE}═══════════════════════════════════════════${NC}" | tee $VERIFICATION_LOG
echo -e "${BLUE}   MisterSmith Rollback Verification${NC}" | tee -a $VERIFICATION_LOG
echo -e "${BLUE}═══════════════════════════════════════════${NC}" | tee -a $VERIFICATION_LOG
echo "Timestamp: $(date)" | tee -a $VERIFICATION_LOG
echo "" | tee -a $VERIFICATION_LOG

# Section 1: Service Health Checks
echo -e "${YELLOW}1. Service Health Checks${NC}" | tee -a $VERIFICATION_LOG
echo "------------------------" | tee -a $VERIFICATION_LOG

# Check Orchestrator
echo -n "Checking Orchestrator service... " | tee -a $VERIFICATION_LOG
if docker ps | grep -q mistersmith-orchestrator; then
    if grpc_health_probe -addr=localhost:${ORCHESTRATOR_PORT} 2>/dev/null; then
        log_result "PASS" "Orchestrator health check" "gRPC responding on port ${ORCHESTRATOR_PORT}"
    else
        log_result "FAIL" "Orchestrator health check" "gRPC not responding"
    fi
else
    log_result "FAIL" "Orchestrator container" "Container not running"
fi

# Check Web UI
echo -n "Checking Web UI service... " | tee -a $VERIFICATION_LOG
if docker ps | grep -q mistersmith-webui; then
    if curl -sf http://localhost:${WEBUI_PORT}/health > /dev/null; then
        log_result "PASS" "Web UI health check" "HTTP responding on port ${WEBUI_PORT}"
    else
        log_result "FAIL" "Web UI health check" "HTTP not responding"
    fi
else
    log_result "FAIL" "Web UI container" "Container not running"
fi

# Check NATS
echo -n "Checking NATS messaging... " | tee -a $VERIFICATION_LOG
if docker ps | grep -q mistersmith-nats; then
    if nats server check connection 2>/dev/null; then
        log_result "PASS" "NATS connectivity" "NATS responding on port ${NATS_PORT}"
    else
        log_result "FAIL" "NATS connectivity" "NATS not responding"
    fi
else
    log_result "FAIL" "NATS container" "Container not running"
fi

# Check PostgreSQL
echo -n "Checking PostgreSQL database... " | tee -a $VERIFICATION_LOG
if sudo -u postgres pg_isready -p ${POSTGRES_PORT} > /dev/null 2>&1; then
    log_result "PASS" "PostgreSQL availability" "Database responding on port ${POSTGRES_PORT}"
else
    log_result "FAIL" "PostgreSQL availability" "Database not responding"
fi

echo "" | tee -a $VERIFICATION_LOG

# Section 2: Data Integrity Checks
echo -e "${YELLOW}2. Data Integrity Checks${NC}" | tee -a $VERIFICATION_LOG
echo "------------------------" | tee -a $VERIFICATION_LOG

# Check database tables
echo -n "Checking database schema... " | tee -a $VERIFICATION_LOG
EXPECTED_TABLES=("agents" "tasks" "discoveries" "hive_sessions")
MISSING_TABLES=()

for table in "${EXPECTED_TABLES[@]}"; do
    if ! sudo -u postgres psql -d mistersmith -c "\dt $table" 2>/dev/null | grep -q $table; then
        MISSING_TABLES+=($table)
    fi
done

if [ ${#MISSING_TABLES[@]} -eq 0 ]; then
    log_result "PASS" "Database schema integrity" "All required tables present"
else
    log_result "FAIL" "Database schema integrity" "Missing tables: ${MISSING_TABLES[*]}"
fi

# Check for data consistency
echo -n "Checking data consistency... " | tee -a $VERIFICATION_LOG
ORPHANED_TASKS=$(sudo -u postgres psql -d mistersmith -t -c "SELECT COUNT(*) FROM tasks t LEFT JOIN agents a ON t.assigned_agent_id = a.id WHERE t.assigned_agent_id IS NOT NULL AND a.id IS NULL;" 2>/dev/null | tr -d ' ')

if [ "$ORPHANED_TASKS" = "0" ] || [ -z "$ORPHANED_TASKS" ]; then
    log_result "PASS" "Data referential integrity" "No orphaned records found"
else
    log_result "WARN" "Data referential integrity" "$ORPHANED_TASKS orphaned task records found"
fi

# Check NATS streams
echo -n "Checking NATS message streams... " | tee -a $VERIFICATION_LOG
if nats stream ls 2>/dev/null | grep -q "MisterSmith-Tasks"; then
    TASK_MESSAGES=$(nats stream info MisterSmith-Tasks -j 2>/dev/null | jq -r '.state.messages // 0')
    log_result "PASS" "NATS Tasks stream" "$TASK_MESSAGES messages in stream"
else
    log_result "FAIL" "NATS Tasks stream" "Stream not found"
fi

echo "" | tee -a $VERIFICATION_LOG

# Section 3: Performance Checks
echo -e "${YELLOW}3. Performance Baseline Checks${NC}" | tee -a $VERIFICATION_LOG
echo "------------------------------" | tee -a $VERIFICATION_LOG

# Check Orchestrator response time
echo -n "Testing Orchestrator response time... " | tee -a $VERIFICATION_LOG
if command -v grpc_health_probe > /dev/null; then
    start_time=$(date +%s%N)
    grpc_health_probe -addr=localhost:${ORCHESTRATOR_PORT} 2>/dev/null
    end_time=$(date +%s%N)
    response_time=$((($end_time - $start_time) / 1000000))
    
    if [ $response_time -lt 1000 ]; then
        log_result "PASS" "Orchestrator response time" "${response_time}ms (< 1000ms threshold)"
    else
        log_result "WARN" "Orchestrator response time" "${response_time}ms (> 1000ms threshold)"
    fi
else
    log_result "WARN" "Orchestrator response time" "grpc_health_probe not available"
fi

# Check database query performance
echo -n "Testing database query performance... " | tee -a $VERIFICATION_LOG
start_time=$(date +%s%N)
sudo -u postgres psql -d mistersmith -c "SELECT COUNT(*) FROM tasks;" > /dev/null 2>&1
end_time=$(date +%s%N)
query_time=$((($end_time - $start_time) / 1000000))

if [ $query_time -lt 100 ]; then
    log_result "PASS" "Database query performance" "${query_time}ms for count query"
else
    log_result "WARN" "Database query performance" "${query_time}ms for count query (high)"
fi

echo "" | tee -a $VERIFICATION_LOG

# Section 4: Configuration Verification
echo -e "${YELLOW}4. Configuration Verification${NC}" | tee -a $VERIFICATION_LOG
echo "-----------------------------" | tee -a $VERIFICATION_LOG

# Check configuration files
echo -n "Checking configuration files... " | tee -a $VERIFICATION_LOG
CONFIG_FILES=(
    "$HOME/.mistersmith/orchestrator/config.json"
    "$HOME/.mistersmith/webui/config.json"
)
MISSING_CONFIGS=()

for config in "${CONFIG_FILES[@]}"; do
    if [ ! -f "$config" ]; then
        MISSING_CONFIGS+=($config)
    fi
done

if [ ${#MISSING_CONFIGS[@]} -eq 0 ]; then
    log_result "PASS" "Configuration files" "All required configs present"
else
    log_result "FAIL" "Configuration files" "Missing: ${MISSING_CONFIGS[*]}"
fi

# Verify configuration validity
echo -n "Validating configuration syntax... " | tee -a $VERIFICATION_LOG
CONFIG_VALID=true
for config in "${CONFIG_FILES[@]}"; do
    if [ -f "$config" ]; then
        if ! jq empty "$config" 2>/dev/null; then
            CONFIG_VALID=false
            break
        fi
    fi
done

if [ "$CONFIG_VALID" = true ]; then
    log_result "PASS" "Configuration syntax" "All configs are valid JSON"
else
    log_result "FAIL" "Configuration syntax" "Invalid JSON in configuration files"
fi

echo "" | tee -a $VERIFICATION_LOG

# Section 5: Network Connectivity
echo -e "${YELLOW}5. Network Connectivity Checks${NC}" | tee -a $VERIFICATION_LOG
echo "------------------------------" | tee -a $VERIFICATION_LOG

# Check internal connectivity
echo -n "Checking internal service connectivity... " | tee -a $VERIFICATION_LOG
if docker exec mistersmith-orchestrator ping -c 1 mistersmith-nats > /dev/null 2>&1; then
    log_result "PASS" "Internal network connectivity" "Services can communicate"
else
    log_result "WARN" "Internal network connectivity" "Inter-service communication may be impaired"
fi

# Check port availability
echo -n "Checking port availability... " | tee -a $VERIFICATION_LOG
PORTS_OK=true
for port in $ORCHESTRATOR_PORT $WEBUI_PORT $NATS_PORT; do
    if ! netstat -tuln 2>/dev/null | grep -q ":$port "; then
        PORTS_OK=false
        break
    fi
done

if [ "$PORTS_OK" = true ]; then
    log_result "PASS" "Port availability" "All required ports are listening"
else
    log_result "FAIL" "Port availability" "Some required ports are not available"
fi

echo "" | tee -a $VERIFICATION_LOG

# Section 6: Log Analysis
echo -e "${YELLOW}6. Log Analysis${NC}" | tee -a $VERIFICATION_LOG
echo "---------------" | tee -a $VERIFICATION_LOG

# Check for recent errors in logs
echo -n "Checking for recent errors... " | tee -a $VERIFICATION_LOG
ERROR_COUNT=0

if docker logs mistersmith-orchestrator 2>&1 | tail -100 | grep -i error > /dev/null; then
    ERROR_COUNT=$((ERROR_COUNT + 1))
fi

if docker logs mistersmith-webui 2>&1 | tail -100 | grep -i error > /dev/null; then
    ERROR_COUNT=$((ERROR_COUNT + 1))
fi

if [ $ERROR_COUNT -eq 0 ]; then
    log_result "PASS" "Service logs" "No recent errors detected"
else
    log_result "WARN" "Service logs" "$ERROR_COUNT services showing errors in logs"
fi

echo "" | tee -a $VERIFICATION_LOG

# Final Summary
echo -e "${BLUE}═══════════════════════════════════════════${NC}" | tee -a $VERIFICATION_LOG
echo -e "${BLUE}           Verification Summary${NC}" | tee -a $VERIFICATION_LOG
echo -e "${BLUE}═══════════════════════════════════════════${NC}" | tee -a $VERIFICATION_LOG
echo "" | tee -a $VERIFICATION_LOG

echo "Total Checks: $TOTAL_CHECKS" | tee -a $VERIFICATION_LOG
echo -e "${GREEN}Passed: $PASSED_CHECKS${NC}" | tee -a $VERIFICATION_LOG
echo -e "${RED}Failed: $FAILED_CHECKS${NC}" | tee -a $VERIFICATION_LOG
echo -e "${YELLOW}Warnings: $WARNINGS${NC}" | tee -a $VERIFICATION_LOG
echo "" | tee -a $VERIFICATION_LOG

# Determine overall status
if [ $FAILED_CHECKS -eq 0 ]; then
    echo -e "${GREEN}✅ ROLLBACK VERIFICATION PASSED${NC}" | tee -a $VERIFICATION_LOG
    echo "All critical systems are operational" | tee -a $VERIFICATION_LOG
    exit 0
else
    echo -e "${RED}❌ ROLLBACK VERIFICATION FAILED${NC}" | tee -a $VERIFICATION_LOG
    echo "$FAILED_CHECKS critical checks failed" | tee -a $VERIFICATION_LOG
    echo "" | tee -a $VERIFICATION_LOG
    echo "Required Actions:" | tee -a $VERIFICATION_LOG
    echo "1. Review the verification log: $VERIFICATION_LOG" | tee -a $VERIFICATION_LOG
    echo "2. Address failed checks before proceeding" | tee -a $VERIFICATION_LOG
    echo "3. Re-run verification after fixes" | tee -a $VERIFICATION_LOG
    exit 1
fi