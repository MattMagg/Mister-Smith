# Claude Code CLI Integration Blueprint for MisterSmith

## How the REPL Pattern Maps to MisterSmith Architecture

This blueprint shows exactly how the Claude Code CLI's main REPL + Task spawning pattern integrates with the existing MisterSmith framework.

## Core Architecture Mapping

### Current MisterSmith Components → Claude CLI Pattern

```rust
// MisterSmith Orchestrator (Main REPL equivalent)
pub struct MisterSmithOrchestrator {
    // Existing components
    agent_pool: Arc<AgentPool>,
    supervision_tree: SupervisionTree,
    nats_transport: Option<NatsTransport>,
    
    // Claude CLI pattern additions
    conversation_context: ConversationContext,
    task_planner: TaskPlanner,
    result_synthesizer: ResultSynthesizer,
}

// Maps to Claude's main REPL session that:
// 1. Maintains conversation context
// 2. Plans task decomposition  
// 3. Spawns sub-agents via Task tool
// 4. Synthesizes results
```

## Integration Pattern

### 1. User Request Handling (Main REPL)
```rust
impl MisterSmithOrchestrator {
    /// Main REPL session handling user requests
    pub async fn handle_request(&mut self, request: &str) -> Result<String> {
        // Analyze request with full context (like Claude main session)
        let analysis = self.analyze_with_context(request).await?;
        
        match analysis.complexity {
            Complexity::Simple => {
                // Handle directly in main session
                self.execute_simple_task(request).await
            }
            Complexity::Complex => {
                // Decompose and spawn sub-tasks (like Claude's Task tool)
                self.orchestrate_complex_task(analysis).await
            }
        }
    }
    
    async fn orchestrate_complex_task(&mut self, analysis: TaskAnalysis) -> Result<String> {
        // This is where Claude CLI's Task pattern shines
        let subtasks = self.task_planner.decompose(&analysis)?;
        
        // Spawn focused sub-agents
        let mut task_handles = vec![];
        for subtask in subtasks {
            let handle = self.spawn_subtask_agent(subtask).await?;
            task_handles.push(handle);
        }
        
        // Collect and synthesize (main retains context)
        let results = self.collect_results(task_handles).await?;
        self.result_synthesizer.synthesize(results, &self.conversation_context)
    }
}
```

### 2. Task Spawning (Claude's Task Tool)
```rust
impl MisterSmithOrchestrator {
    /// Equivalent to Claude's Task tool invocation
    async fn spawn_subtask_agent(&self, subtask: SubTask) -> Result<TaskHandle> {
        // Get agent from pool (reuse existing infrastructure)
        let agent = Agent::new(AgentConfig {
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_turns: Some(subtask.estimated_turns),
            allowed_tools: Some(subtask.required_tools),
            enable_mcp: true,
            timeout_seconds: subtask.timeout,
            memory_limit_mb: Some(256),
        });
        
        // Register with pool (existing pattern)
        let permit = self.agent_pool.register(agent.clone()).await?;
        
        // Spawn with focused context
        let context_slice = self.extract_context_for_task(&subtask);
        let handle = tokio::spawn(async move {
            let result = agent.execute_with_context(&subtask.prompt, context_slice).await;
            // Permit dropped here, releasing agent
            result
        });
        
        Ok(TaskHandle { 
            task_id: subtask.id,
            handle,
            subtask_type: subtask.task_type,
        })
    }
}
```

### 3. Context Management (Hierarchical)
```rust
/// Main REPL maintains full context
pub struct ConversationContext {
    // Full conversation history
    messages: Vec<Message>,
    // Project understanding
    project_context: ProjectContext,
    // Current objectives
    objectives: Vec<Objective>,
    // Results from previous tasks
    task_results: HashMap<TaskId, TaskResult>,
}

impl MisterSmithOrchestrator {
    /// Extract focused context slice for sub-agent
    fn extract_context_for_task(&self, subtask: &SubTask) -> FocusedContext {
        FocusedContext {
            // Only what this specific task needs
            relevant_files: self.get_files_for_task(subtask),
            specific_objective: subtask.objective.clone(),
            constraints: subtask.constraints.clone(),
            // No access to full conversation or other task results
        }
    }
}
```

## Concrete Example: Multi-File Refactoring

### User Request
```
"Refactor the authentication system to use JWT tokens instead of sessions"
```

