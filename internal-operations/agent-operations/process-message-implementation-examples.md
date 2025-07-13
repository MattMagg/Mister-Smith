# Process & Message System Implementation Examples

## Key Component Implementations for Phase 2

### Process Manager Implementation

```rust
// src/runtime/process_manager.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::process::{Child, Command};
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::runtime::{
    config::ProcessConfig,
    error::{ProcessError, ProcessResult},
    supervisor::Supervisor,
    health_monitor::HealthStatus,
};
use crate::message::{Message, MessageRouter, AgentCommand, AgentStatus};

pub type ProcessId = String;

pub struct ProcessManager {
    processes: Arc<RwLock<HashMap<ProcessId, Arc<RwLock<ProcessHandle>>>>>,
    supervisor: Arc<Supervisor>,
    message_router: Arc<MessageRouter>,
    shutdown_tx: mpsc::Sender<()>,
}

pub struct ProcessHandle {
    pub id: ProcessId,
    pub child: Option<Child>,
    pub status: ProcessStatus,
    pub spawn_time: std::time::Instant,
    pub config: ProcessConfig,
}

#[derive(Debug, Clone)]
pub enum ProcessStatus {
    Initializing,
    Idle,
    Busy,
    Paused,
    Error(String),
    Stopping,
    Terminated,
}

impl ProcessManager {
    pub fn new(
        supervisor: Arc<Supervisor>,
        message_router: Arc<MessageRouter>,
    ) -> (Self, mpsc::Receiver<()>) {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        let manager = Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            supervisor,
            message_router,
            shutdown_tx,
        };
        
        (manager, shutdown_rx)
    }

    pub async fn spawn_claude_process(
        &self,
        process_id: ProcessId,
        config: ProcessConfig,
    ) -> ProcessResult<()> {
        info!("Spawning Claude process: {}", process_id);
        
        // Build command
        let mut command = Command::new(&config.command);
        command.args(&config.args);
        command.envs(&config.env);
        command.current_dir(&config.working_dir);
        
        // Spawn process
        let child = command
            .spawn()
            .map_err(|e| ProcessError::SpawnFailed(e.to_string()))?;
        
        let pid = child.id().ok_or(ProcessError::NoPid)?;
        info!("Process {} spawned with PID: {}", process_id, pid);
        
        // Create process handle
        let handle = ProcessHandle {
            id: process_id.clone(),
            child: Some(child),
            status: ProcessStatus::Initializing,
            spawn_time: std::time::Instant::now(),
            config,
        };
        
        // Register with supervisor
        self.supervisor.register_process(process_id.clone(), handle.config.restart_policy.clone()).await?;
        
        // Store handle
        let handle = Arc::new(RwLock::new(handle));
        self.processes.write().await.insert(process_id.clone(), handle.clone());
        
        // Start monitoring
        self.start_process_monitoring(process_id, handle).await;
        
        Ok(())
    }

    async fn start_process_monitoring(
        &self,
        process_id: ProcessId,
        handle: Arc<RwLock<ProcessHandle>>,
    ) {
        let supervisor = self.supervisor.clone();
        let message_router = self.message_router.clone();
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                
                let mut handle_guard = handle.write().await;
                if let Some(ref mut child) = handle_guard.child {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            warn!("Process {} exited with status: {}", process_id, status);
                            handle_guard.status = ProcessStatus::Terminated;
                            
                            // Notify supervisor
                            if let Err(e) = supervisor.handle_process_exit(&process_id).await {
                                error!("Failed to handle process exit: {}", e);
                            }
                            
                            break;
                        }
                        Ok(None) => {
                            // Still running, send status update
                            let status_msg = AgentStatus {
                                agent_id: process_id.clone(),
                                status: handle_guard.status.clone().into(),
                                uptime_seconds: handle_guard.spawn_time.elapsed().as_secs(),
                                // ... other fields
                            };
                            
                            if let Err(e) = message_router.route(Message::AgentStatus(status_msg)).await {
                                warn!("Failed to send status update: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Error checking process status: {}", e);
                        }
                    }
                }
            }
        });
    }

    pub async fn handle_command(&self, cmd: AgentCommand) -> ProcessResult<()> {
        match cmd.command_type {
            CommandType::Execute => {
                if let Some(task_id) = cmd.task_id {
                    self.execute_task(&cmd.target_agent_id, task_id).await
                } else {
                    Err(ProcessError::InvalidCommand("Execute requires task_id".into()))
                }
            }
            CommandType::Stop => self.stop_process(&cmd.target_agent_id).await,
            CommandType::Pause => self.pause_process(&cmd.target_agent_id).await,
            CommandType::Resume => self.resume_process(&cmd.target_agent_id).await,
            _ => Err(ProcessError::UnsupportedCommand(cmd.command_type)),
        }
    }

    pub async fn stop_process(&self, process_id: &ProcessId) -> ProcessResult<()> {
        let processes = self.processes.read().await;
        
        if let Some(handle) = processes.get(process_id) {
            let mut handle_guard = handle.write().await;
            handle_guard.status = ProcessStatus::Stopping;
            
            if let Some(ref mut child) = handle_guard.child {
                // Send SIGTERM
                child.kill().await
                    .map_err(|e| ProcessError::KillFailed(e.to_string()))?;
                    
                // Wait for graceful shutdown
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(30),
                    child.wait()
                ).await {
                    Ok(Ok(status)) => {
                        info!("Process {} terminated with status: {}", process_id, status);
                        handle_guard.status = ProcessStatus::Terminated;
                        Ok(())
                    }
                    Ok(Err(e)) => Err(ProcessError::WaitFailed(e.to_string())),
                    Err(_) => {
                        warn!("Process {} did not terminate gracefully, force killing", process_id);
                        // Force kill would go here
                        Ok(())
                    }
                }
            } else {
                Err(ProcessError::ProcessNotRunning(process_id.clone()))
            }
        } else {
            Err(ProcessError::ProcessNotFound(process_id.clone()))
        }
    }
}
```

