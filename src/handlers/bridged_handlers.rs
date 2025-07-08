//! Bridged MCP Handlers - Enhanced handlers that use collaboration infrastructure
//!
//! These handlers wrap the original MCP handlers with collaboration bridge functionality,
//! providing seamless integration between MCP clients and the collaboration infrastructure.

use super::{Parameters, CallToolResult, McpError};
use crate::collaboration::{DiscoveryType, get_bridge};
use crate::handlers::share_discovery::ShareDiscoveryParams;
use crate::handlers::subscribe_discoveries::SubscribeDiscoveriesParams;
use async_nats::Client;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tracing::{info, error, warn};
use std::collections::HashMap;

/// Enhanced share discovery handler using collaboration bridge
pub async fn bridged_share_discovery_handler(
    params: Parameters<ShareDiscoveryParams>,
    _nats_client: Client,
) -> Result<CallToolResult, McpError> {
    let params = params.params;
    
    // Get collaboration bridge
    let bridge = match get_bridge().await {
        Some(bridge) => bridge,
        None => {
            error!("Collaboration bridge not initialized, falling back to direct handler");
            return crate::handlers::share_discovery::share_discovery_handler(
                Parameters { params },
                _nats_client,
            ).await;
        }
    };
    
    // Validate parameters (same as original)
    if params.agent_id.is_empty() {
        return Err(McpError::InvalidParameters("agent_id cannot be empty".to_string()));
    }
    
    if params.agent_role.is_empty() {
        return Err(McpError::InvalidParameters("agent_role cannot be empty".to_string()));
    }
    
    if params.content.is_empty() {
        return Err(McpError::InvalidParameters("content cannot be empty".to_string()));
    }
    
    if params.confidence < 0.0 || params.confidence > 1.0 {
        return Err(McpError::InvalidParameters("confidence must be between 0.0 and 1.0".to_string()));
    }
    
    // Parse discovery type
    let discovery_type = match params.discovery_type.to_lowercase().as_str() {
        "pattern" => DiscoveryType::Pattern,
        "anomaly" => DiscoveryType::Anomaly,
        "connection" => DiscoveryType::Connection,
        "solution" => DiscoveryType::Solution,
        "question" => DiscoveryType::Question,
        "insight" => DiscoveryType::Insight,
        _ => return Err(McpError::InvalidParameters(
            format!("Invalid discovery_type: {}. Must be one of: pattern, anomaly, connection, solution, question, insight", 
                    params.discovery_type)
        )),
    };
    
    info!("🌉 Processing bridged discovery: {} -> {}", params.agent_role, params.discovery_type);
    
    // Use collaboration bridge for enhanced functionality
    bridge.bridged_share_discovery(
        params.agent_id,
        params.agent_role,
        discovery_type,
        params.content,
        params.confidence,
        params.related_to,
    ).await
}

/// Enhanced subscribe discoveries handler using collaboration bridge
pub async fn bridged_subscribe_discoveries_handler(
    params: Parameters<SubscribeDiscoveriesParams>,
    _nats_client: Client,
) -> Result<CallToolResult, McpError> {
    let params = params.params;
    
    // Get collaboration bridge
    let bridge = match get_bridge().await {
        Some(bridge) => bridge,
        None => {
            error!("Collaboration bridge not initialized, falling back to direct handler");
            return crate::handlers::subscribe_discoveries::subscribe_discoveries_handler(
                Parameters { params },
                _nats_client,
            ).await;
        }
    };
    
    // Validate parameters (same as original)
    if params.subscriber_agent_id.is_empty() {
        return Err(McpError::InvalidParameters("subscriber_agent_id cannot be empty".to_string()));
    }
    
    if let Some(confidence) = params.min_confidence {
        if confidence < 0.0 || confidence > 1.0 {
            return Err(McpError::InvalidParameters("min_confidence must be between 0.0 and 1.0".to_string()));
        }
    }
    
    // Parse discovery types if provided
    let discovery_types = if let Some(types) = params.discovery_types {
        let mut parsed_types = Vec::new();
        for type_str in types {
            let discovery_type = match type_str.to_lowercase().as_str() {
                "pattern" => DiscoveryType::Pattern,
                "anomaly" => DiscoveryType::Anomaly,
                "connection" => DiscoveryType::Connection,
                "solution" => DiscoveryType::Solution,
                "question" => DiscoveryType::Question,
                "insight" => DiscoveryType::Insight,
                _ => return Err(McpError::InvalidParameters(
                    format!("Invalid discovery_type: {}. Must be one of: pattern, anomaly, connection, solution, question, insight", 
                            type_str)
                )),
            };
            parsed_types.push(discovery_type);
        }
        Some(parsed_types)
    } else {
        None
    };
    
    info!("🌉 Processing bridged subscription: {}", params.subscriber_agent_id);
    
    // Use collaboration bridge for enhanced functionality
    bridge.bridged_subscribe_discoveries(
        params.subscriber_agent_id,
        discovery_types,
        params.agent_roles,
        params.min_confidence,
    ).await
}

