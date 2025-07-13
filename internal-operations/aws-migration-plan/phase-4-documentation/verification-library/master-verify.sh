#!/bin/bash

# MS3 AWS Migration - Master Verification Script
# Runs all verification checks and generates comprehensive report

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
export AWS_REGION="${AWS_REGION:-us-east-1}"
export ENVIRONMENT="${ENVIRONMENT:-production}"
export REPORT_FORMAT="${REPORT_FORMAT:-all}" # all, json, junit, markdown
export TIMESTAMP=$(date +%Y%m%d_%H%M%S)
export REPORT_DIR="./reports/${TIMESTAMP}"

# Create report directory
mkdir -p "${REPORT_DIR}"

# Initialize counters
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNINGS=0

# Log function
log() {
    local level=$1
    shift
    echo -e "${TIMESTAMP} [${level}] $*" | tee -a "${REPORT_DIR}/verification.log"
}

# Result tracking
track_result() {
    local check_name=$1
    local status=$2
    local message=$3
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    case $status in
        PASS)
            PASSED_CHECKS=$((PASSED_CHECKS + 1))
            echo -e "${GREEN}✓${NC} ${check_name}: ${message}"
            ;;
        FAIL)
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
            echo -e "${RED}✗${NC} ${check_name}: ${message}"
            ;;
        WARN)
            WARNINGS=$((WARNINGS + 1))
            echo -e "${YELLOW}⚠${NC} ${check_name}: ${message}"
            ;;
    esac
    
    # Log to JSON
    cat >> "${REPORT_DIR}/results.json" <<EOF
{
  "timestamp": "${TIMESTAMP}",
  "check": "${check_name}",
  "status": "${status}",
  "message": "${message}"
},
EOF
}

# Main verification execution
main() {
    log "INFO" "Starting MS3 AWS Migration Verification Suite"
    log "INFO" "Environment: ${ENVIRONMENT}"
    log "INFO" "Region: ${AWS_REGION}"
    
    # Initialize JSON report
    echo "[" > "${REPORT_DIR}/results.json"
    
    # Print header
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}MS3 AWS Migration Verification Suite${NC}"
    echo -e "${BLUE}========================================${NC}\n"
    
    # Run verification categories
    echo -e "\n${YELLOW}[1/5] Infrastructure Verification${NC}"
    ./infrastructure/verify-infrastructure.sh
    
    echo -e "\n${YELLOW}[2/5] Services Verification${NC}"
    ./services/verify-services.sh
    
    echo -e "\n${YELLOW}[3/5] Application Verification${NC}"
    ./application/verify-application.sh
    
    echo -e "\n${YELLOW}[4/5] Data Verification${NC}"
    ./data/verify-data.sh
    
    echo -e "\n${YELLOW}[5/5] Performance Verification${NC}"
    ./performance/verify-performance.sh
    
    # Finalize JSON report
    echo "{}]" >> "${REPORT_DIR}/results.json"
    # Remove trailing comma from last entry
    sed -i '' -e '$ s/,$//' "${REPORT_DIR}/results.json"
    
    # Generate summary
    generate_summary
    
    # Generate reports based on format
    case $REPORT_FORMAT in
        json)
            generate_json_report
            ;;
        junit)
            generate_junit_report
            ;;
        markdown)
            generate_markdown_report
            ;;
        all)
            generate_json_report
            generate_junit_report
            generate_markdown_report
            ;;
    esac
    
    # Exit with appropriate code
    if [ $FAILED_CHECKS -gt 0 ]; then
        exit 1
    else
        exit 0
    fi
}

# Generate summary
generate_summary() {
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}Verification Summary${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo -e "Total Checks: ${TOTAL_CHECKS}"
    echo -e "${GREEN}Passed: ${PASSED_CHECKS}${NC}"
    echo -e "${RED}Failed: ${FAILED_CHECKS}${NC}"
    echo -e "${YELLOW}Warnings: ${WARNINGS}${NC}"
    echo -e "Success Rate: $(( (PASSED_CHECKS * 100) / TOTAL_CHECKS ))%"
    
    if [ $FAILED_CHECKS -eq 0 ]; then
        echo -e "\n${GREEN}✓ All verification checks passed!${NC}"
    else
        echo -e "\n${RED}✗ Some verification checks failed. Please review the report.${NC}"
    fi
}

# Generate JSON report
generate_json_report() {
    cat > "${REPORT_DIR}/summary.json" <<EOF
{
  "timestamp": "${TIMESTAMP}",
  "environment": "${ENVIRONMENT}",
  "region": "${AWS_REGION}",
  "summary": {
    "total": ${TOTAL_CHECKS},
    "passed": ${PASSED_CHECKS},
    "failed": ${FAILED_CHECKS},
    "warnings": ${WARNINGS},
    "successRate": $(( (PASSED_CHECKS * 100) / TOTAL_CHECKS ))
  }
}
EOF
    log "INFO" "JSON report generated: ${REPORT_DIR}/summary.json"
}

# Generate JUnit XML report
generate_junit_report() {
    cat > "${REPORT_DIR}/junit.xml" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="MS3 AWS Migration Verification" tests="${TOTAL_CHECKS}" failures="${FAILED_CHECKS}" time="0">
  <testsuite name="AWS Migration Checks" tests="${TOTAL_CHECKS}" failures="${FAILED_CHECKS}" time="0">
    <!-- Test cases will be added by individual scripts -->
  </testsuite>
</testsuites>
EOF
    log "INFO" "JUnit report generated: ${REPORT_DIR}/junit.xml"
}

# Generate Markdown report
generate_markdown_report() {
    cat > "${REPORT_DIR}/report.md" <<EOF
# MS3 AWS Migration Verification Report

**Date:** $(date)  
**Environment:** ${ENVIRONMENT}  
**Region:** ${AWS_REGION}

## Summary

| Metric | Value |
|--------|-------|
| Total Checks | ${TOTAL_CHECKS} |
| Passed | ${PASSED_CHECKS} |
| Failed | ${FAILED_CHECKS} |
| Warnings | ${WARNINGS} |
| Success Rate | $(( (PASSED_CHECKS * 100) / TOTAL_CHECKS ))% |

## Detailed Results

See individual category reports for detailed findings.

### Categories Verified:
1. Infrastructure
2. Services
3. Application
4. Data
5. Performance

EOF
    log "INFO" "Markdown report generated: ${REPORT_DIR}/report.md"
}

# Export functions for use in sub-scripts
export -f log
export -f track_result

# Run main function
main "$@"