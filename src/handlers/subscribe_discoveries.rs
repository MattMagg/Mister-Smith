//! Subscribe Discoveries Handler - Creates SSE endpoints for real-time discovery streaming

use super::{Parameters, CallToolResult, McpError, DISCOVERY_STORE, DiscoveryFilter};
use crate::collaboration::DiscoveryType;
use async_nats::Client;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, debug};

/// Parameters for subscribing to discoveries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeDiscoveriesParams {
    /// Filter by discovery types (optional)
    pub discovery_types: Option<Vec<String>>,
    /// Filter by agent roles (optional)
    pub agent_roles: Option<Vec<String>>,
    /// Minimum confidence threshold (optional)
    pub min_confidence: Option<f32>,
    /// Client agent ID for logging
    pub subscriber_agent_id: String,
}

/// SSE endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEndpoint {
    pub endpoint_url: String,
    pub subscription_id: String,
    pub filters_applied: HashMap<String, serde_json::Value>,
}

/// Subscribe to discoveries with optional filters
pub async fn subscribe_discoveries_handler(
    params: Parameters<SubscribeDiscoveriesParams>,
    _nats_client: Client,
) -> Result<CallToolResult, McpError> {
    let params = params.params;
    
    // Validate parameters
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
    
    // Create subscription filter
    let filter = DiscoveryFilter {
        types: discovery_types.clone(),
        agent_roles: params.agent_roles.clone(),
        min_confidence: params.min_confidence,
        keywords: None, // Could be added as a parameter in the future
    };
    
    // Generate subscription ID
    let subscription_id = uuid::Uuid::new_v4().to_string();
    
    // Store subscription with agent ID
    DISCOVERY_STORE.add_subscription(params.subscriber_agent_id.clone(), filter.clone()).await;
    
    // Build NATS subject pattern based on filters
    let nats_subject = build_nats_subject_pattern(&filter);
    
    info!(
        "📡 Agent {} subscribing to discoveries with filters: types={:?}, roles={:?}, min_confidence={:?}", 
        params.subscriber_agent_id,
        filter.types.as_ref().map(|types| types.iter()
            .map(|t| format!("{:?}", t))
            .collect::<Vec<_>>()
            .join(", ")
        ),
        filter.agent_roles,
        filter.min_confidence
    );
    
    // Generate SSE endpoint URL
    // In a real implementation, this would be the actual server endpoint
    let sse_endpoint_url = format!("/sse/discoveries/{}", subscription_id);
    
    // Prepare filters summary for response
    let mut filters_applied = HashMap::new();
    
    if let Some(types) = &filter.types {
        filters_applied.insert(
            "discovery_types".to_string(), 
            serde_json::Value::Array(
                types.iter()
                    .map(|t| serde_json::Value::String(format!("{:?}", t).to_lowercase()))
                    .collect()
            )
        );
    }
    
    if let Some(roles) = &filter.agent_roles {
        filters_applied.insert(
            "agent_roles".to_string(),
            serde_json::Value::Array(
                roles.iter()
                    .map(|r| serde_json::Value::String(r.clone()))
                    .collect()
            )
        );
    }
    
    if let Some(confidence) = filter.min_confidence {
        filters_applied.insert(
            "min_confidence".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(confidence as f64).unwrap())
        );
    }
    
    filters_applied.insert(
        "nats_subject".to_string(),
        serde_json::Value::String(nats_subject)
    );
    
    // Create SSE endpoint configuration
    let sse_config = SseEndpoint {
        endpoint_url: sse_endpoint_url.clone(),
        subscription_id: subscription_id.clone(),
        filters_applied: filters_applied.clone(),
    };
    
    // Create response
    let response_json = serde_json::json!({
        "subscription_id": subscription_id,
        "sse_endpoint": sse_config.endpoint_url,
        "filters": filters_applied,
        "instructions": {
            "connect": format!("Connect to {} using Server-Sent Events (SSE)", sse_config.endpoint_url),
            "authentication": "Include your agent token in Authorization header",
            "event_format": "Each event will contain a JSON-encoded Discovery object"
        }
    });
    
    debug!("Created subscription {} with SSE endpoint: {}", subscription_id, sse_endpoint_url);
    
    Ok(CallToolResult::json(response_json))
}

