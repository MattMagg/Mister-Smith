//! Agent Lifecycle Management
//!
//! Handles agent lifecycle operations: spawn, stop, pause, resume.
//! Coordinates between agent state and Claude CLI process management.

use std::time::Duration;
use anyhow::Result;
use thiserror::Error;
use tracing::{info, warn, error};

use crate::agent::{Agent, AgentState};

/// Agent lifecycle operations
impl Agent {
    /// Spawn the Claude CLI process and start the agent
    pub async fn spawn(&self) -> Result<(), LifecycleError> {
        info!(agent_id = %self.id, "Spawning agent");
        
        // Check current state
        let current_state = self.get_state().await;
        if !matches!(current_state, AgentState::Created) {
            return Err(LifecycleError::InvalidState {
                current: current_state,
                expected: "Created".to_string(),
            });
        }
        
        // Set starting state
        self.set_state(AgentState::Starting).await;
        
        // Spawn Claude CLI process
        match self.process_manager.spawn(&self.config).await {
            Ok(_) => {
                self.set_state(AgentState::Running).await;
                info!(agent_id = %self.id, "Agent spawned successfully");
                Ok(())
            }
            Err(e) => {
                self.set_state(AgentState::Error).await;
                error!(agent_id = %self.id, error = %e, "Failed to spawn agent");
                Err(LifecycleError::SpawnFailed(e.to_string()))
            }
        }
    }
    
    /// Stop the agent gracefully
    pub async fn stop(&self) -> Result<(), LifecycleError> {
        info!(agent_id = %self.id, "Stopping agent");
        
        let current_state = self.get_state().await;
        if !current_state.can_stop() {
            return Err(LifecycleError::InvalidState {
                current: current_state,
                expected: "Running, Idle, Paused, or Error".to_string(),
            });
        }
        
        // Set stopping state
        self.set_state(AgentState::Stopping).await;
        
        // Stop Claude CLI process
        match self.process_manager.stop().await {
            Ok(_) => {
                self.set_state(AgentState::Terminated).await;
                info!(agent_id = %self.id, "Agent stopped successfully");
                Ok(())
            }
            Err(e) => {
                self.set_state(AgentState::Error).await;
                error!(agent_id = %self.id, error = %e, "Failed to stop agent");
                Err(LifecycleError::StopFailed(e.to_string()))
            }
        }
    }
    
    /// Pause the agent (suspend input processing)
    pub async fn pause(&self) -> Result<(), LifecycleError> {
        info!(agent_id = %self.id, "Pausing agent");
        
        let current_state = self.get_state().await;
        if !current_state.can_pause() {
            return Err(LifecycleError::InvalidState {
                current: current_state,
                expected: "Running or Idle".to_string(),
            });
        }
        
        // Pause processing (process stays alive)
        match self.process_manager.pause().await {
            Ok(_) => {
                self.set_state(AgentState::Paused).await;
                info!(agent_id = %self.id, "Agent paused successfully");
                Ok(())
            }
            Err(e) => {
                error!(agent_id = %self.id, error = %e, "Failed to pause agent");
                Err(LifecycleError::PauseFailed(e.to_string()))
            }
        }
    }
    
    /// Resume the agent from paused state
    pub async fn resume(&self) -> Result<(), LifecycleError> {
        info!(agent_id = %self.id, "Resuming agent");
        
        let current_state = self.get_state().await;
        if !matches!(current_state, AgentState::Paused) {
            return Err(LifecycleError::InvalidState {
                current: current_state,
                expected: "Paused".to_string(),
            });
        }
        
        // Resume processing
        match self.process_manager.resume().await {
            Ok(_) => {
                self.set_state(AgentState::Running).await;
                info!(agent_id = %self.id, "Agent resumed successfully");
                Ok(())
            }
            Err(e) => {
                error!(agent_id = %self.id, error = %e, "Failed to resume agent");
                Err(LifecycleError::ResumeFailed(e.to_string()))
            }
        }
    }
    
    /// Force kill the agent (for emergency situations)
    pub async fn kill(&self) -> Result<(), LifecycleError> {
        warn!(agent_id = %self.id, "Force killing agent");
        
        match self.process_manager.kill().await {
            Ok(_) => {
                self.set_state(AgentState::Terminated).await;
                warn!(agent_id = %self.id, "Agent killed");
                Ok(())
            }
            Err(e) => {
                self.set_state(AgentState::Error).await;
                error!(agent_id = %self.id, error = %e, "Failed to kill agent");
                Err(LifecycleError::KillFailed(e.to_string()))
            }
        }
    }
    
    /// Check if the agent process is healthy
    pub async fn health_check(&self) -> Result<bool, LifecycleError> {
        match self.process_manager.is_healthy().await {
            Ok(healthy) => {
                if !healthy && self.get_state().await.is_active() {
                    warn!(agent_id = %self.id, "Agent process unhealthy, marking as error");
                    self.set_state(AgentState::Error).await;
                }
                Ok(healthy)
            }
            Err(e) => {
                error!(agent_id = %self.id, error = %e, "Health check failed");
                Err(LifecycleError::HealthCheckFailed(e.to_string()))
            }
        }
    }
}

/// Errors that can occur during agent lifecycle operations
#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("Invalid state transition: current={current}, expected={expected}")]
    InvalidState {
        current: AgentState,
        expected: String,
    },
    
    #[error("Failed to spawn agent: {0}")]
    SpawnFailed(String),
    
    #[error("Failed to stop agent: {0}")]
    StopFailed(String),
    
    #[error("Failed to pause agent: {0}")]
    PauseFailed(String),
    
    #[error("Failed to resume agent: {0}")]
    ResumeFailed(String),
    
    #[error("Failed to kill agent: {0}")]
    KillFailed(String),
    
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),
}
