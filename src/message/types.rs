//! Message Types for Agent Communication
//!
//! Defines all message types used for NATS communication between agents and the framework.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use crate::agent::{AgentId, AgentState, AgentConfig};

/// Base message envelope for all NATS messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    /// Unique message ID
    pub message_id: Uuid,
    /// Message timestamp
    pub timestamp: SystemTime,
    /// Source agent ID (if applicable)
    pub source_agent: Option<AgentId>,
    /// Target agent ID (if applicable)
    pub target_agent: Option<AgentId>,
    /// Message correlation ID for request/response
    pub correlation_id: Option<Uuid>,
    /// Message payload
    pub payload: T,
}

impl<T> MessageEnvelope<T> {
    /// Create a new message envelope
    pub fn new(payload: T) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            source_agent: None,
            target_agent: None,
            correlation_id: None,
            payload,
        }
    }
    
    /// Set source agent
    pub fn from_agent(mut self, agent_id: AgentId) -> Self {
        self.source_agent = Some(agent_id);
        self
    }
    
    /// Set target agent
    pub fn to_agent(mut self, agent_id: AgentId) -> Self {
        self.target_agent = Some(agent_id);
        self
    }
    
    /// Set correlation ID for request/response
    pub fn with_correlation(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
}

/// Agent command messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentCommand {
    /// Spawn a new agent
    Spawn {
        config: AgentConfig,
        reply_to: Option<String>,
    },
    /// Stop an agent
    Stop {
        agent_id: AgentId,
        graceful: bool,
        timeout: Option<Duration>,
    },
    /// Pause an agent
    Pause {
        agent_id: AgentId,
    },
    /// Resume an agent
    Resume {
        agent_id: AgentId,
    },
    /// Query agent status
    Status {
        agent_id: AgentId,
        reply_to: String,
    },
    /// Send input to agent
    Input {
        agent_id: AgentId,
        content: String,
    },
    /// Health check request
    HealthCheck {
        agent_id: AgentId,
        reply_to: String,
    },
}

/// Agent status messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    /// Agent ID
    pub agent_id: AgentId,
    /// Current state
    pub state: AgentState,
    /// Agent uptime
    pub uptime: Duration,
    /// Agent configuration
    pub config: AgentConfig,
    /// Last health check result
    pub healthy: bool,
    /// Resource usage information
    pub resource_usage: Option<ResourceUsage>,
}

/// Agent event notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// Agent spawned successfully
    Spawned {
        agent_id: AgentId,
        config: AgentConfig,
    },
    /// Agent state changed
    StateChanged {
        agent_id: AgentId,
        old_state: AgentState,
        new_state: AgentState,
    },
    /// Agent terminated
    Terminated {
        agent_id: AgentId,
        reason: String,
    },
    /// Agent error occurred
    Error {
        agent_id: AgentId,
        error: String,
    },
    /// Agent output received
    Output {
        agent_id: AgentId,
        content: String,
    },
    /// Agent heartbeat
    Heartbeat {
        agent_id: AgentId,
        timestamp: SystemTime,
    },
}

/// Resource usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory usage in MB
    pub memory_mb: f64,
    /// CPU usage percentage
    pub cpu_percent: f64,
    /// Process ID
    pub pid: Option<u32>,
}

/// Command response messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandResponse {
    /// Command executed successfully
    Success {
        message: String,
        data: Option<serde_json::Value>,
    },
    /// Command failed
    Error {
        error: String,
        code: Option<u32>,
    },
    /// Agent status response
    Status(AgentStatus),
    /// Health check response
    HealthCheck {
        agent_id: AgentId,
        healthy: bool,
        details: Option<String>,
    },
}

/// System control messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemMessage {
    /// System startup notification
    Startup,
    /// System shutdown notification
    Shutdown,
    /// Pool status request
    PoolStatus {
        reply_to: String,
    },
    /// System health check
    SystemHealth {
        reply_to: String,
    },
}

/// Pool status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatusResponse {
    /// Number of active agents
    pub active_count: usize,
    /// Maximum pool capacity
    pub max_capacity: usize,
    /// Available slots
    pub available_slots: usize,
    /// List of active agent IDs
    pub active_agents: Vec<AgentId>,
}

/// Subject builder for NATS topics
pub struct Subject;

impl Subject {
    /// Agent command subject
    pub fn agent_command() -> &'static str {
        "agent.command"
    }
    
    /// Agent status subject
    pub fn agent_status(agent_id: &AgentId) -> String {
        format!("agent.{}.status", agent_id.uuid())
    }
    
    /// Agent events subject
    pub fn agent_events(agent_id: &AgentId) -> String {
        format!("agent.{}.events", agent_id.uuid())
    }
    
    /// Agent input subject
    pub fn agent_input(agent_id: &AgentId) -> String {
        format!("agent.{}.input", agent_id.uuid())
    }
    
    /// Agent output subject
    pub fn agent_output(agent_id: &AgentId) -> String {
        format!("agent.{}.output", agent_id.uuid())
    }
    
    /// System control subject
    pub fn system_control() -> &'static str {
        "system.control"
    }
    
    /// Pool management subject
    pub fn pool_management() -> &'static str {
        "pool.management"
    }
    
    /// All agent events wildcard
    pub fn all_agent_events() -> &'static str {
        "agent.*.events"
    }
    
    /// All agent status wildcard
    pub fn all_agent_status() -> &'static str {
        "agent.*.status"
    }
}

/// Message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 3,
    Normal = 2,
    High = 1,
    Critical = 0,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Normal => write!(f, "normal"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}
