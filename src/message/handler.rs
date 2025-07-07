//! Message Handlers
//!
//! Implements message handlers for different types of agent messages.

use std::sync::Arc;
use anyhow::Result;
use async_nats::{Client, Message};
use thiserror::Error;
use tracing::{info, warn, error, debug};

use crate::agent::{Agent, AgentPool};
use crate::message::{
    MessageEnvelope, AgentCommand, AgentEvent, SystemMessage, CommandResponse,
    Subject, Priority, HandlerError
};

/// Agent message handler that coordinates with the agent pool
#[derive(Debug, Clone)]
pub struct AgentMessageHandler {
    /// Agent pool for managing agents
    agent_pool: Arc<AgentPool>,
    /// NATS client for publishing responses
    nats_client: Client,
}

impl AgentMessageHandler {
    /// Create a new agent message handler
    pub fn new(agent_pool: Arc<AgentPool>, nats_client: Client) -> Self {
        Self {
            agent_pool,
            nats_client,
        }
    }
    
    /// Handle agent command messages
    pub async fn handle_agent_command(&self, message: Message) -> Result<(), HandlerError> {
        // Parse the message envelope
        let envelope: MessageEnvelope<AgentCommand> = serde_json::from_slice(&message.payload)
            .map_err(|e| HandlerError::MessageParseError(e.to_string()))?;
        
        debug!(
            message_id = %envelope.message_id,
            command = ?envelope.payload,
            "Processing agent command"
        );
        
        // Handle the command
        let result = self.process_agent_command(&envelope).await;
        
        // Send response if reply subject is available
        if let Some(reply_to) = message.reply {
            let response = match result {
                Ok(data) => CommandResponse::Success {
                    message: "Command executed successfully".to_string(),
                    data,
                },
                Err(e) => CommandResponse::Error {
                    error: e.to_string(),
                    code: None,
                },
            };
            
            let response_envelope = MessageEnvelope::new(response)
                .with_correlation(envelope.message_id);
            
            self.publish_response(&reply_to, &response_envelope).await?;
        }
        
        Ok(())
    }
    
