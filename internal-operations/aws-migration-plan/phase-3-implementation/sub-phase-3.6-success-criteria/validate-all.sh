#!/bin/bash
# Master Validation Script for MisterSmith AWS Migration
# Runs all validation categories and generates comprehensive report

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPORT_FILE="$SCRIPT_DIR/validation-report-$(date +%Y%m%d-%H%M%S).txt"
SUMMARY_FILE="$SCRIPT_DIR/validation-summary-$(date +%Y%m%d-%H%M%S).json"

echo "🚀 MisterSmith AWS Migration Validation Suite" | tee "$REPORT_FILE"
echo "============================================" | tee -a "$REPORT_FILE"
echo "Started: $(date)" | tee -a "$REPORT_FILE"
echo "" | tee -a "$REPORT_FILE"

# Initialize counters
TOTAL_PASS=0
TOTAL_FAIL=0
CATEGORY_RESULTS=()

# Function to run validation script
run_validation() {
    local category=$1
    local script=$2
    local required_score=$3
    
    echo "📋 Running $category validation..." | tee -a "$REPORT_FILE"
    echo "----------------------------------------" | tee -a "$REPORT_FILE"
    
    if [[ -f "$SCRIPT_DIR/$script" ]]; then
        chmod +x "$SCRIPT_DIR/$script"
        if "$SCRIPT_DIR/$script" >> "$REPORT_FILE" 2>&1; then
            echo "✅ $category: PASSED" | tee -a "$REPORT_FILE"
            CATEGORY_RESULTS+=("{\"category\":\"$category\",\"status\":\"PASS\",\"score\":100}")
            return 0
        else
            echo "❌ $category: FAILED" | tee -a "$REPORT_FILE"
            CATEGORY_RESULTS+=("{\"category\":\"$category\",\"status\":\"FAIL\",\"score\":0}")
            return 1
        fi
    else
        echo "⚠️  $category: SKIPPED (Script not found)" | tee -a "$REPORT_FILE"
        CATEGORY_RESULTS+=("{\"category\":\"$category\",\"status\":\"SKIP\",\"score\":0}")
        return 1
    fi
}

# Run all validation categories
echo "🔍 Starting Validation Process..." | tee -a "$REPORT_FILE"
echo "" | tee -a "$REPORT_FILE"

# Infrastructure (100% required)
if run_validation "Infrastructure" "validate-infrastructure.sh" 100; then
    ((TOTAL_PASS++))
else
    ((TOTAL_FAIL++))
fi
echo "" | tee -a "$REPORT_FILE"

# Application (95% required)
if run_validation "Application" "validate-application.sh" 95; then
    ((TOTAL_PASS++))
else
    ((TOTAL_FAIL++))
fi
echo "" | tee -a "$REPORT_FILE"

# Performance (90% required)
if run_validation "Performance" "validate-performance.sh" 90; then
    ((TOTAL_PASS++))
else
    ((TOTAL_FAIL++))
fi
echo "" | tee -a "$REPORT_FILE"

# Data Integrity (100% required)
if run_validation "Data Integrity" "validate-data-integrity.sh" 100; then
    ((TOTAL_PASS++))
else
    ((TOTAL_FAIL++))
fi
echo "" | tee -a "$REPORT_FILE"

# Operations (95% required)
if run_validation "Operations" "validate-operations.sh" 95; then
    ((TOTAL_PASS++))
else
    ((TOTAL_FAIL++))
fi
echo "" | tee -a "$REPORT_FILE"

# Calculate overall score
TOTAL_CATEGORIES=$((TOTAL_PASS + TOTAL_FAIL))
if [[ $TOTAL_CATEGORIES -gt 0 ]]; then
    OVERALL_SCORE=$((TOTAL_PASS * 100 / TOTAL_CATEGORIES))
else
    OVERALL_SCORE=0
fi

# Generate summary
echo "📊 VALIDATION SUMMARY" | tee -a "$REPORT_FILE"
echo "====================" | tee -a "$REPORT_FILE"
echo "Categories Passed: $TOTAL_PASS/$TOTAL_CATEGORIES" | tee -a "$REPORT_FILE"
echo "Overall Score: ${OVERALL_SCORE}%" | tee -a "$REPORT_FILE"
echo "Completed: $(date)" | tee -a "$REPORT_FILE"
echo "" | tee -a "$REPORT_FILE"

# Determine GO/NO-GO decision
if [[ $OVERALL_SCORE -ge 95 ]]; then
    DECISION="GO"
    DECISION_COLOR="✅"
elif [[ $OVERALL_SCORE -ge 85 ]]; then
    DECISION="CONDITIONAL GO"
    DECISION_COLOR="⚠️"
else
    DECISION="NO-GO"
    DECISION_COLOR="❌"
fi

echo "🚦 MIGRATION DECISION: $DECISION_COLOR $DECISION" | tee -a "$REPORT_FILE"
echo "" | tee -a "$REPORT_FILE"

# Generate JSON summary
cat > "$SUMMARY_FILE" <<EOF
{
  "validation_run": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "overall_score": $OVERALL_SCORE,
  "decision": "$DECISION",
  "categories": [
    $(IFS=,; echo "${CATEGORY_RESULTS[*]}")
  ],
  "details": {
    "categories_passed": $TOTAL_PASS,
    "categories_failed": $TOTAL_FAIL,
    "report_file": "$REPORT_FILE"
  }
}
EOF

# Quick status for monitoring
if [[ "$DECISION" == "GO" ]]; then
    echo "🎉 All validations passed! Migration can proceed." | tee -a "$REPORT_FILE"
    exit 0
elif [[ "$DECISION" == "CONDITIONAL GO" ]]; then
    echo "⚠️  Some validations need attention. Review before proceeding." | tee -a "$REPORT_FILE"
    exit 1
else
    echo "❌ Critical validations failed. Do not proceed with migration." | tee -a "$REPORT_FILE"
    exit 2
fi