#!/bin/bash
# update-status.sh - Updates current phase status
# Usage: ./update-status.sh <phase> <status> <progress> "<notes>"

set -euo pipefail

# Validate arguments
if [ $# -lt 4 ]; then
    echo "Usage: $0 <phase> <status> <progress> \"<notes>\""
    echo "Example: $0 4 IN_PROGRESS 35 \"Completed infrastructure setup\""
    exit 1
fi

# Arguments
PHASE=$1
STATUS=$2
PROGRESS=$3
NOTES=$4

# Configuration
BASE_DIR="${AWS_MIGRATION_DIR:-/Users/mac-main/Mister-Smith/MisterSmith/aws-migration-plan}"
PHASE_STATUS_DIR="$BASE_DIR/phase-status"
CURRENT_PHASE_FILE="$PHASE_STATUS_DIR/current-phase.md"
HISTORY_FILE="$PHASE_STATUS_DIR/phase-history.log"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Validate inputs
if ! [[ "$PHASE" =~ ^[0-9]+$ ]] || [ "$PHASE" -lt 1 ] || [ "$PHASE" -gt 9 ]; then
    echo "Error: Phase must be a number between 1 and 9"
    exit 1
fi

if ! [[ "$PROGRESS" =~ ^[0-9]+$ ]] || [ "$PROGRESS" -lt 0 ] || [ "$PROGRESS" -gt 100 ]; then
    echo "Error: Progress must be a number between 0 and 100"
    exit 1
fi

# Valid status values
VALID_STATUSES=("NOT_STARTED" "IN_PROGRESS" "BLOCKED" "COMPLETE" "FAILED")
if [[ ! " ${VALID_STATUSES[@]} " =~ " ${STATUS} " ]]; then
    echo "Error: Status must be one of: ${VALID_STATUSES[*]}"
    exit 1
fi

# Create phase status directory if it doesn't exist
mkdir -p "$PHASE_STATUS_DIR"

# Get phase details
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

PHASE_NAME="Phase $PHASE - ${PHASE_NAMES[$PHASE-1]}"

# Find phase directory
PHASE_DIR=$(find "$BASE_DIR" -maxdepth 1 -type d -name "phase-$PHASE-*" | head -1)

if [ -z "$PHASE_DIR" ]; then
    echo "Warning: Phase $PHASE directory not found"
    PHASE_DIR="$BASE_DIR/phase-$PHASE"
fi

# Get next steps from phase tracker if available
NEXT_STEPS="To be determined"
if [ -f "$PHASE_DIR/tracker.md" ]; then
    NEXT_STEPS=$(grep -A3 "Next Steps" "$PHASE_DIR/tracker.md" 2>/dev/null | tail -n +2 || echo "To be determined")
fi

# Get dependencies from phase config if available
DEPENDENCIES="[]"
if [ -f "$PHASE_DIR/config.json" ]; then
    DEPENDENCIES=$(jq -r '.dependencies // [] | @json' "$PHASE_DIR/config.json" 2>/dev/null || echo "[]")
fi

# Update current phase file
cat > "$CURRENT_PHASE_FILE" << EOF
# Current Migration Phase

## Phase: $PHASE_NAME
## Status: $STATUS
## Progress: $PROGRESS%
## Updated: $TIMESTAMP

### Recent Activity
$NOTES

### Status Details
- **Started**: $(grep "Phase: $PHASE" "$HISTORY_FILE" 2>/dev/null | head -1 | cut -d'|' -f1 | xargs || echo "$TIMESTAMP")
- **Last Update**: $TIMESTAMP
- **Days in Phase**: $(grep -c "Phase: $PHASE" "$HISTORY_FILE" 2>/dev/null || echo "1")
- **Status Changes**: $(grep "Phase: $PHASE" "$HISTORY_FILE" 2>/dev/null | awk -F'|' '{print $3}' | sort -u | wc -l || echo "1")

### Next Steps
$NEXT_STEPS

### Dependencies
EOF

# Parse and display dependencies
if [ "$DEPENDENCIES" != "[]" ]; then
    echo "$DEPENDENCIES" | jq -r '.[] | "- " + .' >> "$CURRENT_PHASE_FILE" 2>/dev/null || echo "- None" >> "$CURRENT_PHASE_FILE"
else
    echo "- None" >> "$CURRENT_PHASE_FILE"
fi

# Add metrics
cat >> "$CURRENT_PHASE_FILE" << EOF

### Phase Metrics
- **Completion Rate**: $PROGRESS%
- **Blockers**: $(grep -c "BLOCKER" "$BASE_DIR/constraint-tracking/blockers.md" 2>/dev/null || echo "0")
- **Validation Passes**: $(jq -r '.checks | map(select(.status == 0)) | length' "$BASE_DIR/verification-results/latest-validation.json" 2>/dev/null || echo "0")
- **Validation Failures**: $(jq -r '.checks | map(select(.status != 0)) | length' "$BASE_DIR/verification-results/latest-validation.json" 2>/dev/null || echo "0")

### Phase History
\`\`\`
EOF

# Show last 5 history entries for this phase
grep "Phase: $PHASE" "$HISTORY_FILE" 2>/dev/null | tail -5 >> "$CURRENT_PHASE_FILE" || echo "No history available" >> "$CURRENT_PHASE_FILE"

echo '```' >> "$CURRENT_PHASE_FILE"

# Update phase-specific config
if [ -f "$PHASE_DIR/config.json" ]; then
    jq --arg status "$STATUS" --arg progress "$PROGRESS" --arg updated "$TIMESTAMP" \
        '.status = $status | .progress = ($progress | tonumber) | .last_updated = $updated' \
        "$PHASE_DIR/config.json" > "$PHASE_DIR/config.json.tmp" && \
        mv "$PHASE_DIR/config.json.tmp" "$PHASE_DIR/config.json"
fi

# Log to history
echo "$TIMESTAMP | Phase: $PHASE | Status: $STATUS | Progress: $PROGRESS% | $NOTES" >> "$HISTORY_FILE"

# Update phase metrics if dashboard exists
if [ -f "$BASE_DIR/progress-metrics/dashboard.json" ]; then
    # Create temporary metrics update
    METRICS_UPDATE=$(cat << EOF
{
  "phase": "$PHASE_NAME",
  "status": "$STATUS",
  "progress": $PROGRESS,
  "updated": "$TIMESTAMP"
}
EOF
)
    
    # Update dashboard (create if doesn't exist)
    if [ ! -f "$BASE_DIR/progress-metrics/dashboard.json" ]; then
        mkdir -p "$BASE_DIR/progress-metrics"
        echo '{"phases": []}' > "$BASE_DIR/progress-metrics/dashboard.json"
    fi
    
    # Update or add phase data
    jq --argjson update "$METRICS_UPDATE" \
        '.phases = (.phases | map(if .phase == $update.phase then $update else . end) | if any(.phase == $update.phase) then . else . + [$update] end)' \
        "$BASE_DIR/progress-metrics/dashboard.json" > "$BASE_DIR/progress-metrics/dashboard.json.tmp" && \
        mv "$BASE_DIR/progress-metrics/dashboard.json.tmp" "$BASE_DIR/progress-metrics/dashboard.json"
fi

# Send notification if available
if command -v npx &> /dev/null && npx claude-flow --version &> /dev/null 2>&1; then
    npx claude-flow hooks notification --message "Phase $PHASE status updated: $STATUS ($PROGRESS%)" --telemetry true &> /dev/null || true
fi

echo "✅ Phase $PHASE status updated:"
echo "   Status: $STATUS"
echo "   Progress: $PROGRESS%"
echo "   Notes: $NOTES"
echo "   Updated: $TIMESTAMP"

# If phase is complete, suggest next phase
if [ "$STATUS" = "COMPLETE" ] && [ "$PROGRESS" = "100" ] && [ "$PHASE" -lt 9 ]; then
    NEXT_PHASE=$((PHASE + 1))
    echo ""
    echo "🎆 Phase $PHASE complete! Ready to start Phase $NEXT_PHASE - ${PHASE_NAMES[$NEXT_PHASE-1]}"
    echo "   Run: $0 $NEXT_PHASE IN_PROGRESS 0 \"Starting ${PHASE_NAMES[$NEXT_PHASE-1]}\""
fi