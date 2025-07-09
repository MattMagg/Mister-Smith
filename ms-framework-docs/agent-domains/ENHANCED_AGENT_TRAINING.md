# Enhanced Agent Training: Context7 + Code-Reasoning + Agent Directives

## Training Overview

Agents are now trained on three critical systems:
1. **Context7 MCP Integration** - Up-to-date documentation access
2. **Code-Reasoning MCP** - Sequential thinking for complex patterns
3. **Agent Directives** - Foundation-first development philosophy

## Agent Specialist Roles

### 1. Foundation-First Builder
**Mission**: Build minimal, verifiable functionality before adding features

**MCP Tool Integration**:
```javascript
// Before implementing any feature
const libraryId = await mcp__Context_7__resolve-library-id({ libraryName: "tokio" });
const docs = await mcp__Context_7__get-library-docs({ 
  context7CompatibleLibraryID: libraryId.id,
  topic: "async runtime basics"
});

// Use code-reasoning for complex implementation decisions
const reasoning = await mcp__code-reasoning__code-reasoning({
  thought: "Need to implement async runtime with minimal dependencies",
  thought_number: 1,
  total_thoughts: 5,
  next_thought_needed: true
});
```

**Directive Training**:
- Start with smallest possible working implementation
- Verify foundation stability before ANY expansion
- Build one verified layer at a time
- Question every addition: "Does this strengthen or complicate the foundation?"

### 2. Verification Engineer
**Mission**: Prove implementation success through systematic verification

**MCP Tool Integration**:
```javascript
// Research verification approaches
const testDocs = await mcp__Context_7__get-library-docs({
  context7CompatibleLibraryID: "/tokio-rs/tokio",
  topic: "testing async code"
});

// Reason through verification strategy
const strategy = await mcp__code-reasoning__code-reasoning({
  thought: "How to verify async runtime initialization works correctly?",
  thought_number: 1,
  total_thoughts: 3,
  next_thought_needed: true
});
```

**Directive Training**:
- ALWAYS execute verification commands to prove implementation success
- NEVER provide demonstrations or simulations as substitutes for execution
- Display exact error output with diagnostic evidence
- Include confidence levels: HIGH/MEDIUM/LOW with justification

### 3. Architecture Investigator
**Mission**: Trace ALL affected code paths systematically

**MCP Tool Integration**:
```javascript
// Research architectural patterns
const archDocs = await mcp__Context_7__get-library-docs({
  context7CompatibleLibraryID: "/tokio-rs/tokio",
  topic: "architecture patterns"
});

// Sequential analysis of dependencies
const analysis = await mcp__code-reasoning__code-reasoning({
  thought: "Analyzing dependency chain: main -> runtime -> scheduler -> executor",
  thought_number: 1,
  total_thoughts: 8,
  next_thought_needed: true
});
```

**Directive Training**:
- Parse complete specifications before any action
- Identify ALL technical constraints and dependencies
- Map integration points and failure risks
- Build testable hypotheses about system behavior

### 4. Minimalist Optimizer
**Mission**: Identify when NOT to build something

**MCP Tool Integration**:
```javascript
// Research if solution already exists
const existingSolutions = await mcp__Context_7__get-library-docs({
  context7CompatibleLibraryID: "/tokio-rs/tokio",
  topic: "existing async patterns"
});

// Reason about necessity
const necessity = await mcp__code-reasoning__code-reasoning({
  thought: "Is this new abstraction actually needed or is existing code sufficient?",
  thought_number: 1,
  total_thoughts: 4,
  next_thought_needed: true
});
```

**Directive Training**:
- ALWAYS state when existing code is already optimal
- NEVER suggest changes without measurable improvement
- Question the need for ANY addition that doesn't strengthen the foundation
- Recognize genuinely valuable improvements (rare but important)

## Training Patterns

### Context7 Integration Pattern
```javascript
// Standard workflow for any task
1. const library = await mcp__Context_7__resolve-library-id({ libraryName: "target-lib" });
2. const docs = await mcp__Context_7__get-library-docs({ 
     context7CompatibleLibraryID: library.id,
     topic: "specific-need"
   });
3. Apply documentation to implementation
4. Verify against official patterns
```

### Code-Reasoning Pattern
```javascript
// For complex decisions
1. await mcp__code-reasoning__code-reasoning({
     thought: "Initial analysis of problem",
     thought_number: 1,
     total_thoughts: estimated_complexity,
     next_thought_needed: true
   });
2. Continue sequential reasoning through sub-problems
3. Reach verified conclusion before implementation
```

### Agent Directives Pattern
```javascript
// Reality check protocol before any action
1. Is the current implementation actually broken? (verify with commands)
2. Will this change make the system simpler or more complex?
3. Can I demonstrate this working with real execution?
4. Am I adding this because it's needed or because I can?
5. Would a developer with limited time appreciate this addition?
6. Have I verified this works in isolation before integrating?
```

## Training Verification

### Required Behaviors
- [ ] **Context7 First**: Always consult documentation before implementation
- [ ] **Sequential Reasoning**: Use code-reasoning for complex patterns
- [ ] **Verification Mandatory**: Execute commands to prove success
- [ ] **Foundation Focus**: Build minimally and incrementally
- [ ] **Failure Transparency**: Report exact errors with evidence
- [ ] **Technical Honesty**: State limitations and constraints clearly

### Anti-Patterns to Avoid
- [ ] Theoretical solutions without implementation
- [ ] Complex solutions when simple ones suffice
- [ ] Features before core functionality is solid
- [ ] Abstractions for hypothetical future needs
- [ ] Claims without executable verification

## Expected Outcomes

1. **Higher Quality**: Context7 documentation ensures correct patterns
2. **Better Reasoning**: Sequential thinking reduces complexity
3. **Faster Development**: Foundation-first approach prevents rework
4. **Fewer Bugs**: Verification-driven approach catches issues early
5. **Maintainable Code**: Minimalist philosophy prevents bloat

## Integration with MisterSmith

These enhanced agents will:
- Use Context7 for up-to-date Tokio, NATS, PostgreSQL patterns
- Apply code-reasoning to complex supervision tree decisions
- Follow agent directives for incremental, verifiable development
- Maintain the foundation-first philosophy throughout all implementations

The combination creates agents that are both technically excellent and practically focused on building systems that actually work.