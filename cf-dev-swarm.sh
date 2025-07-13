#!/bin/bash

# Claude-Flow Development Swarm Management Script
# Specialized for claude-flow repository development

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
HIVE_DIR="$SCRIPT_DIR/.hive-mind"

# Functions
print_header() {
    echo -e "\n${BOLD}${BLUE}🔧 Claude-Flow Development Swarm Manager${NC}"
    echo -e "${CYAN}Specialized for claude-flow repository enhancement${NC}\n"
}

print_usage() {
    echo -e "${YELLOW}Usage:${NC} $0 <command> [options]"
    echo -e "\n${YELLOW}Commands:${NC}"
    echo -e "  ${GREEN}create-core${NC}    - Create swarm with 5 core specialists (TypeScript, Rust, Hive-Mind, MCP, Testing)"
    echo -e "  ${GREEN}create-full${NC}    - Create swarm with all 10 specialists"
    echo -e "  ${GREEN}create-debug${NC}   - Create debugging swarm (TypeScript, Testing, Performance, Advanced)"
    echo -e "  ${GREEN}create-docs${NC}    - Create documentation swarm (Docs, Advanced Usage)"
    echo -e "  ${GREEN}create-infra${NC}   - Create infrastructure swarm (Docker, DevOps, Performance)"
    echo -e "  ${GREEN}status${NC}         - Show current swarm status"
    echo -e "  ${GREEN}activate${NC}       - Generate Claude Code activation commands"
    echo -e "  ${GREEN}list${NC}           - List all available specialists"
    echo -e "  ${GREEN}clean${NC}          - Remove all swarms and reset database"
    echo -e "\n${YELLOW}Examples:${NC}"
    echo -e "  $0 create-core \"Fix hive-mind worker limitation bug\""
    echo -e "  $0 create-full \"Implement new MCP tools for claude-flow\""
    echo -e "  $0 create-debug \"Debug TypeScript CLI performance issues\""
}

create_swarm() {
    local preset=$1
    local objective=$2
    
    if [ -z "$objective" ]; then
        objective="Claude-Flow Repository Development and Enhancement"
    fi
    
    echo -e "${YELLOW}🚀 Creating Claude-Flow development swarm...${NC}"
    echo -e "${CYAN}Preset: ${preset}${NC}"
    echo -e "${CYAN}Objective: ${objective}${NC}"
    
    cd "$HIVE_DIR"
    node spawn-cf-specialists.js "$preset" "$objective"
}

show_status() {
    echo -e "${YELLOW}📊 Claude-Flow Development Swarm Status${NC}\n"
    npx claude-flow@alpha hive-mind status
}