    /// Process individual agent commands
    async fn process_agent_command(
        &self,
        envelope: &MessageEnvelope<AgentCommand>,
    ) -> Result<Option<serde_json::Value>, AgentOperationError> {
        match &envelope.payload {
            AgentCommand::Spawn { config, reply_to: _ } => {
                info!("Spawning new agent");
                
                // Create new agent
                let agent = Agent::new(config.clone());
                let agent_id = agent.id.clone();
                
                // Register with pool (this may block if pool is full)
                let _permit = self.agent_pool.register(agent.clone()).await
                    .map_err(|e| AgentOperationError::PoolError(e.to_string()))?;
                
                // Spawn the agent process
                agent.spawn().await
                    .map_err(|e| AgentOperationError::SpawnFailed(e.to_string()))?;
                
                // Publish spawn event
                self.publish_agent_event(AgentEvent::Spawned {
                    agent_id: agent_id.clone(),
                    config: config.clone(),
                }).await
                .map_err(|e| AgentOperationError::PoolError(format!("Failed to publish event: {}", e)))?;
                
                info!(agent_id = %agent_id, "Agent spawned successfully");
                
                Ok(Some(serde_json::json!({
                    "agent_id": agent_id,
                    "state": "spawned"
                })))
            }
            
            AgentCommand::Stop { agent_id, graceful, timeout: _ } => {
                info!(agent_id = %agent_id, graceful = graceful, "Stopping agent");
                
                // Get agent from pool
                let agent = self.agent_pool.get(agent_id).await
                    .ok_or_else(|| AgentOperationError::AgentNotFound(agent_id.clone()))?;
                
                // Stop the agent
                if *graceful {
                    agent.stop().await
                        .map_err(|e| AgentOperationError::StopFailed(e.to_string()))?;
                } else {
                    agent.kill().await
                        .map_err(|e| AgentOperationError::KillFailed(e.to_string()))?;
                }
                
                // Unregister from pool
                self.agent_pool.unregister(agent_id).await
                    .map_err(|e| AgentOperationError::PoolError(e.to_string()))?;
                
                // Publish termination event
                self.publish_agent_event(AgentEvent::Terminated {
                    agent_id: agent_id.clone(),
                    reason: if *graceful { "graceful_stop".to_string() } else { "forced_stop".to_string() },
                }).await
                .map_err(|e| AgentOperationError::PoolError(format!("Failed to publish event: {}", e)))?;
                
                info!(agent_id = %agent_id, "Agent stopped successfully");
                
                Ok(Some(serde_json::json!({
                    "agent_id": agent_id,
                    "state": "stopped"
                })))
            }
            
            AgentCommand::Pause { agent_id } => {
                info!(agent_id = %agent_id, "Pausing agent");
                
                let agent = self.agent_pool.get(agent_id).await
                    .ok_or_else(|| AgentOperationError::AgentNotFound(agent_id.clone()))?;
                
                agent.pause().await
                    .map_err(|e| AgentOperationError::PauseFailed(e.to_string()))?;
                
                Ok(Some(serde_json::json!({
                    "agent_id": agent_id,
                    "state": "paused"
                })))
            }
            
            AgentCommand::Resume { agent_id } => {
                info!(agent_id = %agent_id, "Resuming agent");
                
                let agent = self.agent_pool.get(agent_id).await
                    .ok_or_else(|| AgentOperationError::AgentNotFound(agent_id.clone()))?;
                
                agent.resume().await
                    .map_err(|e| AgentOperationError::ResumeFailed(e.to_string()))?;
                
                Ok(Some(serde_json::json!({
                    "agent_id": agent_id,
                    "state": "running"
                })))
            }
            
            AgentCommand::Status { agent_id, reply_to: _ } => {
                debug!(agent_id = %agent_id, "Getting agent status");
                
                let agent = self.agent_pool.get(agent_id).await
                    .ok_or_else(|| AgentOperationError::AgentNotFound(agent_id.clone()))?;
                
                let info = agent.get_info().await;
                
                Ok(Some(serde_json::to_value(info)
                    .map_err(|e| AgentOperationError::SerializationError(e.to_string()))?))
            }
            
            AgentCommand::Input { agent_id, content } => {
                debug!(agent_id = %agent_id, "Sending input to agent");
                
                let agent = self.agent_pool.get(agent_id).await
                    .ok_or_else(|| AgentOperationError::AgentNotFound(agent_id.clone()))?;
                
                // Send input to the agent's process
                agent.process_manager.send_input(content).await
                    .map_err(|e| AgentOperationError::InputFailed(e.to_string()))?;
                
                Ok(Some(serde_json::json!({
                    "agent_id": agent_id,
                    "status": "input_sent"
                })))
            }
            
            AgentCommand::HealthCheck { agent_id, reply_to: _ } => {
                debug!(agent_id = %agent_id, "Performing health check");
                
                let agent = self.agent_pool.get(agent_id).await
                    .ok_or_else(|| AgentOperationError::AgentNotFound(agent_id.clone()))?;
                
                let healthy = agent.health_check().await
                    .map_err(|e| AgentOperationError::HealthCheckFailed(e.to_string()))?;
                
                Ok(Some(serde_json::json!({
                    "agent_id": agent_id,
                    "healthy": healthy
                })))
            }
        }
    }
    
    /// Publish an agent event
    async fn publish_agent_event(&self, event: AgentEvent) -> Result<(), HandlerError> {
        let agent_id = match &event {
            AgentEvent::Spawned { agent_id, .. } => agent_id,
            AgentEvent::StateChanged { agent_id, .. } => agent_id,
            AgentEvent::Terminated { agent_id, .. } => agent_id,
            AgentEvent::Error { agent_id, .. } => agent_id,
            AgentEvent::Output { agent_id, .. } => agent_id,
            AgentEvent::Heartbeat { agent_id, .. } => agent_id,
        };
        
        let subject = Subject::agent_events(agent_id);
        let envelope = MessageEnvelope::new(event);
        
        let payload = serde_json::to_vec(&envelope)
            .map_err(|e| HandlerError::MessageParseError(e.to_string()))?;
        
        self.nats_client.publish(subject, payload.into()).await
            .map_err(|e| HandlerError::ExecutionFailed(e.to_string()))?;
        
        Ok(())
    }
    
