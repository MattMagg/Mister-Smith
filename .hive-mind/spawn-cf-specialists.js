#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const sqlite3 = require('sqlite3').verbose();

// Load specialist configuration
const configPath = path.join(__dirname, 'claude-flow-specialists.json');
const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

// Database path
const dbPath = path.join(__dirname, 'hive.db');

// Colors for output
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m'
};

function log(message, color = 'reset') {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function generateId(prefix) {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`;
}

function createSwarmWithSpecialists(objective, selectedSpecialists = null) {
  const db = new sqlite3.Database(dbPath);
  
  const swarmId = generateId('swarm');
  const hiveName = `hive-${Date.now()}`;
  const timestamp = new Date().toISOString();
  
  // Use selected specialists or all
  const specialists = selectedSpecialists || config.specialists;
  
  log(`\n🚀 Creating Claude-Flow Development Swarm...`, 'bright');
  log(`📋 Objective: ${objective}`, 'yellow');
  log(`👥 Specialists: ${specialists.length}`, 'cyan');
  
  db.serialize(() => {
    // Create swarm
    db.run(`
      INSERT INTO swarms (id, name, objective, queen_type, status, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?, ?)
    `, [swarmId, hiveName, objective, 'strategic', 'active', timestamp, timestamp], 
    (err) => {
      if (err) {
        log(`❌ Error creating swarm: ${err.message}`, 'red');
        return;
      }
      
      log(`✅ Swarm created: ${swarmId}`, 'green');
      
      // Add Queen
      const queenId = generateId('agent');
      db.run(`
        INSERT INTO agents (id, swarm_id, name, type, role, status, capabilities, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      `, [queenId, swarmId, 'Queen Coordinator', 'coordinator', 'queen',
          'active', JSON.stringify(['coordination', 'planning', 'delegation']), timestamp]);
      
      // Add specialists
      specialists.forEach((specialist, index) => {
        const agentId = generateId('agent');
        const workerName = specialist.name || `${specialist.type} Worker ${index + 1}`;
        
        db.run(`
          INSERT INTO agents (id, swarm_id, name, type, role, status, capabilities, created_at)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        `, [agentId, swarmId, workerName, specialist.type || 'specialist', 'worker',
            'active', JSON.stringify(specialist.capabilities), timestamp],
        (err) => {
          if (!err) {
            log(`  ✅ ${specialist.name}`, 'green');
          }
        });
      });
      
      // Add collective memory entries
      const memoryEntries = [
        { key: 'swarm_objective', value: objective },
        { key: 'specialist_config', value: JSON.stringify(specialists) },
        { key: 'topology', value: config.swarm_config.topology }
      ];
      
      memoryEntries.forEach(entry => {
        const memoryId = generateId('memory');
        db.run(`
          INSERT INTO collective_memory (id, swarm_id, key, value, created_at)
          VALUES (?, ?, ?, ?, ?)
        `, [memoryId, swarmId, entry.key, entry.value, timestamp]);
      });
      
      setTimeout(() => {
        log(`\n🎯 Claude-Flow Development Swarm Ready!`, 'bright');
        log(`📊 Swarm ID: ${swarmId}`, 'cyan');
        log(`🐝 Hive: ${hiveName}`, 'yellow');
        log(`\n💡 Next steps:`, 'magenta');
        log(`   1. Run: npx claude-flow@alpha hive-mind status`, 'blue');
        log(`   2. Use: ./cf-dev-swarm.sh activate`, 'blue');
        log(`   3. Start developing with specialized agents!`, 'blue');
        
        db.close();
      }, 500);
    });
  });
}

// Parse command line arguments
const args = process.argv.slice(2);
const command = args[0];
const objective = args.slice(1).join(' ') || 'Claude-Flow Repository Development and Enhancement';

// Specialist presets
const presets = {
  'core': ['cf-typescript-core', 'cf-rust-wasm', 'cf-hive-mind', 'cf-mcp-protocol', 'cf-testing-qa'],
  'full': null, // All specialists
  'debug': ['cf-typescript-core', 'cf-testing-qa', 'cf-performance', 'cf-advanced-usage'],
  'docs': ['cf-documentation', 'cf-advanced-usage'],
  'infra': ['cf-docker-container', 'cf-devops-ci', 'cf-performance']
};

if (command && presets.hasOwnProperty(command)) {
  const selectedIds = presets[command];
  const selectedSpecialists = selectedIds 
    ? config.specialists.filter(s => selectedIds.includes(s.id))
    : config.specialists;
  
  createSwarmWithSpecialists(objective, selectedSpecialists);
} else {
  // Create with all specialists
  createSwarmWithSpecialists(objective);
}