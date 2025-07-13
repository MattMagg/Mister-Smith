#!/bin/bash
# sync-to-memory.sh - Push tracker updates to basic-memory
# Usage: ./sync-to-memory.sh [options]

set -euo pipefail

# Configuration
TRACKER_BASE="/Users/mac-main/Mister-Smith/MisterSmith/aws-migration-plan/trackers"
MEMORY_FOLDER="aws-migration"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Default values
PHASE=""
SUB_PHASE=""
COMPONENT=""
STATUS=""
TYPE="phase"
DRY_RUN=false

# Help function
show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --phase PHASE       Phase number to sync"
    echo "  --sub-phase NAME    Sub-phase name"
    echo "  --component NAME    Component name"
    echo "  --status STATUS     Status (planning|active|complete|verified)"
    echo "  --type TYPE         Type (phase|component|verification|constraint)"
    echo "  --dry-run          Show what would be synced without executing"
    echo "  --help             Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --phase 1 --status complete"
    echo "  $0 --type component --component ecs-cluster --status deployed"
    echo "  $0 --type constraint --component budget-limit"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --phase)
            PHASE="$2"
            shift 2
            ;;
        --sub-phase)
            SUB_PHASE="$2"
            shift 2
            ;;
        --component)
            COMPONENT="$2"
            shift 2
            ;;
        --status)
            STATUS="$2"
            shift 2
            ;;
        --type)
            TYPE="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Function to sync phase data
sync_phase_data() {
    local phase=$1
    local tracker_file="$TRACKER_BASE/phase-${phase}-tracker.md"
    
    if [[ ! -f "$tracker_file" ]]; then
        echo -e "${RED}Error: Tracker file not found: $tracker_file${NC}"
        return 1
    fi
    
    echo -e "${YELLOW}Syncing Phase $phase data...${NC}"
    
    # Extract phase information
    local phase_status=$(grep -E "^## Status:" "$tracker_file" | sed 's/## Status: //' || echo "unknown")
    local components=$(grep -E "^- \[" "$tracker_file" | wc -l || echo "0")
    local completed=$(grep -E "^- \[x\]" "$tracker_file" | wc -l || echo "0")
    
    # Build memory content
    local content=$(cat <<EOF
# AWS Migration - Phase $phase

## Status: $phase_status
## Last Sync: $TIMESTAMP

### Progress
- Total Components: $components
- Completed: $completed
- Completion: $((completed * 100 / (components > 0 ? components : 1)))%

### Tracker File
Location: $tracker_file

### Phase Details
$(grep -A 20 "^## Phase Overview" "$tracker_file" || echo "No overview available")

### Current Components
$(grep -E "^- \[.\]" "$tracker_file" || echo "No components found")

---
*Synced from tracker at $TIMESTAMP*
EOF
)

    # Determine memory path
    local memory_path="aws-migration/phases/phase-$phase"
    if [[ -n "$SUB_PHASE" ]]; then
        memory_path="$memory_path/$SUB_PHASE"
    fi
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}[DRY RUN] Would sync to: $memory_path${NC}"
        echo "$content"
    else
        # Use basic-memory MCP tool to write
        echo "$content" | npx claude-flow memory write \
            --folder "$MEMORY_FOLDER" \
            --title "Phase $phase - $phase_status" \
            --tags "phase-$phase,migration,aws,$phase_status"
        
        echo -e "${GREEN}✓ Phase $phase synced to memory${NC}"
    fi
}

# Function to sync component data
sync_component_data() {
    local component=$1
    local status=${2:-"unknown"}
    
    echo -e "${YELLOW}Syncing component: $component (status: $status)${NC}"
    
    # Find component in tracker files
    local component_info=""
    for tracker in "$TRACKER_BASE"/phase-*-tracker.md; do
        if grep -q "$component" "$tracker"; then
            component_info=$(grep -A 5 -B 5 "$component" "$tracker" || echo "")
            break
        fi
    done
    
    # Build memory content
    local content=$(cat <<EOF
# AWS Migration - Component: $component

## Status: $status
## Last Sync: $TIMESTAMP

### Component Information
$component_info

### Deployment Status
- Status: $status
- Last Updated: $TIMESTAMP

---
*Component tracking synchronized at $TIMESTAMP*
EOF
)

    local memory_path="aws-migration/components/$component"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}[DRY RUN] Would sync to: $memory_path${NC}"
        echo "$content"
    else
        echo "$content" | npx claude-flow memory write \
            --folder "$MEMORY_FOLDER" \
            --title "Component: $component - $status" \
            --tags "component,$component,$status,aws"
        
        echo -e "${GREEN}✓ Component $component synced to memory${NC}"
    fi
}

