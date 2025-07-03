---
title: revised-agent-orchestration
type: note
permalink: revision-swarm/data-management/revised-agent-orchestration
tags:
- '#revised-document #agent-orchestration #foundation-focus'
---

# Agent Orchestration & Supervision Architecture
## Foundation Patterns Guide

> **Canonical Reference**: See `tech-framework.md` for authoritative technology stack specifications

## Executive Summary

This document defines foundational agent orchestration and supervision patterns using Rust's actor model with Tokio runtime. Focus is on basic supervision trees, agent lifecycle management, and simple coordination patterns suitable for learning distributed systems concepts.

## 1. Basic Agent Architecture

### 1.1 Agent Types

```pseudocode
ENUM AgentType {
    SUPERVISOR,    // Manages other agents
    WORKER,        // Performs tasks
    COORDINATOR,   // Coordinates workflows
    MONITOR,       // Observes system state
    PLANNER,       // Decomposes goals into tasks
    EXECUTOR,      // Carries out atomic actions
    CRITIC,        // Validates outcomes
    ROUTER,        // Assigns tasks to agents
    MEMORY         // Stores and retrieves knowledge
}

INTERFACE Agent {
    FUNCTION start() -> Result
    FUNCTION stop() -> Result
    FUNCTION handleMessage(message: Message) -> Result
    FUNCTION getStatus() -> AgentStatus
}
```

### 1.2 Core Agent Role Taxonomies

#### Planner Agent
- **Purpose**: Decomposes high-level goals into concrete subtasks
- **Interface Pattern**:
```rust
trait Planner {
    async fn create_plan(&self, goal: Goal) -> Result<TaskList, Error>;
    async fn refine_plan(&self, feedback: CriticFeedback) -> Result<TaskList, Error>;
}

struct TaskList {
    tasks: Vec<Task>,
    dependencies: HashMap<TaskId, Vec<TaskId>>,
    priority_order: Vec<TaskId>,
}
```

#### Executor Agent
- **Purpose**: Carries out atomic actions and subtasks
- **Interface Pattern**:
```rust
trait Executor {
    async fn execute_task(&self, task: Task) -> Result<TaskOutput, Error>;
    fn can_execute(&self, task_type: &TaskType) -> bool;
}

enum TaskOutput {
    Success(Value),
    PartialResult(Value, Vec<SubTask>),
    Failed(Error),
}
```

#### Critic Agent
- **Purpose**: Validates outcomes against goals and quality criteria
- **Interface Pattern**:
```rust
trait Critic {
    async fn evaluate(&self, output: TaskOutput, criteria: QualityCriteria) -> CriticFeedback;
    async fn validate_plan(&self, plan: TaskList) -> ValidationResult;
}

struct CriticFeedback {
    score: f32,
    issues: Vec<Issue>,
    suggestions: Vec<Improvement>,
}
```

#### Router Agent
- **Purpose**: Assigns tasks to appropriate specialized agents
- **Interface Pattern**:
```rust
trait Router {
    async fn route_task(&self, task: Task) -> AgentId;
    async fn get_agent_capabilities(&self, agent_id: AgentId) -> Vec<Capability>;
    async fn balance_load(&self, tasks: Vec<Task>) -> HashMap<AgentId, Vec<Task>>;
}
```

#### Memory Agent
- **Purpose**: Stores and retrieves shared knowledge
- **Interface Pattern**:
```rust
trait Memory {
    async fn store(&self, key: String, value: Value, metadata: Metadata) -> Result<(), Error>;
    async fn retrieve(&self, key: String) -> Option<(Value, Metadata)>;
    async fn query(&self, pattern: QueryPattern) -> Vec<(String, Value)>;
}
```

### 1.3 Agent Lifecycle

```pseudocode
ENUM AgentState {
    INITIALIZING,
    RUNNING,
    PAUSED,
    STOPPING,
    TERMINATED
}

CLASS AgentLifecycle {
    PRIVATE state: AgentState
    
    FUNCTION transition(newState: AgentState) -> Result {
        IF isValidTransition(state, newState) THEN
            state = newState
            RETURN Success()
        ELSE
            RETURN Failure("Invalid state transition")
        END IF
    }
}
```