### Message Router Implementation

```rust
// src/message/router.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, instrument};
use async_trait::async_trait;

use crate::message::{
    Message, MessageType, BaseMessageEnvelope,
    error::{MessageError, MessageResult},
};
use crate::transport::{Transport, TransportError};

pub struct MessageRouter {
    transports: HashMap<String, Arc<dyn Transport>>,
    routing_table: Arc<RwLock<RoutingTable>>,
    default_transport: Option<Arc<dyn Transport>>,
}

pub struct RoutingTable {
    agent_routes: HashMap<String, String>, // agent_id -> transport_name
    type_routes: HashMap<MessageType, String>, // message_type -> transport_name
}

pub struct RoutingDecision {
    pub transport_name: String,
    pub subject: String,
    pub is_broadcast: bool,
}

impl MessageRouter {
    pub fn new(default_transport: Option<Arc<dyn Transport>>) -> Self {
        Self {
            transports: HashMap::new(),
            routing_table: Arc::new(RwLock::new(RoutingTable {
                agent_routes: HashMap::new(),
                type_routes: HashMap::new(),
            })),
            default_transport,
        }
    }

    pub fn register_transport(&mut self, name: String, transport: Arc<dyn Transport>) {
        self.transports.insert(name, transport);
    }

    #[instrument(skip(self, message))]
    pub async fn route(&self, message: Message) -> MessageResult<()> {
        let envelope = message.envelope();
        let decision = self.make_routing_decision(&envelope).await?;
        
        info!(
            "Routing message {} to {} via {}",
            envelope.message_id, decision.subject, decision.transport_name
        );
        
        let transport = self.transports
            .get(&decision.transport_name)
            .or(self.default_transport.as_ref())
            .ok_or(MessageError::NoTransportAvailable)?;
        
        match transport.send(message).await {
            Ok(_) => Ok(()),
            Err(TransportError::Disconnected) => {
                warn!("Transport disconnected, attempting failover");
                self.failover_route(message).await
            }
            Err(e) => Err(MessageError::TransportError(e.to_string())),
        }
    }

    async fn make_routing_decision(&self, envelope: &BaseMessageEnvelope) -> MessageResult<RoutingDecision> {
        let routing_table = self.routing_table.read().await;
        
        // Priority 1: Direct agent routing
        if let Some(target_agent_id) = &envelope.target_agent_id {
            if let Some(transport_name) = routing_table.agent_routes.get(target_agent_id) {
                return Ok(RoutingDecision {
                    transport_name: transport_name.clone(),
                    subject: format!("mister-smith.agent.{}.{}", 
                        target_agent_id, 
                        envelope.message_type.as_str()
                    ),
                    is_broadcast: false,
                });
            }
        }
        
        // Priority 2: Type-based routing
        if let Some(transport_name) = routing_table.type_routes.get(&envelope.message_type) {
            let subject = if envelope.target_agent_id.is_some() {
                format!("mister-smith.{}.{}", 
                    envelope.message_type.as_str(),
                    envelope.target_agent_id.as_ref().unwrap()
                )
            } else {
                format!("mister-smith.{}.broadcast", envelope.message_type.as_str())
            };
            
            return Ok(RoutingDecision {
                transport_name: transport_name.clone(),
                subject,
                is_broadcast: envelope.target_agent_id.is_none(),
            });
        }
        
        // Priority 3: Default transport
        if self.default_transport.is_some() {
            Ok(RoutingDecision {
                transport_name: "default".to_string(),
                subject: format!("mister-smith.default.{}", envelope.message_type.as_str()),
                is_broadcast: true,
            })
        } else {
            Err(MessageError::NoRouteFound)
        }
    }

    async fn failover_route(&self, message: Message) -> MessageResult<()> {
        // Try all available transports
        for (name, transport) in &self.transports {
            if transport.health_check().await.is_healthy() {
                info!("Failing over to transport: {}", name);
                match transport.send(message.clone()).await {
                    Ok(_) => return Ok(()),
                    Err(e) => warn!("Failover to {} failed: {}", name, e),
                }
            }
        }
        
        Err(MessageError::AllTransportsFailed)
    }

    pub async fn register_agent_route(&self, agent_id: String, transport_name: String) {
        let mut routing_table = self.routing_table.write().await;
        routing_table.agent_routes.insert(agent_id, transport_name);
    }
}
```

