# Phase 2: Containerization - Living Tracker

## Phase Metadata
- **Phase Number**: 2
- **Phase Name**: Container Architecture Implementation
- **Purpose**: Containerize MisterSmith components for ECS
- **Lead Agent**: [ASSIGN: Container Specialist from MS-2]
- **Status**: NOT_STARTED
- **Progress**: 0%
- **Dependencies**: Phase 1 COMPLETE (need ECR endpoints)

## Phase Objectives
1. Create Docker images for all components
2. Optimize containers for ECS Fargate
3. Implement health checks and signals
4. Set up ECR repositories
5. Create base images with MCP support
6. Test container orchestration locally

## Task Breakdown

### Task 2.1: Rust Supervisor Containerization
**Priority**: CRITICAL
**Complexity**: HIGH

**Key Challenges**:
- Current: Direct process spawning with fork()
- Target: Container-safe process management
- MCP server integration in container
- Signal handling for graceful shutdown

**Actions**:
1. Analyze current spawn mechanism in `src/agent/pool.rs`
2. Refactor for container process model
3. Create multi-stage Dockerfile
4. Add health check endpoints
5. Test supervisor lifecycle

**Memory**: `aws-migration/phase-2/dockerfiles/supervisor`

### Task 2.2: Claude CLI Container
**Priority**: CRITICAL
**Complexity**: VERY HIGH

**Key Challenges**:
- Claude CLI expects filesystem access
- MCP configuration in container
- Cost tracking per container
- Process lifecycle management

**Approach**:
```dockerfile
FROM ubuntu:22.04
# Install Claude CLI
# Configure MCP
# Add cost tracking wrapper
# Implement lifecycle hooks
```

**Memory**: `aws-migration/phase-2/dockerfiles/claude-cli`

### Task 2.3: MCP Gateway Container
**Priority**: HIGH
**Complexity**: MEDIUM

**Requirements**:
- WebSocket support
- SSE broadcasting
- gRPC interface
- NATS connection management

**Memory**: `aws-migration/phase-2/dockerfiles/mcp-gateway`

### Task 2.4: ECR Setup
**Priority**: MEDIUM
**Complexity**: LOW

**Repositories Needed**:
- mistersmith/supervisor
- mistersmith/claude-worker
- mistersmith/mcp-gateway
- mistersmith/tools (utility container)

**Memory**: `aws-migration/phase-2/ecr/repositories`

### Task 2.5: Local Testing Framework
**Priority**: HIGH
**Complexity**: MEDIUM

**Docker Compose Stack**:
```yaml
services:
  supervisor:
    build: ./supervisor
    environment:
      - WORKER_POOL_SIZE=10
  
  nats:
    image: nats:latest
    
  postgres:
    image: postgres:15
    
  claude-worker:
    build: ./claude-worker
    deploy:
      replicas: 5
```

**Memory**: `aws-migration/phase-2/testing/docker-compose`

## Progress Tracking

| Task | Status | Progress | Agent | Output | Blockers |
|------|--------|----------|-------|--------|----------|
| 2.1 | NOT_STARTED | 0% | - | - | - |
| 2.2 | NOT_STARTED | 0% | - | - | - |
| 2.3 | NOT_STARTED | 0% | - | - | - |
| 2.4 | NOT_STARTED | 0% | - | - | - |
| 2.5 | NOT_STARTED | 0% | - | - | - |

## Critical Technical Decisions

### Process Model:
```yaml
current_state:
  model: "Fork and exec"
  issue: "Incompatible with containers"
  
migration_options:
  option_1:
    name: "Subprocess with init system"
    pros: ["Minimal code change", "Signal handling"]
    cons: ["Complex container", "Resource overhead"]
    
  option_2:
    name: "Job queue with workers"
    pros: ["Cloud native", "Scalable"]
    cons: ["Major refactor", "State management"]
    
  option_3:
    name: "Sidecar pattern"
    pros: ["Isolation", "Easy scaling"]
    cons: ["Network overhead", "Complex orchestration"]
```

**Decision Memory**: `aws-migration/phase-2/decisions/process-model`

## Container Requirements Matrix

| Component | Base Image | Size Target | Startup Time | Memory | CPU |
|-----------|------------|-------------|--------------|--------|-----|
| Supervisor | rust:slim | <100MB | <5s | 256MB | 0.25 |
| Claude Worker | ubuntu:22.04 | <500MB | <10s | 2GB | 1.0 |
| MCP Gateway | node:18-slim | <150MB | <3s | 512MB | 0.5 |

## Integration Testing Checklist
- [ ] Supervisor spawns workers in container
- [ ] Workers connect to NATS
- [ ] MCP gateway receives discoveries
- [ ] Health checks respond correctly
- [ ] Graceful shutdown works
- [ ] Resource limits enforced
- [ ] Logs properly collected

## Phase Completion Criteria
- [ ] All containers build successfully
- [ ] Local docker-compose stack runs
- [ ] Health checks pass
- [ ] Integration tests green
- [ ] ECR repositories created
- [ ] Images pushed to ECR
- [ ] Performance benchmarks met

## Handoff to Other Phases
**Outputs**:
1. Container images in ECR
2. Docker Compose for local testing
3. ECS task definitions (draft)
4. Resource requirements
5. Health check specifications

**Critical for**:
- Phase 5: Task definitions need container URIs
- Phase 6: Log collection configuration
- Phase 7: Integration testing setup

---
*Phase Initialized*: [timestamp]
*Last Update*: [agent/timestamp]
*Health Status*: [GREEN|YELLOW|RED]