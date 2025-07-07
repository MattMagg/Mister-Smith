//! Core Agent Implementation
//!
//! Defines the fundamental Agent structure and operations for Claude CLI process management.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use anyhow::Result;

use crate::agent::{AgentId, AgentState, AgentConfig};
use crate::runtime::{ProcessManager, ClaudeExecutor};

/// Core Agent representing a managed Claude CLI process
#[derive(Debug, Clone)]
pub struct Agent {
    /// Unique agent identifier
    pub id: AgentId,
    /// Current agent state
    pub state: Arc<RwLock<AgentState>>,
    /// Agent configuration
    pub config: AgentConfig,
    /// Process manager for Claude CLI (for future interactive mode)
    pub process_manager: Arc<ProcessManager>,
    /// Claude executor for single-shot execution
    pub claude_executor: Arc<ClaudeExecutor>,
    /// Creation timestamp
    pub created_at: Instant,
}

impl Agent {
    /// Create a new agent instance
    pub fn new(config: AgentConfig) -> Self {
        let id = AgentId::new();
        let timeout_seconds = config.timeout_seconds;
        
        Self {
            id: id.clone(),
            state: Arc::new(RwLock::new(AgentState::Created)),
            config,
            process_manager: Arc::new(ProcessManager::new(id)),
            claude_executor: Arc::new(ClaudeExecutor::new(timeout_seconds)),
            created_at: Instant::now(),
        }
    }
    
    /// Get current agent state
    pub async fn get_state(&self) -> AgentState {
        *self.state.read().await
    }
    
    /// Set agent state
    pub async fn set_state(&self, new_state: AgentState) {
        *self.state.write().await = new_state;
    }
    
    /// Get agent uptime
    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }
    
    /// Check if agent is active
    pub async fn is_active(&self) -> bool {
        matches!(self.get_state().await, AgentState::Running | AgentState::Idle)
    }
    
    /// Get agent information for status reporting
    pub async fn get_info(&self) -> AgentInfo {
        AgentInfo {
            id: self.id.clone(),
            state: self.get_state().await,
            uptime: self.uptime(),
            config: self.config.clone(),
        }
    }
    
    /// Execute a prompt using Claude (single-shot execution)
    pub async fn execute(&self, prompt: &str) -> Result<String> {
        // Check if agent is in a valid state for execution
        let state = self.get_state().await;
        if !matches!(state, AgentState::Running | AgentState::Idle) {
            return Err(anyhow::anyhow!("Agent not in valid state for execution: {:?}", state));
        }
        
        // Set state to processing
        self.set_state(AgentState::Processing).await;
        
        // Execute prompt using ClaudeExecutor
        let result = self.claude_executor.execute(prompt).await;
        
        // Update state based on result
        match &result {
            Ok(response) => {
                self.set_state(AgentState::Idle).await;
                Ok(response.result.clone().unwrap_or_else(|| "No response".to_string()))
            }
            Err(e) => {
                self.set_state(AgentState::Error).await;
                Err(anyhow::anyhow!("Execution failed: {}", e))
            }
        }
    }
    
    // Note: spawn() and stop() methods are implemented in lifecycle.rs
    // to provide better error handling with LifecycleError
}

/// Agent information for status reporting
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub state: AgentState,
    pub uptime: Duration,
    pub config: AgentConfig,
}