/// Parameters for spawning a collaborative agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnAgentParams {
    pub agent_role: String,
    pub initial_context: String,
}

/// Spawn a new collaborative agent handler
pub async fn spawn_collaborative_agent_handler(
    params: Parameters<SpawnAgentParams>,
    _nats_client: Client,
) -> Result<CallToolResult, McpError> {
    let params = params.params;
    
    // Get collaboration bridge
    let bridge = match get_bridge().await {
        Some(bridge) => bridge,
        None => {
            return Err(McpError::InternalError("Collaboration bridge not initialized".to_string()));
        }
    };
    
    // Validate parameters
    if params.agent_role.is_empty() {
        return Err(McpError::InvalidParameters("agent_role cannot be empty".to_string()));
    }
    
    if params.initial_context.is_empty() {
        return Err(McpError::InvalidParameters("initial_context cannot be empty".to_string()));
    }
    
    info!("🚀 Spawning collaborative agent: {}", params.agent_role);
    
    // Use collaboration bridge to spawn agent
    bridge.spawn_collaborative_agent(
        params.agent_role,
        params.initial_context,
    ).await
}

/// Get collaboration status handler
pub async fn get_collaboration_status_handler(
    _params: Parameters<serde_json::Value>,
    _nats_client: Client,
) -> Result<CallToolResult, McpError> {
    // Get collaboration bridge
    let bridge = match get_bridge().await {
        Some(bridge) => bridge,
        None => {
            return Err(McpError::InternalError("Collaboration bridge not initialized".to_string()));
        }
    };
    
    info!("📊 Getting collaboration status");
    
    // Get status from bridge
    bridge.get_collaboration_status().await
}

/// Parameters for orchestrating a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestateTaskParams {
    pub task_description: String,
    pub required_agents: Option<Vec<String>>,
    pub timeout_seconds: Option<u64>,
}

/// Orchestrate a task across multiple agents
pub async fn orchestrate_task_handler(
    params: Parameters<OrchestateTaskParams>,
    _nats_client: Client,
) -> Result<CallToolResult, McpError> {
    let params = params.params;
    
    // Get collaboration bridge
    let bridge = match get_bridge().await {
        Some(bridge) => bridge,
        None => {
            return Err(McpError::InternalError("Collaboration bridge not initialized".to_string()));
        }
    };
    
    // Validate parameters
    if params.task_description.is_empty() {
        return Err(McpError::InvalidParameters("task_description cannot be empty".to_string()));
    }
    
    info!("🎯 Orchestrating task: {}", params.task_description);
    
    // Get orchestrator from bridge
    let orchestrator = bridge.get_orchestrator();
    
    // Orchestrate the task
    match orchestrator.orchestrate_task(&params.task_description).await {
        Ok(result) => {
            let mut metadata = HashMap::new();
            metadata.insert("task_description".to_string(), serde_json::Value::String(params.task_description));
            metadata.insert("orchestrated_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            metadata.insert("collaboration_enabled".to_string(), serde_json::Value::Bool(true));
            
            Ok(CallToolResult::with_metadata(
                format!("Task orchestrated successfully: {}", result),
                metadata
            ))
        }
        Err(e) => {
            error!("Task orchestration failed: {}", e);
            Err(McpError::InternalError(format!("Task orchestration failed: {}", e)))
        }
    }
}

/// Parameters for sharing context between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareContextParams {
    pub from_agent: String,
    pub to_agent: Option<String>, // None for broadcast
    pub context_type: String,
    pub content: String,
    pub relevance: f32,
}