### NATS Transport Implementation

```rust
// src/transport/nats_transport.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use async_nats::{Client, Subscriber, Message as NatsMessage};
use tracing::{info, warn, error, instrument};
use serde::{Serialize, Deserialize};

use crate::transport::{
    Transport, ConnectionConfig, HealthStatus,
    error::{TransportError, TransportResult},
};
use crate::message::{Message, MessageSerializer};

pub struct NatsTransport {
    client: Option<Client>,
    config: NatsConfig,
    subscriptions: Arc<RwLock<HashMap<String, Subscriber>>>,
    serializer: MessageSerializer,
}

#[derive(Debug, Clone)]
pub struct NatsConfig {
    pub url: String,
    pub cluster_id: String,
    pub client_id: String,
    pub reconnect_delay_ms: u64,
    pub max_reconnect_attempts: u32,
}

impl NatsTransport {
    pub fn new(config: NatsConfig) -> Self {
        Self {
            client: None,
            config,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            serializer: MessageSerializer::new(),
        }
    }

    fn build_subject(&self, routing_subject: &str) -> String {
        // Ensure proper NATS subject format
        routing_subject.replace('/', '.')
    }
}

#[async_trait]
impl Transport for NatsTransport {
    #[instrument(skip(self))]
    async fn connect(&mut self, config: ConnectionConfig) -> TransportResult<()> {
        info!("Connecting to NATS at {}", config.url);
        
        let client = async_nats::connect(&config.url)
            .await
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
        
        self.client = Some(client);
        info!("Successfully connected to NATS");
        
        Ok(())
    }

    async fn disconnect(&mut self) -> TransportResult<()> {
        if let Some(client) = self.client.take() {
            // Unsubscribe all
            let mut subs = self.subscriptions.write().await;
            for (subject, mut sub) in subs.drain() {
                sub.unsubscribe().await
                    .map_err(|e| TransportError::UnsubscribeFailed(e.to_string()))?;
                info!("Unsubscribed from {}", subject);
            }
            
            // Flush pending messages
            client.flush().await
                .map_err(|e| TransportError::FlushFailed(e.to_string()))?;
            
            info!("Disconnected from NATS");
        }
        
        Ok(())
    }

    #[instrument(skip(self, message))]
    async fn send(&self, message: Message) -> TransportResult<()> {
        let client = self.client.as_ref()
            .ok_or(TransportError::NotConnected)?;
        
        // Serialize message
        let payload = self.serializer.serialize(&message)
            .map_err(|e| TransportError::SerializationError(e.to_string()))?;
        
        // Determine subject from message
        let subject = self.build_subject(&message.routing_info().subject);
        
        // Send message
        if let Some(reply_to) = message.envelope().reply_to {
            // Request-response pattern
            let response = client
                .request(subject, payload.into())
                .await
                .map_err(|e| TransportError::SendFailed(e.to_string()))?;
                
            info!("Sent request message {} and received response", message.envelope().message_id);
        } else {
            // Fire-and-forget
            client
                .publish(subject, payload.into())
                .await
                .map_err(|e| TransportError::SendFailed(e.to_string()))?;
                
            info!("Published message {} to {}", message.envelope().message_id, subject);
        }
        
        Ok(())
    }

    async fn subscribe<F>(&self, subject: &str, handler: F) -> TransportResult<()>
    where
        F: Fn(Message) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        let client = self.client.as_ref()
            .ok_or(TransportError::NotConnected)?;
        
        let subject = self.build_subject(subject);
        let subscriber = client
            .subscribe(subject.clone())
            .await
            .map_err(|e| TransportError::SubscribeFailed(e.to_string()))?;
        
        // Store subscription
        self.subscriptions.write().await.insert(subject.clone(), subscriber.clone());
        
        // Spawn handler task
        let serializer = self.serializer.clone();
        let handler = Arc::new(handler);
        
        tokio::spawn(async move {
            let mut subscriber = subscriber;
            while let Some(nats_msg) = subscriber.next().await {
                match serializer.deserialize(&nats_msg.payload) {
                    Ok(message) => {
                        handler(message).await;
                    }
                    Err(e) => {
                        error!("Failed to deserialize message: {}", e);
                    }
                }
            }
        });
        
        info!("Subscribed to {}", subject);
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        match &self.client {
            Some(client) => {
                // Check connection status
                match client.connection_state() {
                    async_nats::connection::State::Connected => HealthStatus::Healthy,
                    async_nats::connection::State::Disconnected => {
                        HealthStatus::Unhealthy("Disconnected from NATS".to_string())
                    }
                    _ => HealthStatus::Degraded("Connection state uncertain".to_string()),
                }
            }
            None => HealthStatus::Unhealthy("Not connected".to_string()),
        }
    }
}
```

