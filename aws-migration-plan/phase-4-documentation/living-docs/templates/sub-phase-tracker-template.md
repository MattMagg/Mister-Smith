# 📋 Sub-Phase X.Y: [Sub-Phase Name] Tracker

## 🎯 Sub-Phase Overview
**Phase**: [X] - [Phase Name]  
**Sub-Phase**: X.Y  
**Name**: [Sub-Phase Name]  
**Status**: [NOT_STARTED|IN_PROGRESS|COMPLETE|BLOCKED]  
**Progress**: [0-100]%  
**Started**: [ISO-8601 timestamp or "Not started"]  
**Completed**: [ISO-8601 timestamp or "In progress"]  
**Memory Keys**: ms3-advanced/phase[X]/subphase[Y]/*  
**Assigned Agent**: [agent-name]  
**Priority**: [CRITICAL|HIGH|MEDIUM|LOW]  

## 📊 Task Progress

```
📋 SUB-PHASE X.Y PROGRESS: [percentage]%
├── [✅|🔄|⭕|❌] Task 1: [Name] ([percentage]%)
├── [✅|🔄|⭕|❌] Task 2: [Name] ([percentage]%)
├── [✅|🔄|⭕|❌] Task 3: [Name] ([percentage]%)
├── [✅|🔄|⭕|❌] Task 4: [Name] ([percentage]%)
└── [✅|🔄|⭕|❌] Task 5: [Name] ([percentage]%)

⏱️ Time Elapsed: [duration]
⏱️ Time Remaining: [estimate]
📝 Tasks Completed: [completed]/[total]
🚨 Blockers: [count]
```

## 📋 Detailed Task List

### ✅ Completed Tasks
```yaml
- task_id: [X][Y]-001
  title: "[Task Title]"
  completed: [timestamp]
  duration: [time]
  agent: [agent-name]
  verification: PASSED
  outputs:
    - [Output 1 description]
    - [Output 2 description]
  memory_keys:
    - ms3-advanced/phase[X]/subphase[Y]/[key1]
    - ms3-advanced/phase[X]/subphase[Y]/[key2]
```

### 🔄 In Progress Tasks
```yaml
- task_id: [X][Y]-002
  title: "[Task Title]"
  status: in_progress
  started: [timestamp]
  progress: [percentage]%
  agent: [agent-name]
  last_update: [timestamp]
  blockers: [list or "none"]
  next_milestone: "[description]"
  estimated_completion: [timestamp]
```

### ⭕ Pending Tasks
```yaml
- task_id: [X][Y]-003
  title: "[Task Title]"
  status: pending
  priority: [1-5]
  estimated_duration: [time]
  dependencies:
    - task_id: [X][Y]-002
      status: waiting
  prerequisites:
    - "[Prerequisite 1]"
    - "[Prerequisite 2]"
```

### ❌ Blocked Tasks
```yaml
- task_id: [X][Y]-004
  title: "[Task Title]"
  status: blocked
  blocked_since: [timestamp]
  blocker_type: [TECHNICAL|DEPENDENCY|RESOURCE|DECISION]
  blocker_description: "[Detailed description]"
  resolution_plan: "[Plan to unblock]"
  assigned_to: [agent-name or "unassigned"]
```

## 🎯 Deliverables

### Expected Outputs
1. **[Deliverable 1]**
   - Description: [What it is]
   - Format: [File type, structure]
   - Location: [Path or memory key]
   - Status: [NOT_STARTED|IN_PROGRESS|COMPLETE|VERIFIED]

2. **[Deliverable 2]**
   - Description: [What it is]
   - Format: [File type, structure]
   - Location: [Path or memory key]
   - Status: [NOT_STARTED|IN_PROGRESS|COMPLETE|VERIFIED]

### Verification Criteria
- [ ] All tasks completed successfully
- [ ] Deliverables meet specifications
- [ ] Integration tests passed
- [ ] Memory keys properly indexed
- [ ] Documentation updated
- [ ] Handoff checklist complete

## 🧠 Memory Operations

### Recent Memory Activity
```yaml
last_24h:
  stores: [count]
  retrieves: [count]
  searches: [count]
  
key_patterns:
  - pattern: "phase[X]/subphase[Y]/config/*"
    count: [number]
    last_update: [timestamp]
  - pattern: "phase[X]/subphase[Y]/results/*"
    count: [number]
    last_update: [timestamp]
    
hot_keys:
  - key: [key_name]
    access_count: [number]
    last_accessed: [timestamp]
  - key: [key_name]
    access_count: [number]
    last_accessed: [timestamp]
```

### Critical Memory Keys
```yaml
configuration:
  key: ms3-advanced/phase[X]/subphase[Y]/config
  description: "Sub-phase configuration and settings"
  
decisions:
  key: ms3-advanced/phase[X]/subphase[Y]/decisions
  description: "Decision log with rationale"
  
state:
  key: ms3-advanced/phase[X]/subphase[Y]/state
  description: "Current execution state and checkpoints"
  
results:
  key: ms3-advanced/phase[X]/subphase[Y]/results
  description: "Completed task outputs and artifacts"
```

## 🔍 Verification Commands

### Task-Level Verification
```bash
# Verify individual task
./scripts/verify-task.sh [X][Y]-[task_number]
# Expected: JSON status with pass/fail

# Check task dependencies
./scripts/check-dependencies.sh --subphase X.Y
# Expected: All dependencies resolved

# Validate outputs
./scripts/validate-outputs.sh --subphase X.Y --task [task_id]
# Expected: All outputs present and valid
```

### Sub-Phase Verification
```bash
# Full sub-phase validation
./scripts/verify-subphase.sh X.Y
# Expected: 100% tasks verified

# Memory consistency check
npx claude-flow memory verify --pattern "phase[X]/subphase[Y]/*"
# Expected: No inconsistencies

# Integration test
./scripts/integration-test.sh --phase X --subphase Y
# Expected: All tests pass
```

## 🚨 Issues & Resolutions

### Active Issues
| Issue ID | Type | Severity | Description | Status | Assigned |
|----------|------|----------|-------------|--------|----------|
| [X][Y]-I001 | [Type] | [HIGH/MED/LOW] | [Description] | [OPEN/IN_PROGRESS] | [Agent] |

### Resolved Issues
| Issue ID | Resolution | Resolved By | Date | Verification |
|----------|------------|-------------|------|--------------|
| [X][Y]-I000 | [How resolved] | [Agent] | [Date] | ✅ Verified |

## 📊 Performance Metrics

### Task Execution Metrics
- **Average Task Duration**: [time]
- **Task Success Rate**: [percentage]%
- **Automation Level**: [percentage]%
- **Manual Interventions**: [count]
- **Rollback Events**: [count]

### Resource Usage
```yaml
agent_time:
  allocated: [hours]
  used: [hours]
  efficiency: [percentage]%
  
api_calls:
  total: [count]
  failed: [count]
  rate_limited: [count]
  
memory_operations:
  reads: [count]
  writes: [count]
  searches: [count]
  avg_latency: [ms]
```

## 🔄 Dependencies & Interfaces

### Upstream Dependencies
- **From Sub-Phase X.[Y-1]**: [What's needed]
  - Status: [RECEIVED|WAITING|PARTIAL]
  - Blocker: [YES|NO]

### Downstream Deliveries
- **To Sub-Phase X.[Y+1]**: [What's provided]
  - Status: [READY|IN_PROGRESS|NOT_STARTED]
  - Deadline: [timestamp]

### Cross-Phase Dependencies
- **Phase [X-1]**: [Any dependencies]
- **Phase [X+1]**: [Any dependencies]

## 📝 Agent Notes

### Key Decisions Made
1. **[timestamp]** - [Decision]
   - Context: [Why this decision]
   - Impact: [What it affects]
   - Alternatives considered: [List]

### Lessons Learned
- [Learning 1]: [How it helps future tasks]
- [Learning 2]: [How it helps future tasks]

### Recommendations
- For next sub-phase: [Recommendation]
- For similar tasks: [Recommendation]
- For automation: [Recommendation]

## 🔗 Quick Links

- [Phase Tracker](../../tracker.md)
- [Sub-Phase Plan](plan.md)
- [Task Details](tasks/README.md)
- [Memory Dashboard](memory://dashboard/phase[X]/subphase[Y])
- [Previous Sub-Phase](../sub-phase-X.[Y-1]-[name]/tracker.md)
- [Next Sub-Phase](../sub-phase-X.[Y+1]-[name]/tracker.md)

---
*Auto-updated every 2 minutes | Last sync: [timestamp]*  
*Agent: [agent-name] | Memory: Active | Verification: Enabled*