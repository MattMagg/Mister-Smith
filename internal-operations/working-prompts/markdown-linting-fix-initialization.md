# Markdown Linting Fix - SuperClaude Initialization Command

## Master Initialization Command

```bash
# Create the main linting fix task with full context
/task:create "Fix 1016 markdown linting errors in internal-operations blocking CI pipeline" \
  --persona-qa --ultrathink --validate --strict --evidence

# Load all context and memory references
/load --depth deep --context --patterns --relationships --structure --health \
  --memory "GitHub Actions Documentation Workflow Debug Session - 2025-07-06" \
  --memory "Markdown Linting Fix - SuperClaude Multi-Agent Deployment Plan"
```

## Quick Deploy All Teams

```bash
# Deploy all 40 agents across teams in parallel
/spawn --task "Deploy 40-agent markdown linting resolution operation" \
       --parallel --specialized --collaborative --sync --merge \
       --validate --strict --coverage --evidence
```

## Manual Step-by-Step Commands

### 1. Load Context

```bash
/load --depth deep --context --patterns --relationships --structure --health
/load --memory "GitHub Actions Documentation Workflow Debug Session - 2025-07-06"
/load --memory "Markdown Linting Fix - SuperClaude Multi-Agent Deployment Plan"
```

### 2. Analyze Current State

```bash
/analyze --forensic --deep --code --files internal-operations/ \
         --persona-qa --strict --evidence --coverage
```

### 3. Deploy Teams (Run in parallel)

```bash
# Alpha Team - Line Length (12 agents)
/spawn --task "Fix MD013 line length errors" --parallel --specialized \
       --collaborative --sync --persona-editor --validate --strict

# Beta Team - Duplicate Headings (8 agents)
/spawn --task "Fix MD024 duplicate headings" --parallel --specialized \
       --collaborative --sync --persona-architect --think --evidence

# Gamma Team - Multiple H1s (6 agents)  
/spawn --task "Fix MD025 multiple H1s" --parallel --specialized \
       --collaborative --sync --persona-architect --think-hard --validate

# Delta Team - Code Fences (8 agents)
/spawn --task "Fix MD040 code fence languages" --parallel --specialized \
       --collaborative --sync --persona-backend --seq --validate
```

### 4. Validation

```bash
# Epsilon Team - Final Validation (6 agents)
/spawn --task "Validate all fixes zero errors" --specialized \
       --collaborative --sync --persona-qa --ultrathink --strict \
       --evidence --coverage --merge
```

### 5. Completion

```bash
/git --commit --validate --pre-commit
/task:complete lint-fix-001
/analyze --code --validate --files .github/workflows/ --evidence
```

## Key SuperClaude Flags Used

### Universal Flags
- `--ultrathink`: Maximum analysis depth (~32K tokens) for complex decisions
- `--think-hard`: Architecture-level analysis (~10K tokens)
- `--think`: Multi-file analysis (~4K tokens)
- `--validate`: Enhanced safety checks before execution
- `--strict`: Zero-tolerance mode for quality
- `--evidence`: Include sources and documentation
- `--coverage`: Comprehensive analysis coverage

### MCP Server Control
- `--seq`: Sequential thinking for logical analysis
- `--c7`: Context7 for documentation lookup

### Personas
- `--persona-qa`: Quality assurance expertise
- `--persona-architect`: System architecture mindset
- `--persona-backend`: Backend development focus
- `--persona-editor`: Text editing and formatting

### Execution Flags
- `--parallel`: Concurrent execution
- `--specialized`: Domain-specific expertise
- `--collaborative`: Multi-agent coordination
- `--sync`: Synchronize results
- `--merge`: Combine outputs

## Document References

### Created Documents
1. **Basic-Memory Note**: `projects/mister-smith/workflows/Markdown Linting Fix - SuperClaude Multi-Agent Deployment Plan.md`
   - Complete 40-agent deployment strategy
   - Team specifications and error distribution

2. **Main Deployment Plan**: `/internal-operations/working-prompts/deployment-plans/Markdown_Linting_Resolution_SuperClaude_Deployment.md`
   - Full execution phases with proper SuperClaude commands
   - Error handling procedures and progress tracking

3. **Initialization Commands**: `/internal-operations/working-prompts/markdown-linting-fix-initialization.md` (this file)
   - Quick-start commands and manual steps

### Referenced Context
- **Previous Session**: [[GitHub Actions Documentation Workflow Debug Session - 2025-07-06]]
- **Summary**: [[MisterSmith Workflow Automation - Session Summary]]
- **Target Directory**: `/internal-operations/` with 1016 errors
- **Success Metric**: Zero linting errors, green CI pipeline