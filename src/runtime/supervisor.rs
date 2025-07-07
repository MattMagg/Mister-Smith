//! Process Supervisor
//!
//! Provides supervision and fault tolerance for Claude CLI processes.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use anyhow::Result;
use thiserror::Error;
use tracing::{info, warn, error};

use crate::agent::{Agent, AgentState};

/// Supervisor for monitoring and managing agent health
#[derive(Debug)]
pub struct AgentSupervisor {
    /// Agent being supervised
    agent: Agent,
    /// Supervision configuration
    config: SupervisorConfig,
    /// Supervision state
    state: Arc<RwLock<SupervisorState>>,
    /// Number of restart attempts
    restart_count: Arc<RwLock<u32>>,
}

impl AgentSupervisor {
    /// Create a new supervisor for the given agent
    pub fn new(agent: Agent, config: SupervisorConfig) -> Self {
        Self {
            agent,
            config,
            state: Arc::new(RwLock::new(SupervisorState::Monitoring)),
            restart_count: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Start supervising the agent
    pub async fn start(&self) -> Result<(), SupervisorError> {
        info!(agent_id = %self.agent.id, "Starting agent supervision");
        
        *self.state.write().await = SupervisorState::Active;
        
        // Start health check loop
        let health_check_task = self.spawn_health_check_task();
        
        // Start restart monitor if enabled
        let restart_task = if self.config.auto_restart {
            Some(self.spawn_restart_monitor_task())
        } else {
            None
        };
        
        // Wait for supervisor to be stopped
        tokio::select! {
            _ = health_check_task => {
                warn!(agent_id = %self.agent.id, "Health check task ended");
            }
            _ = async {
                if let Some(task) = restart_task {
                    task.await
                } else {
                    std::future::pending().await
                }
            } => {
                warn!(agent_id = %self.agent.id, "Restart monitor task ended");
            }
        }
        
        *self.state.write().await = SupervisorState::Stopped;
        Ok(())
    }
    
    /// Stop supervising the agent
    pub async fn stop(&self) -> Result<(), SupervisorError> {
        info!(agent_id = %self.agent.id, "Stopping agent supervision");
        
        *self.state.write().await = SupervisorState::Stopping;
        
        // The supervision tasks will see the state change and exit
        
        Ok(())
    }
    
    /// Get supervision status
    pub async fn status(&self) -> SupervisorStatus {
        SupervisorStatus {
            state: *self.state.read().await,
            restart_count: *self.restart_count.read().await,
            agent_state: self.agent.get_state().await,
            config: self.config.clone(),
        }
    }
    
    /// Spawn health check monitoring task
    fn spawn_health_check_task(&self) -> tokio::task::JoinHandle<()> {
        let agent = self.agent.clone();
        let config = self.config.clone();
        let state = self.state.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(config.health_check_interval);
            
            loop {
                interval.tick().await;
                
                // Check if supervisor should stop
                let current_state = *state.read().await;
                if matches!(current_state, SupervisorState::Stopping | SupervisorState::Stopped) {
                    break;
                }
                
                // Perform health check
                match agent.health_check().await {
                    Ok(healthy) => {
                        if !healthy {
                            warn!(agent_id = %agent.id, "Agent health check failed");
                        }
                    }
                    Err(e) => {
                        error!(agent_id = %agent.id, error = %e, "Health check error");
                    }
                }
            }
            
            info!(agent_id = %agent.id, "Health check task stopped");
        })
    }
    
    /// Spawn restart monitoring task
    fn spawn_restart_monitor_task(&self) -> tokio::task::JoinHandle<()> {
        let agent = self.agent.clone();
        let config = self.config.clone();
        let state = self.state.clone();
        let restart_count = self.restart_count.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(config.restart_check_interval);
            
            loop {
                interval.tick().await;
                
                // Check if supervisor should stop
                let current_state = *state.read().await;
                if matches!(current_state, SupervisorState::Stopping | SupervisorState::Stopped) {
                    break;
                }
                
                // Check if agent needs restart
                let agent_state = agent.get_state().await;
                if matches!(agent_state, AgentState::Error | AgentState::Terminated) {
                    let current_restart_count = *restart_count.read().await;
                    
                    if current_restart_count < config.max_restarts {
                        info!(
                            agent_id = %agent.id,
                            restart_count = current_restart_count,
                            "Attempting agent restart"
                        );
                        
                        match agent.spawn().await {
                            Ok(_) => {
                                *restart_count.write().await += 1;
                                info!(agent_id = %agent.id, "Agent restarted successfully");
                            }
                            Err(e) => {
                                error!(agent_id = %agent.id, error = %e, "Failed to restart agent");
                                *restart_count.write().await += 1;
                            }
                        }
                        
                        // Wait before next restart attempt
                        tokio::time::sleep(config.restart_delay).await;
                    } else {
                        error!(
                            agent_id = %agent.id,
                            restart_count = current_restart_count,
                            max_restarts = config.max_restarts,
                            "Maximum restart attempts reached, giving up"
                        );
                        break;
                    }
                }
            }
            
            info!(agent_id = %agent.id, "Restart monitor task stopped");
        })
    }
}

/// Supervisor configuration
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    /// Interval for health checks
    pub health_check_interval: Duration,
    /// Interval for checking if restart is needed
    pub restart_check_interval: Duration,
    /// Whether to automatically restart failed agents
    pub auto_restart: bool,
    /// Maximum number of restart attempts
    pub max_restarts: u32,
    /// Delay between restart attempts
    pub restart_delay: Duration,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            health_check_interval: Duration::from_secs(30),
            restart_check_interval: Duration::from_secs(10),
            auto_restart: true,
            max_restarts: 3,
            restart_delay: Duration::from_secs(5),
        }
    }
}

/// Supervisor state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorState {
    /// Supervisor is monitoring but not active
    Monitoring,
    /// Supervisor is actively supervising
    Active,
    /// Supervisor is stopping
    Stopping,
    /// Supervisor has stopped
    Stopped,
}

/// Supervisor status information
#[derive(Debug, Clone)]
pub struct SupervisorStatus {
    pub state: SupervisorState,
    pub restart_count: u32,
    pub agent_state: AgentState,
    pub config: SupervisorConfig,
}

/// Errors that can occur during supervision
#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("Supervisor already running")]
    AlreadyRunning,
    
    #[error("Supervisor not running")]
    NotRunning,
    
    #[error("Agent supervision failed: {0}")]
    SupervisionFailed(String),
    
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),
    
    #[error("Restart failed: {0}")]
    RestartFailed(String),
}
