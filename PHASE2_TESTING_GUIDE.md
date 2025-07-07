# Phase 2 Testing Guide for MisterSmith

## Prerequisites

### 1. Install Rust
```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
cargo --version
rustc --version
```

### 2. Install NATS Server (Optional but Recommended)
```bash
# macOS
brew install nats-server

# Or download from: https://github.com/nats-io/nats-server/releases
# Start NATS server
nats-server
```

### 3. Install Claude CLI (Optional for Full Testing)
```bash
# Follow Claude CLI installation instructions
# https://docs.anthropic.com/en/docs/claude-cli
```

## Running Phase 2 Tests

### Quick Test (No External Dependencies)
```bash
cd /Users/mac-main/Mister-Smith/MisterSmith

# Run Phase 2 verification suite
cargo run

# Expected output without NATS:
# ✅ Agent creation and lifecycle
# ✅ Agent pool management
# ✅ Agent state transitions
# ⚠️  NATS tests skipped (server not running)
```

### Full Test (With NATS)
```bash
# Terminal 1: Start NATS server
nats-server

# Terminal 2: Run tests
cd /Users/mac-main/Mister-Smith/MisterSmith
cargo run

# Expected output with NATS:
# ✅ All Phase 2 tests passing
# ✅ NATS messaging operational
```

### Unit Tests
```bash
# Run all unit tests
cargo test

# Run specific test modules
cargo test agent::
cargo test transport::
cargo test runtime::
cargo test supervision::

# Run with output displayed
cargo test -- --nocapture

# Run tests in parallel (default)
cargo test -- --test-threads=4
```

### Integration Tests
```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test files
cargo test --test claude_cli
cargo test --test agent_lifecycle
cargo test --test concurrent_agents
cargo test --test error_scenarios
```

### Performance Tests
```bash
# Run performance benchmarks
cargo bench

# Run specific benchmark
cargo bench performance_baseline
```

## Test Categories

### 1. Core Agent Tests (`src/tests/agent/`)
- Agent creation and configuration
- State machine transitions
- Resource management
- Error handling

### 2. Integration Tests (`src/tests/integration/`)
- **claude_cli.rs**: Claude CLI process management
- **agent_lifecycle.rs**: Full agent lifecycle
- **concurrent_agents.rs**: Multi-agent scenarios
- **error_scenarios.rs**: Failure handling

### 3. Supervision Tests (`src/tests/supervision/`)
- Supervision tree operations
- Failure detection
- Circuit breaker functionality
- Restart strategies

### 4. Main Verification (`src/main.rs`)
Phase 2 end-to-end verification including:
1. Agent creation and basic lifecycle
2. Agent pool management
3. Agent state management
4. Process management (mock)
5. NATS communication (if available)

## Debugging Failed Tests

### Common Issues

1. **NATS Connection Failed**
   ```bash
   # Start NATS server
   nats-server
   # Or run tests without NATS (partial functionality)
   ```

2. **Claude CLI Not Found**
   ```bash
   # Tests will use mock processes if Claude CLI unavailable
   # Install Claude CLI for full integration testing
   ```

3. **Port Already in Use**
   ```bash
   # Kill existing NATS process
   pkill nats-server
   # Or use different port
   nats-server -p 4223
   ```

### Verbose Output
```bash
# Enable debug logging
RUST_LOG=mistersmith=debug cargo run

# Trace-level logging for detailed debugging
RUST_LOG=mistersmith=trace cargo test
```

## Expected Test Results

### Phase 2 Verification Success
```
=== MisterSmith Framework Phase 2 Verification ===
Testing: Single Claude CLI Agent with NATS Communication

✅ Test 1: Agent Creation - PASSED
✅ Test 2: Agent Pool - PASSED  
✅ Test 3: State Management - PASSED
✅ Test 4: Process Management - PASSED
✅ Test 5: NATS Messaging - PASSED

🎉 Phase 2 Verification PASSED!
✅ Single agent architecture working
✅ Process management functional
✅ NATS messaging operational
✅ Agent lifecycle management working

🚀 Ready for Phase 3: Multi-Agent Features
```

### Test Coverage
- Unit Tests: ~85% coverage
- Integration Tests: Core workflows covered
- Performance Baselines: Established

## Next Steps After Testing

1. **If all tests pass**: Ready for Phase 3 implementation
2. **If tests fail**: Check error messages and debug logs
3. **Performance issues**: Run benchmarks and check baselines

## Continuous Testing
```bash
# Watch for changes and rerun tests
cargo watch -x test

# Run tests before commits
cargo test && cargo fmt -- --check && cargo clippy
```