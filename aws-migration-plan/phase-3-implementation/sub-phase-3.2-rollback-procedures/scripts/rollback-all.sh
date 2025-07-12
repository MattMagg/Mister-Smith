#!/bin/bash
# rollback-all.sh
# Master rollback script for complete MisterSmith system rollback

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Configuration
ENVIRONMENT=${1:-production}
CONFIRM=${2:-false}
ROLLBACK_LOG="/var/log/mistersmith/rollback-$(date +%Y%m%d-%H%M%S).log"
NOTIFICATION_WEBHOOK="${MISTERSMITH_WEBHOOK_URL:-}"

# Ensure log directory exists
mkdir -p /var/log/mistersmith

# Function to log with timestamp
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a $ROLLBACK_LOG
}

# Function to send notification
send_notification() {
    local status=$1
    local message=$2
    
    if [ -n "$NOTIFICATION_WEBHOOK" ]; then
        curl -X POST $NOTIFICATION_WEBHOOK \
            -H "Content-Type: application/json" \
            -d "{\"status\": \"$status\", \"message\": \"$message\", \"environment\": \"$ENVIRONMENT\", \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" \
            > /dev/null 2>&1 || true
    fi
}

# Function to check if service exists
service_exists() {
    docker ps -a --format "{{.Names}}" | grep -q "^$1$"
}

# Function to execute rollback step
execute_rollback() {
    local component=$1
    local script=$2
    local critical=${3:-true}
    
    log "Rolling back $component..."
    
    if [ -f "$SCRIPT_DIR/$script" ]; then
        if bash "$SCRIPT_DIR/$script" >> $ROLLBACK_LOG 2>&1; then
            log "✅ $component rollback successful"
            return 0
        else
            log "❌ $component rollback failed"
            if [ "$critical" = "true" ]; then
                return 1
            else
                log "⚠️  Continuing despite $component rollback failure (non-critical)"
                return 0
            fi
        fi
    else
        log "⚠️  Rollback script not found: $script"
        return 1
    fi
}

# Confirmation check
if [ "$CONFIRM" != "--confirm" ]; then
    echo -e "${RED}⚠️  WARNING: This will rollback ALL MisterSmith services to previous version${NC}"
    echo -e "${YELLOW}Current environment: $ENVIRONMENT${NC}"
    echo ""
    echo "This action will:"
    echo "  • Stop all running services"
    echo "  • Restore database from backup"
    echo "  • Restore NATS message streams"
    echo "  • Revert all services to previous versions"
    echo "  • Restore previous configurations"
    echo ""
    echo -e "${YELLOW}Usage: $0 [environment] --confirm${NC}"
    exit 1
fi

# Start rollback
clear
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     MisterSmith System Rollback        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}Environment: $ENVIRONMENT${NC}"
echo -e "${YELLOW}Start time: $(date)${NC}"
echo -e "${YELLOW}Log file: $ROLLBACK_LOG${NC}"
echo ""

log "🔄 Starting complete system rollback for $ENVIRONMENT..."
send_notification "started" "System rollback initiated for $ENVIRONMENT"

# Create rollback state file
ROLLBACK_STATE="/tmp/mistersmith-rollback-state"
echo "IN_PROGRESS" > $ROLLBACK_STATE

# Step 1: Pre-rollback health snapshot
echo -e "${YELLOW}Step 1/8: Capturing current system state...${NC}"
log "Capturing pre-rollback system state"

{
    echo "=== Pre-Rollback System State ==="
    echo "Timestamp: $(date)"
    echo ""
    echo "Running containers:"
    docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
    echo ""
    echo "Database status:"
    sudo -u postgres pg_isready || echo "Database not responding"
    echo ""
    echo "NATS status:"
    nats server check connection 2>/dev/null || echo "NATS not responding"
} >> $ROLLBACK_LOG 2>&1

# Step 2: Stop all services
echo -e "${YELLOW}Step 2/8: Stopping all services...${NC}"
log "Stopping all MisterSmith services"

# Stop services in reverse dependency order
services=("mistersmith-webui" "mistersmith-orchestrator" "mistersmith-nats")
for service in "${services[@]}"; do
    if service_exists "$service"; then
        log "Stopping $service..."
        docker stop "$service" >> $ROLLBACK_LOG 2>&1 || true
        docker rm "$service" >> $ROLLBACK_LOG 2>&1 || true
    fi
done

# Step 3: Backup current state
echo -e "${YELLOW}Step 3/8: Backing up current state...${NC}"
log "Creating emergency backup of current state"

EMERGENCY_BACKUP_DIR="/var/backups/mistersmith/emergency-$(date +%Y%m%d-%H%M%S)"
mkdir -p $EMERGENCY_BACKUP_DIR

# Backup current configs
cp -r ~/.mistersmith $EMERGENCY_BACKUP_DIR/ 2>/dev/null || true

# Quick database backup if accessible
if sudo -u postgres pg_isready > /dev/null 2>&1; then
    sudo -u postgres pg_dump mistersmith > $EMERGENCY_BACKUP_DIR/database-emergency.sql 2>/dev/null || true
fi