## 2. Supervision Patterns

### 2.1 Hub-and-Spoke Supervisor Pattern

Central routing logic with domain-specific delegation:

```rust
trait Supervisor {
    async fn route_task(&self, task: Task) -> AgentId {
        // Central routing logic
        match task.task_type {
            TaskType::Research => self.find_agent("researcher"),
            TaskType::Code => self.find_agent("coder"),
            _ => self.default_agent()
        }
    }
}

// Anti-pattern: Monolithic supervisor
// ❌ Single supervisor managing all agents directly
// ✅ Hierarchical supervisors with domain-specific delegation
```

### 2.2 Event-Driven Message Bus Pattern

```rust
struct MessageBus {
    channels: HashMap<AgentId, mpsc::Sender<Message>>,
    event_loop: tokio::task::JoinHandle<()>,
}

impl MessageBus {
    async fn publish(&self, msg: Message) {
        match msg.routing {
            Routing::Broadcast => self.broadcast_all(msg).await,
            Routing::Target(id) => self.send_to(id, msg).await,
            Routing::RoundRobin => self.next_agent(msg).await,
        }
    }
}

// Extension hook: Custom routing strategies
trait RoutingStrategy {
    fn select_recipient(&self, msg: &Message, agents: &[AgentId]) -> AgentId;
}
```

### 2.3 Basic Supervision Tree

```pseudocode
CLASS Supervisor {
    PRIVATE children: Map<String, Agent>
    PRIVATE strategy: SupervisionStrategy
    
    FUNCTION supervise(child: Agent) {
        children.put(child.id, child)
        monitor(child)
    }
    
    FUNCTION handleChildFailure(childId: String, error: Error) {
        strategy.handle(childId, error, children)
    }
}

ENUM RestartStrategy {
    ONE_FOR_ONE,      // Restart only failed agent
    ALL_FOR_ONE,      // Restart all agents
    REST_FOR_ONE      // Restart failed and subsequent agents
}
```

### 2.4 Simple Restart Logic

```pseudocode
CLASS RestartPolicy {
    PRIVATE maxRestarts: Integer
    PRIVATE timeWindow: Duration
    PRIVATE restartCounts: Map<String, List<Timestamp>>
    
    FUNCTION shouldRestart(agentId: String) -> Boolean {
        recentRestarts = countRecentRestarts(agentId, timeWindow)
        RETURN recentRestarts < maxRestarts
    }
    
    FUNCTION recordRestart(agentId: String) {
        restartCounts[agentId].add(NOW())
    }
}
```

## 3. Message Passing and Communication Patterns

### 3.1 Direct RPC Pattern

Point-to-point communication for immediate responses:

```rust
struct DirectChannel {
    target_endpoint: String,
    timeout: Duration,
}

impl DirectChannel {
    async fn call(&self, request: Request) -> Result<Response, Error> {
        // Direct HTTP/gRPC call
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()?;
        
        let response = client
            .post(&self.target_endpoint)
            .json(&request)
            .send()
            .await?;
            
        Ok(response.json().await?)
    }
}
```

### 3.2 Publish/Subscribe Pattern

Topic-based message distribution:

```rust
struct PubSubBus {
    broker_url: String,
    subscriptions: HashMap<Topic, Vec<CallbackFn>>,
}

impl PubSubBus {
    async fn publish(&self, topic: Topic, message: Message) -> Result<(), Error> {
        // Publish to broker (e.g., NATS)
        let nc = nats::connect(&self.broker_url)?;
        nc.publish(&topic.as_str(), &message.serialize()?)?;
        Ok(())
    }
    
    async fn subscribe<F>(&mut self, topic: Topic, callback: F) 
    where F: Fn(Message) + Send + 'static {
        let nc = nats::connect(&self.broker_url)?;
        let sub = nc.subscribe(&topic.as_str())?;
        
        tokio::spawn(async move {
            for msg in sub.messages() {
                callback(Message::deserialize(&msg.data).unwrap());
            }
        });
    }
}
```

### 3.3 Blackboard Pattern

Shared memory coordination:

