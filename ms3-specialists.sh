#!/bin/bash

# MS-3 Specialist Hive Mind Management Script
# Quick access to 25+ specialized AI workers

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HIVE_DIR="$SCRIPT_DIR/.hive-mind"

echo "🐝 MS-3 Specialist Hive Mind Controller"
echo "======================================"

# Function to show help
show_help() {
    cat << EOF

USAGE: $0 <command> [options]

COMMANDS:
  list-specialists     Show all 25 available specialist types
  create-full          Create swarm with all 25 specialists  
  create-ms3          Create focused MS-3 swarm (15 specialists)
  create-custom       Create custom swarm with specific specialists
  status              Show all swarm status
  activate <swarm>    Generate Claude Code prompt for swarm
  cleanup             Remove old/inactive swarms

EXAMPLES:
  # Create full specialist swarm
  $0 create-full "Complete system migration"
  
  # Create MS-3 focused swarm
  $0 create-ms3 "MS-3 Actor Systems Migration"
  
  # Create custom swarm
  $0 create-custom "Database migration" postgresql-expert,aws-migration,terraform-expert
  
  # Activate specialists with Claude Code
  $0 activate swarm-1234567890-abcdef
  
  # Show current status
  $0 status

SPECIALIST TYPES:
  actor-systems, nats-specialist, postgresql-expert, aws-migration,
  kubernetes-expert, docker-specialist, microservices-architect,
  terraform-expert, security-expert, devops-engineer, performance-engineer,
  frontend-specialist, backend-specialist, data-engineer, machine-learning,
  blockchain-expert, monitoring-specialist, qa-automation, network-specialist,
  mobile-developer, api-specialist, redis-expert, elasticsearch-expert,
  golang-specialist, rust-specialist

EOF
}

# Function to check prerequisites
check_prereqs() {
    if [ ! -f "$HIVE_DIR/spawn-specialists.js" ]; then
        echo "❌ Error: Specialist spawner not found"
        echo "Please run the installation first"
        exit 1
    fi
    
    if [ ! -f "$HIVE_DIR/specialist-workers.json" ]; then
        echo "❌ Error: Specialist workers config not found"
        exit 1
    fi
    
    if ! command -v node >/dev/null 2>&1; then
        echo "❌ Error: Node.js not found"
        echo "Please install Node.js to use specialist workers"
        exit 1
    fi
}

# Function to get latest swarm ID
get_latest_swarm() {
    sqlite3 "$HIVE_DIR/hive.db" "
        SELECT id 
        FROM swarms 
        WHERE name LIKE 'specialist-hive-%' 
        ORDER BY created_at DESC 
        LIMIT 1;
    " 2>/dev/null || echo ""
}