### Supervisor Implementation

```rust
// src/runtime/supervisor.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant, sleep};
use tracing::{info, warn, error};

use crate::runtime::{
    error::{ProcessError, ProcessResult},
    process_manager::ProcessId,
};

pub struct Supervisor {
    policies: Arc<Mutex<HashMap<ProcessId, RestartPolicy>>>,
    restart_trackers: Arc<Mutex<HashMap<ProcessId, RestartTracker>>>,
    escalation_handler: Box<dyn EscalationHandler>,
}

#[derive(Debug, Clone)]
pub enum RestartPolicy {
    Never,
    Always,
    OnFailure,
    ExponentialBackoff {
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
    },
}

#[derive(Debug)]
struct RestartTracker {
    count: u32,
    last_restart: Option<Instant>,
    backoff_delay: Duration,
}

#[async_trait]
pub trait EscalationHandler: Send + Sync {
    async fn handle_escalation(&self, process_id: &ProcessId, reason: &str);
}

impl Supervisor {
    pub fn new(escalation_handler: Box<dyn EscalationHandler>) -> Self {
        Self {
            policies: Arc::new(Mutex::new(HashMap::new())),
            restart_trackers: Arc::new(Mutex::new(HashMap::new())),
            escalation_handler,
        }
    }

    pub async fn register_process(
        &self,
        process_id: ProcessId,
        policy: RestartPolicy,
    ) -> ProcessResult<()> {
        let mut policies = self.policies.lock().await;
        policies.insert(process_id.clone(), policy);
        
        let mut trackers = self.restart_trackers.lock().await;
        trackers.insert(process_id, RestartTracker {
            count: 0,
            last_restart: None,
            backoff_delay: Duration::from_secs(1),
        });
        
        Ok(())
    }

    pub async fn handle_process_exit(&self, process_id: &ProcessId) -> ProcessResult<()> {
        let policies = self.policies.lock().await;
        let policy = policies.get(process_id)
            .ok_or(ProcessError::ProcessNotFound(process_id.clone()))?
            .clone();
        drop(policies);

        match policy {
            RestartPolicy::Never => {
                info!("Process {} configured to never restart", process_id);
                Ok(())
            }
            RestartPolicy::Always => {
                self.restart_process(process_id).await
            }
            RestartPolicy::OnFailure => {
                // In a real implementation, check exit code
                self.restart_process(process_id).await
            }
            RestartPolicy::ExponentialBackoff { initial_delay, max_delay, multiplier } => {
                self.restart_with_backoff(process_id, initial_delay, max_delay, multiplier).await
            }
        }
    }

    async fn restart_process(&self, process_id: &ProcessId) -> ProcessResult<()> {
        let mut trackers = self.restart_trackers.lock().await;
        let tracker = trackers.get_mut(process_id)
            .ok_or(ProcessError::ProcessNotFound(process_id.clone()))?;

        // Check restart limits
        if tracker.count >= 5 {
            error!("Process {} exceeded restart limit", process_id);
            self.escalation_handler.handle_escalation(process_id, "Restart limit exceeded").await;
            return Err(ProcessError::RestartLimitExceeded);
        }

        // Check restart frequency
        if let Some(last_restart) = tracker.last_restart {
            if last_restart.elapsed() < Duration::from_secs(60) {
                warn!("Process {} restarting too frequently", process_id);
                self.escalation_handler.handle_escalation(process_id, "Frequent restarts").await;
            }
        }

        tracker.count += 1;
        tracker.last_restart = Some(Instant::now());

        info!("Restarting process {} (attempt {})", process_id, tracker.count);
        
        // In real implementation, would call ProcessManager to actually restart
        Ok(())
    }

    async fn restart_with_backoff(
        &self,
        process_id: &ProcessId,
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
    ) -> ProcessResult<()> {
        let mut trackers = self.restart_trackers.lock().await;
        let tracker = trackers.get_mut(process_id)
            .ok_or(ProcessError::ProcessNotFound(process_id.clone()))?;

        // Calculate backoff delay
        let delay = if tracker.count == 0 {
            initial_delay
        } else {
            let new_delay = tracker.backoff_delay.as_secs_f64() * multiplier;
            Duration::from_secs_f64(new_delay.min(max_delay.as_secs_f64()))
        };

        tracker.backoff_delay = delay;
        drop(trackers);

        info!("Waiting {:?} before restarting process {}", delay, process_id);
        sleep(delay).await;

        self.restart_process(process_id).await
    }
}

// Example escalation handler
pub struct LoggingEscalationHandler;

#[async_trait]
impl EscalationHandler for LoggingEscalationHandler {
    async fn handle_escalation(&self, process_id: &ProcessId, reason: &str) {
        error!("ESCALATION for process {}: {}", process_id, reason);
        // In real implementation, could send alerts, notify operators, etc.
    }
}
```