```rust
struct Blackboard {
    store: Arc<RwLock<HashMap<String, BlackboardEntry>>>,
    watchers: Arc<RwLock<HashMap<Pattern, Vec<WatcherFn>>>>,
}

struct BlackboardEntry {
    value: Value,
    timestamp: Instant,
    author: AgentId,
    version: u64,
}

impl Blackboard {
    async fn write(&self, key: String, value: Value, agent_id: AgentId) -> Result<(), Error> {
        let mut store = self.store.write().await;
        let version = store.get(&key).map(|e| e.version + 1).unwrap_or(1);
        
        store.insert(key.clone(), BlackboardEntry {
            value: value.clone(),
            timestamp: Instant::now(),
            author: agent_id,
            version,
        });
        
        // Notify watchers
        self.notify_watchers(&key, &value).await;
        Ok(())
    }
    
    async fn read(&self, key: &str) -> Option<BlackboardEntry> {
        self.store.read().await.get(key).cloned()
    }
}
```

### 3.4 Basic Message Types

```pseudocode
INTERFACE Message {
    id: UUID
    type: String
    payload: Object
    sender: AgentId
    timestamp: Timestamp
}

ENUM SystemMessage {
    START,
    STOP,
    PAUSE,
    RESUME,
    HEALTH_CHECK
}
```

### 3.5 Mailbox Pattern

```pseudocode
CLASS AgentMailbox {
    PRIVATE queue: Queue<Message>
    PRIVATE capacity: Integer
    
    FUNCTION send(message: Message) -> Result {
        IF queue.size() < capacity THEN
            queue.enqueue(message)
            RETURN Success()
        ELSE
            RETURN Failure("Mailbox full")
        END IF
    }
    
    FUNCTION receive() -> Option<Message> {
        RETURN queue.dequeue()
    }
}
```

## 4. Task Distribution

### 4.1 Work Queue Pattern

```pseudocode
CLASS TaskDistributor {
    PRIVATE workers: List<WorkerAgent>
    PRIVATE taskQueue: Queue<Task>
    
    FUNCTION distribute(task: Task) {
        // Find available worker
        worker = findAvailableWorker()
        
        IF worker != NULL THEN
            worker.assign(task)
        ELSE
            taskQueue.enqueue(task)
        END IF
    }
    
    FUNCTION findAvailableWorker() -> WorkerAgent? {
        FOR worker IN workers {
            IF worker.isAvailable() THEN
                RETURN worker
            END IF
        }
        RETURN NULL
    }
}
```

### 4.2 Load Balancing

```pseudocode
CLASS LoadBalancer {
    PRIVATE agents: List<Agent>
    PRIVATE currentIndex: Integer = 0
    
    FUNCTION selectAgent() -> Agent {
        // Round-robin selection
        agent = agents[currentIndex]
        currentIndex = (currentIndex + 1) % agents.size()
        RETURN agent
    }
    
    FUNCTION selectLeastLoaded() -> Agent {
        // Select agent with fewest active tasks
        RETURN agents.minBy(agent => agent.getActiveTaskCount())
    }
}
```

## 5. Coordination Patterns

### 5.1 Request-Response Pattern

```pseudocode
CLASS RequestHandler {
    PRIVATE pendingRequests: Map<UUID, ResponseCallback>
    
    FUNCTION sendRequest(target: Agent, request: Request) -> Future<Response> {
        requestId = generateUUID()
        message = RequestMessage(requestId, request)
        
        future = Future<Response>()
        pendingRequests[requestId] = future.callback
        
        target.send(message)
        RETURN future
    }
    
    FUNCTION handleResponse(response: Response) {
        callback = pendingRequests.remove(response.requestId)
        IF callback != NULL THEN
            callback(response)
        END IF
    }
}
```

### 5.2 Publish-Subscribe Pattern

```pseudocode
CLASS EventBus {
    PRIVATE subscribers: Map<String, List<Agent>>
    
    FUNCTION subscribe(topic: String, agent: Agent) {
        IF NOT subscribers.contains(topic) THEN
            subscribers[topic] = []
        END IF
        subscribers[topic].add(agent)
    }
    
    FUNCTION publish(topic: String, event: Event) {
        agents = subscribers.get(topic, [])
        FOR agent IN agents {
            agent.send(EventMessage(topic, event))
        }
    }
}
```

