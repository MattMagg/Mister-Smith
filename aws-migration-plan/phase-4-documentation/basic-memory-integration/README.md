# Basic-Memory Integration for AWS Migration Tracker

## Overview
This module provides persistent memory storage for the AWS migration tracker using the basic-memory MCP server. It enables cross-session state persistence, constraint tracking, and progress recovery.

## Memory URI Scheme

### Phase-Based URIs
```
memory://aws-migration/phases/[phase-number]/[sub-phase]/[component]
```
- Example: `memory://aws-migration/phases/1/planning/requirements`
- Example: `memory://aws-migration/phases/2/pre-migration/dependencies`

### Component-Based URIs
```
memory://aws-migration/components/[component-type]/[status]
```
- Example: `memory://aws-migration/components/ecs-cluster/deployed`
- Example: `memory://aws-migration/components/rds-aurora/migrated`

### Verification URIs
```
memory://aws-migration/verification/[command-set]/[timestamp]
```
- Example: `memory://aws-migration/verification/fargate-tasks/2025-07-11T16:00:00Z`
- Example: `memory://aws-migration/verification/aurora-health/2025-07-11T16:00:00Z`

### Constraint URIs
```
memory://aws-migration/constraints/[constraint-id]
```
- Example: `memory://aws-migration/constraints/budget-limit`
- Example: `memory://aws-migration/constraints/downtime-window`

## Integration Components

### 1. Scripts
- `sync-to-memory.sh` - Push tracker updates to memory
- `restore-from-memory.sh` - Recover state from memory
- `memory-query.sh` - Search and retrieve memory entries
- `memory-monitor.sh` - Real-time memory sync monitoring

### 2. Templates
- Phase state templates
- Component status templates
- Verification result templates
- Constraint definition templates

### 3. Automation
- Automatic sync on tracker updates
- Periodic state snapshots
- Rollback checkpoint creation
- Cross-reference validation

## Usage

### Store Phase Progress
```bash
./scripts/sync-to-memory.sh --phase 1 --status complete
```

### Retrieve Phase History
```bash
./scripts/memory-query.sh --context "memory://aws-migration/phases/1"
```

### Search Constraints
```bash
./scripts/memory-query.sh --search "budget" --type constraints
```

### Restore from Memory
```bash
./scripts/restore-from-memory.sh --timestamp "2025-07-11T16:00:00Z"
```

## Memory Storage Patterns

### Phase Completion States
```json
{
  "phase": 1,
  "sub_phase": "planning",
  "status": "complete",
  "timestamp": "2025-07-11T16:00:00Z",
  "components": ["requirements", "constraints", "architecture"],
  "verification": {
    "passed": true,
    "commands_executed": 15,
    "issues_found": 0
  }
}
```

### Constraint Records
```json
{
  "constraint_id": "budget-limit",
  "type": "financial",
  "value": "$5000/month",
  "priority": "critical",
  "discovered": "2025-07-11T16:00:00Z",
  "affects": ["compute", "storage", "networking"]
}
```

### Verification Results
```json
{
  "command_set": "fargate-tasks",
  "timestamp": "2025-07-11T16:00:00Z",
  "results": {
    "total_commands": 12,
    "successful": 12,
    "failed": 0,
    "warnings": 2
  },
  "details": []
}
```

## Synchronization Strategy

1. **Tracker → Memory**: Real-time push on file updates
2. **Memory → Tracker**: On-demand restore or recovery
3. **Validation**: Cross-reference between sources
4. **Timestamps**: UTC format for consistency

## Error Handling

- Automatic retry on sync failures
- Conflict resolution with timestamps
- Backup before destructive operations
- Rollback capabilities