//! JetStream Transport Implementation
//!
//! JetStream integration for persistent messaging and state management.

use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;
use async_nats::jetstream::{self, Context, stream, consumer};
use async_nats::Client;
use thiserror::Error;
use tracing::{info, warn, error};

/// JetStream transport manager
#[derive(Debug)]
pub struct JetStreamTransport {
    /// JetStream context
    context: Context,
    /// Configured streams
    streams: HashMap<String, StreamInfo>,
    /// Configured consumers
    consumers: HashMap<String, ConsumerInfo>,
}

impl JetStreamTransport {
    /// Create a new JetStream transport
    pub async fn new(client: Client) -> Result<Self, JetStreamError> {
        info!("Initializing JetStream transport");
        
        let context = jetstream::new(client);
        
        let mut transport = Self {
            context,
            streams: HashMap::new(),
            consumers: HashMap::new(),
        };
        
        // Set up default streams
        transport.setup_default_streams().await?;
        
        info!("JetStream transport initialized successfully");
        Ok(transport)
    }
    
    /// Set up default streams for agent communication
    async fn setup_default_streams(&mut self) -> Result<(), JetStreamError> {
        // Agent Events Stream
        self.create_agent_events_stream().await?;
        
        // System Events Stream
        self.create_system_events_stream().await?;
        
        Ok(())
    }
    