    /// Publish a response message
    async fn publish_response<T>(
        &self,
        reply_to: &str,
        response: &MessageEnvelope<T>,
    ) -> Result<(), HandlerError>
    where
        T: serde::Serialize,
    {
        let payload = serde_json::to_vec(response)
            .map_err(|e| HandlerError::MessageParseError(e.to_string()))?;
        
        self.nats_client.publish(reply_to.to_string(), payload.into()).await
            .map_err(|e| HandlerError::ExecutionFailed(e.to_string()))?;
        
        Ok(())
    }
}

/// System message handler for pool and system operations
#[derive(Debug, Clone)]
pub struct SystemMessageHandler {
    agent_pool: Arc<AgentPool>,
    nats_client: Client,
}

impl SystemMessageHandler {
    pub fn new(agent_pool: Arc<AgentPool>, nats_client: Client) -> Self {
        Self {
            agent_pool,
            nats_client,
        }
    }
    
    /// Handle system messages
    pub async fn handle_system_message(&self, message: Message) -> Result<(), HandlerError> {
        let envelope: MessageEnvelope<SystemMessage> = serde_json::from_slice(&message.payload)
            .map_err(|e| HandlerError::MessageParseError(e.to_string()))?;
        
        match &envelope.payload {
            SystemMessage::PoolStatus { reply_to } => {
                let status = self.agent_pool.get_status().await;
                let response = MessageEnvelope::new(CommandResponse::Success {
                    message: "Pool status retrieved".to_string(),
                    data: Some(serde_json::to_value(status)
                        .map_err(|e| HandlerError::ExecutionFailed(e.to_string()))?),
                });
                
                self.publish_response(reply_to, &response).await?;
            }
            
            SystemMessage::SystemHealth { reply_to } => {
                // Perform system health check
                let pool_status = self.agent_pool.get_status().await;
                let health_data = serde_json::json!({
                    "pool_active": pool_status.active_count,
                    "pool_capacity": pool_status.max_capacity,
                    "pool_available": pool_status.available_slots,
                    "healthy": true
                });
                
                let response = MessageEnvelope::new(CommandResponse::Success {
                    message: "System health check completed".to_string(),
                    data: Some(health_data),
                });
                
                self.publish_response(reply_to, &response).await?;
            }
            
            SystemMessage::Startup => {
                info!("System startup notification received");
            }
            
            SystemMessage::Shutdown => {
                info!("System shutdown notification received");
                // Could trigger graceful shutdown of all agents
            }
        }
        
        Ok(())
    }
    
    async fn publish_response<T>(
        &self,
        reply_to: &str,
        response: &MessageEnvelope<T>,
    ) -> Result<(), HandlerError>
    where
        T: serde::Serialize,
    {
        let payload = serde_json::to_vec(response)
            .map_err(|e| HandlerError::MessageParseError(e.to_string()))?;
        
        self.nats_client.publish(reply_to.to_string(), payload.into()).await
            .map_err(|e| HandlerError::ExecutionFailed(e.to_string()))?;
        
        Ok(())
    }
}

// Implement From<AgentOperationError> for HandlerError
impl From<AgentOperationError> for HandlerError {
    fn from(err: AgentOperationError) -> Self {
        HandlerError::OperationFailed(err.to_string())
    }
}

/// Errors specific to agent operations
#[derive(Debug, Error)]
pub enum AgentOperationError {
    #[error("Agent not found: {0}")]
    AgentNotFound(crate::agent::AgentId),
    
    #[error("Pool error: {0}")]
    PoolError(String),
    
    #[error("Spawn failed: {0}")]
    SpawnFailed(String),
    
    #[error("Stop failed: {0}")]
    StopFailed(String),
    
    #[error("Kill failed: {0}")]
    KillFailed(String),
    
    #[error("Pause failed: {0}")]
    PauseFailed(String),
    
    #[error("Resume failed: {0}")]
    ResumeFailed(String),
    
    #[error("Input failed: {0}")]
    InputFailed(String),
    
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}