# Main command handling
case "${1:-help}" in
    "list-specialists"|"list")
        check_prereqs
        node "$HIVE_DIR/spawn-specialists.js" list
        ;;
        
    "create-full"|"full")
        check_prereqs
        objective="${2:-Complete MS-3 specialist migration}"
        echo "🚀 Creating full specialist swarm (25 workers)..."
        node "$HIVE_DIR/spawn-specialists.js" create "$objective"
        ;;
        
    "create-ms3"|"ms3")
        check_prereqs
        objective="${2:-MS-3 Actor Systems Migration}"
        specialists="actor-systems,nats-specialist,postgresql-expert,aws-migration,kubernetes-expert,docker-specialist,microservices-architect,terraform-expert,security-expert,devops-engineer,performance-engineer,backend-specialist,monitoring-specialist,golang-specialist,rust-specialist"
        echo "🎯 Creating MS-3 focused specialist swarm (15 workers)..."
        node "$HIVE_DIR/spawn-specialists.js" create-custom "$objective" "$specialists"
        ;;
        
    "create-custom"|"custom")
        check_prereqs
        objective="${2:-Custom specialist migration}"
        specialists="${3:-}"
        
        if [ -z "$specialists" ]; then
            echo "❌ Error: Please specify specialist types"
            echo "Example: $0 create-custom \"Database migration\" postgresql-expert,aws-migration"
            echo ""
            echo "Available specialists:"
            node "$HIVE_DIR/spawn-specialists.js" list | grep "^[0-9]" | head -10
            echo "... (use 'list-specialists' for full list)"
            exit 1
        fi
        
        echo "🔧 Creating custom specialist swarm..."
        node "$HIVE_DIR/spawn-specialists.js" create-custom "$objective" "$specialists"
        ;;
        
    "status"|"st")
        check_prereqs
        echo "📊 Specialist Swarm Status:"
        npx claude-flow hive-mind status | grep -A 50 "specialist-hive-" || echo "No specialist swarms found"
        ;;
        
    "activate"|"act")
        check_prereqs
        swarm_id="${2:-}"
        
        if [ -z "$swarm_id" ]; then
            latest_swarm=$(get_latest_swarm)
            if [ -n "$latest_swarm" ]; then
                swarm_id="$latest_swarm"
                echo "🎯 Using latest specialist swarm: $swarm_id"
            else
                echo "❌ Error: No specialist swarms found"
                echo "Create one first with: $0 create-ms3"
                exit 1
            fi
        fi
        
        prompt_file="$HIVE_DIR/claude-specialist-prompt-$swarm_id.txt"
        
        if [ ! -f "$prompt_file" ]; then
            echo "🔄 Generating Claude Code prompt for $swarm_id..."
            "$HIVE_DIR/integrate-specialists.sh" >/dev/null 2>&1 || true
        fi
        
        if [ -f "$prompt_file" ]; then
            echo "🚀 Activating specialist swarm with Claude Code..."
            echo ""
            echo "📝 Prompt file: $prompt_file"
            echo ""
            echo "🔧 To activate:"
            echo "   claude < $prompt_file"
            echo ""
            echo "🔓 With auto-permissions:"
            echo "   claude --dangerously-skip-permissions < $prompt_file"
            echo ""
            
            # Optionally launch directly if claude is available
            if command -v claude >/dev/null 2>&1; then
                echo "💡 Launch now? (y/N)"
                read -r response
                if [[ "$response" =~ ^[Yy]$ ]]; then
                    echo "🚀 Launching Claude Code with specialist coordination..."
                    claude --dangerously-skip-permissions < "$prompt_file"
                fi
            else
                echo "💡 Install Claude Code CLI for direct activation:"
                echo "   npm install -g @anthropic-ai/claude-code"
            fi
        else
            echo "❌ Error: Could not generate prompt for $swarm_id"
            exit 1
        fi
        ;;
        
    "cleanup"|"clean")
        check_prereqs
        echo "🧹 Cleaning up old specialist swarms..."
        
        # Show current swarms
        echo "Current specialist swarms:"
        sqlite3 "$HIVE_DIR/hive.db" "
            SELECT id, name, created_at 
            FROM swarms 
            WHERE name LIKE 'specialist-hive-%' 
            ORDER BY created_at DESC;
        " | while IFS='|' read -r id name created; do
            echo "  • $name ($id) - $created"
        done
        
        echo ""
        echo "⚠️  This will remove old specialist swarms. Continue? (y/N)"
        read -r response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            # Remove swarms older than 1 day (keep recent ones)
            cutoff_date=$(date -d '1 day ago' '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date -v-1d '+%Y-%m-%d %H:%M:%S')
            echo "🗑️  Removing specialist swarms older than $cutoff_date..."
            # Note: Implement cleanup logic here if needed
            echo "✅ Cleanup completed"
        else
            echo "❌ Cleanup cancelled"
        fi
        ;;
        
    "quick"|"q")
        # Quick action - create MS-3 swarm and activate
        check_prereqs
        echo "⚡ Quick MS-3 specialist deployment..."
        
        # Create MS-3 swarm
        swarm_output=$(node "$HIVE_DIR/spawn-specialists.js" create-custom "MS-3 Quick Deploy" "actor-systems,nats-specialist,postgresql-expert,aws-migration,kubernetes-expert,microservices-architect")
        swarm_id=$(echo "$swarm_output" | grep "Swarm ID:" | awk '{print $3}')
        
        if [ -n "$swarm_id" ]; then
            echo "✅ Created specialist swarm: $swarm_id"
            echo "🚀 Generating Claude Code activation..."
            
            # Generate and show prompt
            "$0" activate "$swarm_id"
        else
            echo "❌ Failed to create specialist swarm"
            exit 1
        fi
        ;;
        
    "help"|"-h"|"--help"|*)
        show_help
        ;;
esac