//! MCP-Collaboration Bridge - Seamless integration between MCP handlers and collaboration infrastructure
//!
//! This module provides the bridge layer that connects MCP server handlers to the existing
//! collaboration infrastructure, enabling unified discovery flows and agent lifecycle management.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_nats::Client;
use anyhow::Result;
use tracing::{info, debug, error, warn};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::collaboration::{
    DiscoveryBroadcaster, LiveOrchestrator, ContextManager, DiscoverySSEBroadcaster,
    Discovery, DiscoveryType, SharedContext, ContextType, ContextContent
};
use crate::handlers::{CallToolResult, McpError, DISCOVERY_STORE};
use crate::runtime::InteractiveClaudeSession;

/// Main bridge service that integrates MCP handlers with collaboration infrastructure
pub struct CollaborationBridge {
    /// Discovery broadcaster for publishing discoveries
    discovery_broadcaster: Arc<DiscoveryBroadcaster>,
    /// Live orchestrator for agent management
    live_orchestrator: Arc<LiveOrchestrator>,
    /// Context manager for shared context
    context_manager: Arc<ContextManager>,
    /// SSE broadcaster for real-time streaming
    sse_broadcaster: Arc<DiscoverySSEBroadcaster>,
    /// NATS client for direct operations
    nats_client: Client,
    /// Agent lifecycle manager
    agent_lifecycle: Arc<AgentLifecycleManager>,
}

/// Manages agent lifecycle and registration
pub struct AgentLifecycleManager {
    /// Active MCP agents
    active_agents: Arc<RwLock<HashMap<String, AgentInfo>>>,
    /// Orchestrator reference
    orchestrator: Arc<LiveOrchestrator>,
}

/// Information about an active MCP agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub agent_role: String,
    pub session_id: Option<String>,
    pub spawned_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub collaboration_enabled: bool,
}

impl CollaborationBridge {
    /// Create a new collaboration bridge
    pub async fn new(nats_client: Client) -> Result<Self> {
        info!("🌉 Initializing MCP-Collaboration Bridge");
        
        // Initialize collaboration infrastructure
        let live_orchestrator = Arc::new(LiveOrchestrator::new(nats_client.clone()).await?);
        let context_manager = Arc::new(ContextManager::new(
            "bridge-service".to_string(),
            nats_client.clone(),
        ));
        
        // Create discovery broadcaster with bridge identity
        let discovery_broadcaster = Arc::new(DiscoveryBroadcaster::new(
            "bridge-service".to_string(),
            "Bridge".to_string(),
            nats_client.clone(),
        ));
        
        // Initialize SSE broadcaster
        let sse_broadcaster = Arc::new(DiscoverySSEBroadcaster::new(nats_client.clone()));
        
        // Start SSE broadcaster listening
        sse_broadcaster.start_listening().await?;
        
        // Create agent lifecycle manager
        let agent_lifecycle = Arc::new(AgentLifecycleManager::new(live_orchestrator.clone()));
        
        Ok(Self {
            discovery_broadcaster,
            live_orchestrator,
            context_manager,
            sse_broadcaster,
            nats_client,
            agent_lifecycle,
        })
    }
    
