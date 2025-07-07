//! Agent State Management
//!
//! Defines agent states, identifiers, and configuration types for Claude CLI processes.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an agent (Claude CLI process)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    /// Create a new random agent ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    /// Create agent ID from UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
    
    /// Get the underlying UUID
    pub fn uuid(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "agent-{}", self.0)
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent state representing the lifecycle of a Claude CLI process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent created but not yet started
    Created,
    /// Starting Claude CLI process
    Starting,
    /// Claude CLI process running and ready
    Running,
    /// Agent paused (process alive but not processing)
    Paused,
    /// Agent idle (ready for work)
    Idle,
    /// Agent actively processing a request
    Processing,
    /// Agent stopping gracefully
    Stopping,
    /// Agent terminated (process ended)
    Terminated,
    /// Agent in error state
    Error,
}

impl fmt::Display for AgentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentState::Created => write!(f, "created"),
            AgentState::Starting => write!(f, "starting"),
            AgentState::Running => write!(f, "running"),
            AgentState::Paused => write!(f, "paused"),
            AgentState::Idle => write!(f, "idle"),
            AgentState::Processing => write!(f, "processing"),
            AgentState::Stopping => write!(f, "stopping"),
            AgentState::Terminated => write!(f, "terminated"),
            AgentState::Error => write!(f, "error"),
        }
    }
}

impl AgentState {
    /// Check if agent is in a running state
    pub fn is_active(&self) -> bool {
        matches!(self, AgentState::Running | AgentState::Idle | AgentState::Processing)
    }
    
    /// Check if agent can be stopped
    pub fn can_stop(&self) -> bool {
        matches!(self, AgentState::Running | AgentState::Idle | AgentState::Processing | AgentState::Paused | AgentState::Error)
    }
    
    /// Check if agent can be paused
    pub fn can_pause(&self) -> bool {
        matches!(self, AgentState::Running | AgentState::Idle)
    }
    
    /// Check if agent is in terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentState::Terminated | AgentState::Error)
    }
}

/// Configuration for an agent (Claude CLI process)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Claude model to use
    pub model: String,
    /// Maximum number of turns
    pub max_turns: Option<u32>,
    /// Allowed tools for this agent
    pub allowed_tools: Option<Vec<String>>,
    /// Enable MCP integration
    pub enable_mcp: bool,
    /// Timeout for operations
    pub timeout_seconds: u64,
    /// Memory limit in MB
    pub memory_limit_mb: Option<u64>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_turns: None,
            allowed_tools: None,
            enable_mcp: true,
            timeout_seconds: 300, // 5 minutes
            memory_limit_mb: Some(512), // 512MB default
        }
    }
}