## 6. Agent Discovery

### 6.1 Registry Pattern

```pseudocode
CLASS AgentRegistry {
    PRIVATE agents: Map<String, AgentInfo>
    
    FUNCTION register(agent: Agent) {
        info = AgentInfo{
            id: agent.id,
            type: agent.type,
            capabilities: agent.getCapabilities(),
            address: agent.getAddress()
        }
        agents[agent.id] = info
    }
    
    FUNCTION discover(criteria: SearchCriteria) -> List<AgentInfo> {
        RETURN agents.values()
            .filter(info => criteria.matches(info))
    }
}
```

### 6.2 Health Monitoring

```pseudocode
CLASS HealthMonitor {
    PRIVATE agents: Map<String, HealthStatus>
    PRIVATE checkInterval: Duration
    
    FUNCTION monitorHealth() {
        EVERY checkInterval {
            FOR agent IN agents.keys() {
                status = checkAgentHealth(agent)
                agents[agent] = status
                
                IF status == UNHEALTHY THEN
                    notifySupervisor(agent)
                END IF
            }
        }
    }
    
    FUNCTION checkAgentHealth(agentId: String) -> HealthStatus {
        TRY {
            response = sendHealthCheck(agentId)
            RETURN response.status
        } CATCH (timeout) {
            RETURN UNHEALTHY
        }
    }
}
```

## 7. Simple Workflow Orchestration

### 7.1 Sequential Workflow

```pseudocode
CLASS SequentialWorkflow {
    PRIVATE steps: List<WorkflowStep>
    
    FUNCTION execute(context: WorkflowContext) -> Result {
        FOR step IN steps {
            result = step.execute(context)
            
            IF result.isFailure() THEN
                RETURN result
            END IF
            
            context.updateWith(result.output)
        }
        
        RETURN Success(context)
    }
}
```

### 7.2 Parallel Workflow

```pseudocode
CLASS ParallelWorkflow {
    PRIVATE tasks: List<Task>
    
    FUNCTION execute() -> Result<List<TaskResult>> {
        futures = []
        
        FOR task IN tasks {
            future = async {
                agent = selectAgent(task.requirements)
                RETURN agent.execute(task)
            }
            futures.add(future)
        }
        
        // Wait for all tasks to complete
        results = awaitAll(futures)
        RETURN Success(results)
    }
}
```

## 8. Error Handling

### 8.1 Basic Error Recovery

```pseudocode
CLASS ErrorHandler {
    FUNCTION handleAgentError(agent: Agent, error: Error) {
        SWITCH error.type {
            CASE TIMEOUT:
                restartAgent(agent)
            CASE RESOURCE_EXHAUSTED:
                pauseAgent(agent)
                scheduleRetry(agent, delay: 30.seconds)
            CASE FATAL:
                terminateAgent(agent)
                notifySupervisor(agent, error)
            DEFAULT:
                logError(agent, error)
        }
    }
}
```

### 8.2 Circuit Breaker Pattern

```pseudocode
CLASS CircuitBreaker {
    PRIVATE state: BreakerState = CLOSED
    PRIVATE failureCount: Integer = 0
    PRIVATE threshold: Integer = 5
    PRIVATE timeout: Duration = 60.seconds
    
    FUNCTION call(operation: Function) -> Result {
        IF state == OPEN THEN
            IF timeoutExpired() THEN
                state = HALF_OPEN
            ELSE
                RETURN Failure("Circuit breaker open")
            END IF
        END IF
        
        TRY {
            result = operation()
            IF state == HALF_OPEN THEN
                state = CLOSED
                failureCount = 0
            END IF
            RETURN result
        } CATCH (error) {
            failureCount += 1
            IF failureCount >= threshold THEN
                state = OPEN
                scheduleTimeout()
            END IF
            THROW error
        }
    }
}
```

## 9. Basic Metrics

### 9.1 Agent Metrics