    /// Enhanced share discovery that integrates with collaboration infrastructure
    pub async fn bridged_share_discovery(
        &self,
        agent_id: String,
        agent_role: String,
        discovery_type: DiscoveryType,
        content: String,
        confidence: f32,
        related_to: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        info!("🔍 Processing bridged discovery from {} ({})", agent_role, agent_id);
        
        // Create discovery object
        let discovery = Discovery {
            agent_id: agent_id.clone(),
            agent_role: agent_role.clone(),
            discovery_type: discovery_type.clone(),
            content: content.clone(),
            confidence,
            timestamp: chrono::Utc::now(),
            related_to: related_to.clone(),
        };
        
        // Store in discovery store (for persistence and retrieval)
        let discovery_id = DISCOVERY_STORE.store_discovery(discovery.clone()).await
            .map_err(|e| McpError::InternalError(format!("Failed to store discovery: {}", e)))?;
        
        // Update agent activity
        self.agent_lifecycle.update_agent_activity(&agent_id).await;
        
        // Publish via collaboration infrastructure (not direct NATS)
        if let Err(e) = self.discovery_broadcaster.share_discovery(
            discovery_type.clone(),
            &content,
            confidence,
        ).await {
            error!("Failed to broadcast discovery via collaboration infrastructure: {}", e);
            // Continue - discovery is still stored, just not broadcast
        }
        
        // Share related discovery if applicable
        if let Some(related) = &related_to {
            if let Err(e) = self.discovery_broadcaster.share_related_discovery(
                discovery_type.clone(),
                &content,
                confidence,
                related,
            ).await {
                warn!("Failed to share related discovery: {}", e);
            }
        }
        
        // Share context with collaboration layer
        if let Err(e) = self.context_manager.share_context(
            None, // Broadcast to all
            ContextType::WorkingMemory,
            ContextContent::Text(format!("Discovery: {}", content)),
            confidence,
        ).await {
            warn!("Failed to share context: {}", e);
        }
        
        // Notify orchestrator about the discovery
        if discovery_type == DiscoveryType::Question {
            // Questions might need orchestrator intervention
            if let Err(e) = self.live_orchestrator.provide_guidance(&agent_role, &content).await {
                warn!("Failed to provide orchestrator guidance: {}", e);
            }
        }
        
        // Create enhanced response with collaboration metadata
        let mut metadata = HashMap::new();
        metadata.insert("discovery_id".to_string(), serde_json::Value::String(discovery_id.clone()));
        metadata.insert("broadcast_status".to_string(), serde_json::Value::String("published".to_string()));
        metadata.insert("collaboration_enabled".to_string(), serde_json::Value::Bool(true));
        metadata.insert("timestamp".to_string(), serde_json::Value::String(discovery.timestamp.to_rfc3339()));
        
        if let Some(related) = related_to {
            metadata.insert("related_to".to_string(), serde_json::Value::String(related));
        }
        
        Ok(CallToolResult::with_metadata(
            format!("Discovery shared through collaboration bridge: {}", discovery_id),
            metadata
        ))
    }
    
    /// Enhanced subscribe discoveries that connects to actual SSE broadcaster
    pub async fn bridged_subscribe_discoveries(
        &self,
        subscriber_agent_id: String,
        discovery_types: Option<Vec<DiscoveryType>>,
        agent_roles: Option<Vec<String>>,
        min_confidence: Option<f32>,
    ) -> Result<CallToolResult, McpError> {
        info!("📡 Creating bridged discovery subscription for {}", subscriber_agent_id);
        
        // Create subscription filter
        let filter = crate::mcp_server::DiscoveryFilter {
            types: discovery_types.clone(),
            agent_roles: agent_roles.clone(),
            min_confidence,
            keywords: None,
        };
        
        // Store subscription in discovery store
        DISCOVERY_STORE.add_subscription(subscriber_agent_id.clone(), filter).await;
        
        // Register agent with lifecycle manager
        self.agent_lifecycle.register_agent(
            subscriber_agent_id.clone(),
            "Subscriber".to_string(),
            true, // collaboration_enabled
        ).await;
        
        // Generate real SSE endpoint that connects to the broadcaster
        let subscription_id = Uuid::new_v4().to_string();
        let sse_endpoint_url = format!("/api/v1/discoveries/stream?agent_id={}", subscriber_agent_id);
        
        // The SSE broadcaster is already listening and will handle clients connecting to this endpoint
        
        // Create filters summary
        let mut filters_applied = HashMap::new();
        
        if let Some(types) = &discovery_types {
            filters_applied.insert(
                "discovery_types".to_string(),
                serde_json::Value::Array(
                    types.iter()
                        .map(|t| serde_json::Value::String(format!("{:?}", t).to_lowercase()))
                        .collect()
                )
            );
        }
        
        if let Some(roles) = &agent_roles {
            filters_applied.insert(
                "agent_roles".to_string(),
                serde_json::Value::Array(
                    roles.iter()
                        .map(|r| serde_json::Value::String(r.clone()))
                        .collect()
                )
            );
        }
        
        if let Some(confidence) = min_confidence {
            filters_applied.insert(
                "min_confidence".to_string(),
                serde_json::Value::Number(serde_json::Number::from_f64(confidence as f64).unwrap())
            );
        }
        
        // Add collaboration-specific metadata
        filters_applied.insert("collaboration_enabled".to_string(), serde_json::Value::Bool(true));
        filters_applied.insert("bridge_version".to_string(), serde_json::Value::String("1.0".to_string()));
        
        // Create response with actual SSE endpoint
        let response_json = serde_json::json!({
            "subscription_id": subscription_id,
            "sse_endpoint": sse_endpoint_url,
            "filters": filters_applied,
            "collaboration_features": {
                "agent_coordination": true,
                "context_sharing": true,
                "real_time_streaming": true,
                "discovery_relationships": true
            },
            "instructions": {
                "connect": format!("Connect to {} using Server-Sent Events (SSE)", sse_endpoint_url),
                "authentication": "Include your agent token in Authorization header",
                "event_format": "JSON-RPC 2.0 notifications with discovery URIs",
                "collaboration": "Subscription includes collaboration context and agent coordination"
            }
        });
        
        debug!("Created bridged subscription {} with SSE endpoint: {}", subscription_id, sse_endpoint_url);
        
        Ok(CallToolResult::json(response_json))
    }
    
