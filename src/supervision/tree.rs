//! Core supervision tree implementation
//!
//! This module implements the hierarchical supervision tree structure that manages
//! the entire supervision hierarchy and coordinates failure recovery.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use anyhow::Result;

use crate::supervision::types::*;
use crate::supervision::node::SupervisorNode;
use crate::supervision::failure_detector::{FailureDetector, FailureEvent};

/// Root structure managing the entire supervision hierarchy
pub struct SupervisionTree {
    /// Root supervisor node
    root_supervisor: Arc<RwLock<Option<Arc<SupervisorNode>>>>,
    /// Registry of all nodes in the tree
    node_registry: Arc<RwLock<HashMap<NodeId, Arc<SupervisorNode>>>>,
    /// Failure detection system
    failure_detector: Arc<FailureDetector>,
    /// Restart policies for different node types
    restart_policies: Arc<RwLock<HashMap<NodeType, RestartPolicy>>>,
    /// Tree configuration
    config: SupervisionTreeConfig,
}

impl SupervisionTree {
    /// Create a new supervision tree
    pub fn new(config: SupervisionTreeConfig) -> Self {
        let failure_detector = Arc::new(FailureDetector::new(config.failure_detector_config.clone()));
        
        Self {
            root_supervisor: Arc::new(RwLock::new(None)),
            node_registry: Arc::new(RwLock::new(HashMap::new())),
            failure_detector,
            restart_policies: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Set the root supervisor for the tree
    pub async fn set_root(&self, root: Arc<SupervisorNode>) -> Result<()> {
        info!("Setting root supervisor for supervision tree");
        
        // Register the root node
        self.register_node(root.clone()).await?;
        
        // Set as root
        *self.root_supervisor.write().await = Some(root);
        
        Ok(())
    }

    /// Start the supervision tree
    pub async fn start(&self) -> Result<()> {
        info!("Starting supervision tree");
        
        // Start failure detector
        self.failure_detector.start().await?;
        
        // Start root supervisor if set
        if let Some(root) = self.root_supervisor.read().await.as_ref() {
            root.start().await?;
            
            // Start monitoring all nodes
            let nodes = self.node_registry.read().await;
            for (node_id, _) in nodes.iter() {
                self.failure_detector.monitor_node(node_id.clone()).await?;
            }
        } else {
            return Err(SupervisionError::InitializationFailed(
                "No root supervisor set".to_string()
            ).into());
        }
        
        info!("Supervision tree started successfully");
        Ok(())
    }

    /// Stop the supervision tree
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping supervision tree");
        
        // Stop failure detector
        self.failure_detector.stop().await?;
        
        // Stop all supervisors starting from leaves
        if let Some(root) = self.root_supervisor.read().await.as_ref() {
            self.stop_supervisor_recursive(root.clone()).await?;
        }
        
        info!("Supervision tree stopped");
        Ok(())
    }

    /// Register a node in the tree
    pub async fn register_node(&self, node: Arc<SupervisorNode>) -> Result<()> {
        let node_id = node.node_id().clone();
        debug!("Registering node: {}", node_id);
        
        self.node_registry.write().await.insert(node_id.clone(), node);
        
        Ok(())
    }

    /// Unregister a node from the tree
    pub async fn unregister_node(&self, node_id: &NodeId) -> Result<()> {
        debug!("Unregistering node: {}", node_id);
        
        self.node_registry.write().await.remove(node_id);
        self.failure_detector.stop_monitoring(node_id.clone()).await?;
        
        Ok(())
    }

    /// Handle a node failure event
    pub async fn handle_node_failure(&self, event: FailureEvent) -> Result<()> {
        error!("Handling node failure: {:?}", event);
        
        // Find the failed node
        let nodes = self.node_registry.read().await;
        let failed_node = nodes.get(&event.node_id)
            .ok_or_else(|| SupervisionError::SupervisorNotFound(event.node_id.clone()))?;
        
        // Find parent supervisor
        if let Some(parent_id) = &failed_node.parent_id() {
            if let Some(parent) = nodes.get(parent_id) {
                // Let parent handle the failure
                let decision = parent.handle_child_failure(
                    ChildId(event.node_id.0.clone()),
                    event.reason
                ).await?;
                
                match decision {
                    SupervisionDecision::Handled => {
                        info!("Failure handled by parent supervisor");
                    }
                    SupervisionDecision::Escalate(error_msg) => {
                        warn!("Failure escalated: {}", error_msg);
                        // Recursively escalate to grandparent
                        if let Some(grandparent_id) = parent.parent_id() {
                            // Box::pin to handle async recursion
                            Box::pin(self.handle_node_failure(FailureEvent {
                                node_id: parent_id.clone(),
                                reason: FailureReason::ExternalFailure(error_msg),
                                timestamp: event.timestamp,
                            })).await?;
                        }
                    }
                    SupervisionDecision::Stop => {
                        warn!("Stopping supervision due to failure");
                        self.stop().await?;
                    }
                }
            }
        } else {
            // Root node failed - this is critical
            error!("Root supervisor failed!");
            self.stop().await?;
        }
        
        Ok(())
    }

    /// Get a node by ID
    pub async fn get_node(&self, node_id: &NodeId) -> Option<Arc<SupervisorNode>> {
        self.node_registry.read().await.get(node_id).cloned()
    }

    /// Get all nodes in the tree
    pub async fn all_nodes(&self) -> Vec<NodeId> {
        self.node_registry.read().await.keys().cloned().collect()
    }

    /// Set restart policies for node types
    pub async fn set_restart_policies(&self, policies: HashMap<NodeType, RestartPolicy>) {
        *self.restart_policies.write().await = policies;
    }

    /// Get restart policy for a node type
    pub async fn get_restart_policy(&self, node_type: &NodeType) -> Option<RestartPolicy> {
        self.restart_policies.read().await.get(node_type).cloned()
    }

    /// Route a task through the supervision tree (hub-and-spoke pattern)
    pub async fn route_task(&self, task: Task) -> Result<()> {
        debug!("Routing task: {:?}", task);
        
        // Start from root and route down
        if let Some(root) = self.root_supervisor.read().await.as_ref() {
            if let Some(child_id) = root.route_task(task.clone()).await {
                debug!("Task routed to child: {}", child_id.0);
                // TODO: Actually dispatch to the child
            } else {
                warn!("No suitable child found for task");
            }
        }
        
        Ok(())
    }

    /// Get supervision metrics
    pub async fn get_metrics(&self) -> SupervisionTreeMetrics {
        let nodes = self.node_registry.read().await;
        let mut metrics = SupervisionTreeMetrics::default();
        
        metrics.total_nodes = nodes.len();
        
        for node in nodes.values() {
            let node_metrics = node.get_metrics().await;
            metrics.total_restarts += node_metrics.restarts_total;
            metrics.total_failures += node_metrics.failures_total;
        }
        
        metrics
    }

    /// Stop a supervisor and all its children recursively
    async fn stop_supervisor_recursive(&self, node: Arc<SupervisorNode>) -> Result<()> {
        // First stop all children
        let children = node.get_children().await;
        for child_ref in children {
            if let Some(child_node) = self.get_node(&NodeId(child_ref.id.0)).await {
                Box::pin(self.stop_supervisor_recursive(child_node)).await?;
            }
        }
        
        // Then stop the node itself
        node.stop().await?;
        
        Ok(())
    }
}

/// Configuration for the supervision tree
#[derive(Debug, Clone)]
pub struct SupervisionTreeConfig {
    /// Configuration for failure detector
    pub failure_detector_config: crate::supervision::failure_detector::FailureDetectorConfig,
    /// Default restart policy if not specified for node type
    pub default_restart_policy: RestartPolicy,
    /// Whether to enable hub-and-spoke routing
    pub enable_routing: bool,
}

impl Default for SupervisionTreeConfig {
    fn default() -> Self {
        Self {
            failure_detector_config: Default::default(),
            default_restart_policy: RestartPolicy::exponential_backoff(),
            enable_routing: true,
        }
    }
}

/// Metrics for the entire supervision tree
#[derive(Debug, Clone, Default)]
pub struct SupervisionTreeMetrics {
    pub total_nodes: usize,
    pub total_restarts: u64,
    pub total_failures: u64,
    pub active_supervisors: usize,
    pub active_workers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_supervision_tree_creation() {
        let config = SupervisionTreeConfig::default();
        let tree = SupervisionTree::new(config);
        
        // Initially no nodes
        assert_eq!(tree.all_nodes().await.len(), 0);
    }
    
    // More tests will be added in the test module
}