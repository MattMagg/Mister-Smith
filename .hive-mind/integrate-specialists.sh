#!/bin/bash

# Hive Mind Specialist Integration Script
# This script integrates the specialist workers with the existing MS-3 system

echo "🐝 Hive Mind Specialist Integration for MS-3"
echo "=============================================="

# Function to display specialist swarms
show_specialist_swarms() {
    echo ""
    echo "📊 Current Specialist Swarms:"
    sqlite3 .hive-mind/hive.db "
        SELECT 
            s.id,
            s.name,
            s.objective,
            COUNT(a.id) as agent_count,
            s.created_at
        FROM swarms s
        LEFT JOIN agents a ON s.id = a.swarm_id
        WHERE s.name LIKE 'specialist-hive-%'
        GROUP BY s.id
        ORDER BY s.created_at DESC;
    " | while IFS='|' read -r id name objective agents created; do
        echo "  🎯 $name"
        echo "     ID: $id"
        echo "     Objective: $objective"
        echo "     Agents: $agents"
        echo "     Created: $created"
        echo ""
    done
}

# Function to show worker capabilities
show_worker_capabilities() {
    local swarm_id="$1"
    echo "🔬 Specialist Worker Capabilities for $swarm_id:"
    sqlite3 .hive-mind/hive.db "
        SELECT 
            name,
            type,
            capabilities
        FROM agents 
        WHERE swarm_id = '$swarm_id' AND role = 'worker'
        ORDER BY type;
    " | while IFS='|' read -r name type capabilities; do
        echo "  • $name ($type)"
        echo "    Capabilities: $capabilities"
        echo ""
    done
}

