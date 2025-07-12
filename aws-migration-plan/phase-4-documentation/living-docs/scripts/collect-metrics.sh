#!/bin/bash
# collect-metrics.sh - Gathers progress metrics from all phases
# Generates dashboard.json and updates kpi-tracking.csv

set -euo pipefail

# Configuration
BASE_DIR="${AWS_MIGRATION_DIR:-/Users/mac-main/Mister-Smith/MisterSmith/aws-migration-plan}"
METRICS_DIR="$BASE_DIR/progress-metrics"
DASHBOARD_FILE="$METRICS_DIR/dashboard.json"
KPI_FILE="$METRICS_DIR/kpi-tracking.csv"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
DATE=$(date -u +"%Y-%m-%d")

# Create metrics directory if needed
mkdir -p "$METRICS_DIR"

# Initialize KPI file if it doesn't exist
if [ ! -f "$KPI_FILE" ]; then
    echo "Date,Phase,Tasks_Total,Tasks_Completed,Progress_%,Blockers,Risks,Budget_Used_%,Duration_Days" > "$KPI_FILE"
fi

# Initialize dashboard structure
DASHBOARD=$(cat << EOF
{
  "updated": "$TIMESTAMP",
  "phases": [],
  "overall": {
    "progress": 0,
    "phases_total": 9,
    "phases_completed": 0,
    "estimated_completion": "TBD",
    "budget_utilization": 0,
    "risk_score": "LOW",
    "total_tasks": 0,
    "completed_tasks": 0,
    "blocked_tasks": 0
  },
  "metrics": {
    "automation_coverage": 0,
    "validation_pass_rate": 0,
    "blocker_resolution_time_hours": 0,
    "documentation_completeness": 0
  }
}
EOF
)

# Phase names for reference
PHASE_NAMES=(
    "Architecture Analysis"
    "Service Mapping"
    "Strategy Formulation"
    "Documentation Infrastructure"
    "Swarm Orchestration"
    "Infrastructure Setup"
    "Service Migration"
    "Validation & Testing"
    "Production Cutover"
)

# Function to count tasks in a markdown file
count_tasks() {
    local file=$1
    local total=0
    local completed=0
    
    if [ -f "$file" ]; then
        total=$(grep -cE '^[[:space:]]*-[[:space:]]*\[' "$file" 2>/dev/null || echo 0)
        completed=$(grep -cE '^[[:space:]]*-[[:space:]]*\[x\]' "$file" 2>/dev/null || echo 0)
    fi
    
    echo "$total|$completed"
}

# Collect metrics from each phase
TOTAL_PROGRESS=0
PHASES_COMPLETED=0
ALL_TASKS_TOTAL=0
ALL_TASKS_COMPLETED=0

