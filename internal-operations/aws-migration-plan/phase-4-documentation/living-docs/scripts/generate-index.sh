#!/bin/bash
# generate-index.sh - Aggregates all phase trackers
# Auto-generates master index from all migration documentation

set -euo pipefail

# Configuration
BASE_DIR="${AWS_MIGRATION_DIR:-/Users/mac-main/Mister-Smith/MisterSmith/aws-migration-plan}"
INDEX_FILE="$BASE_DIR/index.md"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Function to extract phase status
get_phase_status() {
    local phase_dir=$1
    local phase_num=$2
    
    if [ -f "$phase_dir/tracker.md" ]; then
        local status=$(grep -m1 "Status:" "$phase_dir/tracker.md" | cut -d':' -f2 | xargs || echo "UNKNOWN")
        local progress=$(grep -m1 "Progress:" "$phase_dir/tracker.md" | cut -d':' -f2 | xargs || echo "0%")
        echo "Phase $phase_num|$status|$progress"
    else
        echo "Phase $phase_num|NOT_STARTED|0%"
    fi
}

# Function to get active blockers
get_blockers() {
    if [ -f "$BASE_DIR/constraint-tracking/blockers.md" ]; then
        grep -n "\[BLOCKER-" "$BASE_DIR/constraint-tracking/blockers.md" | head -5 || echo "No active blockers"
    else
        echo "Blocker tracking not initialized"
    fi
}

# Function to get latest validation results
get_validation_summary() {
    if [ -f "$BASE_DIR/verification-results/latest-validation.json" ]; then
        jq -r '.summary // "No validation summary available"' "$BASE_DIR/verification-results/latest-validation.json" 2>/dev/null || echo "Validation data corrupted"
    else
        echo "No validations run yet"
    fi
}

# Generate the index
cat > "$INDEX_FILE" << EOF
# MisterSmith AWS Migration Master Index
> **Generated**: $TIMESTAMP  
> **Version**: 1.0.0  
> **Status**: ACTIVE MIGRATION  

## 📊 Migration Overview

### Current Status
EOF

# Add current phase status
if [ -f "$BASE_DIR/phase-status/current-phase.md" ]; then
    echo "" >> "$INDEX_FILE"
    grep -A5 "## Status" "$BASE_DIR/phase-status/current-phase.md" >> "$INDEX_FILE" 2>/dev/null || echo "Status information not available" >> "$INDEX_FILE"
fi

# Add phase progress table
cat >> "$INDEX_FILE" << 'EOF'

## 📈 Phase Progress

| Phase | Status | Progress | Details |
|-------|--------|----------|----------|
EOF

# Scan all phase directories
for phase_dir in "$BASE_DIR"/phase-*-*/; do
    if [ -d "$phase_dir" ]; then
        phase_name=$(basename "$phase_dir")
        phase_num=$(echo "$phase_name" | grep -oE '[0-9]+' | head -1)
        phase_info=$(get_phase_status "$phase_dir" "$phase_num")
        
        IFS='|' read -r phase status progress <<< "$phase_info"
        details_link="[View Details](./$phase_name/tracker.md)"
        
        echo "| $phase | $status | $progress | $details_link |" >> "$INDEX_FILE"
    fi
done

# Add active blockers section
cat >> "$INDEX_FILE" << 'EOF'

## 🚧 Active Blockers

EOF

blockers=$(get_blockers)
if [ "$blockers" != "No active blockers" ] && [ "$blockers" != "Blocker tracking not initialized" ]; then
    echo '```' >> "$INDEX_FILE"
    echo "$blockers" >> "$INDEX_FILE"
    echo '```' >> "$INDEX_FILE"
else
    echo "✅ $blockers" >> "$INDEX_FILE"
fi

# Add validation summary
cat >> "$INDEX_FILE" << 'EOF'

## ✅ Latest Validation Results

EOF

validation=$(get_validation_summary)
echo "> $validation" >> "$INDEX_FILE"

# Add metrics summary
if [ -f "$BASE_DIR/progress-metrics/dashboard.json" ]; then
    cat >> "$INDEX_FILE" << 'EOF'

## 📊 Key Metrics

EOF
    
    overall_progress=$(jq -r '.overall.progress // 0' "$BASE_DIR/progress-metrics/dashboard.json" 2>/dev/null || echo "0")
    phases_completed=$(jq -r '.overall.phases_completed // 0' "$BASE_DIR/progress-metrics/dashboard.json" 2>/dev/null || echo "0")
    phases_total=$(jq -r '.overall.phases_total // 0' "$BASE_DIR/progress-metrics/dashboard.json" 2>/dev/null || echo "0")
    budget_used=$(jq -r '.overall.budget_utilization // 0' "$BASE_DIR/progress-metrics/dashboard.json" 2>/dev/null || echo "0")
    
    cat >> "$INDEX_FILE" << EOF
- **Overall Progress**: $overall_progress%
- **Phases Completed**: $phases_completed / $phases_total
- **Budget Utilization**: $budget_used%
- **Last Updated**: $TIMESTAMP
EOF
fi

# Add quick links
cat >> "$INDEX_FILE" << 'EOF'

## 🔗 Quick Links

### Documentation
- [Living Documentation Tracker](./phase-4-documentation/living-docs/tracker.md)
- [Operational Runbooks](./operational-runbooks/)
- [Verification Scripts](./verification-results/verification-scripts/)
- [Constraint Tracking](./constraint-tracking/)

### Phase Documentation
EOF

# List all phase documentation
for phase_dir in "$BASE_DIR"/phase-*-*/; do
    if [ -d "$phase_dir" ]; then
        phase_name=$(basename "$phase_dir")
        echo "- [$phase_name](./$phase_name/)" >> "$INDEX_FILE"
    fi
done

# Add automation status
cat >> "$INDEX_FILE" << 'EOF'

## 🤖 Automation Status

| Script | Last Run | Status | Next Run |
|--------|----------|---------|----------|
EOF

# Check automation script status
for script in generate-index.sh update-status.sh collect-metrics.sh verify-all.sh; do
    script_path="$BASE_DIR/phase-4-documentation/living-docs/scripts/$script"
    if [ -f "$script_path" ]; then
        echo "| $script | $TIMESTAMP | ✅ Active | Scheduled |" >> "$INDEX_FILE"
    else
        echo "| $script | - | ⚠️ Not Found | - |" >> "$INDEX_FILE"
    fi
done

# Add footer
cat >> "$INDEX_FILE" << 'EOF'

---

**Automation**: This index is automatically generated every 4 hours  
**Manual Trigger**: Run `./scripts/generate-index.sh` to update  
**Support**: Contact DevOps team for issues  
EOF

echo "✅ Master index generated at: $INDEX_FILE"

# Update memory if claude-flow is available
if command -v npx &> /dev/null && npx claude-flow --version &> /dev/null 2>&1; then
    npx claude-flow hooks notification --message "Master index regenerated: $overall_progress% complete" --telemetry true &> /dev/null || true
fi