### Message Serialization

```rust
// src/message/serialization.rs

use serde::{Serialize, Deserialize};
use crate::message::{Message, error::{MessageError, MessageResult}};

#[derive(Debug, Clone)]
pub enum SerializationFormat {
    Json,
    MessagePack,
}

#[derive(Clone)]
pub struct MessageSerializer {
    format: SerializationFormat,
}

impl MessageSerializer {
    pub fn new() -> Self {
        Self {
            format: SerializationFormat::Json,
        }
    }

    pub fn with_format(format: SerializationFormat) -> Self {
        Self { format }
    }

    pub fn serialize(&self, message: &Message) -> MessageResult<Vec<u8>> {
        match self.format {
            SerializationFormat::Json => {
                serde_json::to_vec(message)
                    .map_err(|e| MessageError::SerializationError(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::to_vec(message)
                    .map_err(|e| MessageError::SerializationError(e.to_string()))
            }
        }
    }

    pub fn deserialize(&self, data: &[u8]) -> MessageResult<Message> {
        match self.format {
            SerializationFormat::Json => {
                serde_json::from_slice(data)
                    .map_err(|e| MessageError::DeserializationError(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::from_slice(data)
                    .map_err(|e| MessageError::DeserializationError(e.to_string()))
            }
        }
    }
}
```

## Integration Example

