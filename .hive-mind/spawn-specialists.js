#!/usr/bin/env node

/**
 * Enhanced Hive Mind Spawner for 25+ Specialized Workers
 * This script bypasses the hardcoded 4-worker limitation
 */

const Database = require('better-sqlite3');
const path = require('path');
const fs = require('fs');
const chalk = require('chalk');

class SpecialistHiveMind {
  constructor() {
    this.dbPath = path.join(process.cwd(), '.hive-mind', 'hive.db');
    this.configPath = path.join(process.cwd(), '.hive-mind', 'specialist-workers.json');
    this.db = null;
    this.specialists = {};
    
    this.loadSpecialists();
  }
  
  loadSpecialists() {
    try {
      const config = JSON.parse(fs.readFileSync(this.configPath, 'utf8'));
      this.specialists = config.specialistWorkers;
      console.log(chalk.green(`✓ Loaded ${Object.keys(this.specialists).length} specialist worker types`));
    } catch (error) {
      console.error(chalk.red('Failed to load specialist workers config:'), error.message);
      process.exit(1);
    }
  }
  
  connectDatabase() {
    try {
      this.db = new Database(this.dbPath);
      console.log(chalk.green('✓ Connected to hive database'));
    } catch (error) {
      console.error(chalk.red('Failed to connect to database:'), error.message);
      process.exit(1);
    }
  }
  
  createSpecialistSwarm(objective, specialistTypes = [], maxWorkers = 25) {
    const timestamp = Date.now();
    const randomPart = Math.random().toString(36).substring(2, 11);
    const swarmId = `swarm-${timestamp}-${randomPart}`;
    
    console.log(chalk.blue(`\n🐝 Creating Specialist Swarm: ${swarmId}`));
    console.log(chalk.cyan(`Objective: ${objective}`));
    console.log(chalk.cyan(`Max Workers: ${maxWorkers}`));
    
    try {
      // Create swarm record
      const insertSwarm = this.db.prepare(`
        INSERT INTO swarms (id, name, objective, queen_type, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
      `);
      
      insertSwarm.run(
        swarmId,
        `specialist-hive-${timestamp}`,
        objective,
        'adaptive',
        'active',
        new Date().toISOString(),
        new Date().toISOString()
      );
      
      console.log(chalk.green('✓ Swarm record created'));
      
      // Create Queen coordinator
      this.createQueen(swarmId);
      
      // Create specialist workers
      this.createSpecialistWorkers(swarmId, specialistTypes, maxWorkers);
      
      console.log(chalk.green(`\n🎉 Specialist swarm created successfully!`));
      console.log(chalk.cyan(`Swarm ID: ${swarmId}`));
      
      return swarmId;
      
    } catch (error) {
      console.error(chalk.red('Failed to create specialist swarm:'), error.message);
      throw error;
    }
  }
  
  createQueen(swarmId) {
    const insertAgent = this.db.prepare(`
      INSERT INTO agents (id, swarm_id, name, type, role, status, capabilities, created_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `);
    
    const queenId = `queen-${swarmId}`;
    const capabilities = [
      'orchestration', 'coordination', 'strategic-planning', 
      'decision-making', 'resource-allocation', 'consensus-building',
      'specialist-coordination', 'adaptive-leadership'
    ];
    
    insertAgent.run(
      queenId,
      swarmId,
      'Queen Coordinator',
      'coordinator',
      'queen',
      'active',
      JSON.stringify(capabilities),
      new Date().toISOString()
    );
    
    console.log(chalk.green('✓ Queen coordinator created'));
  }
  
  createSpecialistWorkers(swarmId, requestedTypes, maxWorkers) {
    const insertAgent = this.db.prepare(`
      INSERT INTO agents (id, swarm_id, name, type, role, status, capabilities, created_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `);
    
    // If no specific types requested, use all available specialists
    const typesToCreate = requestedTypes.length > 0 
      ? requestedTypes 
      : Object.keys(this.specialists);
    
    let workerCount = 0;
    const createdWorkers = [];
    
    // Create workers up to maxWorkers limit
    for (const specialistType of typesToCreate) {
      if (workerCount >= maxWorkers) break;
      
      const specialist = this.specialists[specialistType];
      if (!specialist) {
        console.warn(chalk.yellow(`⚠️  Unknown specialist type: ${specialistType}`));
        continue;
      }
      
      const workerId = `worker-${swarmId}-${workerCount}`;
      const workerName = `${this.toTitleCase(specialistType)} Specialist ${workerCount + 1}`;
      
      insertAgent.run(
        workerId,
        swarmId,
        workerName,
        specialistType,
        'worker',
        'idle',
        JSON.stringify(specialist.capabilities),
        new Date().toISOString()
      );
      
      createdWorkers.push({
        id: workerId,
        name: workerName,
        type: specialistType,
        capabilities: specialist.capabilities
      });
      
      workerCount++;
    }
    
    console.log(chalk.green(`✓ Created ${workerCount} specialist workers:`));
    createdWorkers.forEach(worker => {
      console.log(chalk.gray(`  • ${worker.name} (${worker.type})`));
    });
    
    return createdWorkers;
  }
  
  listSpecialists() {
    console.log(chalk.blue('\n🔬 Available Specialist Workers:\n'));
    
    Object.entries(this.specialists).forEach(([type, config], index) => {
      console.log(chalk.cyan(`${index + 1}. ${this.toTitleCase(type)}`));
      console.log(chalk.gray(`   Description: ${config.description}`));
      console.log(chalk.gray(`   Capabilities: ${config.capabilities.slice(0, 3).join(', ')}${config.capabilities.length > 3 ? '...' : ''}`));
      console.log('');
    });
  }
  
