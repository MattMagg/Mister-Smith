//! Discovery State Management for MCP Server
//! 
//! Provides thread-safe in-memory storage for discoveries with efficient lookups
//! and subscription management.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tracing::{debug, warn};

// Import Discovery and DiscoveryType from collaboration module
use crate::collaboration::discovery_sharing::{Discovery, DiscoveryType};

/// Maximum number of discoveries to store before eviction
const MAX_DISCOVERIES: usize = 1000;

/// Filter criteria for discovery subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryFilter {
    /// Filter by discovery types
    pub types: Option<Vec<DiscoveryType>>,
    /// Filter by agent roles
    pub agent_roles: Option<Vec<String>>,
    /// Minimum confidence level
    pub min_confidence: Option<f32>,
    /// Filter by keywords in content
    pub keywords: Option<Vec<String>>,
}

impl DiscoveryFilter {
    /// Check if a discovery matches this filter
    pub fn matches(&self, discovery: &Discovery) -> bool {
        // Check type filter
        if let Some(types) = &self.types {
            if !types.iter().any(|t| std::mem::discriminant(t) == std::mem::discriminant(&discovery.discovery_type)) {
                return false;
            }
        }

        // Check agent role filter
        if let Some(roles) = &self.agent_roles {
            if !roles.contains(&discovery.agent_role) {
                return false;
            }
        }

        // Check confidence filter
        if let Some(min_conf) = self.min_confidence {
            if discovery.confidence < min_conf {
                return false;
            }
        }

        // Check keyword filter
        if let Some(keywords) = &self.keywords {
            let content_lower = discovery.content.to_lowercase();
            if !keywords.iter().any(|kw| content_lower.contains(&kw.to_lowercase())) {
                return false;
            }
        }

        true
    }
}

/// Thread-safe in-memory discovery store with efficient lookups
pub struct DiscoveryStore {
    /// All discoveries by ID
    discoveries: Arc<RwLock<HashMap<String, Discovery>>>,
    /// Discovery IDs by agent
    by_agent: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Discovery IDs by type
    by_type: Arc<RwLock<HashMap<DiscoveryType, Vec<String>>>>,
    /// Agent subscriptions
    subscriptions: Arc<RwLock<HashMap<String, DiscoveryFilter>>>,
    /// Order of discovery IDs for FIFO eviction
    discovery_order: Arc<RwLock<VecDeque<String>>>,
}

