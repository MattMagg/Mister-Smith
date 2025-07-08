//! Discovery Sharing - Real-time insight broadcasting between agents

use async_nats::Client;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{info, debug, error};
use futures::StreamExt;

/// A discovery that agents share in real-time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discovery {
    pub agent_id: String,
    pub agent_role: String,
    pub discovery_type: DiscoveryType,
    pub content: String,
    pub confidence: f32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub related_to: Option<String>, // Links discoveries
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DiscoveryType {
    Pattern,      // Found a pattern
    Anomaly,      // Something unusual
    Connection,   // Linked two things
    Solution,     // Potential fix
    Question,     // Need help
    Insight,      // General insight
}

/// Enables agents to share discoveries in real-time
#[derive(Clone, Debug)]
pub struct DiscoveryBroadcaster {
    agent_id: String,
    agent_role: String,
    nats: Client,
    discoveries: Arc<RwLock<Vec<Discovery>>>,
}

impl DiscoveryBroadcaster {
    pub fn new(agent_id: String, agent_role: String, nats: Client) -> Self {
        Self {
            agent_id,
            agent_role,
            nats,
            discoveries: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Broadcast a discovery to all listening agents
    pub async fn share_discovery(
        &self,
        discovery_type: DiscoveryType,
        content: &str,
        confidence: f32,
    ) -> Result<()> {
        let discovery = Discovery {
            agent_id: self.agent_id.clone(),
            agent_role: self.agent_role.clone(),
            discovery_type: discovery_type.clone(),
            content: content.to_string(),
            confidence,
            timestamp: chrono::Utc::now(),
            related_to: None,
        };
        
        info!(
            "🔍 {} sharing {}: {}", 
            self.agent_role,
            match &discovery_type {
                DiscoveryType::Pattern => "pattern",
                DiscoveryType::Anomaly => "anomaly",
                DiscoveryType::Connection => "connection",
                DiscoveryType::Solution => "solution",
                DiscoveryType::Question => "question",
                DiscoveryType::Insight => "insight",
            },
            content
        );
        
        // Store locally
        self.discoveries.write().await.push(discovery.clone());
        
        // Broadcast to team
        let subject = format!("discoveries.{}", 
            match discovery_type {
                DiscoveryType::Question => "questions",
                _ => "insights",
            }
        );
        
        self.nats.publish(
            subject,
            serde_json::to_vec(&discovery)?.into()
        ).await?;
        
        Ok(())
    }
    
    /// Share a discovery that builds on another
    pub async fn share_related_discovery(
        &self,
        discovery_type: DiscoveryType,
        content: &str,
        confidence: f32,
        related_to: &str,
    ) -> Result<()> {
        let mut discovery = Discovery {
            agent_id: self.agent_id.clone(),
            agent_role: self.agent_role.clone(),
            discovery_type,
            content: content.to_string(),
            confidence,
            timestamp: chrono::Utc::now(),
            related_to: Some(related_to.to_string()),
        };
        
        info!("🔗 {} building on previous discovery", self.agent_role);
        
        self.discoveries.write().await.push(discovery.clone());
        
        self.nats.publish(
            "discoveries.connections",
            serde_json::to_vec(&discovery)?.into()
        ).await?;
        
        Ok(())
    }
}

/// Listens for discoveries from other agents
#[derive(Clone, Debug)]
pub struct DiscoveryListener {
    agent_id: String,
    agent_role: String,
    nats: Client,
}

impl DiscoveryListener {
    pub fn new(agent_id: String, agent_role: String, nats: Client) -> Self {
        Self { agent_id, agent_role, nats }
    }
    
    /// Start listening for discoveries with a callback
    pub async fn listen<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(Discovery) -> Result<()> + Send + 'static,
    {
        let mut sub = self.nats.subscribe("discoveries.>").await?;
        
        info!("👂 {} listening for discoveries", self.agent_role);
        
        while let Some(msg) = sub.next().await {
            match serde_json::from_slice::<Discovery>(&msg.payload) {
                Ok(discovery) => {
                    // Don't process our own discoveries
                    if discovery.agent_id != self.agent_id {
                        debug!(
                            "📡 {} received {} from {}", 
                            self.agent_role,
                            match discovery.discovery_type {
                                DiscoveryType::Pattern => "pattern",
                                DiscoveryType::Anomaly => "anomaly",
                                DiscoveryType::Connection => "connection",
                                DiscoveryType::Solution => "solution",
                                DiscoveryType::Question => "question",
                                DiscoveryType::Insight => "insight",
                            },
                            discovery.agent_role
                        );
                        
                        if let Err(e) = callback(discovery) {
                            error!("Discovery callback error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse discovery: {}", e);
                }
            }
        }
        
        Ok(())
    }
}

/// Example of how discoveries create emergent behavior
pub mod patterns {
    use super::*;
    
    /// When multiple agents find related patterns
    pub async fn convergent_discovery_example() -> Result<()> {
        let nats = async_nats::connect("nats://localhost:4222").await?;
        
        // Security agent finds something
        let security = DiscoveryBroadcaster::new(
            "agent-1".to_string(),
            "Security".to_string(),
            nats.clone()
        );
        
        security.share_discovery(
            DiscoveryType::Anomaly,
            "Unusual login patterns from IP range 192.168.x.x",
            0.7
        ).await?;
        
        // Performance agent finds something related
        let performance = DiscoveryBroadcaster::new(
            "agent-2".to_string(),
            "Performance".to_string(),
            nats.clone()
        );
        
        performance.share_discovery(
            DiscoveryType::Pattern,
            "Database queries spiking from internal network",
            0.8
        ).await?;
        
        // Network agent connects the dots
        let network = DiscoveryBroadcaster::new(
            "agent-3".to_string(),
            "Network".to_string(),
            nats.clone()
        );
        
        network.share_related_discovery(
            DiscoveryType::Connection,
            "Internal bot scanning for SQL injection vulnerabilities!",
            0.95,
            "security-anomaly-001"
        ).await?;
        
        // This creates an "aha!" moment - three separate observations
        // combine into understanding of an active security incident
        
        Ok(())
    }
}

// Real-time discovery sharing enables emergent intelligence!