# Function to sync verification results
sync_verification_data() {
    local command_set=$1
    local verification_log="$TRACKER_BASE/../verification-logs/${command_set}-verification.log"
    
    if [[ ! -f "$verification_log" ]]; then
        echo -e "${YELLOW}Creating verification result from current state...${NC}"
        verification_log="/tmp/${command_set}-verification.log"
        echo "Verification for $command_set at $TIMESTAMP" > "$verification_log"
    fi
    
    # Build memory content
    local content=$(cat <<EOF
# AWS Migration - Verification: $command_set

## Timestamp: $TIMESTAMP

### Verification Results
$(cat "$verification_log" 2>/dev/null || echo "No verification log available")

### Summary
- Command Set: $command_set
- Executed: $TIMESTAMP
- Status: Completed

---
*Verification results synchronized at $TIMESTAMP*
EOF
)

    local memory_path="aws-migration/verification/$command_set"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}[DRY RUN] Would sync to: $memory_path${NC}"
        echo "$content"
    else
        echo "$content" | npx claude-flow memory write \
            --folder "$MEMORY_FOLDER" \
            --title "Verification: $command_set - $TIMESTAMP" \
            --tags "verification,$command_set,aws"
        
        echo -e "${GREEN}✓ Verification $command_set synced to memory${NC}"
    fi
}

# Function to sync constraint data
sync_constraint_data() {
    local constraint_id=$1
    local constraints_file="$TRACKER_BASE/../CONSTRAINTS.md"
    
    echo -e "${YELLOW}Syncing constraint: $constraint_id${NC}"
    
    # Extract constraint information
    local constraint_info=""
    if [[ -f "$constraints_file" ]]; then
        constraint_info=$(grep -A 10 -B 2 "$constraint_id" "$constraints_file" || echo "Constraint not found in file")
    fi
    
    # Build memory content
    local content=$(cat <<EOF
# AWS Migration - Constraint: $constraint_id

## Last Sync: $TIMESTAMP

### Constraint Details
$constraint_info

### Tracking Information
- Constraint ID: $constraint_id
- Source File: $constraints_file
- Last Updated: $TIMESTAMP

---
*Constraint synchronized at $TIMESTAMP*
EOF
)

    local memory_path="aws-migration/constraints/$constraint_id"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}[DRY RUN] Would sync to: $memory_path${NC}"
        echo "$content"
    else
        echo "$content" | npx claude-flow memory write \
            --folder "$MEMORY_FOLDER" \
            --title "Constraint: $constraint_id" \
            --tags "constraint,$constraint_id,aws"
        
        echo -e "${GREEN}✓ Constraint $constraint_id synced to memory${NC}"
    fi
}

# Main execution
echo -e "${GREEN}AWS Migration Tracker → Basic-Memory Sync${NC}"
echo "Timestamp: $TIMESTAMP"
echo ""

case $TYPE in
    phase)
        if [[ -z "$PHASE" ]]; then
            echo -e "${RED}Error: --phase required for phase sync${NC}"
            exit 1
        fi
        sync_phase_data "$PHASE"
        ;;
    component)
        if [[ -z "$COMPONENT" ]]; then
            echo -e "${RED}Error: --component required for component sync${NC}"
            exit 1
        fi
        sync_component_data "$COMPONENT" "$STATUS"
        ;;
    verification)
        if [[ -z "$COMPONENT" ]]; then
            echo -e "${RED}Error: --component required for verification sync${NC}"
            exit 1
        fi
        sync_verification_data "$COMPONENT"
        ;;
    constraint)
        if [[ -z "$COMPONENT" ]]; then
            echo -e "${RED}Error: --component required for constraint sync${NC}"
            exit 1
        fi
        sync_constraint_data "$COMPONENT"
        ;;
    *)
        echo -e "${RED}Error: Unknown type: $TYPE${NC}"
        show_help
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}Sync operation completed${NC}"