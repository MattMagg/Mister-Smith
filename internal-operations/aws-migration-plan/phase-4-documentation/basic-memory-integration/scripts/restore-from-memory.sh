#!/bin/bash
# restore-from-memory.sh - Recover state from basic-memory
# Usage: ./restore-from-memory.sh [options]

set -euo pipefail

# Configuration
TRACKER_BASE="/Users/mac-main/Mister-Smith/MisterSmith/aws-migration-plan/trackers"
BACKUP_DIR="/Users/mac-main/Mister-Smith/MisterSmith/aws-migration-plan/backups"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
RESTORE_TYPE="latest"
SPECIFIC_TIME=""
PHASE=""
COMPONENT=""
DRY_RUN=false
CREATE_BACKUP=true

# Help function
show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --type TYPE         Restore type (latest|timestamp|phase|component)"
    echo "  --timestamp TIME    Specific timestamp to restore from"
    echo "  --phase PHASE       Phase number to restore"
    echo "  --component NAME    Component to restore"
    echo "  --no-backup        Skip creating backup before restore"
    echo "  --dry-run          Show what would be restored without executing"
    echo "  --help             Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --type latest"
    echo "  $0 --type timestamp --timestamp 2025-07-11T16:00:00Z"
    echo "  $0 --type phase --phase 1"
    echo "  $0 --type component --component ecs-cluster"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --type)
            RESTORE_TYPE="$2"
            shift 2
            ;;
        --timestamp)
            SPECIFIC_TIME="$2"
            shift 2
            ;;
        --phase)
            PHASE="$2"
            shift 2
            ;;
        --component)
            COMPONENT="$2"
            shift 2
            ;;
        --no-backup)
            CREATE_BACKUP=false
            shift
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

# Function to create backup
create_backup() {
    if [[ "$CREATE_BACKUP" == "true" ]]; then
        echo -e "${YELLOW}Creating backup before restore...${NC}"
        
        mkdir -p "$BACKUP_DIR"
        local backup_name="pre-restore-${TIMESTAMP//:/}"
        
        if [[ "$DRY_RUN" == "true" ]]; then
            echo -e "${YELLOW}[DRY RUN] Would create backup: $BACKUP_DIR/$backup_name${NC}"
        else
            cp -r "$TRACKER_BASE" "$BACKUP_DIR/$backup_name"
            echo -e "${GREEN}✓ Backup created: $BACKUP_DIR/$backup_name${NC}"
        fi
    fi
}

# Function to query memory for latest state
query_latest_memory() {
    echo -e "${BLUE}Querying basic-memory for latest state...${NC}"
    
    # Use npx claude-flow memory commands
    local recent_activity=$(npx claude-flow memory recent \
        --timeframe "7d" \
        --type "aws-migration" 2>/dev/null || echo "")
    
    if [[ -z "$recent_activity" ]]; then
        echo -e "${RED}No recent memory entries found${NC}"
        return 1
    fi
    
    echo -e "${GREEN}Found recent memory entries:${NC}"
    echo "$recent_activity" | head -20
}

# Function to restore phase from memory
restore_phase_from_memory() {
    local phase=$1
    local tracker_file="$TRACKER_BASE/phase-${phase}-tracker.md"
    
    echo -e "${BLUE}Restoring Phase $phase from memory...${NC}"
    
    # Build memory URI for phase
    local memory_uri="memory://aws-migration/phases/phase-$phase"
    
    # Retrieve from memory
    local memory_content=$(npx claude-flow memory read \
        --uri "$memory_uri" 2>/dev/null || echo "")
    
    if [[ -z "$memory_content" ]]; then
        echo -e "${RED}No memory found for Phase $phase${NC}"
        return 1
    fi
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}[DRY RUN] Would restore to: $tracker_file${NC}"
        echo "Memory content preview:"
        echo "$memory_content" | head -30
    else
        # Extract tracker content from memory
        # This is a simplified extraction - in production, parse more carefully
        echo "$memory_content" > "${tracker_file}.restored"
        echo -e "${GREEN}✓ Phase $phase restored to: ${tracker_file}.restored${NC}"
        echo -e "${YELLOW}Review the restored file and rename to replace original if correct${NC}"
    fi
}

