//! Message Router
//!
//! Routes NATS messages to appropriate handlers based on subject patterns.

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use async_nats::{Client, Message, Subscriber};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use crate::message::{MessageEnvelope, AgentCommand, AgentEvent, SystemMessage, CommandResponse};

/// Message router for handling NATS subscriptions and routing
#[derive(Debug)]
pub struct MessageRouter {
    /// NATS client
    client: Client,
    /// Active subscriptions
    subscriptions: Arc<RwLock<HashMap<String, Subscriber>>>,
    /// Message handlers
    handlers: Arc<RwLock<HashMap<String, Box<dyn MessageHandler + Send + Sync>>>>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new(client: Client) -> Self {
        Self {
            client,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Subscribe to a subject with a handler
    pub async fn subscribe<H>(
        &self,
        subject: &str,
        handler: H,
    ) -> Result<(), RouterError>
    where
        H: MessageHandler + Send + Sync + 'static,
    {
        info!(subject = subject, "Subscribing to NATS subject");
        
        // Create subscription
        let subscriber = self.client
            .subscribe(subject.to_string())
            .await
            .map_err(|e| RouterError::SubscriptionFailed(e.to_string()))?;
        
        // Store subscription and handler
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.insert(subject.to_string(), subscriber);
        }
        
        {
            let mut handlers = self.handlers.write().await;
            handlers.insert(subject.to_string(), Box::new(handler));
        }
        
        // Start message processing task
        self.spawn_message_processor(subject.to_string()).await?;
        
        Ok(())
    }
    
    /// Unsubscribe from a subject
    pub async fn unsubscribe(&self, subject: &str) -> Result<(), RouterError> {
        info!(subject = subject, "Unsubscribing from NATS subject");
        
        // Remove subscription and handler
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.remove(subject);
        }
        
        {
            let mut handlers = self.handlers.write().await;
            handlers.remove(subject);
        }
        
        Ok(())
    }
    
    /// Publish a message to a subject
    pub async fn publish<T>(
        &self,
        subject: &str,
        message: &MessageEnvelope<T>,
    ) -> Result<(), RouterError>
    where
        T: serde::Serialize,
    {
        let payload = serde_json::to_vec(message)
            .map_err(|e| RouterError::SerializationFailed(e.to_string()))?;
        
        self.client
            .publish(subject.to_string(), payload.into())
            .await
            .map_err(|e| RouterError::PublishFailed(e.to_string()))?;
        
        debug!(subject = subject, message_id = %message.message_id, "Published message");
        
        Ok(())
    }
    
    /// Request-response pattern
    pub async fn request<T, R>(
        &self,
        subject: &str,
        message: &MessageEnvelope<T>,
        timeout: std::time::Duration,
    ) -> Result<MessageEnvelope<R>, RouterError>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let payload = serde_json::to_vec(message)
            .map_err(|e| RouterError::SerializationFailed(e.to_string()))?;
        
        let response = tokio::time::timeout(
            timeout,
            self.client.request(subject.to_string(), payload.into())
        )
        .await
        .map_err(|_| RouterError::RequestTimeout)?
        .map_err(|e| RouterError::RequestFailed(e.to_string()))?;
        
        let response_message: MessageEnvelope<R> = serde_json::from_slice(&response.payload)
            .map_err(|e| RouterError::DeserializationFailed(e.to_string()))?;
        
        debug!(
            subject = subject,
            request_id = %message.message_id,
            response_id = %response_message.message_id,
            "Received response"
        );
        
