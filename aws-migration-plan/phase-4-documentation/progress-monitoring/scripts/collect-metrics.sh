#!/bin/bash
# MS-3 Advanced AWS Migration - Progress Metrics Collection
# Monitoring Specialist - Automated Metrics Gathering

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Paths
MIGRATION_ROOT="/Users/mac-main/Mister-Smith/MisterSmith/aws-migration-plan"
MONITORING_ROOT="${MIGRATION_ROOT}/phase-4-documentation/progress-monitoring"
DATA_DIR="${MONITORING_ROOT}/data"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo -e "${BLUE}[MS-3 Progress Monitoring] Collecting metrics...${NC}"

# Initialize metrics file
METRICS_FILE="${DATA_DIR}/metrics_${TIMESTAMP}.json"
cat > "${METRICS_FILE}" << 'EOF'
{
  "timestamp": "TIMESTAMP_PLACEHOLDER",
  "phases": {},
  "overall": {
    "totalTasks": 0,
    "completedTasks": 0,
    "inProgressTasks": 0,
    "blockedTasks": 0,
    "completionPercentage": 0
  },
  "constraints": {
    "total": 0,
    "resolved": 0,
    "active": 0,
    "critical": 0
  },
  "verification": {
    "totalChecks": 0,
    "passedChecks": 0,
    "failedChecks": 0,
    "passRate": 0
  },
  "risks": {
    "high": 0,
    "medium": 0,
    "low": 0
  }
}
EOF

# Update timestamp
sed -i '' "s/TIMESTAMP_PLACEHOLDER/$(date -u +%Y-%m-%dT%H:%M:%SZ)/" "${METRICS_FILE}"

# Function to count files by pattern
count_files() {
    local pattern="$1"
    local count=$(find . -name "${pattern}" 2>/dev/null | wc -l | tr -d ' ')
    echo "${count}"
}

# Function to analyze phase progress
analyze_phase() {
    local phase_num="$1"
    local phase_name="$2"
    local phase_dir="${MIGRATION_ROOT}/phase-${phase_num}-*"
    
    echo -e "${YELLOW}Analyzing Phase ${phase_num}: ${phase_name}${NC}"
    
    # Count tasks by status (based on file markers)
    cd "${MIGRATION_ROOT}" 2>/dev/null || return
    
    local total_tasks=0
    local completed_tasks=0
    local in_progress_tasks=0
    
    # Count verification files as completed tasks
    if [[ -d "${phase_dir}/verification" ]]; then
        completed_tasks=$(find "${phase_dir}/verification" -name "*.md" -o -name "*.sh" 2>/dev/null | wc -l | tr -d ' ')
    fi
    
    # Count implementation files
    if [[ -d "${phase_dir}/implementation" ]]; then
        local impl_files=$(find "${phase_dir}/implementation" -name "*.md" -o -name "*.sh" -o -name "*.tf" 2>/dev/null | wc -l | tr -d ' ')
        total_tasks=$((total_tasks + impl_files))
    fi
    
    # Estimate in-progress (files with TODO markers)
    if [[ -d "${phase_dir}" ]]; then
        in_progress_tasks=$(grep -r "TODO\|WIP\|PENDING" "${phase_dir}" 2>/dev/null | wc -l | tr -d ' ')
    fi
    
    # Calculate percentage
    local percentage=0
    if [[ ${total_tasks} -gt 0 ]]; then
        percentage=$((completed_tasks * 100 / total_tasks))
    fi
    
    # Update metrics JSON using temporary file
    local temp_file="${METRICS_FILE}.tmp"
    jq --arg phase "phase${phase_num}" \
       --arg name "${phase_name}" \
       --argjson total "${total_tasks}" \
       --argjson completed "${completed_tasks}" \
       --argjson progress "${in_progress_tasks}" \
       --argjson percent "${percentage}" \
       '.phases[$phase] = {
          "name": $name,
          "totalTasks": $total,
          "completedTasks": $completed,
          "inProgressTasks": $progress,
          "completionPercentage": $percent
       }' "${METRICS_FILE}" > "${temp_file}" && mv "${temp_file}" "${METRICS_FILE}"
}

# Analyze each phase
analyze_phase 1 "Assessment"
analyze_phase 2 "Setup"
analyze_phase 3 "Migration"
analyze_phase 4 "Documentation"

# Collect constraint metrics
echo -e "${YELLOW}Analyzing constraints...${NC}"
CONSTRAINTS_DIR="${MIGRATION_ROOT}/ms3-advanced/constraints"
if [[ -d "${CONSTRAINTS_DIR}" ]]; then
    cd "${CONSTRAINTS_DIR}"
    total_constraints=$(find . -name "*.md" 2>/dev/null | wc -l | tr -d ' ')
    resolved_constraints=$(grep -l "RESOLVED\|IMPLEMENTED" *.md 2>/dev/null | wc -l | tr -d ' ')
    critical_constraints=$(grep -l "CRITICAL\|BLOCKER" *.md 2>/dev/null | wc -l | tr -d ' ')
    
    # Update constraints in metrics
    temp_file="${METRICS_FILE}.tmp"
    jq --argjson total "${total_constraints}" \
       --argjson resolved "${resolved_constraints}" \
       --argjson critical "${critical_constraints}" \
       '.constraints.total = $total |
        .constraints.resolved = $resolved |
        .constraints.active = ($total - $resolved) |
        .constraints.critical = $critical' "${METRICS_FILE}" > "${temp_file}" && mv "${temp_file}" "${METRICS_FILE}"