  getSwarmStatus(swarmId) {
    try {
      // Get swarm info
      const swarm = this.db.prepare('SELECT * FROM swarms WHERE id = ?').get(swarmId);
      if (!swarm) {
        console.error(chalk.red(`Swarm ${swarmId} not found`));
        return;
      }
      
      // Get agents
      const agents = this.db.prepare('SELECT * FROM agents WHERE swarm_id = ?').all(swarmId);
      const queen = agents.find(a => a.role === 'queen');
      const workers = agents.filter(a => a.role === 'worker');
      
      console.log(chalk.blue(`\n🐝 Swarm Status: ${swarmId}`));
      console.log(chalk.cyan(`Name: ${swarm.name}`));
      console.log(chalk.cyan(`Objective: ${swarm.objective}`));
      console.log(chalk.cyan(`Queen Type: ${swarm.queen_type}`));
      console.log(chalk.cyan(`Status: ${swarm.status}`));
      console.log(chalk.cyan(`Created: ${new Date(swarm.created_at).toLocaleString()}`));
      
      if (queen) {
        console.log(chalk.magenta(`\n👑 Queen: ${queen.name} (${queen.status})`));
      }
      
      console.log(chalk.blue(`\n🔬 Specialist Workers (${workers.length}):`));
      workers.forEach(worker => {
        const statusColor = worker.status === 'active' ? 'green' : 
                           worker.status === 'busy' ? 'yellow' : 'gray';
        console.log(chalk.gray(`  • ${worker.name} - ${chalk[statusColor](worker.status)}`));
      });
      
      // Group by specialist type
      const workersByType = workers.reduce((acc, worker) => {
        acc[worker.type] = (acc[worker.type] || 0) + 1;
        return acc;
      }, {});
      
      console.log(chalk.blue('\n📊 Worker Distribution:'));
      Object.entries(workersByType).forEach(([type, count]) => {
        console.log(chalk.gray(`  • ${this.toTitleCase(type)}: ${count}`));
      });
      
    } catch (error) {
      console.error(chalk.red('Error getting swarm status:'), error.message);
    }
  }
  
  toTitleCase(str) {
    return str.split('-').map(word => 
      word.charAt(0).toUpperCase() + word.slice(1)
    ).join(' ');
  }
  
  close() {
    if (this.db) {
      this.db.close();
      console.log(chalk.gray('Database connection closed'));
    }
  }
}

// CLI Interface
function showHelp() {
  console.log(chalk.yellow(`
🐝 Specialist Hive Mind Controller

USAGE:
  node spawn-specialists.js <command> [options]

COMMANDS:
  create <objective>           Create specialist swarm with all 25+ workers
  create-custom <objective>    Create swarm with specific specialist types
  list                         List all available specialist types
  status <swarmId>            Show swarm status
  help                        Show this help

EXAMPLES:
  # Create full specialist swarm
  node spawn-specialists.js create "Build microservices platform"
  
  # Create custom swarm with specific specialists
  node spawn-specialists.js create-custom "AWS migration project" actor-systems,aws-migration,postgresql-expert,kubernetes-expert
  
  # List available specialists
  node spawn-specialists.js list
  
  # Check swarm status
  node spawn-specialists.js status swarm-1234567890-abcdef
`));
}

async function main() {
  const args = process.argv.slice(2);
  const command = args[0];
  
  if (!command || command === 'help') {
    showHelp();
    return;
  }
  
  const hiveMind = new SpecialistHiveMind();
  
  try {
    switch (command) {
      case 'list':
        hiveMind.listSpecialists();
        break;
        
      case 'create':
        const objective = args.slice(1).join(' ');
        if (!objective) {
          console.error(chalk.red('Error: Please provide an objective'));
          console.log('Example: node spawn-specialists.js create "Build microservices platform"');
          process.exit(1);
        }
        
        hiveMind.connectDatabase();
        const swarmId = hiveMind.createSpecialistSwarm(objective, [], 25);
        hiveMind.getSwarmStatus(swarmId);
        break;
        
      case 'create-custom':
        const customObjective = args[1];
        const specialistTypes = args[2] ? args[2].split(',') : [];
        
        if (!customObjective) {
          console.error(chalk.red('Error: Please provide an objective and specialist types'));
          console.log('Example: node spawn-specialists.js create-custom "AWS migration" actor-systems,aws-migration,postgresql-expert');
          process.exit(1);
        }
        
        hiveMind.connectDatabase();
        const customSwarmId = hiveMind.createSpecialistSwarm(customObjective, specialistTypes, 25);
        hiveMind.getSwarmStatus(customSwarmId);
        break;
        
      case 'status':
        const statusSwarmId = args[1];
        if (!statusSwarmId) {
          console.error(chalk.red('Error: Please provide a swarm ID'));
          process.exit(1);
        }
        
        hiveMind.connectDatabase();
        hiveMind.getSwarmStatus(statusSwarmId);
        break;
        
      default:
        console.error(chalk.red(`Unknown command: ${command}`));
        showHelp();
        process.exit(1);
    }
    
  } catch (error) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  } finally {
    hiveMind.close();
  }
}

if (require.main === module) {
  main();
}

module.exports = { SpecialistHiveMind };