/// Build NATS subject pattern based on subscription filters
fn build_nats_subject_pattern(filter: &DiscoveryFilter) -> String {
    // If no specific filters, subscribe to all discoveries
    if filter.types.is_none() && filter.agent_roles.is_none() {
        return "discoveries.>".to_string();
    }
    
    // Build specific subject pattern based on discovery types
    if let Some(types) = &filter.types {
        // Subscribe to specific discovery type channels
        let _type_subjects: Vec<String> = types.iter()
            .map(|dt| match dt {
                DiscoveryType::Question => "discoveries.*.questions",
                DiscoveryType::Anomaly => "discoveries.*.anomalies",
                DiscoveryType::Pattern => "discoveries.*.patterns",
                DiscoveryType::Connection => "discoveries.*.connections",
                DiscoveryType::Solution => "discoveries.*.solutions",
                DiscoveryType::Insight => "discoveries.*.insights",
            })
            .map(|s| s.to_string())
            .collect();
        
        // For now, return a wildcard pattern
        // In production, you'd create multiple subscriptions
        return "discoveries.>".to_string();
    }
    
    // Default to all discoveries
    "discoveries.>".to_string()
}

// Note: SSE handler implementation would be part of the web server layer
// The actual implementation would:
// 1. Maintain a mapping of subscription_id to agent_id  
// 2. Connect to NATS with the appropriate subject pattern
// 3. Stream filtered discoveries to the SSE client
// 
// Example conceptual flow:
// - Store subscription_id -> agent_id mapping when creating subscription
// - When SSE client connects with subscription_id, retrieve agent_id
// - Use agent_id to get filter from DiscoveryStore
// - Subscribe to NATS and stream matching discoveries

// Note: The DiscoveryFilter.matches() method is used for filtering

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_subscription_creation() {
        let params = Parameters {
            params: SubscribeDiscoveriesParams {
                discovery_types: Some(vec!["pattern".to_string(), "anomaly".to_string()]),
                agent_roles: Some(vec!["Security".to_string()]),
                min_confidence: Some(0.7),
                subscriber_agent_id: "test-subscriber".to_string(),
            }
        };
        
        let nats_client = async_nats::connect("nats://localhost:4222").await.unwrap();
        let result = subscribe_discoveries_handler(params, nats_client).await;
        
        assert!(result.is_ok());
        
        if let Ok(CallToolResult { content }) = result {
            // Verify response contains subscription details
            assert!(!content.is_empty());
        }
    }
    
    #[tokio::test]
    async fn test_filter_validation() {
        // Test invalid discovery type
        let params = Parameters {
            params: SubscribeDiscoveriesParams {
                discovery_types: Some(vec!["invalid_type".to_string()]),
                agent_roles: None,
                min_confidence: None,
                subscriber_agent_id: "test-subscriber".to_string(),
            }
        };
        
        let nats_client = async_nats::connect("nats://localhost:4222").await.unwrap();
        let result = subscribe_discoveries_handler(params, nats_client).await;
        
        assert!(matches!(result, Err(McpError::InvalidParameters(_))));
    }
    
    #[tokio::test]
    async fn test_confidence_validation() {
        // Test confidence > 1.0
        let params = Parameters {
            params: SubscribeDiscoveriesParams {
                discovery_types: None,
                agent_roles: None,
                min_confidence: Some(1.5),
                subscriber_agent_id: "test-subscriber".to_string(),
            }
        };
        
        let nats_client = async_nats::connect("nats://localhost:4222").await.unwrap();
        let result = subscribe_discoveries_handler(params, nats_client).await;
        
        assert!(matches!(result, Err(McpError::InvalidParameters(_))));
    }
}