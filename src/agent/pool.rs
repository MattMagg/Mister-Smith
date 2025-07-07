//! Agent Pool Management
//!
//! Resource-bounded pool management for Claude CLI agents.
//! Manages concurrent agent limits and resource allocation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, Semaphore};
use anyhow::Result;
use thiserror::Error;
use tracing::{info, warn, error};
use futures::future::join_all;

use crate::agent::{Agent, AgentId};

/// Agent pool for managing concurrent Claude CLI agents
#[derive(Debug)]
pub struct AgentPool {
    /// Maximum number of concurrent agents
    max_size: usize,
    /// Currently active agents
    active_agents: Arc<RwLock<HashMap<AgentId, AgentPoolEntry>>>,
    /// Semaphore for controlling concurrent spawns
    spawn_semaphore: Arc<Semaphore>,
    /// Pool metrics
    metrics: PoolMetrics,
}

impl AgentPool {
    /// Create a new agent pool with specified maximum size
    pub fn new(max_size: usize) -> Self {
        info!(max_size = max_size, "Creating agent pool");
        
        Self {
            max_size,
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            spawn_semaphore: Arc::new(Semaphore::new(max_size)),
            metrics: PoolMetrics::new(),
        }
    }
    
    /// Register an agent in the pool
    pub async fn register(&self, agent: Agent) -> Result<(), PoolError> {
        // Acquire spawn permit (this will block if pool is full)
        let _permit = self.spawn_semaphore.acquire().await
            .map_err(|e| PoolError::PermitAcquisitionFailed(e.to_string()))?;
        
        // Forget the permit so it doesn't get dropped
        std::mem::forget(_permit);
        
        let agent_id = agent.id.clone();
        let entry = AgentPoolEntry {
            agent,
            registered_at: Instant::now(),
        };
        
        // Register agent in pool
        {
            let mut agents = self.active_agents.write().await;
            agents.insert(agent_id.clone(), entry);
        }
        
        // Update metrics
        self.metrics.increment_active_count();
        
        info!(agent_id = %agent_id, active_count = self.metrics.active_count(), "Agent registered in pool");
        
        Ok(())
    }
    
    /// Unregister an agent from the pool
    pub async fn unregister(&self, agent_id: &AgentId) -> Result<(), PoolError> {
        let removed = {
            let mut agents = self.active_agents.write().await;
            agents.remove(agent_id)
        };
        
        if removed.is_some() {
            // Update metrics
            self.metrics.decrement_active_count();
            
            info!(agent_id = %agent_id, active_count = self.metrics.active_count(), "Agent unregistered from pool");
            Ok(())
        } else {
            warn!(agent_id = %agent_id, "Attempted to unregister unknown agent");
            Err(PoolError::AgentNotFound(agent_id.clone()))
        }
    }
    
    /// Get an agent from the pool
    pub async fn get(&self, agent_id: &AgentId) -> Option<Agent> {
        let agents = self.active_agents.read().await;
        agents.get(agent_id).map(|entry| entry.agent.clone())
    }
    
    /// List all active agents
    pub async fn list_active_agents(&self) -> Vec<AgentId> {
        let agents = self.active_agents.read().await;
        agents.keys().cloned().collect()
    }
    
    /// Get pool status
    pub async fn get_status(&self) -> PoolStatus {
        let agents = self.active_agents.read().await;
        
        PoolStatus {
            active_count: agents.len(),
            max_capacity: self.max_size,
            available_slots: self.spawn_semaphore.available_permits(),
            agents: agents.keys().cloned().collect(),
        }
    }
    
    /// Check if pool has available capacity
    pub fn has_capacity(&self) -> bool {
        self.spawn_semaphore.available_permits() > 0
    }
    
    /// Get metrics
    pub fn metrics(&self) -> &PoolMetrics {
        &self.metrics
    }
    
    /// Shutdown all agents in the pool
    pub async fn shutdown_all(&self) -> Result<(), PoolError> {
        info!("Shutting down all agents in pool");
        
        let agents: Vec<Agent> = {
            let agents_map = self.active_agents.read().await;
            agents_map.values().map(|entry| entry.agent.clone()).collect()
        };
        
        let mut errors = Vec::new();
        
        // Stop all agents concurrently
        let stop_futures: Vec<_> = agents.iter().map(|agent| async {
            if let Err(e) = agent.stop().await {
                error!(agent_id = %agent.id, error = %e, "Failed to stop agent during shutdown");
                return Err(e);
            }
            Ok(())
        }).collect();
        
        // Wait for all stops to complete
        for result in join_all(stop_futures).await {
            if let Err(e) = result {
                errors.push(e.to_string());
            }
        }
        
        // Clear the pool
        {
            let mut agents_map = self.active_agents.write().await;
            agents_map.clear();
        }
        
        // Reset metrics
        self.metrics.reset();
        
        if errors.is_empty() {
            info!("All agents shut down successfully");
            Ok(())
        } else {
            error!(errors = ?errors, "Some agents failed to shut down");
            Err(PoolError::ShutdownErrors(errors))
        }
    }
}

/// Entry in the agent pool
#[derive(Debug, Clone)]
struct AgentPoolEntry {
    agent: Agent,
    registered_at: Instant,
}

/// Pool status information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoolStatus {
    pub active_count: usize,
    pub max_capacity: usize,
    pub available_slots: usize,
    pub agents: Vec<AgentId>,
}

/// Pool metrics
#[derive(Debug)]
pub struct PoolMetrics {
    active_count: Arc<std::sync::atomic::AtomicUsize>,
    total_spawned: Arc<std::sync::atomic::AtomicUsize>,
    total_terminated: Arc<std::sync::atomic::AtomicUsize>,
}

impl PoolMetrics {
    fn new() -> Self {
        use std::sync::atomic::AtomicUsize;
        
        Self {
            active_count: Arc::new(AtomicUsize::new(0)),
            total_spawned: Arc::new(AtomicUsize::new(0)),
            total_terminated: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    fn increment_active_count(&self) {
        use std::sync::atomic::Ordering;
        
        self.active_count.fetch_add(1, Ordering::Relaxed);
        self.total_spawned.fetch_add(1, Ordering::Relaxed);
    }
    
    fn decrement_active_count(&self) {
        use std::sync::atomic::Ordering;
        
        self.active_count.fetch_sub(1, Ordering::Relaxed);
        self.total_terminated.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn active_count(&self) -> usize {
        use std::sync::atomic::Ordering;
        self.active_count.load(Ordering::Relaxed)
    }
    
    pub fn total_spawned(&self) -> usize {
        use std::sync::atomic::Ordering;
        self.total_spawned.load(Ordering::Relaxed)
    }
    
    pub fn total_terminated(&self) -> usize {
        use std::sync::atomic::Ordering;
        self.total_terminated.load(Ordering::Relaxed)
    }
    
    fn reset(&self) {
        use std::sync::atomic::Ordering;
        
        self.active_count.store(0, Ordering::Relaxed);
        // Note: Don't reset total counters as they're cumulative
    }
}

/// Errors that can occur in agent pool operations
#[derive(Debug, Error)]
pub enum PoolError {
    #[error("Failed to acquire spawn permit: {0}")]
    PermitAcquisitionFailed(String),
    
    #[error("Agent not found: {0}")]
    AgentNotFound(AgentId),
    
    #[error("Pool is at maximum capacity")]
    PoolFull,
    
    #[error("Shutdown errors: {0:?}")]
    ShutdownErrors(Vec<String>),
}
