#!/bin/bash
# MS-3 Advanced AWS Migration - Progress Dashboard Generator
# Monitoring Specialist - Real-time Dashboard Creation

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'
BOLD='\033[1m'

# Paths
MONITORING_ROOT="/Users/mac-main/Mister-Smith/MisterSmith/aws-migration-plan/phase-4-documentation/progress-monitoring"
DATA_DIR="${MONITORING_ROOT}/data"
DASHBOARD_DIR="${MONITORING_ROOT}/dashboards"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Get latest metrics
METRICS_FILE="${DATA_DIR}/latest_metrics.json"
if [[ ! -f "${METRICS_FILE}" ]]; then
    echo -e "${RED}Error: No metrics found. Run collect-metrics.sh first.${NC}"
    exit 1
fi

# Dashboard file
DASHBOARD_FILE="${DASHBOARD_DIR}/dashboard_${TIMESTAMP}.txt"
DASHBOARD_MD="${DASHBOARD_DIR}/dashboard_${TIMESTAMP}.md"

# Function to create progress bar
create_progress_bar() {
    local percentage=$1
    local width=50
    local filled=$((percentage * width / 100))
    local empty=$((width - filled))
    
    printf "["
    printf "%0.s█" $(seq 1 $filled)
    printf "%0.s░" $(seq 1 $empty)
    printf "] %3d%%" $percentage
}

# Function to get color based on percentage
get_color_by_percentage() {
    local percentage=$1
    if [[ $percentage -ge 80 ]]; then
        echo "${GREEN}"
    elif [[ $percentage -ge 50 ]]; then
        echo "${YELLOW}"
    else
        echo "${RED}"
    fi
}

# Function to get risk color
get_risk_color() {
    local level=$1
    case $level in
        "high") echo "${RED}" ;;
        "medium") echo "${YELLOW}" ;;
        "low") echo "${GREEN}" ;;
        *) echo "${NC}" ;;
    esac
}