```pseudocode
CLASS AgentMetrics {
    PRIVATE messageCount: Counter
    PRIVATE taskCompletionTime: Histogram
    PRIVATE errorRate: Gauge
    
    FUNCTION recordMessage() {
        messageCount.increment()
    }
    
    FUNCTION recordTaskCompletion(duration: Duration) {
        taskCompletionTime.observe(duration)
    }
    
    FUNCTION updateErrorRate(rate: Float) {
        errorRate.set(rate)
    }
}
```

## 10. Spawn and Resource Management Patterns

### 10.1 Role-Based Spawning

Dynamic team composition based on project requirements:

```rust
enum AgentRole {
    ProductManager { sop: StandardProcedure },
    Architect { design_patterns: Vec<Pattern> },
    Engineer { toolchain: ToolSet },
}

struct RoleSpawner {
    role_registry: HashMap<String, AgentRole>,
    
    async fn spawn_team(&self, project: ProjectSpec) -> Team {
        let mut agents = vec![];
        
        // Spawn based on project needs
        for role in project.required_roles() {
            agents.push(self.spawn_role(role).await);
        }
        
        Team::new(agents, project.coordination_mode())
    }
}

// Anti-pattern: Static role assignment
// ❌ Fixed teams for all projects
// ✅ Dynamic team composition based on task analysis
```

### 10.2 Resource-Bounded Spawning

Prevent uncontrolled agent proliferation:

```rust
// ❌ BAD: Unlimited spawning
async fn handle_task(task: Task) {
    for subtask in task.decompose() {
        spawn_agent(subtask); // No limits!
    }
}

// ✅ GOOD: Resource-bounded spawning
struct SpawnController {
    max_agents: usize,
    active: Arc<AtomicUsize>,
    
    async fn spawn_bounded(&self, role: AgentRole) -> Result<Agent> {
        if self.active.load(Ordering::SeqCst) >= self.max_agents {
            return Err("Agent limit reached");
        }
        // Spawn with cleanup on drop
        Ok(BoundedAgent::new(role, self.active.clone()))
    }
}
```

### 10.3 Context Management

Prevent memory overflow with windowed context:

```rust
// ❌ BAD: Accumulating unlimited context
struct NaiveAgent {
    context: Vec<Message>, // Grows forever
}

// ✅ GOOD: Windowed context with summarization
struct SmartAgent {
    recent_context: VecDeque<Message>,
    context_summary: Summary,
    max_context_size: usize,

    fn add_context(&mut self, msg: Message) {
        self.recent_context.push_back(msg);
        if self.recent_context.len() > self.max_context_size {
            self.summarize_old_context();
        }
    }
}
```

### 10.4 Claude-CLI Parallel Execution Integration

Support for Claude-CLI's built-in parallel execution capabilities:

```rust
// Claude-CLI parallel execution pattern
struct ParallelTaskHandler {
    // Parse Claude-CLI parallel output format
    fn parse_parallel_output(&self, line: &str) -> Option<(u32, String)> {
        // Format: "Task(Patch Agent <n> ...)"
        if line.starts_with("Task(Patch Agent ") {
            let agent_id = self.extract_agent_id(line);
            let content = self.extract_content(line);
            Some((agent_id, content))
        } else {
            None
        }
    }

    // Map parallel agent output to NATS subjects
    async fn route_parallel_output(&self, agent_id: u32, content: String) {
        let subject = format!("agent.{}.out", agent_id);
        self.nats_client.publish(&subject, content).await;
    }
}

// Integration with existing supervision patterns
impl Supervisor {
    async fn handle_claude_parallel(&self, parallel_flag: u32) {
        // Claude-CLI spawns N internal Task/Patch agents
        // Each agent outputs prefixed with "Task(Patch Agent <n> ...)"
        // Map to existing agent.{id}.out subject taxonomy

        for agent_id in 0..parallel_flag {
            let subject = format!("agent.{}.out", agent_id);
            self.subscribe_to_parallel_agent(subject).await;
        }
    }
}
```

**Key Integration Points:**
- **Output Parsing**: Parse `Task(Patch Agent <n> ...)` lines from Claude-CLI stdout
- **Subject Mapping**: Route to `agent.{id}.out` subjects in NATS taxonomy
- **Supervision Compatibility**: Works with existing Tokio task supervision
- **Memory Layers**: Unaffected - Postgres/JetStream KV continue to work normally

