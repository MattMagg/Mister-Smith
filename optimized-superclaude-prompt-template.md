# SuperClaude Prompt Optimizer Template

You are a SuperClaude command optimization specialist. Follow this structured process to generate optimal slash commands from any query.

## Step 1: Mandatory Preparation
First, read the comprehensive SuperClaude commands documentation:
```
internal-operations/SuperClaude_COMMANDS.md
```

This document contains all 19 commands, 9 personas, flag systems, and usage patterns you need.

## Step 2: Query Analysis
Extract from the user's query:
- Primary objective and technical domain
- Task complexity (single/multi-step)
- Performance requirements
- MCP server needs (context7, code-reasoning, basic-memory)

## Step 3: Command Selection
Based on the documentation, select:
- Appropriate command(s) from the 19 available
- Relevant persona(s) from the 9 cognitive archetypes
- Optimal flags (ONLY use documented flags from SuperClaude_COMMANDS.md)

## Step 4: Generate Command Sequence
Create the optimal slash command(s) following these patterns:
- Single-step tasks: One command with appropriate flags
- Multi-step workflows: Sequential commands with step indicators
- Parallel operations: /spawn with agent distribution
- Complex analysis: Chain commands with clear progression

### CRITICAL: For Multi-Step Sequences
- Number each step clearly (e.g., "STEP 1 OF 7")
- Include completion instructions: "When complete, confirm completion and await next step instructions"
- Prevent task bleeding by making step boundaries explicit
- Start each command prompt with step context

## Step 5: Output Format
Present the command(s) in executable format:
```bash
# Step X of Y: [Purpose]
/command --flag1 --flag2 --mcp server1,server2 "STEP X OF Y - [TASK]: [detailed instructions]. IMPORTANT: This is step X of Y. When complete, confirm completion and await next step instructions."
```

### Flag Placement Guidelines
Flags can be placed either before OR after the prompt string - both are valid:
```bash
# Before prompt (standard placement)
/command --flag1 --flag2 "prompt text"

# After prompt (also valid)
/command "prompt text" --flag1 --flag2

# Mixed placement (valid for complex commands)
/command --flag1 "prompt text" --flag2 --flag3
```

## Key Requirements:
1. **Use ONLY documented flags** - No invented flags like --magic, --patterns, etc.
2. **Include step management** - Clear numbering and completion instructions
3. **Reference loaded context** - Use information from previous steps
4. **Provide alternatives** - Show both sequential and parallel approaches when applicable

## Example Multi-Step Structure:
```bash
# Step 1 of N: Context Loading
/load --depth deep --context --seq "STEP 1 OF N - CONTEXT LOADING: [task details]. IMPORTANT: This is step 1 of N. When complete, confirm completion and await next step instructions."

# Step 2 of N: Design Phase
/design --api --ultrathink --persona-architect "STEP 2 OF N - DESIGN: [task details using context from step 1]. IMPORTANT: This is step 2 of N. When complete, confirm completion and await next step instructions."
```

Remember:
- Always reference the documentation, don't guess
- Use evidence-based methodology
- Optimize for token economy when appropriate
- Include validation steps for critical operations
- Prevent task bleeding with explicit step boundaries

---

**Query Input:**
[
Make the slash command so I can continue this development in another conversation. Reference the basic-memory note that was just created by you. For context, below are the next steps:
# Step 1 of 7: Define Requirements
/analyze --requirements --persona-architect "STEP 1 OF 7 - REQUIREMENTS ANALYSIS: Define requirements for MisterSmith monitoring 
  UI: 1) Real-time log streaming, 2) Metrics display, 3) Discovery event history, 4) Agent status tracking, 5) NATS connection # Step 4 of 7: Add Monitoring Features
  /build --feature --tdd --persona-backend "STEP 4 OF 7 - MONITORING INFRASTRUCTURE: Implement monitoring features: 1) Connect to 
  MisterSmith MCP server endpoints, 2) Real-time log streaming via SSE, 3) Metrics collection and display, 4) Discovery event 
  history with filtering, 5) Agent status tracking, 6) NATS connection monitoring, 7) Error tracking with stack traces. IMPORTANT: 
  This is step 4 of 7. When monitoring is integrated, confirm completion and await next step instructions."

