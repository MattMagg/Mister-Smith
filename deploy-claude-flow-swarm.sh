#!/bin/bash

# Claude Flow Development Swarm Deployment Script
# Phase 3 Master Integration - Production Deployment System
# Created by: Swarm Master Configuration & Integration Coordinator

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/claude-flow-development-swarm.json"
SWARM_NAME="claude-flow-dev-master"
LOG_FILE="$SCRIPT_DIR/deployment.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

# Prerequisites check
check_prerequisites() {
    log "Checking deployment prerequisites..."
    
    # Check Node.js version
    if ! command -v node >/dev/null 2>&1; then
        error "Node.js not found. Please install Node.js >= 18.0.0"
        exit 1
    fi
    
    local node_version=$(node --version | cut -d'v' -f2)
    if ! node -e "process.exit(process.version.split('.')[0].substring(1) >= 18 ? 0 : 1)"; then
        error "Node.js version $node_version is too old. Minimum required: 18.0.0"
        exit 1
    fi
    
    # Check Claude Flow
    if ! command -v npx >/dev/null 2>&1; then
        error "npx not found. Please install npm/npx"
        exit 1
    fi
    
    # Check configuration file
    if [ ! -f "$CONFIG_FILE" ]; then
        error "Configuration file not found: $CONFIG_FILE"
        exit 1
    fi
    
    # Check available memory
    if command -v free >/dev/null 2>&1; then
        local available_mem=$(free -m | awk 'NR==2{printf "%.1f", $7/1024}')
        if (( $(echo "$available_mem < 2.0" | bc -l) )); then
            warning "Available memory ($available_mem GB) is below recommended minimum (2GB)"
        fi
    fi
    
    success "Prerequisites check completed"
}

# Initialize Claude Flow MCP
initialize_claude_flow() {
    log "Initializing Claude Flow MCP system..."
    
    # Install/update claude-flow
    npx claude-flow@alpha --version || {
        log "Installing Claude Flow..."
        npm install -g claude-flow@alpha
    }
    
    # Initialize swarm with configuration
    log "Initializing hierarchical-mesh hybrid swarm..."
    npx claude-flow@alpha swarm init \
        --topology hierarchical \
        --max-agents 30 \
        --name "$SWARM_NAME" \
        --auto-scaling true \
        --fault-tolerance circuit-breaker
    
    success "Claude Flow MCP initialized"
}

# Deploy coordination agents
deploy_coordination_agents() {
    log "Deploying coordination agents..."
    
    # Deploy Queen Coordinator
    log "Spawning Queen Coordinator (adaptive leadership)..."
    npx claude-flow@alpha agent spawn \
        --type coordinator \
        --name "Queen Coordinator" \
        --capabilities "orchestration,strategic_planning,resource_allocation,consensus_building" \
        --neural-model "enhanced_ensemble_v6" \
        --authority executive
    
    # Deploy Swarm Orchestrator  
    log "Spawning Swarm Orchestrator (topology optimization)..."
    npx claude-flow@alpha agent spawn \
        --type orchestrator \
        --name "Swarm Orchestrator" \
        --capabilities "topology_optimization,load_balancing,agent_lifecycle,performance_monitoring" \
        --neural-model "coordination_v5"
    
    success "Coordination agents deployed"
}

# Deploy specialist agents in parallel
deploy_specialist_agents() {
    log "Deploying specialist agents in parallel batches..."
    
    # Core Development Team (7 agents)
    log "Deploying core development team..."
    npx claude-flow@alpha agent spawn --batch \
        --type architect --count 2 --capabilities "system_design,architecture_patterns,scalability_planning" \
        --type coder --count 3 --capabilities "implementation,code_generation,refactoring,optimization" \
        --type backend_specialist --count 2 --capabilities "api_design,database_optimization,performance_tuning"
    
    # Infrastructure Team (6 agents)
    log "Deploying infrastructure specialists..."
    npx claude-flow@alpha agent spawn --batch \
        --type aws_migration_expert --count 2 --capabilities "cloud_architecture,migration_planning,cost_optimization" \
        --type kubernetes_expert --count 1 --capabilities "container_orchestration,helm,operators,networking" \
        --type terraform_expert --count 1 --capabilities "infrastructure_as_code,state_management,modules" \
        --type devops_engineer --count 2 --capabilities "ci_cd,automation,monitoring,deployment"
    
    # Quality Team (3 agents)
    log "Deploying quality specialists..."
    npx claude-flow@alpha agent spawn --batch \
        --type security_expert --count 1 --capabilities "security_audits,vulnerability_assessment,compliance" \
        --type performance_engineer --count 1 --capabilities "load_testing,optimization,profiling,bottleneck_analysis" \
        --type qa_automation --count 1 --capabilities "test_automation,integration_testing,quality_assurance"
    
    # Domain Specialists (5 agents)
    log "Deploying domain specialists..."
    npx claude-flow@alpha agent spawn --batch \
        --type actor_systems_expert --count 2 --capabilities "actor_model,supervision_trees,fault_tolerance" \
        --type nats_specialist --count 1 --capabilities "message_streaming,jetstream,event_sourcing" \
        --type postgresql_expert --count 1 --capabilities "query_optimization,indexing,replication" \
        --type monitoring_specialist --count 1 --capabilities "observability,metrics,alerting,dashboards"
    
    success "Specialist agents deployed (23 total)"
}

