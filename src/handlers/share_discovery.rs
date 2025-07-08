//! Share Discovery Handler - Broadcasts discoveries to other agents via NATS

use super::{Parameters, CallToolResult, McpError, DISCOVERY_STORE};
use crate::collaboration::{Discovery, DiscoveryType};
use async_nats::Client;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, error, debug};

/// Parameters for sharing a discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareDiscoveryParams {
    pub agent_id: String,
    pub agent_role: String,
    pub discovery_type: String, // Will be parsed to DiscoveryType
    pub content: String,
    pub confidence: f32,
    pub related_to: Option<String>,
}

/// Share a discovery with other agents via NATS
pub async fn share_discovery_handler(
    params: Parameters<ShareDiscoveryParams>,
    nats_client: Client,
) -> Result<CallToolResult, McpError> {
    let params = params.params;
    
    // Validate parameters
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
    
    // Create discovery
    let discovery = Discovery {
        agent_id: params.agent_id.clone(),
        agent_role: params.agent_role.clone(),
        discovery_type: discovery_type.clone(),
        content: params.content.clone(),
        confidence: params.confidence,
        timestamp: chrono::Utc::now(),
        related_to: params.related_to.clone(),
    };
    
    // Store discovery
    let discovery_id = DISCOVERY_STORE.store_discovery(discovery.clone()).await
        .map_err(|e| McpError::InternalError(format!("Failed to store discovery: {}", e)))?;
    
    info!(
        "🔍 {} ({}) sharing {}: {} (confidence: {})", 
        params.agent_role,
        params.agent_id,
        params.discovery_type,
        params.content,
        params.confidence
    );
    
    // Determine NATS subject based on discovery type
    let subject = format!("discoveries.{}.{}", 
        params.agent_id,
        match &discovery_type {
            DiscoveryType::Question => "questions",
            DiscoveryType::Anomaly => "anomalies",
            DiscoveryType::Pattern => "patterns",
            DiscoveryType::Connection => "connections",
            DiscoveryType::Solution => "solutions",
            DiscoveryType::Insight => "insights",
        }
    );
    
    // Publish to NATS
    match nats_client.publish(
        subject.clone(),
        serde_json::to_vec(&discovery)
            .map_err(|e| McpError::InternalError(format!("Failed to serialize discovery: {}", e)))?
            .into()
    ).await {
        Ok(_) => {
            debug!("Successfully published discovery to NATS subject: {}", subject);
            
            // Create response with metadata
            let mut metadata = HashMap::new();
            metadata.insert("discovery_id".to_string(), serde_json::Value::String(discovery_id.clone()));
            metadata.insert("nats_subject".to_string(), serde_json::Value::String(subject));
            metadata.insert("timestamp".to_string(), serde_json::Value::String(discovery.timestamp.to_rfc3339()));
            
            if let Some(related) = params.related_to {
                metadata.insert("related_to".to_string(), serde_json::Value::String(related));
            }
            
            Ok(CallToolResult::with_metadata(
                format!("Discovery shared successfully: {}", discovery_id),
                metadata
            ))
        }
        Err(e) => {
            error!("Failed to publish discovery to NATS: {}", e);
            Err(McpError::ConnectionError(format!("Failed to publish to NATS: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_parameter_validation() {
        // Test empty agent_id
        let params = Parameters {
            params: ShareDiscoveryParams {
                agent_id: "".to_string(),
                agent_role: "TestAgent".to_string(),
                discovery_type: "pattern".to_string(),
                content: "Test discovery".to_string(),
                confidence: 0.8,
                related_to: None,
            }
        };
        
        let nats_client = async_nats::connect("nats://localhost:4222").await.unwrap();
        let result = share_discovery_handler(params, nats_client).await;
        
        assert!(matches!(result, Err(McpError::InvalidParameters(_))));
    }
    
    #[tokio::test]
    async fn test_invalid_discovery_type() {
        let params = Parameters {
            params: ShareDiscoveryParams {
                agent_id: "test-agent".to_string(),
                agent_role: "TestAgent".to_string(),
                discovery_type: "invalid".to_string(),
                content: "Test discovery".to_string(),
                confidence: 0.8,
                related_to: None,
            }
        };
        
        let nats_client = async_nats::connect("nats://localhost:4222").await.unwrap();
        let result = share_discovery_handler(params, nats_client).await;
        
        assert!(matches!(result, Err(McpError::InvalidParameters(_))));
    }
    
    #[tokio::test]
    async fn test_confidence_bounds() {
        // Test confidence > 1.0
        let params = Parameters {
            params: ShareDiscoveryParams {
                agent_id: "test-agent".to_string(),
                agent_role: "TestAgent".to_string(),
                discovery_type: "pattern".to_string(),
                content: "Test discovery".to_string(),
                confidence: 1.5,
                related_to: None,
            }
        };
        
        let nats_client = async_nats::connect("nats://localhost:4222").await.unwrap();
        let result = share_discovery_handler(params, nats_client).await;
        
        assert!(matches!(result, Err(McpError::InvalidParameters(_))));
    }
}