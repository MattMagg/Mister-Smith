//! Handler Integration for MCP Server
//!
//! Provides integration utilities for connecting MCP handlers
//! with NATS client and other server components.

use std::sync::Arc;
use anyhow::Result;
use async_nats::Client;
use serde_json::Value;
use tracing::{debug, info};

use super::{
    McpServerState, 
    nats_config::NatsClientManager,
    discovery_state::DiscoveryStore,
};
use crate::handlers::{
    share_discovery::{share_discovery_handler, ShareDiscoveryParams},
    subscribe_discoveries::{subscribe_discoveries_handler, SubscribeDiscoveriesParams},
    Parameters, CallToolResult, McpError,
};

/// Handler integration utility for MCP server
pub struct HandlerIntegration {
    /// NATS client manager
    nats_manager: Arc<NatsClientManager>,
    /// Discovery store
    discovery_store: Arc<DiscoveryStore>,
}

impl HandlerIntegration {
    /// Create new handler integration
    pub fn new(nats_manager: Arc<NatsClientManager>, discovery_store: Arc<DiscoveryStore>) -> Self {
        Self {
            nats_manager,
            discovery_store,
        }
    }

    /// Execute share_discovery handler with NATS client
    pub async fn execute_share_discovery(&self, params: ShareDiscoveryParams) -> Result<CallToolResult, McpError> {
        debug!("🔗 Handler Integration: Executing share_discovery");
        
        // Increment published message count for monitoring
        self.nats_manager.increment_published();
        
        // Create parameter wrapper
        let wrapped_params = Parameters { params };
        
        // Call handler with NATS client
        let result = share_discovery_handler(wrapped_params, self.nats_manager.client().clone()).await;
        
        match &result {
            Ok(_) => {
                debug!("✅ Handler Integration: share_discovery completed successfully");
            }
            Err(e) => {
                debug!("❌ Handler Integration: share_discovery failed: {}", e);
            }
        }
        
        result
    }

    /// Execute subscribe_discoveries handler with NATS client
    pub async fn execute_subscribe_discoveries(&self, params: SubscribeDiscoveriesParams) -> Result<CallToolResult, McpError> {
        debug!("🔗 Handler Integration: Executing subscribe_discoveries");
        
        // Create parameter wrapper
        let wrapped_params = Parameters { params };
        
        // Call handler with NATS client
        let result = subscribe_discoveries_handler(wrapped_params, self.nats_manager.client().clone()).await;
        
        match &result {
            Ok(_) => {
                debug!("✅ Handler Integration: subscribe_discoveries completed successfully");
            }
            Err(e) => {
                debug!("❌ Handler Integration: subscribe_discoveries failed: {}", e);
            }
        }
        
        result
    }

    /// Execute handler by name with JSON parameters
    pub async fn execute_handler(&self, method: &str, params: Value) -> Result<CallToolResult, McpError> {
        match method {
            "share_discovery" => {
                let params: ShareDiscoveryParams = serde_json::from_value(params)?;
                self.execute_share_discovery(params).await
            }
            "subscribe_discoveries" => {
                let params: SubscribeDiscoveriesParams = serde_json::from_value(params)?;
                self.execute_subscribe_discoveries(params).await
            }
            _ => Err(McpError::InvalidParameters(format!("Unknown handler method: {}", method))),
        }
    }

    /// Get NATS client for direct use
    pub fn nats_client(&self) -> &Client {
        self.nats_manager.client()
    }

    /// Get discovery store for direct use
    pub fn discovery_store(&self) -> &Arc<DiscoveryStore> {
        &self.discovery_store
    }

    /// Get NATS connection statistics
    pub fn get_nats_stats(&self) -> super::nats_config::ConnectionStats {
        self.nats_manager.get_stats()
    }

    /// Check if NATS connection is healthy
    pub fn is_nats_healthy(&self) -> bool {
        self.nats_manager.is_healthy()
    }
}

/// Handler integration factory
pub struct HandlerIntegrationFactory;

impl HandlerIntegrationFactory {
    /// Create handler integration from server state
    pub fn from_server_state(state: &McpServerState) -> HandlerIntegration {
        HandlerIntegration::new(
            state.nats_manager.clone(),
            state.discovery_store.clone(),
        )
    }

    /// Create handler integration with custom components
    pub fn with_components(
        nats_manager: Arc<NatsClientManager>,
        discovery_store: Arc<DiscoveryStore>,
    ) -> HandlerIntegration {
        HandlerIntegration::new(nats_manager, discovery_store)
    }
}

/// Handler execution context
#[derive(Clone)]
pub struct HandlerContext {
    /// Handler integration
    integration: Arc<HandlerIntegration>,
    /// Request ID for tracing
    request_id: String,
    /// Agent ID making the request
    agent_id: Option<String>,
}

impl HandlerContext {
    /// Create new handler context
    pub fn new(integration: Arc<HandlerIntegration>, request_id: String) -> Self {
        Self {
            integration,
            request_id,
            agent_id: None,
        }
    }