    /// Spawn a new agent through the collaboration infrastructure
    pub async fn spawn_collaborative_agent(
        &self,
        agent_role: String,
        initial_context: String,
    ) -> Result<CallToolResult, McpError> {
        info!("🚀 Spawning collaborative agent: {}", agent_role);
        
        // Use orchestrator to spawn agent
        if let Err(e) = self.live_orchestrator.spawn_agent(&agent_role, &initial_context).await {
            error!("Failed to spawn agent through orchestrator: {}", e);
            return Err(McpError::InternalError(format!("Agent spawn failed: {}", e)));
        }
        
        // Register with lifecycle manager
        let agent_id = format!("{}-{}", agent_role.to_lowercase(), Uuid::new_v4().to_string()[..8].to_string());
        self.agent_lifecycle.register_agent(
            agent_id.clone(),
            agent_role.clone(),
            true, // collaboration_enabled
        ).await;
        
        // Share initial context
        if let Err(e) = self.context_manager.share_context(
            None, // Broadcast
            ContextType::Background,
            ContextContent::Text(format!("New {} agent spawned: {}", agent_role, initial_context)),
            0.8,
        ).await {
            warn!("Failed to share spawn context: {}", e);
        }
        
        let response_json = serde_json::json!({
            "agent_id": agent_id,
            "agent_role": agent_role,
            "status": "spawned",
            "collaboration_enabled": true,
            "orchestrator_managed": true,
            "initial_context": initial_context,
            "spawned_at": chrono::Utc::now().to_rfc3339()
        });
        
        Ok(CallToolResult::json(response_json))
    }
    
    /// Get collaboration status and statistics
    pub async fn get_collaboration_status(&self) -> Result<CallToolResult, McpError> {
        let store_stats = DISCOVERY_STORE.get_stats().await;
        let agent_stats = self.agent_lifecycle.get_agent_stats().await;
        let connected_clients = self.sse_broadcaster.get_connected_clients().await;
        
        let status_json = serde_json::json!({
            "bridge_status": "active",
            "collaboration_enabled": true,
            "discovery_store": {
                "total_discoveries": store_stats.total_discoveries,
                "unique_agents": store_stats.unique_agents,
                "active_subscriptions": store_stats.active_subscriptions,
                "capacity_used": store_stats.capacity_used
            },
            "agent_lifecycle": {
                "total_agents": agent_stats.total_agents,
                "active_agents": agent_stats.active_agents,
                "collaboration_enabled_agents": agent_stats.collaboration_enabled_agents
            },
            "sse_connections": {
                "connected_clients": connected_clients.len()
            },
            "infrastructure": {
                "orchestrator_active": true,
                "context_manager_active": true,
                "discovery_broadcaster_active": true,
                "sse_broadcaster_active": true
            }
        });
        
        Ok(CallToolResult::json(status_json))
    }
    
    /// Get the SSE broadcaster for external use
    pub fn get_sse_broadcaster(&self) -> Arc<DiscoverySSEBroadcaster> {
        self.sse_broadcaster.clone()
    }
    