# Configure memory and coordination
configure_coordination() {
    log "Configuring coordination protocols..."
    
    # Setup memory namespaces
    npx claude-flow@alpha memory namespace create claude-flow-dev
    npx claude-flow@alpha memory namespace create claude-flow-dev/coordination
    npx claude-flow@alpha memory namespace create claude-flow-dev/agents
    npx claude-flow@alpha memory namespace create claude-flow-dev/neural
    
    # Configure batch operation protocols
    npx claude-flow@alpha memory store claude-flow-dev/protocols/batch-operations \
        '{"mandatory": true, "patterns": ["parallel_task_spawning", "concurrent_file_operations", "batch_memory_operations"]}'
    
    # Setup neural training
    log "Initializing neural training systems..."
    npx claude-flow@alpha neural train \
        --pattern-type coordination \
        --continuous true \
        --ensemble-mode true
    
    # Configure fault tolerance
    npx claude-flow@alpha swarm configure \
        --circuit-breaker true \
        --health-check-interval 30 \
        --auto-recovery true
    
    success "Coordination protocols configured"
}

# Verification and health check
verify_deployment() {
    log "Verifying deployment..."
    
    # Check swarm status
    local swarm_status=$(npx claude-flow@alpha swarm status --json)
    local agent_count=$(echo "$swarm_status" | jq '.agents | length')
    
    if [ "$agent_count" -ge 25 ]; then
        success "Agent deployment verified: $agent_count agents active"
    else
        error "Insufficient agents deployed: $agent_count (expected: 25+)"
        return 1
    fi
    
    # Check coordination protocols
    npx claude-flow@alpha memory retrieve claude-flow-dev/protocols/batch-operations >/dev/null 2>&1 || {
        error "Coordination protocols not properly configured"
        return 1
    }
    
    # Check neural training
    local neural_status=$(npx claude-flow@alpha neural status --json)
    if echo "$neural_status" | jq -e '.training_active' >/dev/null; then
        success "Neural training systems active"
    else
        warning "Neural training not fully initialized"
    fi
    
    # Performance validation
    log "Running performance validation..."
    npx claude-flow@alpha benchmark run --suite coordination
    
    success "Deployment verification completed"
}

# Generate deployment summary
generate_summary() {
    log "Generating deployment summary..."
    
    local summary_file="$SCRIPT_DIR/deployment-summary-$(date +%Y%m%d-%H%M%S).json"
    
    cat > "$summary_file" << EOF
{
  "deployment": {
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "swarm_name": "$SWARM_NAME",
    "configuration": "$CONFIG_FILE",
    "status": "deployed",
    "verification": "passed"
  },
  "agents": {
    "coordination": 2,
    "core_development": 7,
    "infrastructure": 6,
    "quality": 3,
    "domain_specialists": 5,
    "total": 23
  },
  "capabilities": [
    "hierarchical_mesh_coordination",
    "parallel_batch_operations",
    "neural_enhanced_learning",
    "fault_tolerant_recovery",
    "continuous_optimization"
  ],
  "management": {
    "monitor_command": "./manage-claude-flow-swarm.sh status",
    "scale_command": "./manage-claude-flow-swarm.sh scale",
    "backup_command": "./manage-claude-flow-swarm.sh backup"
  }
}
EOF
    
    success "Deployment summary: $summary_file"
}

# Main deployment function
main() {
    log "Starting Claude Flow Development Swarm Deployment"
    log "Configuration: $CONFIG_FILE"
    log "Target: $SWARM_NAME"
    
    check_prerequisites
    initialize_claude_flow
    deploy_coordination_agents
    deploy_specialist_agents
    configure_coordination
    verify_deployment
    generate_summary
    
    success "Claude Flow Development Swarm deployed successfully!"
    
    echo ""
    echo "🎉 Deployment Complete!"
    echo "📊 Management: ./manage-claude-flow-swarm.sh"
    echo "📋 Status: npx claude-flow@alpha swarm status"
    echo "🧠 Neural: npx claude-flow@alpha neural status"
    echo "💾 Memory: npx claude-flow@alpha memory list claude-flow-dev"
    echo ""
    echo "Ready for development coordination!"
}

# Error handling
trap 'error "Deployment failed at line $LINENO. Check $LOG_FILE for details."; exit 1' ERR

# Run main function
main "$@"