activate_swarm() {
    echo -e "${YELLOW}🎯 Claude Code Activation Commands${NC}\n"
    
    # Get latest swarm ID
    local swarm_info=$(npx claude-flow@alpha hive-mind status | grep -A1 "Swarm:" | tail -1)
    local swarm_id=$(echo "$swarm_info" | grep -oP 'swarm-\d+-\w+' | head -1)
    
    if [ -z "$swarm_id" ]; then
        echo -e "${RED}❌ No active swarm found. Create one first with:${NC}"
        echo -e "${GREEN}$0 create-core${NC}"
        return 1
    fi
    
    echo -e "${GREEN}✅ Found swarm: ${swarm_id}${NC}\n"
    
    echo -e "${BOLD}📋 Copy and run these commands in Claude Code:${NC}\n"
    
    cat << 'EOF'
# Initialize Claude-Flow development swarm coordination
mcp__claude-flow__swarm_init { topology: "mesh", maxAgents: 10, strategy: "parallel" }

# Store swarm configuration
mcp__claude-flow__memory_usage { 
  action: "store", 
  key: "cf-dev/swarm-config", 
  value: {
    purpose: "claude-flow-development",
    specialists: ["typescript", "rust", "hive-mind", "mcp", "testing", "docs", "performance", "devops", "docker", "advanced"],
    focus: "repository enhancement and bug fixes"
  }
}

# Create specialized task for claude-flow development
Task("You are the TypeScript Core specialist. Focus on CLI and MCP server code in src/. MANDATORY: Use claude-flow@alpha hooks for coordination.")
Task("You are the Rust WASM specialist. Focus on neural models and performance in wasm/. MANDATORY: Use claude-flow@alpha hooks for coordination.")
Task("You are the Hive-Mind architect. Focus on swarm orchestration in src/cli/simple-commands/hive-mind.js. MANDATORY: Use claude-flow@alpha hooks.")
Task("You are the Testing/QA engineer. Focus on test coverage and bug fixes. MANDATORY: Use claude-flow@alpha hooks for coordination.")
Task("You are the Documentation specialist. Update README and docs/. MANDATORY: Use claude-flow@alpha hooks for coordination.")

# Create comprehensive todo list for tracking
TodoWrite { todos: [
  { id: "analyze", content: "Analyze current claude-flow architecture", status: "pending", priority: "high" },
  { id: "bugs", content: "Identify and fix critical bugs", status: "pending", priority: "high" },
  { id: "tests", content: "Improve test coverage to 80%+", status: "pending", priority: "high" },
  { id: "perf", content: "Optimize WASM neural performance", status: "pending", priority: "medium" },
  { id: "docs", content: "Update documentation and examples", status: "pending", priority: "medium" },
  { id: "mcp", content: "Add new MCP tool integrations", status: "pending", priority: "medium" },
  { id: "docker", content: "Improve Docker containerization", status: "pending", priority: "low" },
  { id: "ci", content: "Enhance GitHub Actions workflows", status: "pending", priority: "low" }
]}

# Monitor swarm performance
mcp__claude-flow__swarm_monitor { swarmId: "current", interval: 5000 }

# Ready for claude-flow development!
EOF
    
    echo -e "\n${MAGENTA}💡 Pro tip: Save this as a Claude Code template for quick reuse${NC}"
}

list_specialists() {
    echo -e "${YELLOW}📋 Available Claude-Flow Development Specialists${NC}\n"
    
    cat << EOF
${BOLD}Core Specialists (5):${NC}
  ${GREEN}1.${NC}  TypeScript Core  - CLI, MCP server, Web UI
  ${GREEN}2.${NC}  Rust WASM       - Neural models, performance
  ${GREEN}3.${NC}  Hive-Mind       - Swarm orchestration
  ${GREEN}4.${NC}  MCP Protocol    - Tool integration
  ${GREEN}5.${NC}  Testing/QA      - Jest, benchmarking

${BOLD}Additional Specialists (5):${NC}
  ${GREEN}6.${NC}  Documentation   - README, API docs
  ${GREEN}7.${NC}  Performance     - Optimization, profiling
  ${GREEN}8.${NC}  DevOps/CI       - GitHub Actions, npm
  ${GREEN}9.${NC}  Docker/K8s      - Containerization
  ${GREEN}10.${NC} Advanced Usage  - Best practices

${BOLD}Preset Configurations:${NC}
  ${CYAN}core${NC}   - Essential 5 for core development
  ${CYAN}full${NC}   - All 10 specialists
  ${CYAN}debug${NC}  - TypeScript, Testing, Performance, Advanced
  ${CYAN}docs${NC}   - Documentation, Advanced Usage
  ${CYAN}infra${NC}  - Docker, DevOps, Performance
EOF
}

clean_swarms() {
    echo -e "${YELLOW}🧹 Cleaning all swarms...${NC}"
    
    if [ -f "$HIVE_DIR/hive.db" ]; then
        rm -f "$HIVE_DIR/hive.db"
        echo -e "${GREEN}✅ Database cleaned${NC}"
    fi
    
    # Reinitialize
    npx claude-flow@alpha hive-mind init
    echo -e "${GREEN}✅ Hive-mind reinitialized${NC}"
}

# Main script
print_header

case "$1" in
    create-core)
        create_swarm "core" "$2"
        ;;
    create-full)
        create_swarm "full" "$2"
        ;;
    create-debug)
        create_swarm "debug" "$2"
        ;;
    create-docs)
        create_swarm "docs" "$2"
        ;;
    create-infra)
        create_swarm "infra" "$2"
        ;;
    status)
        show_status
        ;;
    activate)
        activate_swarm
        ;;
    list)
        list_specialists
        ;;
    clean)
        clean_swarms
        ;;
    *)
        print_usage
        exit 1
        ;;
esac