# Function to restore component from memory
restore_component_from_memory() {
    local component=$1
    
    echo -e "${BLUE}Restoring component $component from memory...${NC}"
    
    # Search for component in memory
    local search_results=$(npx claude-flow memory search \
        --query "$component" \
        --type "component" 2>/dev/null || echo "")
    
    if [[ -z "$search_results" ]]; then
        echo -e "${RED}No memory found for component $component${NC}"
        return 1
    fi
    
    echo -e "${GREEN}Found component in memory:${NC}"
    echo "$search_results"
    
    # Find which tracker file contains this component
    local target_tracker=""
    for tracker in "$TRACKER_BASE"/phase-*-tracker.md; do
        if grep -q "$component" "$tracker" 2>/dev/null; then
            target_tracker="$tracker"
            break
        fi
    done
    
    if [[ -z "$target_tracker" ]]; then
        echo -e "${YELLOW}Component not found in current trackers, creating new entry${NC}"
        target_tracker="$TRACKER_BASE/component-${component}.md"
    fi
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}[DRY RUN] Would update: $target_tracker${NC}"
    else
        # In a real implementation, merge the component data intelligently
        echo -e "${YELLOW}Component data retrieved. Manual merge required.${NC}"
        echo "$search_results" > "${target_tracker}.component-restore"
        echo -e "${GREEN}✓ Component data saved to: ${target_tracker}.component-restore${NC}"
    fi
}

# Function to restore from specific timestamp
restore_from_timestamp() {
    local timestamp=$1
    
    echo -e "${BLUE}Restoring state from timestamp: $timestamp${NC}"
    
    # Search memory for entries around this timestamp
    local memory_entries=$(npx claude-flow memory search \
        --query "$timestamp" \
        --limit 20 2>/dev/null || echo "")
    
    if [[ -z "$memory_entries" ]]; then
        echo -e "${RED}No memory entries found for timestamp: $timestamp${NC}"
        return 1
    fi
    
    echo -e "${GREEN}Found memory entries from timestamp:${NC}"
    echo "$memory_entries"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}[DRY RUN] Would restore entries from this timestamp${NC}"
    else
        # Create a restore directory for timestamp-based restore
        local restore_dir="$TRACKER_BASE/restore-$timestamp"
        mkdir -p "$restore_dir"
        
        echo "$memory_entries" > "$restore_dir/memory-entries.txt"
        echo -e "${GREEN}✓ Memory entries saved to: $restore_dir/memory-entries.txt${NC}"
        echo -e "${YELLOW}Manual review and restoration required${NC}"
    fi
}

# Function to show restore summary
show_restore_summary() {
    echo ""
    echo -e "${BLUE}=== Restore Summary ===${NC}"
    echo "Restore Type: $RESTORE_TYPE"
    echo "Timestamp: $TIMESTAMP"
    
    if [[ "$CREATE_BACKUP" == "true" ]]; then
        echo "Backup Created: Yes"
    else
        echo "Backup Created: No"
    fi
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}Mode: DRY RUN (no changes made)${NC}"
    else
        echo -e "${GREEN}Mode: LIVE (changes applied)${NC}"
    fi
    
    echo ""
    echo -e "${YELLOW}Next Steps:${NC}"
    echo "1. Review restored files (*.restored or *.component-restore)"
    echo "2. Compare with current trackers"
    echo "3. Rename restored files to replace originals if correct"
    echo "4. Run verification after restore"
}

# Main execution
echo -e "${GREEN}Basic-Memory → AWS Migration Tracker Restore${NC}"
echo "Timestamp: $TIMESTAMP"
echo ""

# Create backup if requested
create_backup

# Execute restore based on type
case $RESTORE_TYPE in
    latest)
        query_latest_memory
        ;;
    phase)
        if [[ -z "$PHASE" ]]; then
            echo -e "${RED}Error: --phase required for phase restore${NC}"
            exit 1
        fi
        restore_phase_from_memory "$PHASE"
        ;;
    component)
        if [[ -z "$COMPONENT" ]]; then
            echo -e "${RED}Error: --component required for component restore${NC}"
            exit 1
        fi
        restore_component_from_memory "$COMPONENT"
        ;;
    timestamp)
        if [[ -z "$SPECIFIC_TIME" ]]; then
            echo -e "${RED}Error: --timestamp required for timestamp restore${NC}"
            exit 1
        fi
        restore_from_timestamp "$SPECIFIC_TIME"
        ;;
    *)
        echo -e "${RED}Error: Unknown restore type: $RESTORE_TYPE${NC}"
        show_help
        exit 1
        ;;
esac

# Show summary
show_restore_summary

echo ""
echo -e "${GREEN}Restore operation completed${NC}"