impl DiscoveryStore {
    /// Create a new discovery store
    pub fn new() -> Self {
        Self {
            discoveries: Arc::new(RwLock::new(HashMap::new())),
            by_agent: Arc::new(RwLock::new(HashMap::new())),
            by_type: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            discovery_order: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Store a discovery and return its ID
    pub async fn store_discovery(&self, discovery: Discovery) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        
        // Check if we need to evict old discoveries
        {
            let order = self.discovery_order.read().await;
            if order.len() >= MAX_DISCOVERIES {
                drop(order); // Release read lock before eviction
                self.evict_oldest().await?;
            }
        }

        // Store the discovery
        {
            let mut discoveries = self.discoveries.write().await;
            discoveries.insert(id.clone(), discovery.clone());
        }

        // Update indices
        {
            let mut by_agent = self.by_agent.write().await;
            by_agent.entry(discovery.agent_id.clone())
                .or_insert_with(Vec::new)
                .push(id.clone());
        }

        {
            let mut by_type = self.by_type.write().await;
            by_type.entry(discovery.discovery_type.clone())
                .or_insert_with(Vec::new)
                .push(id.clone());
        }

        // Track order for FIFO eviction
        {
            let mut order = self.discovery_order.write().await;
            order.push_back(id.clone());
        }

        debug!(
            "Stored discovery {} from agent {} (type: {:?})", 
            id, discovery.agent_id, discovery.discovery_type
        );

        Ok(id)
    }

    /// Get a discovery by ID
    pub async fn get_discovery(&self, id: &str) -> Option<Discovery> {
        let discoveries = self.discoveries.read().await;
        discoveries.get(id).cloned()
    }

    /// Get all discoveries for a specific agent
    pub async fn get_discoveries_for_agent(&self, agent_id: &str) -> Vec<Discovery> {
        let by_agent = self.by_agent.read().await;
        let discoveries = self.discoveries.read().await;

        if let Some(ids) = by_agent.get(agent_id) {
            ids.iter()
                .filter_map(|id| discoveries.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all discoveries by type
    pub async fn get_discoveries_by_type(&self, discovery_type: DiscoveryType) -> Vec<Discovery> {
        let by_type = self.by_type.read().await;
        let discoveries = self.discoveries.read().await;

        if let Some(ids) = by_type.get(&discovery_type) {
            ids.iter()
                .filter_map(|id| discoveries.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Add a subscription for an agent
    pub async fn add_subscription(&self, agent_id: String, filter: DiscoveryFilter) {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(agent_id.clone(), filter);
        debug!("Added subscription for agent {}", agent_id);
    }

    /// Remove a subscription for an agent
    pub async fn remove_subscription(&self, agent_id: &str) {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.remove(agent_id);
        debug!("Removed subscription for agent {}", agent_id);
    }

    /// Get subscription filter for an agent
    pub async fn get_subscription(&self, agent_id: &str) -> Option<DiscoveryFilter> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.get(agent_id).cloned()
    }

    /// Get all agents that should be notified about a discovery
    pub async fn get_subscribed_agents(&self, discovery: &Discovery) -> Vec<String> {
        let subscriptions = self.subscriptions.read().await;
        
        subscriptions.iter()
            .filter_map(|(agent_id, filter)| {
                if filter.matches(discovery) && agent_id != &discovery.agent_id {
                    Some(agent_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all discoveries matching a filter
    pub async fn get_discoveries_by_filter(&self, filter: &DiscoveryFilter) -> Vec<Discovery> {
        let discoveries = self.discoveries.read().await;
        
        discoveries.values()
            .filter(|d| filter.matches(d))
            .cloned()
            .collect()
    }

    /// Get recent discoveries (last N)
    pub async fn get_recent_discoveries(&self, count: usize) -> Vec<Discovery> {
        let order = self.discovery_order.read().await;
        let discoveries = self.discoveries.read().await;
        
        order.iter()
            .rev()
            .take(count)
            .filter_map(|id| discoveries.get(id).cloned())
            .collect()
    }

    /// Evict the oldest discovery (FIFO)
    async fn evict_oldest(&self) -> Result<()> {
        let oldest_id = {
            let mut order = self.discovery_order.write().await;
            order.pop_front()
        };

        if let Some(id) = oldest_id {
            // Get discovery details before removal
            let discovery = {
                let mut discoveries = self.discoveries.write().await;
                discoveries.remove(&id)
            };

            if let Some(discovery) = discovery {
                // Remove from agent index
                {
                    let mut by_agent = self.by_agent.write().await;
                    if let Some(ids) = by_agent.get_mut(&discovery.agent_id) {
                        ids.retain(|i| i != &id);
                        if ids.is_empty() {
                            by_agent.remove(&discovery.agent_id);
                        }
                    }
                }

                // Remove from type index
                {
                    let mut by_type = self.by_type.write().await;
                    if let Some(ids) = by_type.get_mut(&discovery.discovery_type) {
                        ids.retain(|i| i != &id);
                        if ids.is_empty() {
                            by_type.remove(&discovery.discovery_type);
                        }
                    }
                }

                warn!(
                    "Evicted oldest discovery {} due to capacity limit", 
                    id
                );
            }
        }

        Ok(())
    }

    /// Get store statistics
    pub async fn get_stats(&self) -> StoreStats {
        let discoveries = self.discoveries.read().await;
        let by_agent = self.by_agent.read().await;
        let by_type = self.by_type.read().await;
        let subscriptions = self.subscriptions.read().await;

        StoreStats {
            total_discoveries: discoveries.len(),
            unique_agents: by_agent.len(),
            discovery_types: by_type.len(),
            active_subscriptions: subscriptions.len(),
            capacity_used: (discoveries.len() as f32 / MAX_DISCOVERIES as f32) * 100.0,
        }
    }

    /// Clear all discoveries (useful for testing)
    #[cfg(test)]
    pub async fn clear(&self) {
        let mut discoveries = self.discoveries.write().await;
        let mut by_agent = self.by_agent.write().await;
        let mut by_type = self.by_type.write().await;
        let mut order = self.discovery_order.write().await;

        discoveries.clear();
        by_agent.clear();
        by_type.clear();
        order.clear();
    }
}

/// Statistics about the discovery store
#[derive(Debug, Clone, Serialize)]
pub struct StoreStats {
    pub total_discoveries: usize,
    pub unique_agents: usize,
    pub discovery_types: usize,
    pub active_subscriptions: usize,
    pub capacity_used: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_discovery(agent_id: &str, discovery_type: DiscoveryType) -> Discovery {
        Discovery {
            agent_id: agent_id.to_string(),
            agent_role: "TestAgent".to_string(),
            discovery_type,
            content: "Test discovery content".to_string(),
            confidence: 0.8,
            timestamp: Utc::now(),
            related_to: None,
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = DiscoveryStore::new();
        let discovery = create_test_discovery("agent-1", DiscoveryType::Pattern);
        
        let id = store.store_discovery(discovery.clone()).await.unwrap();
        let retrieved = store.get_discovery(&id).await.unwrap();
        
        assert_eq!(retrieved.agent_id, discovery.agent_id);
        assert_eq!(retrieved.content, discovery.content);
    }

    #[tokio::test]
    async fn test_get_by_agent() {
        let store = DiscoveryStore::new();
        
        // Store discoveries from different agents
        store.store_discovery(create_test_discovery("agent-1", DiscoveryType::Pattern)).await.unwrap();
        store.store_discovery(create_test_discovery("agent-1", DiscoveryType::Insight)).await.unwrap();
        store.store_discovery(create_test_discovery("agent-2", DiscoveryType::Question)).await.unwrap();
        
        let agent1_discoveries = store.get_discoveries_for_agent("agent-1").await;
        assert_eq!(agent1_discoveries.len(), 2);
        
        let agent2_discoveries = store.get_discoveries_for_agent("agent-2").await;
        assert_eq!(agent2_discoveries.len(), 1);
    }

    #[tokio::test]
    async fn test_get_by_type() {
        let store = DiscoveryStore::new();
        
        // Store discoveries of different types
        store.store_discovery(create_test_discovery("agent-1", DiscoveryType::Pattern)).await.unwrap();
        store.store_discovery(create_test_discovery("agent-2", DiscoveryType::Pattern)).await.unwrap();
        store.store_discovery(create_test_discovery("agent-3", DiscoveryType::Question)).await.unwrap();
        
        let patterns = store.get_discoveries_by_type(DiscoveryType::Pattern).await;
        assert_eq!(patterns.len(), 2);
        
        let questions = store.get_discoveries_by_type(DiscoveryType::Question).await;
        assert_eq!(questions.len(), 1);
    }

    #[tokio::test]
    async fn test_subscriptions() {
        let store = DiscoveryStore::new();
        
        // Add subscription for patterns with high confidence
        let filter = DiscoveryFilter {
            types: Some(vec![DiscoveryType::Pattern]),
            agent_roles: None,
            min_confidence: Some(0.7),
            keywords: None,
        };
        
        store.add_subscription("subscriber-1".to_string(), filter).await;
        
        // Test matching discovery
        let matching_discovery = Discovery {
            agent_id: "agent-1".to_string(),
            agent_role: "Analyzer".to_string(),
            discovery_type: DiscoveryType::Pattern,
            content: "Found a pattern".to_string(),
            confidence: 0.8,
            timestamp: Utc::now(),
            related_to: None,
        };
        
        let subscribers = store.get_subscribed_agents(&matching_discovery).await;
        assert_eq!(subscribers.len(), 1);
        assert_eq!(subscribers[0], "subscriber-1");
        
        // Test non-matching discovery (low confidence)
        let non_matching = Discovery {
            agent_id: "agent-2".to_string(),
            agent_role: "Analyzer".to_string(),
            discovery_type: DiscoveryType::Pattern,
            content: "Maybe a pattern".to_string(),
            confidence: 0.5,
            timestamp: Utc::now(),
            related_to: None,
        };
        
        let subscribers = store.get_subscribed_agents(&non_matching).await;
        assert_eq!(subscribers.len(), 0);
    }

    #[tokio::test]
    async fn test_eviction() {
        let store = DiscoveryStore::new();
        store.clear().await; // Ensure clean state
        
        // Store MAX_DISCOVERIES + 1 to trigger eviction
        for i in 0..(MAX_DISCOVERIES + 1) {
            let discovery = Discovery {
                agent_id: format!("agent-{}", i),
                agent_role: "TestAgent".to_string(),
                discovery_type: DiscoveryType::Insight,
                content: format!("Discovery {}", i),
                confidence: 0.5,
                timestamp: Utc::now(),
                related_to: None,
            };
            store.store_discovery(discovery).await.unwrap();
        }
        
        let stats = store.get_stats().await;
        assert_eq!(stats.total_discoveries, MAX_DISCOVERIES);
        
        // First discovery should have been evicted
        let first_agent_discoveries = store.get_discoveries_for_agent("agent-0").await;
        assert_eq!(first_agent_discoveries.len(), 0);
    }
}