    /// Set agent ID for the context
    pub fn with_agent_id(mut self, agent_id: String) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    /// Execute handler with context
    pub async fn execute(&self, method: &str, params: Value) -> Result<CallToolResult, McpError> {
        info!(
            "🎯 Handler Context: Executing {} (request: {}, agent: {:?})",
            method, self.request_id, self.agent_id
        );
        
        let result = self.integration.execute_handler(method, params).await;
        
        match &result {
            Ok(_) => {
                info!("✅ Handler Context: {} completed successfully", method);
            }
            Err(e) => {
                info!("❌ Handler Context: {} failed: {}", method, e);
            }
        }
        
        result
    }

    /// Get request ID
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Get agent ID
    pub fn agent_id(&self) -> Option<&str> {
        self.agent_id.as_deref()
    }

    /// Get handler integration
    pub fn integration(&self) -> &Arc<HandlerIntegration> {
        &self.integration
    }
}

/// Handler metrics for monitoring
#[derive(Debug, Clone)]
pub struct HandlerMetrics {
    /// Total handler executions
    pub total_executions: u64,
    /// Total successful executions
    pub successful_executions: u64,
    /// Total failed executions
    pub failed_executions: u64,
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,
    /// Handler execution counts by method
    pub method_counts: std::collections::HashMap<String, u64>,
}

impl Default for HandlerMetrics {
    fn default() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            avg_execution_time_ms: 0.0,
            method_counts: std::collections::HashMap::new(),
        }
    }
}

/// Handler metrics collector
pub struct HandlerMetricsCollector {
    /// Metrics data
    metrics: Arc<std::sync::RwLock<HandlerMetrics>>,
}

impl HandlerMetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(std::sync::RwLock::new(HandlerMetrics::default())),
        }
    }

    /// Record handler execution
    pub fn record_execution(&self, method: &str, duration_ms: f64, success: bool) {
        let mut metrics = self.metrics.write().unwrap();
        
        metrics.total_executions += 1;
        if success {
            metrics.successful_executions += 1;
        } else {
            metrics.failed_executions += 1;
        }
        
        // Update average execution time
        let total_time = metrics.avg_execution_time_ms * (metrics.total_executions - 1) as f64;
        metrics.avg_execution_time_ms = (total_time + duration_ms) / metrics.total_executions as f64;
        
        // Update method counts
        *metrics.method_counts.entry(method.to_string()).or_insert(0) += 1;
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> HandlerMetrics {
        self.metrics.read().unwrap().clone()
    }
}

impl Default for HandlerMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware for handler execution with metrics
pub struct HandlerMiddleware {
    /// Handler integration
    integration: Arc<HandlerIntegration>,
    /// Metrics collector
    metrics: Arc<HandlerMetricsCollector>,
}

impl HandlerMiddleware {
    /// Create new handler middleware
    pub fn new(integration: Arc<HandlerIntegration>) -> Self {
        Self {
            integration,
            metrics: Arc::new(HandlerMetricsCollector::new()),
        }
    }

    /// Execute handler with metrics collection
    pub async fn execute(&self, method: &str, params: Value) -> Result<CallToolResult, McpError> {
        let start_time = std::time::Instant::now();
        
        let result = self.integration.execute_handler(method, params).await;
        
        let duration_ms = start_time.elapsed().as_millis() as f64;
        let success = result.is_ok();
        
        self.metrics.record_execution(method, duration_ms, success);
        
        result
    }

    /// Get metrics
    pub fn get_metrics(&self) -> HandlerMetrics {
        self.metrics.get_metrics()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_server::{NatsConfig, NatsClientManager};

    #[tokio::test]
    async fn test_handler_integration() {
        // This test requires a running NATS server
        if let Ok(nats_manager) = NatsClientManager::new(NatsConfig::default()).await {
            let discovery_store = Arc::new(DiscoveryStore::new());
            let integration = HandlerIntegration::new(Arc::new(nats_manager), discovery_store);
            
            assert!(integration.is_nats_healthy());
            
            // Test sharing a discovery
            let params = ShareDiscoveryParams {
                agent_id: "test-agent".to_string(),
                agent_role: "TestRole".to_string(),
                discovery_type: "insight".to_string(),
                content: "Test discovery content".to_string(),
                confidence: 0.8,
                related_to: None,
            };
            
            let result = integration.execute_share_discovery(params).await;
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_handler_metrics() {
        let collector = HandlerMetricsCollector::new();
        
        collector.record_execution("test_method", 100.0, true);
        collector.record_execution("test_method", 200.0, false);
        
        let metrics = collector.get_metrics();
        assert_eq!(metrics.total_executions, 2);
        assert_eq!(metrics.successful_executions, 1);
        assert_eq!(metrics.failed_executions, 1);
        assert_eq!(metrics.avg_execution_time_ms, 150.0);
        assert_eq!(metrics.method_counts.get("test_method"), Some(&2));
    }

    #[test]
    fn test_handler_context() {
        let nats_manager = Arc::new(
            futures::executor::block_on(
                NatsClientManager::new(NatsConfig::default())
            ).unwrap()
        );
        let discovery_store = Arc::new(DiscoveryStore::new());
        let integration = Arc::new(HandlerIntegration::new(nats_manager, discovery_store));
        
        let context = HandlerContext::new(integration, "test-request-123".to_string())
            .with_agent_id("test-agent".to_string());
        
        assert_eq!(context.request_id(), "test-request-123");
        assert_eq!(context.agent_id(), Some("test-agent"));
    }
}