    /// Create agent events stream
    async fn create_agent_events_stream(&mut self) -> Result<(), JetStreamError> {
        let stream_config = stream::Config {
            name: "AGENT_EVENTS".to_string(),
            subjects: vec![
                "agent.*.events".to_string(),
                "agent.*.status".to_string(),
                "agent.*.output".to_string(),
            ],
            retention: stream::RetentionPolicy::Limits,
            max_age: Some(Duration::from_secs(24 * 60 * 60)), // 24 hours
            max_bytes: Some(1024 * 1024 * 1024), // 1GB
            max_messages: Some(1_000_000),
            max_message_size: Some(1024 * 1024), // 1MB
            storage: stream::StorageType::File,
            num_replicas: 1, // Single replica for simple setup
            discard: stream::DiscardPolicy::Old,
            duplicate_window: Some(Duration::from_secs(120)),
            ..Default::default()
        };
        
        match self.context.create_stream(stream_config.clone()).await {
            Ok(stream) => {
                info!(stream_name = %stream_config.name, "Created agent events stream");
                self.streams.insert(
                    stream_config.name.clone(),
                    StreamInfo {
                        name: stream_config.name,
                        subjects: stream_config.subjects,
                        config: stream_config,
                    }
                );
            }
            Err(e) => {
                // Stream might already exist, try to get it
                match self.context.get_stream("AGENT_EVENTS").await {
                    Ok(_) => {
                        info!("Agent events stream already exists");
                        self.streams.insert(
                            stream_config.name.clone(),
                            StreamInfo {
                                name: stream_config.name,
                                subjects: stream_config.subjects,
                                config: stream_config,
                            }
                        );
                    }
                    Err(_) => {
                        error!(error = %e, "Failed to create agent events stream");
                        return Err(JetStreamError::StreamCreationFailed(e.to_string()));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Create system events stream
    async fn create_system_events_stream(&mut self) -> Result<(), JetStreamError> {
        let stream_config = stream::Config {
            name: "SYSTEM_EVENTS".to_string(),
            subjects: vec![
                "system.control".to_string(),
                "system.health".to_string(),
                "pool.management".to_string(),
            ],
            retention: stream::RetentionPolicy::Limits,
            max_age: Some(Duration::from_secs(7 * 24 * 60 * 60)), // 7 days
            max_bytes: Some(512 * 1024 * 1024), // 512MB
            max_messages: Some(100_000),
            max_message_size: Some(256 * 1024), // 256KB
            storage: stream::StorageType::File,
            num_replicas: 1,
            discard: stream::DiscardPolicy::Old,
            duplicate_window: Some(Duration::from_secs(60)),
            ..Default::default()
        };
        
        match self.context.create_stream(stream_config.clone()).await {
            Ok(_) => {
                info!(stream_name = %stream_config.name, "Created system events stream");
                self.streams.insert(
                    stream_config.name.clone(),
                    StreamInfo {
                        name: stream_config.name,
                        subjects: stream_config.subjects,
                        config: stream_config,
                    }
                );
            }
            Err(e) => {
                // Stream might already exist
                match self.context.get_stream("SYSTEM_EVENTS").await {
                    Ok(_) => {
                        info!("System events stream already exists");
                        self.streams.insert(
                            stream_config.name.clone(),
                            StreamInfo {
                                name: stream_config.name,
                                subjects: stream_config.subjects,
                                config: stream_config,
                            }
                        );
                    }
                    Err(_) => {
                        warn!(error = %e, "Failed to create system events stream, continuing without persistence");
                        // Don't fail completely - system can work without persistence
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Create a consumer for a stream
    pub async fn create_consumer(
        &mut self,
        stream_name: &str,
        consumer_name: &str,
        filter_subject: Option<&str>,
    ) -> Result<(), JetStreamError> {
        let consumer_config = consumer::pull::Config {
            name: Some(consumer_name.to_string()),
            deliver_policy: consumer::DeliverPolicy::New,
            ack_policy: consumer::AckPolicy::Explicit,
            ack_wait: Duration::from_secs(30),
            max_deliver: Some(3),
            filter_subject: filter_subject.map(|s| s.to_string()),
            replay_policy: consumer::ReplayPolicy::Instant,
            max_ack_pending: Some(1000),
            ..Default::default()
        };
        
        match self.context.create_consumer_on_stream(
            consumer_config.clone().into(),
            stream_name
        ).await {
            Ok(_) => {
                info!(
                    stream_name = stream_name,
                    consumer_name = consumer_name,
                    "Created JetStream consumer"
                );
                
                self.consumers.insert(
                    consumer_name.to_string(),
                    ConsumerInfo {
                        name: consumer_name.to_string(),
                        stream_name: stream_name.to_string(),
                        filter_subject: filter_subject.map(|s| s.to_string()),
                    }
                );
            }
            Err(e) => {
                error!(
                    error = %e,
                    stream_name = stream_name,
                    consumer_name = consumer_name,
                    "Failed to create JetStream consumer"
                );
                return Err(JetStreamError::ConsumerCreationFailed(e.to_string()));
            }
        }
        
        Ok(())
    }
    
    /// Publish a message to JetStream
    pub async fn publish(
        &self,
        subject: &str,
        payload: Vec<u8>,
    ) -> Result<jetstream::PublishAck, JetStreamError> {
        self.context
            .publish(subject.to_string(), payload.into())
            .await
            .map_err(|e| JetStreamError::PublishFailed(e.to_string()))
    }
    
    /// Get a consumer for message processing
    pub async fn get_consumer(
        &self,
        stream_name: &str,
        consumer_name: &str,
    ) -> Result<jetstream::consumer::pull::Consumer, JetStreamError> {
        self.context
            .get_consumer_from_stream(consumer_name, stream_name)
            .await
            .map_err(|e| JetStreamError::ConsumerNotFound(e.to_string()))
    }
    
    /// Get JetStream context
    pub fn context(&self) -> &Context {
        &self.context
    }
    
    /// List configured streams
    pub fn list_streams(&self) -> Vec<String> {
        self.streams.keys().cloned().collect()
    }
    
    /// List configured consumers
    pub fn list_consumers(&self) -> Vec<String> {
        self.consumers.keys().cloned().collect()
    }
    
    /// Get JetStream account information
    pub async fn account_info(&self) -> Result<jetstream::AccountInfo, JetStreamError> {
        self.context
            .account_info()
            .await
            .map_err(|e| JetStreamError::AccountInfoFailed(e.to_string()))
    }
    
    /// Get statistics for all streams
    pub async fn get_stats(&self) -> Result<JetStreamStats, JetStreamError> {
        let account_info = self.account_info().await?;
        
        Ok(JetStreamStats {
            streams_count: self.streams.len(),
            consumers_count: self.consumers.len(),
            memory_usage: account_info.memory,
            storage_usage: account_info.storage,
            api_requests: account_info.api.total,
            api_errors: account_info.api.errors,
        })
    }
}

/// Stream information
#[derive(Debug, Clone)]
struct StreamInfo {
    name: String,
    subjects: Vec<String>,
    config: stream::Config,
}

/// Consumer information
#[derive(Debug, Clone)]
struct ConsumerInfo {
    name: String,
    stream_name: String,
    filter_subject: Option<String>,
}

/// JetStream statistics
#[derive(Debug, Clone)]
pub struct JetStreamStats {
    pub streams_count: usize,
    pub consumers_count: usize,
    pub memory_usage: u64,
    pub storage_usage: u64,
    pub api_requests: u64,
    pub api_errors: u64,
}

/// JetStream-specific errors
#[derive(Debug, Error)]
pub enum JetStreamError {
    #[error("Stream creation failed: {0}")]
    StreamCreationFailed(String),
    
    #[error("Consumer creation failed: {0}")]
    ConsumerCreationFailed(String),
    
    #[error("Consumer not found: {0}")]
    ConsumerNotFound(String),
    
    #[error("Publish failed: {0}")]
    PublishFailed(String),
    
    #[error("Account info failed: {0}")]
    AccountInfoFailed(String),
    
    #[error("Stream not found: {0}")]
    StreamNotFound(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