for i in {1..9}; do
    PHASE_NAME="Phase $i - ${PHASE_NAMES[$i-1]}"
    PHASE_DIR=$(find "$BASE_DIR" -maxdepth 1 -type d -name "phase-$i-*" | head -1)
    
    # Default values
    STATUS="NOT_STARTED"
    PROGRESS=0
    TASKS_TOTAL=0
    TASKS_DONE=0
    DURATION_DAYS=0
    BLOCKERS=0
    RISKS=0
    
    # Check if phase directory exists
    if [ -n "$PHASE_DIR" ] && [ -d "$PHASE_DIR" ]; then
        # Try to get status from config.json first
        if [ -f "$PHASE_DIR/config.json" ]; then
            STATUS=$(jq -r '.status // "IN_PROGRESS"' "$PHASE_DIR/config.json" 2>/dev/null || echo "IN_PROGRESS")
            PROGRESS=$(jq -r '.progress // 0' "$PHASE_DIR/config.json" 2>/dev/null || echo 0)
        fi
        
        # Count tasks from tracker.md
        if [ -f "$PHASE_DIR/tracker.md" ]; then
            IFS='|' read -r TASKS_TOTAL TASKS_DONE <<< "$(count_tasks "$PHASE_DIR/tracker.md")"
            
            # Calculate progress from tasks if not in config
            if [ "$PROGRESS" -eq 0 ] && [ "$TASKS_TOTAL" -gt 0 ]; then
                PROGRESS=$((TASKS_DONE * 100 / TASKS_TOTAL))
            fi
            
            # Update status based on progress
            if [ "$PROGRESS" -eq 100 ]; then
                STATUS="COMPLETE"
            elif [ "$PROGRESS" -gt 0 ]; then
                STATUS="IN_PROGRESS"
            fi
        fi
        
        # Calculate duration (simplified - days since first log entry)
        if [ -f "$BASE_DIR/phase-status/phase-history.log" ]; then
            FIRST_DATE=$(grep "Phase: $i" "$BASE_DIR/phase-status/phase-history.log" 2>/dev/null | head -1 | cut -d' ' -f1 || echo "")
            if [ -n "$FIRST_DATE" ]; then
                # Calculate days difference (platform agnostic)
                DURATION_DAYS=$(( ($(date +%s) - $(date -d "$FIRST_DATE" +%s 2>/dev/null || date -j -f "%Y-%m-%d" "$FIRST_DATE" +%s 2>/dev/null || echo 0)) / 86400 ))
            fi
        fi
    fi
    
    # Count phase as completed
    if [ "$STATUS" = "COMPLETE" ] && [ "$PROGRESS" -eq 100 ]; then
        ((PHASES_COMPLETED++))
    fi
    
    # Add to totals
    TOTAL_PROGRESS=$((TOTAL_PROGRESS + PROGRESS))
    ALL_TASKS_TOTAL=$((ALL_TASKS_TOTAL + TASKS_TOTAL))
    ALL_TASKS_COMPLETED=$((ALL_TASKS_COMPLETED + TASKS_DONE))
    
    # Count blockers and risks for this phase
    if [ -f "$BASE_DIR/constraint-tracking/blockers.md" ]; then
        BLOCKERS=$(grep -c "Phase $i" "$BASE_DIR/constraint-tracking/blockers.md" 2>/dev/null || echo 0)
    fi
    
    if [ -f "$BASE_DIR/constraint-tracking/risk-register.md" ]; then
        RISKS=$(grep -c "Phase $i" "$BASE_DIR/constraint-tracking/risk-register.md" 2>/dev/null || echo 0)
    fi
    
    # Add phase data to dashboard
    PHASE_JSON=$(cat << EOF
    {
      "name": "$PHASE_NAME",
      "status": "$STATUS",
      "progress": $PROGRESS,
      "tasks_total": $TASKS_TOTAL,
      "tasks_completed": $TASKS_DONE,
      "duration_days": $DURATION_DAYS,
      "blockers": $BLOCKERS,
      "risks": $RISKS
    }
EOF
)
    
    # Update dashboard JSON
    DASHBOARD=$(echo "$DASHBOARD" | jq --argjson phase "$PHASE_JSON" '.phases += [$phase]')
    
    # Add to KPI tracking CSV (only for active/completed phases)
    if [ "$STATUS" != "NOT_STARTED" ]; then
        # Calculate budget used (simplified - based on progress)
        BUDGET_USED=$((PROGRESS * 85 / 9))  # Assuming 8500 total, linear distribution
        
        echo "$DATE,$PHASE_NAME,$TASKS_TOTAL,$TASKS_DONE,$PROGRESS,$BLOCKERS,$RISKS,$BUDGET_USED,$DURATION_DAYS" >> "$KPI_FILE"
    fi
done

# Calculate overall metrics
OVERALL_PROGRESS=$((TOTAL_PROGRESS / 9))
BUDGET_UTILIZATION=$((OVERALL_PROGRESS * 85 / 10))  # Rough estimate

# Determine risk score based on blockers
TOTAL_BLOCKERS=$(grep -c "\[BLOCKER-" "$BASE_DIR/constraint-tracking/blockers.md" 2>/dev/null || echo 0)
if [ "$TOTAL_BLOCKERS" -eq 0 ]; then
    RISK_SCORE="LOW"
elif [ "$TOTAL_BLOCKERS" -le 2 ]; then
    RISK_SCORE="MEDIUM"
else
    RISK_SCORE="HIGH"
fi

# Calculate estimated completion (simplified - based on current pace)
if [ "$PHASES_COMPLETED" -gt 0 ]; then
    # Average days per phase
    AVG_DAYS_PER_PHASE=$(echo "$DASHBOARD" | jq '[.phases[] | select(.status == "COMPLETE") | .duration_days] | add / length' 2>/dev/null || echo 14)
    REMAINING_PHASES=$((9 - PHASES_COMPLETED))
    ESTIMATED_DAYS=$((REMAINING_PHASES * ${AVG_DAYS_PER_PHASE%.*}))
    ESTIMATED_COMPLETION=$(date -u -d "+$ESTIMATED_DAYS days" +"%Y-%m-%d" 2>/dev/null || date -u -v +${ESTIMATED_DAYS}d +"%Y-%m-%d" 2>/dev/null || echo "TBD")