```rust
// src/integration/process_message.rs

use std::sync::Arc;
use crate::runtime::{ProcessManager, Supervisor, ProcessConfig};
use crate::message::{MessageRouter, Message, AgentCommand};
use crate::transport::NatsTransport;

pub async fn setup_integrated_system() -> Result<(), Box<dyn std::error::Error>> {
    // Create supervisor
    let escalation_handler = Box::new(LoggingEscalationHandler);
    let supervisor = Arc::new(Supervisor::new(escalation_handler));

    // Create message router
    let mut message_router = MessageRouter::new(None);
    
    // Setup NATS transport
    let nats_config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        cluster_id: "mister-smith".to_string(),
        client_id: "orchestrator".to_string(),
        reconnect_delay_ms: 1000,
        max_reconnect_attempts: 10,
    };
    
    let mut nats_transport = NatsTransport::new(nats_config);
    nats_transport.connect(ConnectionConfig {
        url: "nats://localhost:4222".to_string(),
        timeout: Duration::from_secs(10),
        retry_attempts: 3,
    }).await?;
    
    message_router.register_transport("nats".to_string(), Arc::new(nats_transport));
    let message_router = Arc::new(message_router);

    // Create process manager
    let (process_manager, _shutdown_rx) = ProcessManager::new(supervisor, message_router.clone());

    // Subscribe to agent commands
    let pm_clone = process_manager.clone();
    message_router.subscribe("mister-smith.command.*", move |msg| {
        let pm = pm_clone.clone();
        Box::pin(async move {
            if let Message::AgentCommand(cmd) = msg {
                if let Err(e) = pm.handle_command(cmd).await {
                    error!("Failed to handle command: {}", e);
                }
            }
        })
    }).await?;

    // Spawn a Claude process
    let process_config = ProcessConfig {
        command: "claude".to_string(),
        args: vec!["--mode".to_string(), "agent".to_string()],
        env: HashMap::new(),
        working_dir: PathBuf::from("/app"),
        restart_policy: RestartPolicy::ExponentialBackoff {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
        },
        resource_limits: ResourceLimits {
            max_memory_mb: Some(1024),
            max_cpu_percent: Some(50.0),
        },
    };

    process_manager.spawn_claude_process("claude-agent-1".to_string(), process_config).await?;

    Ok(())
}
```

## Testing Examples

```rust
// src/runtime/tests/process_manager_test.rs

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use tokio::test;

    mock! {
        Transport {}
        
        #[async_trait]
        impl Transport for Transport {
            async fn connect(&mut self, config: ConnectionConfig) -> TransportResult<()>;
            async fn disconnect(&mut self) -> TransportResult<()>;
            async fn send(&self, message: Message) -> TransportResult<()>;
            async fn subscribe<F>(&self, subject: &str, handler: F) -> TransportResult<()>
            where F: Fn(Message) -> BoxFuture<'static, ()> + Send + Sync + 'static;
            async fn health_check(&self) -> HealthStatus;
        }
    }

    #[test]
    async fn test_process_spawn() {
        let supervisor = Arc::new(Supervisor::new(Box::new(LoggingEscalationHandler)));
        let message_router = Arc::new(MessageRouter::new(None));
        let (process_manager, _) = ProcessManager::new(supervisor, message_router);

        let config = ProcessConfig {
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            env: HashMap::new(),
            working_dir: PathBuf::from("."),
            restart_policy: RestartPolicy::Never,
            resource_limits: ResourceLimits::default(),
        };

        let result = process_manager.spawn_claude_process("test-1".to_string(), config).await;
        assert!(result.is_ok());
    }

    #[test]
    async fn test_message_routing() {
        let mut mock_transport = MockTransport::new();
        mock_transport
            .expect_send()
            .times(1)
            .returning(|_| Ok(()));

        let mut router = MessageRouter::new(None);
        router.register_transport("mock".to_string(), Arc::new(mock_transport));

        let message = Message::AgentCommand(AgentCommand {
            command_type: CommandType::Execute,
            target_agent_id: "test-agent".to_string(),
            task_id: Some("task-1".to_string()),
        });

        let result = router.route(message).await;
        assert!(result.is_ok());
    }
}
```

This implementation provides a working foundation for the process management and message system that can be expanded in Phase 3.