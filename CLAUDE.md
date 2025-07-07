# MisterSmith Project Instructions

## PROJECT CONTEXT

MisterSmith is a multi-agent orchestration framework for distributed AI systems. The project follows a documentation-first approach with incremental implementation.

**CURRENT STATUS**: Phase 2 (Single Agent) implementation COMPLETE ✅

Your role: Work with both existing implementation and documentation to evolve the framework incrementally.

## CURRENT PROJECT STATE

### What Exists

- 📁 `ms-framework-docs/` - Technical framework documentation (150+ pages)
- 📁 `internal-operations/` - Working prompts and planning documents
- 📁 `src/` - Phase 2 Rust implementation
- ✅ Working single-agent architecture
- ✅ Basic NATS messaging integration
- ✅ Agent lifecycle management
- ✅ Process management framework
- ✅ Comprehensive architectural specifications
- ✅ Detailed component designs for future phases

### What's Built (Phase 2)

- ✅ Single Claude CLI agent implementation
- ✅ Agent pool management with capacity control
- ✅ State management system (Created → Running → Terminated)
- ✅ NATS transport layer (basic)
- ✅ Message routing infrastructure
- ✅ Process supervision structure

### What's NOT Built Yet

- ❌ Multi-agent coordination (Phase 3)
- ❌ Persistence layer (PostgreSQL/Redis)
- ❌ Security implementation (mTLS, RBAC)
- ❌ Production deployment features
- ❌ Advanced monitoring and observability

See `IMPLEMENTATION_STATUS.md` for detailed tracking.

## CRITICAL CONSTRAINTS FOR AGENTS

### Implementation Phase Rules

**FORBIDDEN**: Implementing features beyond current phase without planning
**FORBIDDEN**: Breaking working functionality while adding features
**FORBIDDEN**: Claiming unimplemented features "work" or are "ready"
**REQUIRED**: Verify all changes with actual execution
**REQUIRED**: Maintain documentation accuracy with implementation state
**REQUIRED**: Build incrementally on proven foundations

### Reality Check Questions

Before ANY action:
1. Will this change break existing Phase 2 functionality?
2. Is this feature part of the current phase or future planning?
3. Have I verified this works with actual execution?
4. Does documentation match the implementation reality?
5. Am I building on proven foundations or creating new complexity?

## DEVELOPMENT PHASES

### ✅ PHASE 1: Foundation Setup (COMPLETE)
- Created Cargo.toml with core dependencies
- Established project structure
- Integrated Tokio runtime
- Set up basic module organization

### ✅ PHASE 2: Single Agent (COMPLETE - CURRENT)
- Implemented single Claude CLI agent
- Basic message handling via NATS
- Agent lifecycle management
- Process supervision framework
- Verification tests passing

### 🚧 PHASE 3: Multi-Agent Features (NEXT)
Planning required before implementation:
- Multiple agent type support
- Inter-agent communication
- Distributed coordination
- Enhanced routing patterns

### 📋 PHASE 4: Production Features (FUTURE)
- Full persistence layer
- Security implementation
- Monitoring and observability
- Kubernetes deployment

## DEVELOPMENT GUIDELINES

### Working with Existing Code
```bash
# Before making changes:
cargo test  # Ensure tests pass
cargo run   # Run Phase 2 verification

# When modifying:
git diff    # Review changes carefully
cargo check # Verify compilation
cargo test  # Ensure no regressions
```

### Code Quality Standards
- **Incremental**: Build on verified foundations
- **Tested**: All features must have verification
- **Documented**: Update docs to match reality
- **Proven**: Execute and verify before claiming success

## DOCUMENTATION GUIDELINES

### When Working in ms-framework-docs/
```bash
# Before making changes:
grep -r "YourTopic" ms-framework-docs/  # Check existing mentions
find ms-framework-docs -name "*.md" | xargs grep -l "component"  # Find related docs

# After making changes:
# Verify consistency
grep -A5 -B5 "modified_term" ms-framework-docs/**/*.md
```

### Documentation Standards
- **Technical Accuracy**: Specifications must be implementable
- **Consistency**: Terms, patterns, and concepts uniform across docs
- **Completeness**: Minimize TODO/TBD sections
- **Reality**: Documentation must reflect actual implementation state
- **Clarity**: Distinguish between "implemented" and "planned"

### Cross-Reference Management
When updating documentation:
1. Check all files that reference the modified component
2. Update related specifications
3. Ensure examples remain consistent
4. Validate integration points

## VERIFICATION FOR IMPLEMENTATION

### Code Verification
```bash
# Run Phase 2 verification suite
cargo run

# Run all tests
cargo test

# Check with NATS running
nats-server &  # Start NATS first
RUST_LOG=mistersmith=debug cargo run
```

### Documentation Accuracy Checks
```bash
# Verify documentation matches implementation
grep -r "Phase 2" *.md

# Check for outdated claims
grep -r "no implementation\|not implemented" . --include="*.md"

# Find TODO/FIXME items
grep -r "TODO\|TBD\|FIXME" ms-framework-docs/
```

### Consistency Verification
- Agent types mentioned consistently across docs
- Message schemas align between different files
- Architecture decisions propagated to all relevant docs
- No contradictions between specifications

## PROJECT-SPECIFIC GOTCHAS

### Documentation Complexity Trap
The specs describe a complex distributed system. Remember:
- Docs can be ambitious, implementation will be incremental
- Don't add complexity to make docs "complete"
- Focus on clarity over comprehensiveness

### Over-Engineering Trap
- NO implementing features beyond current phase
- NO adding complexity without proven need
- NO breaking working code for "better" designs
- Build incrementally on verified foundations