### Main REPL Orchestration
```rust
async fn handle_refactoring_request(&mut self) -> Result<String> {
    // 1. Main analyzes with full context
    let plan = self.analyze_refactoring_scope().await?;
    
    // 2. Decompose into focused subtasks
    let subtasks = vec![
        SubTask {
            id: TaskId::new(),
            task_type: TaskType::Analysis,
            prompt: "Analyze current session-based auth implementation in src/auth/",
            required_tools: vec!["read".into(), "grep".into()],
            timeout: 60,
        },
        SubTask {
            id: TaskId::new(),
            task_type: TaskType::Implementation,
            prompt: "Implement JWT token generation and validation utilities",
            required_tools: vec!["write".into(), "edit".into()],
            timeout: 120,
        },
        SubTask {
            id: TaskId::new(),
            task_type: TaskType::Migration,
            prompt: "Update auth middleware to use JWT instead of sessions",
            required_tools: vec!["edit".into(), "multiedit".into()],
            timeout: 120,
        },
        SubTask {
            id: TaskId::new(),
            task_type: TaskType::Testing,
            prompt: "Write tests for JWT authentication flow",
            required_tools: vec!["write".into(), "bash".into()],
            timeout: 90,
        },
    ];
    
    // 3. Spawn sub-agents (like Claude's Task tool)
    let mut results = HashMap::new();
    for subtask in subtasks {
        let handle = self.spawn_subtask_agent(subtask.clone()).await?;
        results.insert(subtask.id, handle);
    }
    
    // 4. Collect results as they complete
    let mut completed_results = vec![];
    for (task_id, handle) in results {
        match handle.await? {
            Ok(result) => completed_results.push((task_id, result)),
            Err(e) => {
                // Main can handle failures intelligently
                self.handle_subtask_failure(task_id, e).await?;
            }
        }
    }
    
    // 5. Synthesize with full context
    self.synthesize_refactoring_results(completed_results).await
}
```

## Key Integration Points

### 1. Agent Pool Integration
```rust
// Existing AgentPool works perfectly for Task pattern
impl AgentPool {
    // Already supports temporary agent registration
    pub async fn register(&self, agent: Agent) -> Result<PoolPermit> {
        // Permit ensures cleanup when task completes
        // This maps directly to Claude's Task lifecycle
    }
}
```

### 2. Supervision Integration  
```rust
// AgentPoolSupervisor already handles hierarchical supervision
impl AgentPoolSupervisor {
    // Main REPL is the top-level supervisor
    // Sub-agents are supervised children
    // Existing implementation supports this pattern
}
```

### 3. NATS Integration
```rust
// Optional: Use NATS for sub-agent coordination
impl MisterSmithOrchestrator {
    async fn coordinate_subtasks(&self, tasks: Vec<SubTask>) -> Result<()> {
        // Publish task assignments
        for task in tasks {
            self.nats_transport.publish(
                Subject::task_assignment(&task.id),
                &task
            ).await?;
        }
        
        // Subscribe to results
        let mut results = self.nats_transport
            .subscribe(Subject::task_results())
            .await?;
            
        // But main REPL still controls flow
    }
}
```

## Benefits Realized

### 1. Simplicity
```rust
// User interaction is simple
let response = orchestrator.handle_request("Do complex task").await?;
// Everything else is hidden behind the orchestrator
```

### 2. Resource Efficiency
```rust
// Only spawn agents when needed
if needs_parallel_work(&request) {
    spawn_workers(&subtasks).await?;
} 
// Otherwise handle in main session
```

### 3. Context Isolation
```rust
// Sub-agents can't pollute main context
// Sub-agents can't see each other's work
// Main synthesizes coherent result
```

### 4. Failure Handling
```rust
// Failures are localized
match subtask_result {
    Ok(r) => integrate_result(r),
    Err(_) => {
        // Main can retry, skip, or compensate
        // Other subtasks unaffected
    }
}
```

## Migration Path

### Phase 3 Enhancement
```rust
// Extend existing Agent to support Claude CLI pattern
impl Agent {
    /// New method for context-aware execution
    pub async fn execute_with_context(
        &self,
        prompt: &str,
        context: FocusedContext,
    ) -> Result<String> {
        // Format prompt with context
        let full_prompt = format!(
            "Context: {:?}\n\nTask: {}",
            context, prompt
        );
        
        // Use existing execute infrastructure
        self.execute(&full_prompt).await
    }
}
```

## Conclusion

The Claude Code CLI pattern integrates seamlessly with MisterSmith's existing architecture:

1. **AgentPool** → Perfect for temporary Task agents
2. **SupervisionTree** → Natural hierarchical structure
3. **NATS Transport** → Optional coordination layer
4. **Main Orchestrator** → Maps to Claude's main REPL
5. **Agent lifecycle** → Matches Task tool lifecycle

No architectural changes needed - just extend the orchestrator to follow the REPL + Task spawning pattern that Claude Code CLI has proven successful.