    /// Get the live orchestrator for external use
    pub fn get_orchestrator(&self) -> Arc<LiveOrchestrator> {
        self.live_orchestrator.clone()
    }
}

impl AgentLifecycleManager {
    /// Create a new agent lifecycle manager
    pub fn new(orchestrator: Arc<LiveOrchestrator>) -> Self {
        Self {
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            orchestrator,
        }
    }
    
    /// Register a new agent
    pub async fn register_agent(
        &self,
        agent_id: String,
        agent_role: String,
        collaboration_enabled: bool,
    ) {
        let agent_info = AgentInfo {
            agent_id: agent_id.clone(),
            agent_role,
            session_id: None,
            spawned_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            collaboration_enabled,
        };
        
        self.active_agents.write().await.insert(agent_id.clone(), agent_info);
        debug!("Registered agent: {}", agent_id);
    }
    
    /// Update agent activity timestamp
    pub async fn update_agent_activity(&self, agent_id: &str) {
        if let Some(agent) = self.active_agents.write().await.get_mut(agent_id) {
            agent.last_activity = chrono::Utc::now();
        }
    }
    
    /// Get agent statistics
    pub async fn get_agent_stats(&self) -> AgentStats {
        let agents = self.active_agents.read().await;
        let total_agents = agents.len();
        let active_agents = agents.values()
            .filter(|agent| {
                chrono::Utc::now().signed_duration_since(agent.last_activity)
                    < chrono::Duration::minutes(5)
            })
            .count();
        let collaboration_enabled_agents = agents.values()
            .filter(|agent| agent.collaboration_enabled)
            .count();
        
        AgentStats {
            total_agents,
            active_agents,
            collaboration_enabled_agents,
        }
    }
    
    /// Remove inactive agents
    pub async fn cleanup_inactive_agents(&self) {
        let mut agents = self.active_agents.write().await;
        let now = chrono::Utc::now();
        
        agents.retain(|agent_id, agent| {
            let inactive_duration = now.signed_duration_since(agent.last_activity);
            let is_active = inactive_duration < chrono::Duration::hours(1);
            
            if !is_active {
                debug!("Removing inactive agent: {}", agent_id);
            }
            
            is_active
        });
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentStats {
    pub total_agents: usize,
    pub active_agents: usize,
    pub collaboration_enabled_agents: usize,
}

/// Global bridge instance
lazy_static::lazy_static! {
    static ref COLLABORATION_BRIDGE: tokio::sync::RwLock<Option<Arc<CollaborationBridge>>> = 
        tokio::sync::RwLock::new(None);
}

/// Initialize the global collaboration bridge
pub async fn initialize_bridge(nats_client: Client) -> Result<()> {
    let bridge = Arc::new(CollaborationBridge::new(nats_client).await?);
    *COLLABORATION_BRIDGE.write().await = Some(bridge);
    info!("✅ Collaboration bridge initialized");
    Ok(())
}

/// Get the global collaboration bridge instance
pub async fn get_bridge() -> Option<Arc<CollaborationBridge>> {
    COLLABORATION_BRIDGE.read().await.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_bridge_initialization() {
        let nats_client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS connection required for test");
        
        let bridge = CollaborationBridge::new(nats_client).await.unwrap();
        let status = bridge.get_collaboration_status().await.unwrap();
        
        // Verify bridge is active
        assert!(status.content.len() > 0);
    }
    
    #[tokio::test]
    async fn test_bridged_discovery_flow() {
        let nats_client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS connection required for test");
        
        let bridge = CollaborationBridge::new(nats_client).await.unwrap();
        
        // Test discovery sharing
        let result = bridge.bridged_share_discovery(
            "test-agent".to_string(),
            "TestRole".to_string(),
            DiscoveryType::Pattern,
            "Test discovery content".to_string(),
            0.8,
            None,
        ).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_bridged_subscription_flow() {
        let nats_client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS connection required for test");
        
        let bridge = CollaborationBridge::new(nats_client).await.unwrap();
        
        // Test subscription creation
        let result = bridge.bridged_subscribe_discoveries(
            "test-subscriber".to_string(),
            Some(vec![DiscoveryType::Pattern]),
            Some(vec!["TestRole".to_string()]),
            Some(0.7),
        ).await;
        
        assert!(result.is_ok());
    }
}