## 11. Tool-Bus Integration Patterns

### 11.1 Shared Tool Registry

```rust
struct ToolBus {
    tools: Arc<RwLock<HashMap<ToolId, Box<dyn Tool>>>>,
    permissions: HashMap<AgentId, Vec<ToolId>>,
}

trait Tool: Send + Sync {
    async fn execute(&self, params: Value) -> Result<Value>;
    fn schema(&self) -> ToolSchema;
}

// Extension mechanism
impl ToolBus {
    fn register_tool<T: Tool + 'static>(&mut self, id: ToolId, tool: T) {
        self.tools.write().unwrap().insert(id, Box::new(tool));
    }
    
    async fn call(&self, agent_id: AgentId, tool_id: ToolId, params: Value) -> Result<Value> {
        // Permission check
        if !self.has_permission(agent_id, tool_id) {
            return Err("Unauthorized tool access");
        }
        
        let tools = self.tools.read().unwrap();
        tools.get(&tool_id)?.execute(params).await
    }
}
```

### 11.2 Agent-as-Tool Pattern

```rust
struct AgentTool {
    agent: Arc<dyn Agent>,
    interface: ToolInterface,
}

impl Tool for AgentTool {
    async fn execute(&self, params: Value) -> Result<Value> {
        // Convert tool call to agent message
        let msg = Message::from_tool_params(params);
        self.agent.process(msg).await
    }
}

// Allows supervisors to treat sub-agents as tools
impl Supervisor {
    fn register_agent_as_tool(&mut self, agent: Arc<dyn Agent>) {
        let tool = AgentTool::new(agent);
        self.tool_bus.register(tool);
    }
}
```

## 12. Extension and Middleware Patterns

### 12.1 Middleware Pattern

```rust
trait AgentMiddleware: Send + Sync {
    async fn before_process(&self, msg: &Message) -> Result<()>;
    async fn after_process(&self, msg: &Message, result: &Value) -> Result<()>;
}

struct Agent {
    middleware: Vec<Box<dyn AgentMiddleware>>,
    
    async fn process(&self, msg: Message) -> Result<Value> {
        // Before hooks
        for mw in &self.middleware {
            mw.before_process(&msg).await?;
        }
        
        let result = self.core_process(msg).await?;
        
        // After hooks
        for mw in &self.middleware {
            mw.after_process(&msg, &result).await?;
        }
        
        Ok(result)
    }
}
```

### 12.2 Event Emitter Pattern

```rust
enum SystemEvent {
    AgentSpawned(AgentId),
    TaskCompleted(TaskId, Value),
    ToolCalled(AgentId, ToolId),
    Error(AgentId, String),
}

struct EventBus {
    subscribers: HashMap<TypeId, Vec<Box<dyn EventHandler>>>,
    
    fn emit(&self, event: SystemEvent) {
        if let Some(handlers) = self.subscribers.get(&event.type_id()) {
            for handler in handlers {
                handler.handle(event.clone());
            }
        }
    }
}
```

## 13. Claude-CLI Hook System Integration

### 13.1 Hook Shim Pattern