        Ok(response_message)
    }
    
    /// Spawn message processor task for a subject
    async fn spawn_message_processor(&self, subject: String) -> Result<(), RouterError> {
        let subscriptions = self.subscriptions.clone();
        let handlers = self.handlers.clone();
        
        tokio::spawn(async move {
            loop {
                // Get subscription and handler
                let (mut subscriber, handler): (Subscriber, &Box<dyn MessageHandler + Send + Sync>) = {
                    let subs = subscriptions.read().await;
                    let handlers_guard = handlers.read().await;
                    
                    match (subs.get(&subject), handlers_guard.get(&subject)) {
                        (Some(sub), Some(handler)) => {
                            // Get subscriber and handler references
                            let subscriber = sub;
                            let handler = handler.as_ref();
                            
                            // We need to work with the handler differently
                            // For now, let's break out of this and handle differently
                            break;
                        }
                        _ => {
                            debug!(subject = %subject, "Subscription or handler not found, stopping processor");
                            break;
                        }
                    }
                };
            }
        });
        
        Ok(())
    }
    
    /// Get list of active subscriptions
    pub async fn get_subscriptions(&self) -> Vec<String> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.keys().cloned().collect()
    }
    
    /// Get router statistics
    pub async fn get_stats(&self) -> RouterStats {
        let subscriptions = self.subscriptions.read().await;
        let handlers = self.handlers.read().await;
        
        RouterStats {
            active_subscriptions: subscriptions.len(),
            registered_handlers: handlers.len(),
            subjects: subscriptions.keys().cloned().collect(),
        }
    }
}

/// Trait for message handlers
pub trait MessageHandler: std::fmt::Debug {
    /// Handle an incoming message
    fn handle_message(
        &self,
        message: Message,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), HandlerError>> + Send + '_>>;
}

/// Simple function-based message handler
pub struct FunctionHandler<F> {
    function: F,
}

impl<F> std::fmt::Debug for FunctionHandler<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionHandler").finish()
    }
}

impl<F> FunctionHandler<F>
where
    F: Fn(Message) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), HandlerError>> + Send>> + Send + Sync,
{
    pub fn new(function: F) -> Self {
        Self { function }
    }
}

impl<F> MessageHandler for FunctionHandler<F>
where
    F: Fn(Message) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), HandlerError>> + Send>> + Send + Sync,
{
    fn handle_message(
        &self,
        message: Message,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), HandlerError>> + Send + '_>> {
        (self.function)(message)
    }
}

/// Agent command handler
#[derive(Debug)]
pub struct AgentCommandHandler {
    // Handler implementation would go here
}

impl AgentCommandHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl MessageHandler for AgentCommandHandler {
    fn handle_message(
        &self,
        message: Message,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), HandlerError>> + Send + '_>> {
        Box::pin(async move {
            // Parse message
            let envelope: MessageEnvelope<AgentCommand> = serde_json::from_slice(&message.payload)
                .map_err(|e| HandlerError::MessageParseError(e.to_string()))?;
            
            debug!(message_id = %envelope.message_id, "Handling agent command");
            
            // Handle command based on type
            match &envelope.payload {
                AgentCommand::Spawn { config, reply_to } => {
                    info!("Handling spawn command");
                    // Implementation would spawn agent and optionally reply
                }
                AgentCommand::Stop { agent_id, graceful, timeout } => {
                    info!(agent_id = %agent_id, graceful = graceful, "Handling stop command");
                    // Implementation would stop agent
                }
                AgentCommand::Status { agent_id, reply_to } => {
                    info!(agent_id = %agent_id, "Handling status request");
                    // Implementation would get status and reply
                }
                _ => {
                    warn!("Unhandled agent command type");
                }
            }
            
            Ok(())
        })
    }
}

/// Router statistics
#[derive(Debug, Clone)]
pub struct RouterStats {
    pub active_subscriptions: usize,
    pub registered_handlers: usize,
    pub subjects: Vec<String>,
}

/// Router errors
#[derive(Debug, Error)]
pub enum RouterError {
    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),
    
    #[error("Publish failed: {0}")]
    PublishFailed(String),
    
    #[error("Request failed: {0}")]
    RequestFailed(String),
    
    #[error("Request timeout")]
    RequestTimeout,
    
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
    
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
    
    #[error("Handler error: {0}")]
    HandlerError(String),
}

/// Handler errors
#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("Message parse error: {0}")]
    MessageParseError(String),
    
    #[error("Handler execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}