# Function to generate Claude Code coordination prompt
generate_claude_coordination() {
    local swarm_id="$1"
    local prompt_file=".hive-mind/claude-specialist-prompt-$swarm_id.txt"
    
    echo "📝 Generating Claude Code coordination prompt..."
    
    # Get swarm info
    local swarm_info=$(sqlite3 .hive-mind/hive.db "SELECT name, objective FROM swarms WHERE id = '$swarm_id';")
    local swarm_name=$(echo "$swarm_info" | cut -d'|' -f1)
    local objective=$(echo "$swarm_info" | cut -d'|' -f2)
    
    # Get specialist workers
    local specialists=$(sqlite3 .hive-mind/hive.db "
        SELECT type, name, capabilities 
        FROM agents 
        WHERE swarm_id = '$swarm_id' AND role = 'worker' 
        ORDER BY type;
    ")
    
    cat > "$prompt_file" << EOF
🧠 SPECIALIST HIVE MIND COORDINATION
═══════════════════════════════════

You are the Queen Coordinator of a specialized AI swarm with 25+ domain experts.

🎯 MISSION: $objective
🐝 SWARM: $swarm_name
🆔 SWARM ID: $swarm_id

🔬 AVAILABLE SPECIALIST WORKERS:

EOF

    # Add each specialist to the prompt
    echo "$specialists" | while IFS='|' read -r type name capabilities; do
        cat >> "$prompt_file" << EOF
• $name ($type)
  Capabilities: $capabilities

EOF
    done
    
    cat >> "$prompt_file" << EOF

🚀 COORDINATION PROTOCOL:

1. **IMMEDIATE PARALLEL INITIALIZATION** (Single BatchTool Message):
   [BatchTool]:
   mcp__claude-flow__swarm_init { "topology": "hierarchical", "swarmId": "$swarm_id", "maxAgents": 25 }
   mcp__claude-flow__memory_usage { "action": "store", "key": "specialist-swarm/$swarm_id/objective", "value": "$objective" }
   mcp__claude-flow__memory_usage { "action": "store", "key": "specialist-swarm/$swarm_id/status", "value": "initialized" }

2. **SPECIALIST TASK ASSIGNMENT**:
   - Actor Systems Expert → Actor model migration, fault tolerance
   - NATS Specialist → Message streaming, event sourcing
   - PostgreSQL Expert → Database optimization, migration
   - AWS Migration Expert → Cloud architecture, cost optimization
   - Kubernetes Expert → Container orchestration, scaling
   - Security Expert → Security audits, compliance
   - Performance Engineer → System optimization, load testing

3. **COLLABORATIVE INTELLIGENCE**:
   - Each specialist uses mcp__claude-flow__memory_usage to share findings
   - Critical decisions require mcp__claude-flow__consensus_vote
   - Progress tracked via mcp__claude-flow__task_orchestrate
   - Neural learning via mcp__claude-flow__neural_train

4. **MS-3 SPECIFIC TASKS**:
   - Migrate MisterSmith agent system to cloud-native architecture
   - Implement Actor model with NATS messaging
   - Optimize PostgreSQL for high-throughput workloads
   - Design AWS infrastructure for auto-scaling
   - Ensure security and compliance standards

⚡ EXECUTION RULES:
✅ ALWAYS batch operations in single messages
✅ Store ALL decisions in specialist memory namespace
✅ Use consensus for architecture decisions
✅ Coordinate between related specialists
✅ Learn from each migration step

Remember: You're coordinating $swarm_name specialists working on: $objective

BEGIN SPECIALIST COORDINATION NOW!
EOF

    echo "✅ Claude coordination prompt generated: $prompt_file"
    echo ""
    echo "🚀 To activate specialists with Claude Code:"
    echo "   claude < $prompt_file"
    echo ""
    echo "🔧 Or with auto-permissions:"
    echo "   claude --dangerously-skip-permissions < $prompt_file"
}

# Function to update existing hive-mind configuration
update_hive_config() {
    echo "🔧 Updating hive-mind configuration to support specialists..."
    
    # Backup existing config
    if [ -f ".hive-mind/config.json" ]; then
        cp ".hive-mind/config.json" ".hive-mind/config.json.backup"
        echo "✅ Backed up existing config to config.json.backup"
    fi
    
    # Update config to support specialists
    cat > ".hive-mind/config.json" << EOF
{
  "version": "2.0.0-specialist",
  "initialized": "$(date -u +"%Y-%m-%dT%H:%M:%S.%3NZ")",
  "defaults": {
    "queenType": "adaptive",
    "maxWorkers": 25,
    "consensusAlgorithm": "weighted",
    "memorySize": 500,
    "autoScale": true,
    "encryption": false,
    "specialistMode": true
  },
  "specialists": {
    "enabled": true,
    "configFile": "specialist-workers.json",
    "maxSpecialists": 25,
    "autoAssignment": true
  },
  "mcpTools": {
    "enabled": true,
    "parallel": true,
    "timeout": 60000,
    "batchOperations": true
  },
  "coordination": {
    "memory_namespace": "specialist-swarm",
    "consensus_threshold": 0.7,
    "neural_learning": true,
    "performance_tracking": true
  }
}
EOF
    
    echo "✅ Updated hive-mind configuration for specialist support"
}

# Main execution
echo ""
echo "🎯 Checking for specialist swarms..."
show_specialist_swarms

echo ""
echo "🤖 Available Actions:"
echo "1. Show specialist swarm capabilities"
echo "2. Generate Claude Code coordination prompt"
echo "3. Update hive-mind configuration"
echo "4. Create new specialist swarm"
echo ""

# Get the latest specialist swarm ID
LATEST_SWARM=$(sqlite3 .hive-mind/hive.db "
    SELECT id 
    FROM swarms 
    WHERE name LIKE 'specialist-hive-%' 
    ORDER BY created_at DESC 
    LIMIT 1;
")

if [ -n "$LATEST_SWARM" ]; then
    echo "🎯 Latest specialist swarm: $LATEST_SWARM"
    echo ""
    echo "Choose action (1-4) or press Enter to generate coordination for latest swarm:"
    read -r action
    
    case $action in
        1)
            show_worker_capabilities "$LATEST_SWARM"
            ;;
        2|"")
            generate_claude_coordination "$LATEST_SWARM"
            ;;
        3)
            update_hive_config
            ;;
        4)
            echo "Creating focused MS-3 specialist swarm..."
            node .hive-mind/spawn-specialists.js create-custom "MS-3 Actor Systems Migration" "actor-systems,nats-specialist,postgresql-expert,aws-migration,kubernetes-expert,microservices-architect,security-expert,performance-engineer"
            ;;
        *)
            echo "Invalid option. Generating coordination prompt for latest swarm..."
            generate_claude_coordination "$LATEST_SWARM"
            ;;
    esac
else
    echo "❌ No specialist swarms found. Creating one now..."
    node .hive-mind/spawn-specialists.js create "MS-3 Complete Specialist Migration"
fi

echo ""
echo "🎉 Specialist integration complete!"
echo ""
echo "📋 Next Steps:"
echo "1. Use the generated Claude coordination prompt"
echo "2. Monitor progress with: claude-flow hive-mind status"
echo "3. Check specialist capabilities with this script"
echo "4. Scale specialists as needed"