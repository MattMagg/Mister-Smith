# Claude-Flow Development Specialist Swarm Guide

## 🔧 Overview

The Claude-Flow Development Swarm is a specialized hive-mind configuration designed for enhancing, debugging, and optimizing the claude-flow repository. It includes 10 domain-specific specialists focused on different aspects of the codebase.

## 🚀 Quick Start

```bash
# Create a core development swarm (5 essential specialists)
./cf-dev-swarm.sh create-core "Fix hive-mind worker limitation"

# Create a full swarm (all 10 specialists)
./cf-dev-swarm.sh create-full "Implement new neural architecture"

# Check swarm status
./cf-dev-swarm.sh status

# Get Claude Code activation commands
./cf-dev-swarm.sh activate
```

## 👥 Specialist Roster

### Core Specialists (Essential 5)

1. **TypeScript Core Specialist**
   - Focus: CLI, MCP server, Web interface
   - Capabilities: TypeScript, Node.js, Express, WebSockets
   - Key Files: `src/cli/`, `src/mcp/`, `src/web/`

2. **Rust WASM Specialist**
   - Focus: Neural models, WASM runtime, performance
   - Capabilities: Rust, WebAssembly, SIMD optimization
   - Key Files: `wasm/`, neural model implementations

3. **Hive-Mind Architect**
   - Focus: Swarm orchestration, agent coordination
   - Capabilities: SQLite, distributed systems, consensus
   - Key Files: `src/cli/simple-commands/hive-mind.js`

4. **MCP Protocol Expert**
   - Focus: 87+ MCP tools, protocol development
   - Capabilities: JSON-RPC, stdio/HTTP servers
   - Key Files: MCP tool implementations

5. **Testing/QA Engineer**
   - Focus: Test coverage, bug fixes, benchmarking
   - Capabilities: Jest, performance testing
   - Key Files: `tests/`, `benchmark/`

### Additional Specialists (5)

6. **Documentation Specialist**
   - Focus: README, API docs, architecture guides
   - Capabilities: Technical writing, Mermaid diagrams

7. **Performance Engineer**
   - Focus: WASM optimization, profiling, metrics
   - Capabilities: Performance analysis, caching

8. **DevOps/CI Specialist**
   - Focus: GitHub Actions, npm publishing
   - Capabilities: CI/CD, release automation

9. **Docker/Container Specialist**
   - Focus: Containerization, Kubernetes
   - Capabilities: Docker, K8s manifests

10. **Advanced Usage Expert**
    - Focus: Complex workflows, best practices
    - Capabilities: Troubleshooting, optimization

## 🎯 Preset Configurations

### `create-core`
Essential 5 specialists for core development:
- TypeScript Core, Rust WASM, Hive-Mind, MCP Protocol, Testing/QA

### `create-full`
All 10 specialists for comprehensive development

### `create-debug`
Debugging-focused team:
- TypeScript Core, Testing/QA, Performance, Advanced Usage

### `create-docs`
Documentation team:
- Documentation Specialist, Advanced Usage Expert

### `create-infra`
Infrastructure team:
- Docker/Container, DevOps/CI, Performance

## 📝 Usage Examples

### Fixing Bugs
```bash
./cf-dev-swarm.sh create-debug "Fix TypeScript memory leak in terminal manager"
```

### Adding Features
```bash
./cf-dev-swarm.sh create-core "Add new MCP tool for database operations"
```

### Performance Optimization
```bash
./cf-dev-swarm.sh create-infra "Optimize WASM neural network performance"
```

### Documentation Update
```bash
./cf-dev-swarm.sh create-docs "Update API documentation and examples"
```

## 🔄 Workflow Integration

1. **Create Swarm**: Choose appropriate preset based on task
2. **Activate in Claude Code**: Copy activation commands
3. **Parallel Execution**: All specialists work simultaneously
4. **Coordination**: Via claude-flow@alpha hooks
5. **Progress Tracking**: Monitor with hive-mind status

## ⚡ Best Practices

1. **Always use `npx claude-flow@alpha`** - Required for latest features
2. **Batch operations** - Use parallel Task execution
3. **Coordinate via hooks** - Essential for swarm synchronization
4. **Monitor progress** - Regular status checks
5. **Clean when done** - `./cf-dev-swarm.sh clean`

## 🐛 Common Issues

### Issue: Swarm only shows 4 workers
**Solution**: This script bypasses the limitation by directly modifying SQLite

### Issue: Specialists not coordinating
**Solution**: Ensure all Task prompts include coordination hooks

### Issue: Performance degradation
**Solution**: Clean and recreate swarm, check memory usage

## 🔧 Technical Details

- **Database**: SQLite (.hive-mind/hive.db)
- **Bypass Method**: Direct database insertion
- **Compatibility**: Full hive-mind command support
- **Memory**: 200MB collective memory allocation
- **Topology**: Mesh (optimal for development)

## 📊 Monitoring

```bash
# Real-time status
npx claude-flow@alpha hive-mind status

# Performance metrics
npx claude-flow@alpha hive-mind metrics

# Memory usage
npx claude-flow@alpha hive-mind memory
```

## 🚨 Important Notes

1. This swarm is specifically for **claude-flow repository** development
2. Not related to MS-3 or MisterSmith systems
3. Requires `npx claude-flow@alpha` (not stable or @2.0.0)
4. Specialists are virtual coordination patterns, not actual coders
5. All code is written by Claude Code with specialist guidance

---

**Created for claude-flow development** | **Use responsibly** | **Always coordinate**