else
    ESTIMATED_COMPLETION="TBD"
fi

# Calculate additional metrics
AUTOMATION_COVERAGE=85  # Based on living docs statement
VALIDATION_PASS_RATE=0
if [ -f "$BASE_DIR/verification-results/latest-validation.json" ]; then
    TOTAL_CHECKS=$(jq '.checks | length' "$BASE_DIR/verification-results/latest-validation.json" 2>/dev/null || echo 0)
    PASSED_CHECKS=$(jq '[.checks[] | select(.status == 0)] | length' "$BASE_DIR/verification-results/latest-validation.json" 2>/dev/null || echo 0)
    if [ "$TOTAL_CHECKS" -gt 0 ]; then
        VALIDATION_PASS_RATE=$((PASSED_CHECKS * 100 / TOTAL_CHECKS))
    fi
fi

# Documentation completeness (based on expected vs actual files)
EXPECTED_DOCS=50
ACTUAL_DOCS=$(find "$BASE_DIR" -name "*.md" -type f | wc -l)
DOC_COMPLETENESS=$((ACTUAL_DOCS * 100 / EXPECTED_DOCS))
if [ "$DOC_COMPLETENESS" -gt 100 ]; then
    DOC_COMPLETENESS=100
fi

# Update overall metrics in dashboard
DASHBOARD=$(echo "$DASHBOARD" | jq \
    --arg progress "$OVERALL_PROGRESS" \
    --arg completed "$PHASES_COMPLETED" \
    --arg estimated "$ESTIMATED_COMPLETION" \
    --arg budget "$BUDGET_UTILIZATION" \
    --arg risk "$RISK_SCORE" \
    --arg total_tasks "$ALL_TASKS_TOTAL" \
    --arg completed_tasks "$ALL_TASKS_COMPLETED" \
    --arg blocked "$TOTAL_BLOCKERS" \
    --arg automation "$AUTOMATION_COVERAGE" \
    --arg validation "$VALIDATION_PASS_RATE" \
    --arg docs "$DOC_COMPLETENESS" \
    '.overall.progress = ($progress | tonumber) |
     .overall.phases_completed = ($completed | tonumber) |
     .overall.estimated_completion = $estimated |
     .overall.budget_utilization = ($budget | tonumber) |
     .overall.risk_score = $risk |
     .overall.total_tasks = ($total_tasks | tonumber) |
     .overall.completed_tasks = ($completed_tasks | tonumber) |
     .overall.blocked_tasks = ($blocked | tonumber) |
     .metrics.automation_coverage = ($automation | tonumber) |
     .metrics.validation_pass_rate = ($validation | tonumber) |
     .metrics.documentation_completeness = ($docs | tonumber)')

# Save dashboard
echo "$DASHBOARD" | jq '.' > "$DASHBOARD_FILE"

# Generate burndown chart data
BURNDOWN_FILE="$METRICS_DIR/burndown-data.json"
if [ -f "$KPI_FILE" ]; then
    echo '[' > "$BURNDOWN_FILE"
    tail -n +2 "$KPI_FILE" | awk -F',' '{print "{\"date\":\"" $1 "\",\"remaining\":" (100-$5) "},"}' >> "$BURNDOWN_FILE"
    sed -i '$ s/,$//' "$BURNDOWN_FILE" 2>/dev/null || sed -i '' '$ s/,$//' "$BURNDOWN_FILE"
    echo ']' >> "$BURNDOWN_FILE"
fi

# Summary output
echo "📊 Metrics collection complete!"
echo ""
echo "Overall Progress: $OVERALL_PROGRESS%"
echo "Phases Completed: $PHASES_COMPLETED / 9"
echo "Total Tasks: $ALL_TASKS_COMPLETED / $ALL_TASKS_TOTAL"
echo "Active Blockers: $TOTAL_BLOCKERS"
echo "Risk Level: $RISK_SCORE"
echo "Estimated Completion: $ESTIMATED_COMPLETION"
echo ""
echo "Dashboard saved to: $DASHBOARD_FILE"
echo "KPIs updated in: $KPI_FILE"

# Send notification if available
if command -v npx &> /dev/null && npx claude-flow --version &> /dev/null 2>&1; then
    npx claude-flow hooks notification --message "Metrics collected: $OVERALL_PROGRESS% complete, Risk: $RISK_SCORE" --telemetry true &> /dev/null || true
fi