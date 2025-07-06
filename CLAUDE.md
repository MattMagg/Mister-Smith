# MisterSmith Project Instructions

## PROJECT CONTEXT

MisterSmith is a multi-agent orchestration framework for distributed AI systems. **CRITICAL**: Currently in DOCUMENTATION PHASE ONLY. No implementation exists. The framework specifications are being developed in `ms-framework-docs/`.

Your role: Work within the documentation framework to refine, validate, and prepare specifications for eventual implementation.

## CURRENT PROJECT STATE

### What Exists

- 📁 `ms-framework-docs/` - Technical framework documentation (150+ pages)
- 📁 `internal-operations/` - Working prompts and planning documents
- ✅ Comprehensive architectural specifications
- ✅ Detailed component designs

### What DOES NOT Exist

- ❌ No Rust implementation
- ❌ No working agents
- ❌ No actual code beyond documentation
- ❌ No Cargo.toml or src/ directory

## CRITICAL CONSTRAINTS FOR AGENTS

### Documentation Phase Rules

**FORBIDDEN**: Creating implementation code before documentation is complete and validated
**FORBIDDEN**: Adding Cargo.toml, src/, or other implementation scaffolding
**FORBIDDEN**: Claiming any component "works" or is "ready"
**REQUIRED**: Focus on documentation quality, consistency, and completeness

### Reality Check Questions

Before ANY action:
1. Am I working on documentation or trying to implement?
2. Does this change improve specification clarity?
3. Is this adding unnecessary complexity to the docs?
4. Am I maintaining consistency across all framework docs?

## DEVELOPMENT PHASES

### CURRENT PHASE: Documentation & Specification

- Review and refine existing specifications
- Ensure consistency across all documentation
- Identify gaps or contradictions
- Validate technical feasibility
- NO IMPLEMENTATION CODE

### FUTURE PHASE 1: Foundation Setup (NOT YET)
When documentation is complete:
- Create minimal Cargo.toml
- Single "hello world" agent
- Basic tokio runtime proof
- NO complex features

### FUTURE PHASE 2: Single Agent (NOT YET)
After foundation verified:
- Implement ONE agent type
- Basic message handling
- Simple lifecycle management
- NO distributed features

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
- **Completeness**: Minimize TODO/TBD sections (currently only 1 in code examples)
- **Reality**: Don't describe features as implemented

### Cross-Reference Management
When updating documentation:
1. Check all files that reference the modified component
2. Update related specifications
3. Ensure examples remain consistent
4. Validate integration points

## VERIFICATION FOR DOCUMENTATION PHASE

### Document Quality Checks
```bash
# Check for incomplete sections
grep -r "TODO\|TBD\|FIXME" ms-framework-docs/

# Find inconsistent terminology
# (manually review for now, no working code to test against)

# Verify no implementation claims
grep -r "works\|implemented\|ready\|production" ms-framework-docs/
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

### Premature Implementation Trap
- NO creating stub code "to test the design"
- NO scaffolding projects "for later"
- NO proof-of-concept implementations
- Stay in documentation until explicitly directed otherwise

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

### Documentation is "Ready" When

- All specifications complete and consistent
- Minimal TODO/TBD sections (currently only 1 `todo!()` in code example)
- Cross-references verified
- Technical feasibility validated
- Human approves moving to implementation

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

You are building the FOUNDATION for MisterSmith through careful documentation. The elaborate distributed system in the specs is the DESTINATION. Your current job is to ensure the documentation is clear, consistent, and implementable - not to implement it.

The best documentation enables future implementation to proceed smoothly, one verified step at a time. Focus on clarity and consistency, not complexity or completeness.
