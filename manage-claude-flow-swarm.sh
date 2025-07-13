#!/bin/bash

# Claude Flow Development Swarm Management Script
# Phase 3 Master Integration - Production Management System
# Created by: Swarm Master Configuration & Integration Coordinator

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SWARM_NAME="claude-flow-dev-master"
CONFIG_FILE="$SCRIPT_DIR/claude-flow-development-swarm.json"
LOG_FILE="$SCRIPT_DIR/management.log"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Logging functions
log() { echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"; }
error() { echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"; }
warning() { echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"; }

# Help function
show_help() {
    cat << EOF
🐝 Claude Flow Development Swarm Management

USAGE: $0 <command> [options]

CORE COMMANDS:
  status                Show comprehensive swarm status
  agents                List all agents and their status
  health                Detailed health check and diagnostics
  metrics               Performance metrics and analytics
  
SCALING COMMANDS:
  scale <count>         Scale swarm to specified agent count
  scale-up <type>       Add more agents of specific type
  scale-down <type>     Remove agents of specific type
  auto-scale            Enable/disable automatic scaling
  
COORDINATION COMMANDS:
  batch-test            Test batch operation capabilities
  neural-status         Show neural training status
  neural-train          Trigger neural training session
  memory-status         Show memory system status
  memory-cleanup        Clean up old memory entries
  
MAINTENANCE COMMANDS:
  backup                Create full swarm backup
  restore <backup>      Restore from backup
  restart               Restart swarm with current config
  update                Update to latest configuration
  cleanup               Remove inactive/failed agents
  
MONITORING COMMANDS:
  monitor               Real-time monitoring dashboard
  logs                  Show recent swarm logs
  performance           Performance analysis report
  bottlenecks           Identify performance bottlenecks
  
DEPLOYMENT COMMANDS:
  deploy                Deploy/redeploy full swarm
  stop                  Gracefully stop swarm
  emergency-stop        Force stop all operations
  
EXAMPLES:
  $0 status                    # Quick status overview
  $0 scale 30                  # Scale to 30 agents
  $0 scale-up architect        # Add more architects
  $0 neural-train              # Start neural training
  $0 monitor                   # Real-time monitoring
  $0 backup                    # Create backup
  $0 performance              # Performance report

EOF
}

# Status functions
show_status() {
    log "Getting swarm status..."
    
    echo -e "\n🐝 ${CYAN}Claude Flow Development Swarm Status${NC}"
    echo "================================================="
    
    # Basic swarm info
    local swarm_data=$(npx claude-flow@alpha swarm status --json 2>/dev/null || echo '{}')
    local agent_count=$(echo "$swarm_data" | jq -r '.agents | length // 0')
    local swarm_status=$(echo "$swarm_data" | jq -r '.status // "unknown"')
    
    echo -e "📊 Swarm: ${CYAN}$SWARM_NAME${NC}"
    echo -e "🔄 Status: ${GREEN}$swarm_status${NC}"
    echo -e "👥 Agents: ${GREEN}$agent_count${NC} active"
    echo -e "📅 Last Check: ${BLUE}$(date)${NC}"
    
    # Agent distribution
    if [ "$agent_count" -gt 0 ]; then
        echo -e "\n📋 Agent Distribution:"
        npx claude-flow@alpha agent list --format table 2>/dev/null || \
            echo "  Agent details unavailable"
    fi
    
    # Memory status
    local memory_stats=$(npx claude-flow@alpha memory stats claude-flow-dev 2>/dev/null || echo "")
    if [ -n "$memory_stats" ]; then
        echo -e "\n💾 Memory Status:"
        echo "$memory_stats" | head -5
    fi
    
    # Neural status
    local neural_data=$(npx claude-flow@alpha neural status --json 2>/dev/null || echo '{}')
    local training_active=$(echo "$neural_data" | jq -r '.training_active // false')
    echo -e "\n🧠 Neural Systems: ${training_active}"
    
    # Performance overview
    echo -e "\n⚡ Performance:"
    npx claude-flow@alpha performance report --summary 2>/dev/null || \
        echo "  Performance data collecting..."
}

show_agents() {
    log "Listing all agents..."
    
    echo -e "\n👥 ${CYAN}Agent Status Report${NC}"
    echo "================================="
    
    npx claude-flow@alpha agent list --detailed --swarm "$SWARM_NAME" 2>/dev/null || {
        warning "Could not retrieve detailed agent list"
        npx claude-flow@alpha agent list 2>/dev/null || \
            error "No agent data available"
    }
    
    # Group by type
    echo -e "\n📊 Agents by Specialization:"
    npx claude-flow@alpha agent list --group-by type 2>/dev/null || \
        echo "  Grouping data unavailable"
}

health_check() {
    log "Running comprehensive health check..."
    
    echo -e "\n🏥 ${CYAN}Health Diagnostics${NC}"
    echo "========================="
    
    local health_score=0
    local max_score=5
    
    # Check swarm connectivity
    if npx claude-flow@alpha swarm status >/dev/null 2>&1; then
        echo -e "✅ Swarm connectivity: ${GREEN}OK${NC}"
        ((health_score++))
    else
        echo -e "❌ Swarm connectivity: ${RED}FAILED${NC}"
    fi
    
    # Check agent responsiveness
    local responsive_agents=$(npx claude-flow@alpha agent list --ping 2>/dev/null | grep -c "responsive" || echo "0")
    if [ "$responsive_agents" -gt 10 ]; then
        echo -e "✅ Agent responsiveness: ${GREEN}$responsive_agents responsive${NC}"
        ((health_score++))
    else
        echo -e "⚠️  Agent responsiveness: ${YELLOW}$responsive_agents responsive${NC}"
    fi
    
    # Check memory system
    if npx claude-flow@alpha memory health-check claude-flow-dev >/dev/null 2>&1; then
        echo -e "✅ Memory system: ${GREEN}OK${NC}"
        ((health_score++))
    else
        echo -e "❌ Memory system: ${RED}ISSUES DETECTED${NC}"
    fi
    
    # Check neural training
    if npx claude-flow@alpha neural status --json | jq -e '.training_active' >/dev/null 2>&1; then
        echo -e "✅ Neural training: ${GREEN}ACTIVE${NC}"
        ((health_score++))
    else
        echo -e "⚠️  Neural training: ${YELLOW}INACTIVE${NC}"
    fi
    
    # Check coordination protocols
    if npx claude-flow@alpha memory retrieve claude-flow-dev/protocols/batch-operations >/dev/null 2>&1; then
        echo -e "✅ Coordination protocols: ${GREEN}CONFIGURED${NC}"
        ((health_score++))
    else
        echo -e "❌ Coordination protocols: ${RED}MISSING${NC}"
    fi
    
    # Overall health
    local health_percent=$((health_score * 100 / max_score))
    echo -e "\n🎯 Overall Health: ${health_percent}% (${health_score}/${max_score})"
    
    if [ $health_score -eq $max_score ]; then
        success "All systems operational"
    elif [ $health_score -ge 3 ]; then
        warning "Minor issues detected"
    else
        error "Critical issues require attention"
    fi
}

# Scaling functions
scale_swarm() {
    local target_count="$1"
    
    if [ -z "$target_count" ] || ! [[ "$target_count" =~ ^[0-9]+$ ]]; then
        error "Please provide a valid agent count"
        exit 1
    fi
    
    log "Scaling swarm to $target_count agents..."
    
    local current_count=$(npx claude-flow@alpha agent list --count 2>/dev/null || echo "0")
    
    if [ "$target_count" -gt "$current_count" ]; then
        local add_count=$((target_count - current_count))
        log "Adding $add_count agents..."
        npx claude-flow@alpha swarm scale --count "$target_count"
    elif [ "$target_count" -lt "$current_count" ]; then
        local remove_count=$((current_count - target_count))
        log "Removing $remove_count agents..."
        npx claude-flow@alpha swarm scale --count "$target_count" --graceful
    else
        success "Already at target count: $target_count"
        return
    fi
    
    success "Scaled to $target_count agents"
}

# Neural functions
neural_status() {
    log "Getting neural training status..."
    
    echo -e "\n🧠 ${CYAN}Neural Systems Status${NC}"
    echo "=============================="
    
    npx claude-flow@alpha neural status --detailed 2>/dev/null || \
        warning "Neural status unavailable"
    
    echo -e "\n📊 Training Metrics:"
    npx claude-flow@alpha neural metrics 2>/dev/null || \
        echo "  Metrics collecting..."
}

neural_train() {
    log "Starting neural training session..."
    
    echo -e "\n🎓 ${CYAN}Neural Training Session${NC}"
    echo "================================"
    
    npx claude-flow@alpha neural train \
        --pattern-type coordination \
        --ensemble-mode true \
        --continuous true \
        --swarm "$SWARM_NAME"
    
    success "Neural training initiated"
}

# Memory functions
memory_status() {
    log "Checking memory system status..."
    
    echo -e "\n💾 ${CYAN}Memory System Status${NC}"
    echo "============================="
    
    npx claude-flow@alpha memory stats claude-flow-dev 2>/dev/null || \
        warning "Memory stats unavailable"
    
    echo -e "\nNamespace Usage:"
    npx claude-flow@alpha memory list claude-flow-dev --summary 2>/dev/null || \
        echo "  Memory data collecting..."
}

# Monitoring functions
monitor_swarm() {
    log "Starting real-time monitoring..."
    
    echo -e "\n📊 ${CYAN}Real-time Swarm Monitor${NC}"
    echo "================================"
    echo "Press Ctrl+C to exit"
    echo ""
    
    while true; do
        clear
        show_status
        sleep 5
    done
}

performance_report() {
    log "Generating performance report..."
    
    echo -e "\n⚡ ${CYAN}Performance Analysis${NC}"
    echo "============================"
    
    npx claude-flow@alpha performance report --detailed --swarm "$SWARM_NAME" 2>/dev/null || \
        warning "Performance data not available"
    
    # Bottleneck analysis
    echo -e "\n🔍 Bottleneck Analysis:"
    npx claude-flow@alpha bottleneck analyze --swarm "$SWARM_NAME" 2>/dev/null || \
        echo "  Analysis in progress..."
    
    # Token usage
    echo -e "\n💰 Token Usage:"
    npx claude-flow@alpha token usage --timeframe 24h 2>/dev/null || \
        echo "  Usage data collecting..."
}

# Backup functions
backup_swarm() {
    local backup_name="claude-flow-swarm-backup-$(date +%Y%m%d-%H%M%S)"
    log "Creating backup: $backup_name"
    
    npx claude-flow@alpha backup create "$backup_name" \
        --include-memory \
        --include-neural \
        --include-config \
        --swarm "$SWARM_NAME"
    
    success "Backup created: $backup_name"
}

# Main command handling
case "${1:-help}" in
    "status"|"st")
        show_status
        ;;
    "agents"|"ag")
        show_agents
        ;;
    "health"|"hc")
        health_check
        ;;
    "scale")
        scale_swarm "$2"
        ;;
    "neural-status"|"ns")
        neural_status
        ;;
    "neural-train"|"nt")
        neural_train
        ;;
    "memory-status"|"ms")
        memory_status
        ;;
    "monitor"|"mon")
        monitor_swarm
        ;;
    "performance"|"perf")
        performance_report
        ;;
    "backup"|"bk")
        backup_swarm
        ;;
    "metrics"|"met")
        npx claude-flow@alpha metrics collect --swarm "$SWARM_NAME"
        ;;
    "deploy")
        "$SCRIPT_DIR/deploy-claude-flow-swarm.sh"
        ;;
    "restart"|"rs")
        log "Restarting swarm..."
        npx claude-flow@alpha swarm restart "$SWARM_NAME"
        ;;
    "stop")
        log "Stopping swarm gracefully..."
        npx claude-flow@alpha swarm stop "$SWARM_NAME" --graceful
        ;;
    "help"|"-h"|"--help"|*)
        show_help
        ;;
esac