### Framework Documentation Navigation

```
ms-framework-docs/
├── core-architecture/     # System design, patterns
├── agent-domains/        # Agent type specifications  
├── data-management/      # Storage, messaging specs
├── transport/           # Communication protocols
├── security/            # Auth, authorization specs
├── operations/          # Deployment, monitoring
└── testing/             # Test specifications
```

## WHEN MODIFICATIONS ARE NEEDED

### Valid Documentation Changes
- Fixing inconsistencies between files
- Clarifying ambiguous specifications
- Adding missing integration details
- Removing over-engineered complexity
- Updating based on feasibility analysis

### Invalid Changes
- Adding implementation code
- Creating build files
- "Simplifying" by removing necessary complexity
- Adding features not requested
- Changing fundamental architecture without discussion

## COLLABORATION WITH HUMAN

### When to Ask for Clarification
- Specifications seem contradictory
- Major architectural decision needed
- Unsure if change aligns with project vision
- Found significant feasibility issue

### Progress Reporting
When completing documentation tasks:
```markdown
## Documentation Update Summary
- **Files Modified**: [list files]
- **Changes Made**: [specific changes]
- **Consistency Verified**: [what was checked]
- **Open Questions**: [any concerns]
```

## PARALLEL AGENT OPERATIONS

### Multi-Agent Documentation Work

The MisterSmith framework itself is designed for multi-agent orchestration. Similarly, documentation work benefits from parallel agent operations:

**Encouraged Parallel Patterns**:

- Multiple agents analyzing different documentation sections simultaneously
- Parallel consistency checking across document sets
- Concurrent validation of specifications against different criteria
- Distributed cross-reference verification

**Coordination Requirements**:

```bash
# When working in parallel, verify no conflicts:
git status  # Check for concurrent modifications
git pull   # Sync before major changes

# Claim your work area to prevent conflicts:
echo "Agent X working on: core-architecture/*.md" >> .agent-work-log
```

**Parallel Verification Example**:

```bash
# Agent 1: Check message schema consistency
find ms-framework-docs -name "*message*.md" -exec grep -l "MessageType" {} \;

# Agent 2: Verify agent lifecycle references
grep -r "lifecycle" ms-framework-docs/agent-domains/

# Agent 3: Validate security specifications
grep -r "authentication\|authorization" ms-framework-docs/security/
```

**Best Practices for Parallel Work**:

1. Communicate work scope clearly in commits
2. Focus on non-overlapping documentation areas
3. Use verification commands to ensure consistency
4. Coordinate on high-impact files (see ms-framework-docs/CLAUDE.md)

## FOLDER-SPECIFIC INSTRUCTIONS

Note: When entering `ms-framework-docs/` or other directories, additional CLAUDE.md files may load with folder-specific guidance. These complement but never override these project-level instructions.

## SUCCESS METRICS FOR DOCUMENTATION PHASE

### Phase is "Complete" When

- All planned features implemented and verified
- Tests passing consistently
- Documentation updated to reflect reality
- No regressions from previous phases
- Verification suite confirms functionality
- Ready to plan next phase

### NOT Success Metrics

- Lines of documentation written
- Number of specification files
- Complexity of described system
- "Comprehensive" coverage

## AUTOMATED TOOLING SUPPORT

### Documentation Verification Scripts

Create these scripts to automate verification tasks:

**consistency-check.sh**:

```bash
#!/bin/bash
# Automated consistency checker for ms-framework-docs

echo "=== Checking for TODOs ==="
TODO_COUNT=$(grep -r "TODO\|TBD\|FIXME" ms-framework-docs/ | wc -l)
echo "Found $TODO_COUNT TODO/TBD items"

echo -e "\n=== Checking cross-references ==="
# Find broken internal links
find ms-framework-docs -name "*.md" -exec grep -l "\[.*\](" {} \; | while read file; do
    grep -o '\[.*\](.*\.md[^)]*)' "$file" | while read link; do
        target=$(echo "$link" | sed 's/.*](//' | sed 's/).*//')
        if [[ ! -f "ms-framework-docs/$target" ]]; then
            echo "Broken link in $file: $target"
        fi
    done
done

echo -e "\n=== Version consistency ==="
grep -r "tokio.*=.*\"1\." ms-framework-docs/ | grep -v "1\.38" | head -5
grep -r "async-nats.*=.*\"0\." ms-framework-docs/ | grep -v "0\.34" | head -5
```

**parallel-work-tracker.sh**:

```bash
#!/bin/bash
# Track parallel agent work to prevent conflicts

WORK_LOG=".agent-work-log"

# Show current agent assignments
echo "=== Current Agent Work Assignments ==="
if [[ -f "$WORK_LOG" ]]; then
    cat "$WORK_LOG"
else
    echo "No agents currently working"
fi

# Check for potential conflicts
echo -e "\n=== Recent Git Activity ==="
git log --oneline -10 --pretty=format:"%h %s [%an]"

echo -e "\n\n=== Modified Files ==="
git status --porcelain
```

### Integration with CI/CD

When implementation phase begins, these checks can integrate with:

- Pre-commit hooks for documentation consistency
- GitHub Actions for automated verification
- Merge request validation pipelines

## REMEMBER

You are building MisterSmith INCREMENTALLY through proven phases. The elaborate distributed system in the specs is the DESTINATION. Your current job is to:

1. **Maintain** working Phase 2 functionality
2. **Plan** Phase 3 features carefully before implementation
3. **Verify** everything with actual execution
4. **Document** the reality, not aspirations
5. **Build** on proven foundations only

The best implementation proceeds smoothly, one verified step at a time. Focus on working code over theoretical completeness.