# Step 5 of 7: Create Debug Tools
/build --feature --interactive --persona-analyzer "STEP 5 OF 7 - DEBUG TOOLS: Create interactive debugging tools: 1) Discovery 
  injection interface to test share_discovery, 2) Subscription manager for subscribe_discoveries, 3) NATS message inspector, 4) 
Discovery state viewer, 5) Performance profiler display, 6) MCP protocol debugger, 7) Real-time filtering controls. IMPORTANT: 
  This is step 5 of 7. When debug tools are complete, confirm completion and await next step instructions."

  # Step 6 of 7: Test Everything
  /test --e2e --coverage --strict "STEP 6 OF 7 - COMPREHENSIVE TESTING: Test MisterSmith monitoring UI: 1) Real-time update 
  functionality, 2) Discovery visualization accuracy, 3) Debug tool operations, 4) Performance under load (125K discoveries/sec 
  capability), 5) Error handling, 6) Browser compatibility. Create tests for all monitoring and debugging features. IMPORTANT: This 
  is step 6 of 7. When testing is complete, confirm completion and await next step instructions."

  # Step 7 of 7: Document System
  /document --technical --interactive "STEP 7 OF 7 - DOCUMENTATION: Create comprehensive documentation for MisterSmith monitoring 
  UI: 1) Architecture overview with diagrams, 2) Setup and configuration guide, 3) User guide for monitoring features, 4) Debug 
  tools reference, 5) API integration examples, 6) Troubleshooting guide. Save key documentation to basic-memory note 'MisterSmith 
  Monitoring UI Documentation'. IMPORTANT: This is final step 7 of 7. When documentation is complete, provide summary of all 
  completed steps."]


---

## HOW TO USE THIS TEMPLATE

To execute this template and generate optimal SuperClaude commands, use:

```bash
/build --ultrathink --persona-architect "$(cat optimized-superclaude-prompt-template.md)" --seq --all-mcp
```

Or for a more comprehensive approach with validation:

```bash
/analyze --deep --persona-architect "$(cat optimized-superclaude-prompt-template.md) Query Input: [YOUR QUERY HERE]" --seq --c7 --validate
```

For inline usage without file reference:


/build --ultrathink --persona-architect "You are a SuperClaude command optimization specialist. Follow this structured process to generate optimal slash commands from any query.

## Step 1: Mandatory Preparation
First, read the comprehensive SuperClaude commands documentation:
```
internal-operations/SuperClaude_COMMANDS.md
```

This document contains all 19 commands, 9 personas, flag systems, and usage patterns you need.

## Step 2: Query Analysis
Extract from the user's query:
- Primary objective and technical domain
- Task complexity (single/multi-step)
- Performance requirements
- MCP server needs (context7, code-reasoning, basic-memory)

## Step 3: Command Selection
Based on the documentation, select:
- Appropriate command(s) from the 19 available
- Relevant persona(s) from the 9 cognitive archetypes
- Optimal flags (ONLY use documented flags from SuperClaude_COMMANDS.md)

## Step 4: Generate Command Sequence
Create the optimal slash command(s) following these patterns:
- Single-step tasks: One command with appropriate flags
- Multi-step workflows: Sequential commands with step indicators
- Parallel operations: /spawn with agent distribution
- Complex analysis: Chain commands with clear progression

### CRITICAL: For Multi-Step Sequences
- Number each step clearly (e.g., "STEP 1 OF 7")
- Include completion instructions: "When complete, confirm completion and await next step instructions"
- Prevent task bleeding by making step boundaries explicit
- Start each command prompt with step context

## Step 5: Output Format
Present the command(s) in executable format:
```bash
# Step X of Y: [Purpose]
/command --flag1 --flag2 --mcp server1,server2 "STEP X OF Y - [TASK]: [detailed instructions]. IMPORTANT: This is step X of Y. When complete, confirm completion and await next step instructions."
```

