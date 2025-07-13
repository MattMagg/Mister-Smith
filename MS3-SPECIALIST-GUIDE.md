# MS-3 Specialist Hive Mind Configuration Guide

## 🎯 Problem Solved

The default hive-mind system was limited to only 4 basic worker types (`researcher`, `coder`, `analyst`, `tester`), regardless of the `--max-workers` setting. This guide provides a complete technical solution to configure **25+ specialized workers** for the MS-3 Actor Systems migration project.

## ✅ Technical Solution Implemented

### Files Created:
1. **`.hive-mind/specialist-workers.json`** - Configuration for 25 specialist types
2. **`.hive-mind/spawn-specialists.js`** - Bypasses hardcoded limitations  
3. **`.hive-mind/integrate-specialists.sh`** - Integration script
4. **`ms3-specialists.sh`** - Convenient management interface

### Root Cause Analysis:
- **Hardcoded Types**: `/node_modules/claude-flow/src/cli/simple-commands/hive-mind.js` lines 579-581 default to only 4 types
- **Limited Capabilities**: Only 8 basic agent types defined in capabilities mapping
- **Auto-spawn Logic**: HiveMind.ts auto-spawn only creates basic topologies

## 🚀 Quick Start

### 1. Create MS-3 Focused Specialist Swarm (15 specialists)
```bash
./ms3-specialists.sh create-ms3 "MS-3 Actor Systems Migration"
```

### 2. Create Full Specialist Swarm (25 specialists)  
```bash
./ms3-specialists.sh create-full "Complete MS-3 Migration"
```

### 3. Activate Specialists with Claude Code
```bash
./ms3-specialists.sh activate
```

### 4. Monitor Progress
```bash
./ms3-specialists.sh status
npx claude-flow hive-mind status
```

## 🔬 Available Specialist Types (25 Total)

### Core MS-3 Specialists:
- **`actor-systems`** - Actor model, concurrency, fault tolerance
- **`nats-specialist`** - NATS streaming, JetStream, messaging  
- **`postgresql-expert`** - Database optimization, migration
- **`aws-migration`** - Cloud architecture, cost optimization
- **`kubernetes-expert`** - Container orchestration, scaling
- **`microservices-architect`** - Service mesh, API gateway
- **`security-expert`** - Security audits, compliance
- **`performance-engineer`** - Load testing, optimization

### Infrastructure Specialists:
- **`terraform-expert`** - Infrastructure as code
- **`docker-specialist`** - Containerization, optimization
- **`devops-engineer`** - CI/CD, automation
- **`monitoring-specialist`** - Prometheus, Grafana, observability
- **`network-specialist`** - VPN, DNS, load balancing

### Development Specialists:
- **`backend-specialist`** - Node.js, API design
- **`frontend-specialist`** - React, TypeScript, UI/UX
- **`golang-specialist`** - Go concurrency, microservices
- **`rust-specialist`** - Systems programming, performance
- **`api-specialist`** - REST, GraphQL, documentation

### Data & Analytics:
- **`data-engineer`** - ETL pipelines, big data
- **`machine-learning`** - AI/ML, model training
- **`redis-expert`** - Caching, session management
- **`elasticsearch-expert`** - Search, indexing

### Additional Specialists:
- **`qa-automation`** - Test automation, quality assurance  
- **`mobile-developer`** - React Native, Flutter
- **`blockchain-expert`** - Smart contracts, Web3

## 📊 Database Verification

```bash
# Check specialist worker count
sqlite3 .hive-mind/hive.db "SELECT COUNT(*) FROM agents WHERE role='worker';"

# List all specialist types  
sqlite3 .hive-mind/hive.db "SELECT DISTINCT type FROM agents WHERE role='worker';"

# Show specialist capabilities
sqlite3 .hive-mind/hive.db "SELECT name, type, capabilities FROM agents WHERE role='worker' ORDER BY type;"
```

## 🧠 Claude Code Integration

### Generated Coordination Prompt Includes:
- **Queen Coordinator** with adaptive leadership
- **25 Domain Specialists** with specific capabilities
- **MCP Tool Integration** for coordination
- **Parallel Execution** protocols
- **Collective Intelligence** mechanisms

### Activation Commands:
```bash
# Basic activation
claude < .hive-mind/claude-specialist-prompt-[swarm-id].txt

# With auto-permissions  
claude --dangerously-skip-permissions < .hive-mind/claude-specialist-prompt-[swarm-id].txt
```

## 🔧 Advanced Usage

### Custom Specialist Selection:
```bash
# Create targeted swarm with specific specialists
./ms3-specialists.sh create-custom "Database Migration" "postgresql-expert,aws-migration,terraform-expert"
```

