# 📊 Phase [X]: [Phase Name] Tracker

## 🎯 Phase Overview
**Phase**: [X]  
**Name**: [Phase Name]  
**Status**: [PENDING|IN_PROGRESS|COMPLETE|BLOCKED]  
**Progress**: [0-100]%  
**Started**: [ISO-8601 timestamp or "Not started"]  
**Completed**: [ISO-8601 timestamp or "In progress"]  
**Memory Namespace**: ms3-advanced/phase[X]/*  
**Agents Deployed**: [count]  
**Automation Level**: [0-100]%  

## 📈 Progress Dashboard

```
📊 PHASE [X] PROGRESS: [percentage]%
├── [✅|🔄|⭕|❌] Sub-phase X.1: [Name] ([percentage]%)
├── [✅|🔄|⭕|❌] Sub-phase X.2: [Name] ([percentage]%)
├── [✅|🔄|⭕|❌] Sub-phase X.3: [Name] ([percentage]%)
├── [✅|🔄|⭕|❌] Sub-phase X.4: [Name] ([percentage]%)
├── [✅|🔄|⭕|❌] Sub-phase X.5: [Name] ([percentage]%)
└── [✅|🔄|⭕|❌] Sub-phase X.6: [Name] ([percentage]%)

⏱️ Estimated Completion: [date]
💰 Budget Used: $[amount]/$[allocated] ([percentage]%)
🚨 Active Blockers: [count]
📝 Tasks Completed: [completed]/[total]
```

## 🔗 Sub-Phase Navigation

### ✅ Completed Sub-Phases
- [X.1 [Name]](sub-phase-X.1-[slug]/tracker.md) - [Brief description]
  - Key deliverables: [list]
  - Memory keys: [count]
  
### 🔄 Active Sub-Phases
- [X.2 [Name]](sub-phase-X.2-[slug]/tracker.md) - [Brief description]
  - Current focus: [description]
  - Blockers: [list or "None"]

### ⭕ Pending Sub-Phases
- X.3 [Name] - [Brief description]
- X.4 [Name] - [Brief description]

## 📋 Task Status

### Summary
- **Total Tasks**: [count]
- **Completed**: [count] ([percentage]%)
- **In Progress**: [count] ([percentage]%)
- **Pending**: [count] ([percentage]%)
- **Blocked**: [count] ([percentage]%)

### 🔄 Active Tasks
```yaml
- task_id: [phase][subphase]-[number]
  title: "[Task Title]"
  sub_phase: X.Y
  status: in_progress
  assigned_agent: [agent-name]
  started: [timestamp]
  progress: [percentage]%
  dependencies: [list]
  blockers: [list]
  memory_keys:
    - ms3-advanced/phase[X]/[key1]
    - ms3-advanced/phase[X]/[key2]
```

### ⭕ Queued Tasks
1. **[Task Name]** - Sub-phase X.Y - Priority: [HIGH|MEDIUM|LOW]
2. **[Task Name]** - Sub-phase X.Y - Priority: [HIGH|MEDIUM|LOW]

### ✅ Recent Completions
- ✅ [timestamp] - [Task name] - [Agent] - [Duration]
- ✅ [timestamp] - [Task name] - [Agent] - [Duration]

## 🧠 Memory Integration

```yaml
namespace: ms3-advanced/phase[X]
total_keys: [count]
sub_namespaces:
  - phase[X]/planning: [count] keys
  - phase[X]/implementation: [count] keys
  - phase[X]/validation: [count] keys
  - phase[X]/decisions: [count] keys
recent_activity:
  - key: [key_name]
    operation: [store|retrieve|search]
    agent: [agent_name]
    timestamp: [ISO-8601]
cross_phase_references:
  - from_phase1: [count] references
  - from_phase2: [count] references
  - from_phase3: [count] references
```

## 🔍 Verification Status

### Phase Verification Commands
```bash
# Overall phase health check
./scripts/verify-phase.sh [X]
# Expected: All sub-phases reporting status

# Memory consistency check
npx claude-flow memory search --pattern "ms3-advanced/phase[X]/*" --verify
# Expected: No orphaned keys

# Agent coordination check
npx claude-flow swarm status --phase [X]
# Expected: All agents synchronized
```

### Sub-Phase Verification Results
- **X.1**: ✅ Passed ([count]/[total] checks)
- **X.2**: 🔄 In Progress
- **X.3**: ⭕ Not Started

## 🚨 Constraints & Risks

### Active Constraints
1. **[Constraint Name]**: [Description]
   - Impact: [HIGH|MEDIUM|LOW]
   - Mitigation: [Strategy]
   
### Identified Risks
| Risk | Probability | Impact | Mitigation | Status |
|------|------------|--------|------------|--------|
| [Risk 1] | [H/M/L] | [H/M/L] | [Strategy] | [Active/Planned] |
| [Risk 2] | [H/M/L] | [H/M/L] | [Strategy] | [Active/Planned] |

## 📊 Phase Metrics

### Performance Metrics
- **Automation Efficiency**: [percentage]%
- **Agent Utilization**: [percentage]%
- **Task Velocity**: [tasks/hour]
- **Error Rate**: [percentage]%
- **Rollback Count**: [number]

### Resource Utilization
```yaml
compute:
  allocated: [amount]
  used: [amount]
  efficiency: [percentage]%
storage:
  allocated: [amount]GB
  used: [amount]GB
  growth_rate: [amount]GB/day
api_calls:
  limit: [amount]/hour
  current: [amount]/hour
  peak: [amount]/hour
```

## 🔄 Dependencies

### Upstream Dependencies (Waiting On)
- Phase [X-1]: [Specific deliverable needed]
- External: [Any external dependencies]

### Downstream Dependencies (Blocking)
- Phase [X+1]: [What they're waiting for]
- Phase [X+2]: [What they're waiting for]

## 📝 Decision Log

### Recent Decisions
1. **[timestamp]** - [Decision summary]
   - Rationale: [Brief explanation]
   - Impact: [What changed]
   - Memory: ms3-advanced/phase[X]/decisions/[key]

2. **[timestamp]** - [Decision summary]
   - Rationale: [Brief explanation]
   - Impact: [What changed]
   - Memory: ms3-advanced/phase[X]/decisions/[key]

## 🎯 Success Criteria

### Phase Success Metrics
- [ ] All sub-phases completed
- [ ] Zero critical errors in verification
- [ ] Memory namespace fully populated
- [ ] Downstream dependencies unblocked
- [ ] Budget within 10% of allocation
- [ ] All deliverables accepted

### Quality Gates
1. **Entry Criteria Met**: [YES|NO|PARTIAL]
2. **Implementation Complete**: [YES|NO|PARTIAL]
3. **Verification Passed**: [YES|NO|PARTIAL]
4. **Documentation Updated**: [YES|NO|PARTIAL]
5. **Handoff Ready**: [YES|NO|PARTIAL]

## 🔗 Quick Links

- [Master Tracker](../tracker.md)
- [Phase Plan](MASTER-PHASE-[X].md)
- [Memory Dashboard](memory://dashboard/phase[X])
- [Verification Suite](verification/phase-[X]-tests.md)
- [Previous Phase](../phase-[X-1]-[name]/tracker.md)
- [Next Phase](../phase-[X+1]-[name]/tracker.md)

---
*Auto-updated every 5 minutes | Last sync: [timestamp]*  
*Memory persistence: Enabled | Automation: Active*