# Step 4: Rollback Database
echo -e "${YELLOW}Step 4/8: Rolling back database...${NC}"
if ! execute_rollback "Database" "rollback-database.sh"; then
    log "CRITICAL: Database rollback failed"
    echo "FAILED" > $ROLLBACK_STATE
    send_notification "failed" "Database rollback failed - manual intervention required"
    exit 1
fi

# Step 5: Rollback NATS
echo -e "${YELLOW}Step 5/8: Rolling back NATS messaging...${NC}"
if ! execute_rollback "NATS" "rollback-nats.sh"; then
    log "CRITICAL: NATS rollback failed"
    echo "FAILED" > $ROLLBACK_STATE
    send_notification "failed" "NATS rollback failed - manual intervention required"
    exit 1
fi

# Step 6: Rollback Orchestrator
echo -e "${YELLOW}Step 6/8: Rolling back Orchestrator service...${NC}"
if ! execute_rollback "Orchestrator" "rollback-orchestrator.sh"; then
    log "CRITICAL: Orchestrator rollback failed"
    echo "FAILED" > $ROLLBACK_STATE
    send_notification "failed" "Orchestrator rollback failed - manual intervention required"
    exit 1
fi

# Step 7: Rollback Web UI
echo -e "${YELLOW}Step 7/8: Rolling back Web UI...${NC}"
if ! execute_rollback "Web UI" "rollback-webui.sh" false; then
    log "WARNING: Web UI rollback encountered issues"
fi

# Step 8: Verify rollback
echo -e "${YELLOW}Step 8/8: Verifying system health...${NC}"
log "Running comprehensive health checks"

# Execute verification script
if [ -f "$SCRIPT_DIR/verify-rollback.sh" ]; then
    if bash "$SCRIPT_DIR/verify-rollback.sh" >> $ROLLBACK_LOG 2>&1; then
        ROLLBACK_SUCCESS=true
        log "✅ All health checks passed"
    else
        ROLLBACK_SUCCESS=false
        log "⚠️  Some health checks failed"
    fi
else
    log "⚠️  Verification script not found"
    ROLLBACK_SUCCESS=false
fi

# Generate rollback summary
echo "" | tee -a $ROLLBACK_LOG
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}" | tee -a $ROLLBACK_LOG
echo -e "${BLUE}║        Rollback Summary                ║${NC}" | tee -a $ROLLBACK_LOG
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}" | tee -a $ROLLBACK_LOG
echo "" | tee -a $ROLLBACK_LOG

{
    echo "Environment: $ENVIRONMENT"
    echo "Start time: $(head -1 $ROLLBACK_LOG | cut -d' ' -f1-2)"
    echo "End time: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "Duration: $SECONDS seconds"
    echo ""
    echo "Component Status:"
    echo "  • Database: $(grep -q "Database rollback successful" $ROLLBACK_LOG && echo "✅ Rolled back" || echo "❌ Failed")"
    echo "  • NATS: $(grep -q "NATS rollback successful" $ROLLBACK_LOG && echo "✅ Rolled back" || echo "❌ Failed")"
    echo "  • Orchestrator: $(grep -q "Orchestrator rollback successful" $ROLLBACK_LOG && echo "✅ Rolled back" || echo "❌ Failed")"
    echo "  • Web UI: $(grep -q "Web UI rollback successful" $ROLLBACK_LOG && echo "✅ Rolled back" || echo "⚠️  Issues")"
    echo ""
    echo "Post-Rollback Health:"
    docker ps --format "table {{.Names}}\t{{.Status}}" | grep mistersmith || echo "  No services running"
    echo ""
} | tee -a $ROLLBACK_LOG

# Final status
if [ "$ROLLBACK_SUCCESS" = true ]; then
    echo -e "${GREEN}✅ ROLLBACK COMPLETED SUCCESSFULLY${NC}" | tee -a $ROLLBACK_LOG
    echo "COMPLETED" > $ROLLBACK_STATE
    send_notification "completed" "System rollback completed successfully"
    
    # Post-rollback recommendations
    echo "" | tee -a $ROLLBACK_LOG
    echo -e "${YELLOW}Post-Rollback Actions Required:${NC}" | tee -a $ROLLBACK_LOG
    echo "1. Review the rollback log: $ROLLBACK_LOG" | tee -a $ROLLBACK_LOG
    echo "2. Verify application functionality manually" | tee -a $ROLLBACK_LOG
    echo "3. Update monitoring alerts if needed" | tee -a $ROLLBACK_LOG
    echo "4. Schedule post-mortem meeting" | tee -a $ROLLBACK_LOG
    echo "5. Document lessons learned" | tee -a $ROLLBACK_LOG
    
    exit 0
else
    echo -e "${RED}❌ ROLLBACK COMPLETED WITH ERRORS${NC}" | tee -a $ROLLBACK_LOG
    echo "COMPLETED_WITH_ERRORS" > $ROLLBACK_STATE
    send_notification "completed_with_errors" "System rollback completed with errors - review required"
    
    echo "" | tee -a $ROLLBACK_LOG
    echo -e "${RED}⚠️  Manual intervention may be required${NC}" | tee -a $ROLLBACK_LOG
    echo "Emergency backup location: $EMERGENCY_BACKUP_DIR" | tee -a $ROLLBACK_LOG
    
    exit 1
fi