### Direct Node.js Interface:
```bash
# List all specialists
node .hive-mind/spawn-specialists.js list

# Create full swarm
node .hive-mind/spawn-specialists.js create "Objective"

# Create custom swarm
node .hive-mind/spawn-specialists.js create-custom "Objective" "specialist1,specialist2"
```

### Integration Script:
```bash
# Full integration workflow
.hive-mind/integrate-specialists.sh
```

## 📈 Performance Metrics

### Before (Limited System):
- **4 worker types maximum** (researcher, coder, analyst, tester)
- **Basic capabilities** only
- **Limited specialization** for complex projects

### After (Specialist System):
- **25+ specialist types** with domain expertise
- **Targeted capabilities** per specialist
- **Scalable architecture** up to 25 workers per swarm
- **Multiple concurrent swarms** support

## 🛠️ Technical Implementation Details

### 1. Bypassed Hardcoded Limitations:
The original system hardcoded worker types in:
```javascript
const workerTypes = ['researcher', 'coder', 'analyst', 'tester']; // LIMITATION
```

Our solution dynamically loads from `specialist-workers.json`.

### 2. Extended Capabilities Mapping:
Each specialist has detailed capabilities:
```json
{
  "actor-systems": {
    "capabilities": ["actor-model", "concurrency", "fault-tolerance", "message-passing", "supervision"],
    "description": "Actor Systems specialist for Akka, Orleans, and distributed systems"
  }
}
```

### 3. Database Schema Compatible:
Uses existing SQLite schema without modifications:
- `swarms` table for swarm records
- `agents` table for specialist workers  
- `capabilities` stored as JSON strings

### 4. MCP Tool Integration:
Generated prompts include full MCP coordination:
- `mcp__claude-flow__swarm_init`
- `mcp__claude-flow__agent_spawn`
- `mcp__claude-flow__memory_usage`
- `mcp__claude-flow__consensus_vote`

## 🎯 MS-3 Specific Configuration

### Recommended Specialist Configuration:
```bash
# Core MS-3 team (8 specialists)
actor-systems,nats-specialist,postgresql-expert,aws-migration,kubernetes-expert,microservices-architect,security-expert,performance-engineer

# Extended MS-3 team (15 specialists) - RECOMMENDED
actor-systems,nats-specialist,postgresql-expert,aws-migration,kubernetes-expert,docker-specialist,microservices-architect,terraform-expert,security-expert,devops-engineer,performance-engineer,backend-specialist,monitoring-specialist,golang-specialist,rust-specialist
```

### Quick Deploy for MS-3:
```bash
./ms3-specialists.sh quick
```

This creates and activates a focused MS-3 specialist swarm with optimal configuration.

## 🔍 Troubleshooting

### Common Issues:

1. **"Failed to load specialist workers config"**
   - Ensure `specialist-workers.json` exists
   - Check file permissions

2. **"Database connection failed"**  
   - Initialize hive-mind: `npx claude-flow hive-mind init`
   - Check `.hive-mind/hive.db` exists

3. **"No specialist swarms found"**
   - Create swarm first: `./ms3-specialists.sh create-ms3`
   - Check database: `sqlite3 .hive-mind/hive.db "SELECT * FROM swarms;"`

4. **"Claude Code not found"**
   - Install: `npm install -g @anthropic-ai/claude-code`
   - Use manual prompt file activation

## 🎉 Success Verification

### Confirm 25+ Workers Active:
```bash
./ms3-specialists.sh status | grep "Workers:" | head -1
# Should show: 🔬 Specialist Workers (25): or (15): for focused swarm
```

### Verify Specialist Types:
```bash
sqlite3 .hive-mind/hive.db "SELECT COUNT(DISTINCT type) as unique_types FROM agents WHERE role='worker';"
# Should return 25 (or number of specialists created)
```

### Check Claude Integration:
```bash
ls .hive-mind/claude-specialist-prompt-*.txt
# Should show generated coordination prompts
```

## 📋 Next Steps for MS-3

1. **Activate Specialists**: Use `./ms3-specialists.sh activate` 
2. **Begin Migration**: Focus on Actor Systems → NATS → PostgreSQL → AWS
3. **Monitor Progress**: Use `npx claude-flow hive-mind status`
4. **Scale as Needed**: Add more specialists with custom configurations
5. **Iterate**: Use specialist feedback to refine the migration strategy

The specialist hive-mind system is now configured and ready to handle the complete MS-3 Actor Systems migration with 25+ domain experts working in coordinated parallel execution.