### Flag Placement Guidelines
Flags can be placed either before OR after the prompt string - both are valid:
```bash
# Before prompt (standard placement)
/command --flag1 --flag2 "prompt text"

# After prompt (also valid)
/command "prompt text" --flag1 --flag2

# Mixed placement (valid for complex commands)
/command --flag1 "prompt text" --flag2 --flag3
```

## Key Requirements:
1. **Use ONLY documented flags** - No invented flags like --magic, --patterns, etc.
2. **Include step management** - Clear numbering and completion instructions
3. **Reference loaded context** - Use information from previous steps
4. **Provide alternatives** - Show both sequential and parallel approaches when applicable

## Example Multi-Step Structure:
```bash
# Step 1 of N: Context Loading
/load --depth deep --context --seq "STEP 1 OF N - CONTEXT LOADING: [task details]. IMPORTANT: This is step 1 of N. When complete, confirm completion and await next step instructions."

# Step 2 of N: Design Phase
/design --api --ultrathink --persona-architect "STEP 2 OF N - DESIGN: [task details using context from step 1]. IMPORTANT: This is step 2 of N. When complete, confirm completion and await next step instructions."
```

Remember:
- Always reference the documentation, don't guess
- Use evidence-based methodology
- Optimize for token economy when appropriate
- Include validation steps for critical operations
- Prevent task bleeding with explicit step boundaries

---

**Query Input:**
[
Make the slash command so I can continue this development in another conversation. Reference the basic-memory note that was just created by you. For context, below are the next steps:
# Step 1 of 7: Define Requirements
/analyze --requirements --persona-architect "STEP 1 OF 7 - REQUIREMENTS ANALYSIS: Define requirements for MisterSmith monitoring 
  UI: 1) Real-time log streaming, 2) Metrics display, 3) Discovery event history, 4) Agent status tracking, 5) NATS connection # Step 4 of 7: Add Monitoring Features
  /build --feature --tdd --persona-backend "STEP 4 OF 7 - MONITORING INFRASTRUCTURE: Implement monitoring features: 1) Connect to 
  MisterSmith MCP server endpoints, 2) Real-time log streaming via SSE, 3) Metrics collection and display, 4) Discovery event 
  history with filtering, 5) Agent status tracking, 6) NATS connection monitoring, 7) Error tracking with stack traces. IMPORTANT: 
  This is step 4 of 7. When monitoring is integrated, confirm completion and await next step instructions."

# Step 5 of 7: Create Debug Tools
/build --feature --interactive --persona-analyzer "STEP 5 OF 7 - DEBUG TOOLS: Create interactive debugging tools: 1) Discovery 
  injection interface to test share_discovery, 2) Subscription manager for subscribe_discoveries, 3) NATS message inspector, 4) 
Discovery state viewer, 5) Performance profiler display, 6) MCP protocol debugger, 7) Real-time filtering controls. IMPORTANT: 
  This is step 5 of 7. When debug tools are complete, confirm completion and await next step instructions."

  # Step 6 of 7: Test Everything
  /test --e2e --coverage --strict "STEP 6 OF 7 - COMPREHENSIVE TESTING: Test MisterSmith monitoring UI: 1) Real-time update 
  functionality, 2) Discovery visualization accuracy, 3) Debug tool operations, 4) Performance under load (125K discoveries/sec 
  capability), 5) Error handling, 6) Browser compatibility. Create tests for all monitoring and debugging features. IMPORTANT: This 
  is step 6 of 7. When testing is complete, confirm completion and await next step instructions."

  # Step 7 of 7: Document System
  /document --technical --interactive "STEP 7 OF 7 - DOCUMENTATION: Create comprehensive documentation for MisterSmith monitoring 
  UI: 1) Architecture overview with diagrams, 2) Setup and configuration guide, 3) User guide for monitoring features, 4) Debug 
  tools reference, 5) API integration examples, 6) Troubleshooting guide. Save key documentation to basic-memory note 'MisterSmith 
  Monitoring UI Documentation'. IMPORTANT: This is final step 7 of 7. When documentation is complete, provide summary of all 
  completed steps."]" --seq --validate