# Generate ASCII Dashboard
{
    echo "═══════════════════════════════════════════════════════════════════════════════"
    echo "                       MS-3 AWS MIGRATION PROGRESS DASHBOARD                    "
    echo "═══════════════════════════════════════════════════════════════════════════════"
    echo "Generated: $(date '+%Y-%m-%d %H:%M:%S %Z')"
    echo "───────────────────────────────────────────────────────────────────────────────"
    echo ""
    
    # Overall Progress
    overall_percent=$(jq -r '.overall.completionPercentage' "$METRICS_FILE")
    total_tasks=$(jq -r '.overall.totalTasks' "$METRICS_FILE")
    completed_tasks=$(jq -r '.overall.completedTasks' "$METRICS_FILE")
    in_progress=$(jq -r '.overall.inProgressTasks' "$METRICS_FILE")
    blocked=$(jq -r '.overall.blockedTasks' "$METRICS_FILE")
    
    echo "OVERALL PROGRESS"
    echo "$(get_color_by_percentage $overall_percent)$(create_progress_bar $overall_percent)${NC}"
    echo ""
    echo "┌─────────────┬──────────┬──────────────┬─────────┐"
    echo "│ Total Tasks │ Complete │ In Progress  │ Blocked │"
    echo "├─────────────┼──────────┼──────────────┼─────────┤"
    printf "│ %11d │ %8d │ %12d │ %7d │\n" $total_tasks $completed_tasks $in_progress $blocked
    echo "└─────────────┴──────────┴──────────────┴─────────┘"
    echo ""
    
    # Phase Progress
    echo "PHASE BREAKDOWN"
    echo "┌─────────────────────────┬──────────┬────────────────────────────────────────────┐"
    echo "│ Phase                   │ Progress │ Status                                     │"
    echo "├─────────────────────────┼──────────┼────────────────────────────────────────────┤"
    
    # Phase 1
    phase1_percent=$(jq -r '.phases.phase1.completionPercentage // 0' "$METRICS_FILE")
    phase1_color=$(get_color_by_percentage $phase1_percent)
    printf "│ Phase 1: Assessment     │ %s%6d%%${NC} │ %s │\n" \
        "$phase1_color" $phase1_percent "$(create_progress_bar $phase1_percent)"
    
    # Phase 2
    phase2_percent=$(jq -r '.phases.phase2.completionPercentage // 0' "$METRICS_FILE")
    phase2_color=$(get_color_by_percentage $phase2_percent)
    printf "│ Phase 2: Setup          │ %s%6d%%${NC} │ %s │\n" \
        "$phase2_color" $phase2_percent "$(create_progress_bar $phase2_percent)"
    
    # Phase 3
    phase3_percent=$(jq -r '.phases.phase3.completionPercentage // 0' "$METRICS_FILE")
    phase3_color=$(get_color_by_percentage $phase3_percent)
    printf "│ Phase 3: Migration      │ %s%6d%%${NC} │ %s │\n" \
        "$phase3_color" $phase3_percent "$(create_progress_bar $phase3_percent)"
    
    # Phase 4
    phase4_percent=$(jq -r '.phases.phase4.completionPercentage // 0' "$METRICS_FILE")
    phase4_color=$(get_color_by_percentage $phase4_percent)
    printf "│ Phase 4: Documentation  │ %s%6d%%${NC} │ %s │\n" \
        "$phase4_color" $phase4_percent "$(create_progress_bar $phase4_percent)"
    
    echo "└─────────────────────────┴──────────┴────────────────────────────────────────────┘"
    echo ""
    
    # Verification Status
    echo "VERIFICATION STATUS"
    total_checks=$(jq -r '.verification.totalChecks' "$METRICS_FILE")
    passed_checks=$(jq -r '.verification.passedChecks' "$METRICS_FILE")
    failed_checks=$(jq -r '.verification.failedChecks' "$METRICS_FILE")
    pass_rate=$(jq -r '.verification.passRate' "$METRICS_FILE")
    
    echo "┌──────────────┬─────────┬─────────┬───────────┐"
    echo "│ Total Checks │ Passed  │ Failed  │ Pass Rate │"
    echo "├──────────────┼─────────┼─────────┼───────────┤"
    
    pass_color=$(get_color_by_percentage $pass_rate)
    printf "│ %12d │ ${GREEN}%7d${NC} │ ${RED}%7d${NC} │ %s%8d%%${NC} │\n" \
        $total_checks $passed_checks $failed_checks "$pass_color" $pass_rate
    
    echo "└──────────────┴─────────┴─────────┴───────────┘"
    echo ""
    
    # Constraints Status
    echo "CONSTRAINTS STATUS"
    total_constraints=$(jq -r '.constraints.total' "$METRICS_FILE")
    resolved_constraints=$(jq -r '.constraints.resolved' "$METRICS_FILE")
    active_constraints=$(jq -r '.constraints.active' "$METRICS_FILE")
    critical_constraints=$(jq -r '.constraints.critical' "$METRICS_FILE")
    
    echo "┌───────────┬──────────┬────────┬──────────┐"
    echo "│ Total     │ Resolved │ Active │ Critical │"
    echo "├───────────┼──────────┼────────┼──────────┤"
    printf "│ %9d │ ${GREEN}%8d${NC} │ ${YELLOW}%6d${NC} │ ${RED}%8d${NC} │\n" \
        $total_constraints $resolved_constraints $active_constraints $critical_constraints
    echo "└───────────┴──────────┴────────┴──────────┘"
    echo ""
    
    # Risk Assessment
    echo "RISK ASSESSMENT"
    high_risks=$(jq -r '.risks.high' "$METRICS_FILE")
    medium_risks=$(jq -r '.risks.medium' "$METRICS_FILE")
    low_risks=$(jq -r '.risks.low' "$METRICS_FILE")
    
    echo "┌────────────────────────────────────────────┐"
    echo "│              Risk Indicators               │"
    echo "├────────────────────────────────────────────┤"
    printf "│ ${RED}● High Risk Issues:    %19d${NC} │\n" $high_risks
    printf "│ ${YELLOW}● Medium Risk Issues:  %19d${NC} │\n" $medium_risks
    printf "│ ${GREEN}● Low Risk Issues:     %19d${NC} │\n" $low_risks
    echo "└────────────────────────────────────────────┘"
    echo ""
    
    # Time Analysis
    echo "TIME ANALYSIS"
    echo "┌────────────────────────────────────────────┐"
    echo "│            Estimated Completion            │"
    echo "├────────────────────────────────────────────┤"
    
    # Simple time estimation based on current progress
    if [[ $overall_percent -gt 0 && $overall_percent -lt 100 ]]; then
        # Assume project started 30 days ago (configurable)
        days_elapsed=30
        days_per_percent=$(echo "scale=2; $days_elapsed / $overall_percent" | bc)
        remaining_percent=$((100 - overall_percent))
        days_remaining=$(echo "scale=0; $days_per_percent * $remaining_percent" | bc)
        estimated_completion=$(date -d "+${days_remaining} days" '+%Y-%m-%d' 2>/dev/null || date -v +${days_remaining}d '+%Y-%m-%d')
        
        printf "│ Days Elapsed:      %23d │\n" $days_elapsed
        printf "│ Progress Rate:     %20.1f%%/day │\n" $(echo "scale=1; $overall_percent / $days_elapsed" | bc)
        printf "│ Est. Days Remaining: %20d │\n" ${days_remaining%.*}
        printf "│ Est. Completion:   %23s │\n" "$estimated_completion"
    else
        printf "│ %42s │\n" "Insufficient data for estimation"
    fi
    
    echo "└────────────────────────────────────────────┘"
    echo ""
    
    # Legend
    echo "LEGEND"
    echo "┌────────────────────────────────────────────┐"
    echo "│ Progress Indicators:                       │"
    echo "│   ${GREEN}█${NC} = Completed  ${YELLOW}█${NC} = In Progress         │"
    echo "│   ${RED}█${NC} = Blocked    ░ = Not Started         │"
    echo "└────────────────────────────────────────────┘"
    echo ""
    echo "═══════════════════════════════════════════════════════════════════════════════"
    
} > "$DASHBOARD_FILE"

