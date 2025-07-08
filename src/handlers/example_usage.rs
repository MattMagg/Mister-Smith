//! Example usage of the discovery sharing MCP handlers

use super::*;

/// Example of how to use the share_discovery_handler in an MCP server
pub async fn example_share_discovery() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to NATS
    let nats_client = async_nats::connect("nats://localhost:4222").await?;
    
    // Create discovery parameters
    let params = Parameters {
        params: share_discovery::ShareDiscoveryParams {
            agent_id: "security-agent-001".to_string(),
            agent_role: "Security Analyst".to_string(),
            discovery_type: "anomaly".to_string(),
            content: "Detected unusual API access patterns from IP 192.168.1.100".to_string(),
            confidence: 0.85,
            related_to: None,
        }
    };
    
    // Handle the discovery sharing
    match share_discovery::share_discovery_handler(params, nats_client).await {
        Ok(result) => {
            println!("Discovery shared successfully: {:?}", result);
        }
        Err(e) => {
            eprintln!("Failed to share discovery: {}", e);
        }
    }
    
    Ok(())
}

/// Example of how to use the subscribe_discoveries_handler in an MCP server
pub async fn example_subscribe_discoveries() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to NATS
    let nats_client = async_nats::connect("nats://localhost:4222").await?;
    
    // Create subscription parameters
    let params = Parameters {
        params: subscribe_discoveries::SubscribeDiscoveriesParams {
            discovery_types: Some(vec!["anomaly".to_string(), "solution".to_string()]),
            agent_roles: Some(vec!["Security Analyst".to_string()]),
            min_confidence: Some(0.7),
            subscriber_agent_id: "monitoring-agent-001".to_string(),
        }
    };
    
    // Handle the discovery subscription
    match subscribe_discoveries::subscribe_discoveries_handler(params, nats_client).await {
        Ok(result) => {
            println!("Subscription created successfully: {:?}", result);
        }
        Err(e) => {
            eprintln!("Failed to create subscription: {}", e);
        }
    }
    
    Ok(())
}

/// Example of a complete MCP server workflow
pub async fn example_full_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to NATS
    let nats_client = async_nats::connect("nats://localhost:4222").await?;
    
    // Step 1: Create a subscription
    let subscribe_params = Parameters {
        params: subscribe_discoveries::SubscribeDiscoveriesParams {
            discovery_types: Some(vec!["pattern".to_string()]),
            agent_roles: None,
            min_confidence: Some(0.6),
            subscriber_agent_id: "analysis-agent-001".to_string(),
        }
    };
    
    let subscription_result = subscribe_discoveries::subscribe_discoveries_handler(
        subscribe_params, 
        nats_client.clone()
    ).await?;
    
    println!("Created subscription: {:?}", subscription_result);
    
    // Step 2: Share a discovery
    let share_params = Parameters {
        params: share_discovery::ShareDiscoveryParams {
            agent_id: "pattern-detection-agent".to_string(),
            agent_role: "Pattern Detector".to_string(),
            discovery_type: "pattern".to_string(),
            content: "Recurring database query pattern detected in auth service".to_string(),
            confidence: 0.9,
            related_to: None,
        }
    };
    
    let share_result = share_discovery::share_discovery_handler(
        share_params, 
        nats_client.clone()
    ).await?;
    
    println!("Shared discovery: {:?}", share_result);
    
    // Step 3: In a real implementation, the SSE endpoint would now stream this discovery
    // to any connected subscribers matching the filter
    
    Ok(())
}

/// Example of handling different discovery types
pub async fn example_different_types() -> Result<(), Box<dyn std::error::Error>> {
    let nats_client = async_nats::connect("nats://localhost:4222").await?;
    
    let discovery_types = vec![
        ("pattern", "CPU utilization spikes every 15 minutes"),
        ("anomaly", "Unusual network traffic from internal subnet"),
        ("connection", "Database slowdown correlates with auth service spikes"),
        ("solution", "Added connection pooling to reduce DB load"),
        ("question", "Should we implement rate limiting on the API?"),
        ("insight", "User behavior patterns suggest need for caching"),
    ];
    
    for (disc_type, content) in discovery_types {
        let params = Parameters {
            params: share_discovery::ShareDiscoveryParams {
                agent_id: format!("{}-agent", disc_type),
                agent_role: "Multi-Agent".to_string(),
                discovery_type: disc_type.to_string(),
                content: content.to_string(),
                confidence: 0.75,
                related_to: None,
            }
        };
        
        match share_discovery::share_discovery_handler(params, nats_client.clone()).await {
            Ok(result) => {
                println!("Shared {} discovery: {:?}", disc_type, result);
            }
            Err(e) => {
                eprintln!("Failed to share {} discovery: {}", disc_type, e);
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_handler_integration() {
        // This test requires a running NATS server
        if async_nats::connect("nats://localhost:4222").await.is_err() {
            println!("Skipping integration test - NATS not available");
            return;
        }
        
        // Test the full workflow
        example_full_workflow().await.expect("Workflow should complete successfully");
    }
}