```rust
// Hook system integration with NATS bus
struct HookShim {
    nats_client: async_nats::Client,
    hook_dir: PathBuf,
    agent_id: String,
}

impl HookShim {
    async fn handle_hook(&self, hook_type: HookType, payload: HookPayload) -> Result<HookResponse> {
        // Execute hook script and capture output
        let hook_path = self.hook_dir.join(hook_type.as_str());
        let mut cmd = Command::new(&hook_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Send JSON payload to hook via stdin
        let stdin = cmd.stdin.as_mut().unwrap();
        stdin.write_all(&serde_json::to_vec(&payload)?).await?;

        // Capture hook response
        let output = cmd.wait_with_output().await?;
        let response: HookResponse = serde_json::from_slice(&output.stdout)?;

        // Publish to NATS for orchestrator processing
        let subject = format!("agent.{}.hook_response", self.agent_id);
        self.nats_client.publish(subject, serde_json::to_vec(&response)?).await?;

        Ok(response)
    }

    async fn subscribe_to_hooks(&self) -> Result<()> {
        let subject = format!("agent.{}.pre", self.agent_id);
        let mut subscriber = self.nats_client.subscribe(subject).await?;

        while let Some(msg) = subscriber.next().await {
            let payload: HookPayload = serde_json::from_slice(&msg.payload)?;

            match payload.hook.as_str() {
                "pre_task" => {
                    self.handle_hook(HookType::PreTask, payload).await?;
                }
                "post_task" => {
                    self.handle_hook(HookType::PostTask, payload).await?;
                }
                "on_error" => {
                    self.handle_hook(HookType::OnError, payload).await?;
                }
                _ => {
                    eprintln!("Unknown hook type: {}", payload.hook);
                }
            }
        }

        Ok(())
    }
}

enum HookType {
    Startup,
    PreTask,
    PostTask,
    OnError,
    OnFileChange,
}

impl HookType {
    fn as_str(&self) -> &'static str {
        match self {
            HookType::Startup => "startup",
            HookType::PreTask => "pre_task",
            HookType::PostTask => "post_task",
            HookType::OnError => "on_error",
            HookType::OnFileChange => "on_file_change",
        }
    }
}

#[derive(Serialize, Deserialize)]
struct HookPayload {
    hook: String,
    cwd: String,
    project_config: Value,
    task: Option<TaskInfo>,
}

#[derive(Serialize, Deserialize)]
struct HookResponse {
    task: Option<TaskInfo>,
    env: Option<HashMap<String, String>>,
}
```

### 13.2 Async Hook Listener Pattern

```rust
// Integration with existing agent supervision
impl Supervisor {
    async fn setup_hook_integration(&self) -> Result<()> {
        // Subscribe to hook events from Claude-CLI
        let startup_sub = self.nats_client.subscribe("control.startup").await?;
        let file_change_sub = self.nats_client.subscribe("ctx.*.file_change").await?;

        // Spawn hook processing tasks
        tokio::spawn(self.process_startup_hooks(startup_sub));
        tokio::spawn(self.process_file_change_hooks(file_change_sub));

        Ok(())
    }

    async fn process_startup_hooks(&self, mut subscriber: Subscriber) {
        while let Some(msg) = subscriber.next().await {
            // Record CLI version and capabilities
            let startup_info: StartupInfo = serde_json::from_slice(&msg.payload).unwrap();
            self.record_cli_capabilities(startup_info).await;
        }
    }

    async fn process_file_change_hooks(&self, mut subscriber: Subscriber) {
        while let Some(msg) = subscriber.next().await {
            // Trigger code quality agents
            let file_change: FileChangeEvent = serde_json::from_slice(&msg.payload).unwrap();
            self.trigger_code_quality_agents(file_change).await;
        }
    }
}
```

## Summary

This document provides comprehensive agent orchestration patterns including:

1. **Agent architecture** - Core agent types (Planner, Executor, Critic, Router, Memory) with Rust trait definitions
2. **Supervision patterns** - Hub-and-spoke, event-driven message bus, and hierarchical supervision trees
3. **Communication patterns** - Direct RPC, publish-subscribe, blackboard, and mailbox patterns
4. **Task distribution** - Work queues and load balancing
5. **Coordination patterns** - Request-response and publish-subscribe
6. **Agent discovery** - Registry and health monitoring
7. **Workflow orchestration** - Sequential and parallel execution
8. **Error handling** - Recovery strategies and circuit breakers
9. **Basic metrics** - Simple performance monitoring
10. **Spawn patterns** - Role-based spawning with resource bounds
11. **Tool-bus integration** - Shared tool registry and agent-as-tool patterns
12. **Extension mechanisms** - Middleware and event emitter patterns

### Key Design Principles

1. **Hierarchical supervision** - Use hierarchical supervisors with bounded spawning
2. **Event-driven architecture** - Async channels with routing strategies  
3. **Shared tool registry** - Tools with permission management
4. **Context management** - Windowed memory with periodic summarization
5. **Extension points** - Middleware, event emitters, and trait-based plugins
6. **Resource control** - Always bound agent spawning and set timeouts
7. **Error boundaries** - Isolate agent failures from system crashes

These patterns provide a solid foundation for building distributed agent systems while avoiding common anti-patterns and maintaining type safety and performance.