# Generate Markdown Dashboard
{
    echo "# MS-3 AWS Migration Progress Dashboard"
    echo ""
    echo "**Generated:** $(date '+%Y-%m-%d %H:%M:%S %Z')"
    echo ""
    echo "## 📊 Overall Progress"
    echo ""
    echo "**Progress:** ${overall_percent}%"
    echo ""
    echo "\`\`\`"
    create_progress_bar $overall_percent
    echo ""
    echo "\`\`\`"
    echo ""
    echo "| Metric | Value |"
    echo "|--------|-------|"
    echo "| Total Tasks | ${total_tasks} |"
    echo "| Completed | ${completed_tasks} |"
    echo "| In Progress | ${in_progress} |"
    echo "| Blocked | ${blocked} |"
    echo ""
    echo "## 📈 Phase Breakdown"
    echo ""
    echo "| Phase | Progress | Tasks | Status |"
    echo "|-------|----------|-------|---------|"
    
    # Add phase data
    for phase in phase1 phase2 phase3 phase4; do
        phase_name=$(jq -r ".phases.${phase}.name // 'Unknown'" "$METRICS_FILE")
        phase_percent=$(jq -r ".phases.${phase}.completionPercentage // 0" "$METRICS_FILE")
        phase_total=$(jq -r ".phases.${phase}.totalTasks // 0" "$METRICS_FILE")
        phase_complete=$(jq -r ".phases.${phase}.completedTasks // 0" "$METRICS_FILE")
        
        if [[ $phase_percent -ge 80 ]]; then
            status="✅ On Track"
        elif [[ $phase_percent -ge 50 ]]; then
            status="⚠️ Attention"
        else
            status="🔴 Behind"
        fi
        
        echo "| ${phase_name} | ${phase_percent}% | ${phase_complete}/${phase_total} | ${status} |"
    done
    
    echo ""
    echo "## ✅ Verification Status"
    echo ""
    echo "| Metric | Value |"
    echo "|--------|-------|"
    echo "| Total Checks | ${total_checks} |"
    echo "| Passed | ${passed_checks} |"
    echo "| Failed | ${failed_checks} |"
    echo "| Pass Rate | ${pass_rate}% |"
    echo ""
    echo "## 🚧 Constraints"
    echo ""
    echo "| Status | Count |"
    echo "|--------|-------|"
    echo "| Total | ${total_constraints} |"
    echo "| Resolved | ${resolved_constraints} |"
    echo "| Active | ${active_constraints} |"
    echo "| Critical | ${critical_constraints} |"
    echo ""
    echo "## ⚠️ Risk Assessment"
    echo ""
    echo "| Risk Level | Count | Description |"
    echo "|------------|-------|-------------|"
    echo "| 🔴 High | ${high_risks} | Immediate attention required |"
    echo "| 🟡 Medium | ${medium_risks} | Monitor closely |"
    echo "| 🟢 Low | ${low_risks} | Standard tracking |"
    echo ""
    echo "## 📅 Timeline"
    echo ""
    
    if [[ $overall_percent -gt 0 && $overall_percent -lt 100 ]]; then
        echo "- **Project Start:** $(date -d '-30 days' '+%Y-%m-%d' 2>/dev/null || date -v -30d '+%Y-%m-%d')"
        echo "- **Current Progress:** ${overall_percent}%"
        echo "- **Estimated Completion:** ${estimated_completion:-TBD}"
        echo "- **Days Remaining:** ${days_remaining%.*}"
    else
        echo "- Timeline estimation requires active progress data"
    fi
    
    echo ""
    echo "---"
    echo ""
    echo "*Dashboard generated by MS-3 Monitoring Specialist*"
    
} > "$DASHBOARD_MD"

# Display the dashboard
cat "$DASHBOARD_FILE"

# Create symlinks to latest
ln -sf "dashboard_${TIMESTAMP}.txt" "${DASHBOARD_DIR}/latest_dashboard.txt"
ln -sf "dashboard_${TIMESTAMP}.md" "${DASHBOARD_DIR}/latest_dashboard.md"

# Store decision in memory
npx claude-flow hooks post-edit --file "${DASHBOARD_FILE}" --memory-key "ms3/monitoring/dashboard" || true

echo -e "\n${GREEN}Dashboard saved to:${NC}"
echo -e "  ASCII: ${DASHBOARD_FILE}"
echo -e "  Markdown: ${DASHBOARD_MD}"

exit 0