fi

# Collect verification metrics
echo -e "${YELLOW}Analyzing verification results...${NC}"
VERIFICATION_PATTERN="verification-report-*.md"
total_checks=0
passed_checks=0

for phase_dir in "${MIGRATION_ROOT}"/phase-*/verification; do
    if [[ -d "${phase_dir}" ]]; then
        cd "${phase_dir}"
        phase_checks=$(find . -name "${VERIFICATION_PATTERN}" 2>/dev/null | wc -l | tr -d ' ')
        phase_passed=$(grep -l "PASSED\|SUCCESS\|✓" ${VERIFICATION_PATTERN} 2>/dev/null | wc -l | tr -d ' ')
        total_checks=$((total_checks + phase_checks))
        passed_checks=$((passed_checks + phase_passed))
    fi
done

# Calculate verification pass rate
pass_rate=0
if [[ ${total_checks} -gt 0 ]]; then
    pass_rate=$((passed_checks * 100 / total_checks))
fi

# Update verification metrics
temp_file="${METRICS_FILE}.tmp"
jq --argjson total "${total_checks}" \
   --argjson passed "${passed_checks}" \
   --argjson failed "$((total_checks - passed_checks))" \
   --argjson rate "${pass_rate}" \
   '.verification.totalChecks = $total |
    .verification.passedChecks = $passed |
    .verification.failedChecks = $failed |
    .verification.passRate = $rate' "${METRICS_FILE}" > "${temp_file}" && mv "${temp_file}" "${METRICS_FILE}"

# Calculate overall metrics
echo -e "${YELLOW}Calculating overall progress...${NC}"
overall_total=0
overall_completed=0
overall_progress=0

# Sum up all phase metrics
temp_file="${METRICS_FILE}.tmp"
jq '.phases | to_entries | map(.value) | 
    {
      totalTasks: (map(.totalTasks) | add),
      completedTasks: (map(.completedTasks) | add),
      inProgressTasks: (map(.inProgressTasks) | add)
    } as $sums |
    . as $root |
    $root | 
    .overall.totalTasks = $sums.totalTasks |
    .overall.completedTasks = $sums.completedTasks |
    .overall.inProgressTasks = $sums.inProgressTasks |
    .overall.blockedTasks = (.constraints.critical // 0) |
    .overall.completionPercentage = (if $sums.totalTasks > 0 then ($sums.completedTasks * 100 / $sums.totalTasks) else 0 end)' \
    "${METRICS_FILE}" > "${temp_file}" && mv "${temp_file}" "${METRICS_FILE}"

# Analyze risks based on metrics
echo -e "${YELLOW}Analyzing risk indicators...${NC}"
high_risks=0
medium_risks=0
low_risks=0

# Check for high risks
if [[ $(jq '.overall.completionPercentage' "${METRICS_FILE}") -lt 20 ]]; then
    high_risks=$((high_risks + 1))
fi
if [[ $(jq '.constraints.critical' "${METRICS_FILE}") -gt 5 ]]; then
    high_risks=$((high_risks + 1))
fi
if [[ $(jq '.verification.passRate' "${METRICS_FILE}") -lt 50 ]]; then
    high_risks=$((high_risks + 1))
fi

# Check for medium risks
if [[ $(jq '.overall.blockedTasks' "${METRICS_FILE}") -gt 0 ]]; then
    medium_risks=$((medium_risks + 1))
fi
if [[ $(jq '.verification.failedChecks' "${METRICS_FILE}") -gt 3 ]]; then
    medium_risks=$((medium_risks + 1))
fi

# Update risk metrics
temp_file="${METRICS_FILE}.tmp"
jq --argjson high "${high_risks}" \
   --argjson medium "${medium_risks}" \
   --argjson low "${low_risks}" \
   '.risks.high = $high |
    .risks.medium = $medium |
    .risks.low = $low' "${METRICS_FILE}" > "${temp_file}" && mv "${temp_file}" "${METRICS_FILE}"

# Create latest symlink
ln -sf "metrics_${TIMESTAMP}.json" "${DATA_DIR}/latest_metrics.json"

# Display summary
echo -e "\n${GREEN}[MS-3 Progress Monitoring] Metrics collection complete!${NC}"
echo -e "${BLUE}Metrics saved to: ${METRICS_FILE}${NC}"
echo -e "\n${YELLOW}Summary:${NC}"
jq -r '
    "Overall Progress: \(.overall.completionPercentage)%\n" +
    "Total Tasks: \(.overall.totalTasks)\n" +
    "Completed: \(.overall.completedTasks)\n" +
    "In Progress: \(.overall.inProgressTasks)\n" +
    "Blocked: \(.overall.blockedTasks)\n" +
    "\nVerification Pass Rate: \(.verification.passRate)%\n" +
    "Active Constraints: \(.constraints.active)\n" +
    "Risk Level: High=\(.risks.high), Medium=\(.risks.medium), Low=\(.risks.low)"
' "${METRICS_FILE}"

# Store in Claude Flow memory
echo -e "\n${BLUE}Storing metrics in memory...${NC}"
npx claude-flow hooks notification --message "Metrics collected: $(jq -c . "${METRICS_FILE}")" || true

exit 0