/// Share context between agents
pub async fn share_context_handler(
    params: Parameters<ShareContextParams>,
    _nats_client: Client,
) -> Result<CallToolResult, McpError> {
    let params = params.params;
    
    // Get collaboration bridge
    let bridge = match get_bridge().await {
        Some(bridge) => bridge,
        None => {
            return Err(McpError::InternalError("Collaboration bridge not initialized".to_string()));
        }
    };
    
    // Validate parameters
    if params.from_agent.is_empty() {
        return Err(McpError::InvalidParameters("from_agent cannot be empty".to_string()));
    }
    
    if params.content.is_empty() {
        return Err(McpError::InvalidParameters("content cannot be empty".to_string()));
    }
    
    if params.relevance < 0.0 || params.relevance > 1.0 {
        return Err(McpError::InvalidParameters("relevance must be between 0.0 and 1.0".to_string()));
    }
    
    // Parse context type
    let context_type = match params.context_type.to_lowercase().as_str() {
        "background" => crate::collaboration::ContextType::Background,
        "working_memory" => crate::collaboration::ContextType::WorkingMemory,
        "patterns" => crate::collaboration::ContextType::Patterns,
        "questions" => crate::collaboration::ContextType::Questions,
        "partial_solutions" => crate::collaboration::ContextType::PartialSolutions,
        "warnings" => crate::collaboration::ContextType::Warnings,
        _ => return Err(McpError::InvalidParameters(
            format!("Invalid context_type: {}. Must be one of: background, working_memory, patterns, questions, partial_solutions, warnings", 
                    params.context_type)
        )),
    };
    
    info!("🤝 Sharing context from {} to {:?}", params.from_agent, params.to_agent);
    
    // Create context manager with from_agent identity
    let context_manager = crate::collaboration::ContextManager::new(
        params.from_agent.clone(),
        _nats_client,
    );
    
    // Share context
    match context_manager.share_context(
        params.to_agent.clone(),
        context_type,
        crate::collaboration::ContextContent::Text(params.content.clone()),
        params.relevance,
    ).await {
        Ok(_) => {
            let mut metadata = HashMap::new();
            metadata.insert("from_agent".to_string(), serde_json::Value::String(params.from_agent));
            metadata.insert("context_type".to_string(), serde_json::Value::String(params.context_type));
            metadata.insert("relevance".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(params.relevance as f64).unwrap()));
            metadata.insert("shared_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            
            if let Some(to_agent) = params.to_agent {
                metadata.insert("to_agent".to_string(), serde_json::Value::String(to_agent));
            } else {
                metadata.insert("broadcast".to_string(), serde_json::Value::Bool(true));
            }
            
            Ok(CallToolResult::with_metadata(
                "Context shared successfully".to_string(),
                metadata
            ))
        }
        Err(e) => {
            error!("Context sharing failed: {}", e);
            Err(McpError::InternalError(format!("Context sharing failed: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collaboration::initialize_bridge;
    
    #[tokio::test]
    async fn test_bridged_share_discovery() {
        let nats_client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS connection required for test");
        
        // Initialize bridge
        initialize_bridge(nats_client.clone()).await.unwrap();
        
        let params = Parameters {
            params: ShareDiscoveryParams {
                agent_id: "test-agent".to_string(),
                agent_role: "TestAgent".to_string(),
                discovery_type: "pattern".to_string(),
                content: "Test discovery content".to_string(),
                confidence: 0.8,
                related_to: None,
            }
        };
        
        let result = bridged_share_discovery_handler(params, nats_client).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_bridged_subscribe_discoveries() {
        let nats_client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS connection required for test");
        
        // Initialize bridge
        initialize_bridge(nats_client.clone()).await.unwrap();
        
        let params = Parameters {
            params: SubscribeDiscoveriesParams {
                discovery_types: Some(vec!["pattern".to_string()]),
                agent_roles: Some(vec!["TestAgent".to_string()]),
                min_confidence: Some(0.7),
                subscriber_agent_id: "test-subscriber".to_string(),
            }
        };
        
        let result = bridged_subscribe_discoveries_handler(params, nats_client).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_spawn_collaborative_agent() {
        let nats_client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS connection required for test");
        
        // Initialize bridge
        initialize_bridge(nats_client.clone()).await.unwrap();
        
        let params = Parameters {
            params: SpawnAgentParams {
                agent_role: "TestAgent".to_string(),
                initial_context: "Test context".to_string(),
            }
        };
        
        let result = spawn_collaborative_agent_handler(params, nats_client).await;
        assert!(result.is_ok());
    }
}