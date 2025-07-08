use serde_json::{json, Value};
use std::collections::HashMap;

/// Generator for realistic test data for MCP server testing
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate a realistic discovery of a given type
    pub fn generate_discovery(discovery_type: &str, agent_id: &str, sequence: u32) -> (String, Value) {
        let content = match discovery_type {
            "pattern" => format!("Pattern #{}: Repeating behavior detected in agent responses - agents tend to use similar error handling patterns across different tasks", sequence),
            "anomaly" => format!("Anomaly #{}: Unusual resource usage spike detected - memory consumption increased by 300% during batch processing", sequence),
            "connection" => format!("Connection #{}: Discovered relationship between agent performance and task complexity - linear correlation found", sequence),
            "solution" => format!("Solution #{}: Implemented caching mechanism that reduces API calls by 40% while maintaining response quality", sequence),
            "question" => format!("Question #{}: How should we handle rate limiting when multiple agents are making concurrent API calls?", sequence),
            "insight" => format!("Insight #{}: Agents perform better when given structured templates rather than free-form instructions", sequence),
            _ => format!("Generic discovery #{}: {}", sequence, discovery_type),
        };

        let metadata = match discovery_type {
            "pattern" => json!({
                "confidence": 0.85,
                "frequency": "high",
                "impact": "medium",
                "evidence": ["observation_1", "observation_2", "observation_3"]
            }),
            "anomaly" => json!({
                "severity": "high",
                "duration": "5m",
                "affected_components": ["memory", "cpu"],
                "threshold_exceeded": true
            }),
            "connection" => json!({
                "correlation_strength": 0.92,
                "variables": ["task_complexity", "response_time"],
                "statistical_significance": true
            }),
            "solution" => json!({
                "implementation_effort": "medium",
                "performance_gain": 40,
                "risk_level": "low",
                "prerequisites": ["caching_library", "redis_setup"]
            }),
            "question" => json!({
                "urgency": "medium",
                "domain": "architecture",
                "stakeholders": ["tech_lead", "devops"],
                "complexity": "high"
            }),
            "insight" => json!({
                "actionable": true,
                "validation_required": false,
                "applicability": "broad",
                "source": "empirical_observation"
            }),
            _ => json!({
                "type": discovery_type,
                "generated": true
            })
        };

        (content, metadata)
    }

    /// Generate a batch of discoveries for testing
    pub fn generate_discovery_batch(count: usize, agent_id: &str) -> Vec<(String, String, Value)> {
        let discovery_types = ["pattern", "anomaly", "connection", "solution", "question", "insight"];
        let mut discoveries = Vec::new();

        for i in 0..count {
            let discovery_type = discovery_types[i % discovery_types.len()];
            let (content, metadata) = Self::generate_discovery(discovery_type, agent_id, i as u32 + 1);
            discoveries.push((discovery_type.to_string(), content, metadata));
        }

        discoveries
    }

    /// Generate realistic agent IDs for testing
    pub fn generate_agent_ids(count: usize) -> Vec<String> {
        let prefixes = ["claude", "analyst", "researcher", "coordinator", "specialist"];
        let mut agent_ids = Vec::new();

        for i in 0..count {
            let prefix = prefixes[i % prefixes.len()];
            agent_ids.push(format!("{}_agent_{}", prefix, i + 1));
        }

        agent_ids
    }

    /// Generate filter parameters for subscription testing
    pub fn generate_subscription_filters() -> Vec<HashMap<String, Value>> {
        vec![
            // No filters (subscribe to everything)
            HashMap::new(),
            
            // Filter by agent ID
            {
                let mut filter = HashMap::new();
                filter.insert("agent_id".to_string(), json!("claude_agent_1"));
                filter
            },
            
            // Filter by discovery types
            {
                let mut filter = HashMap::new();
                filter.insert("discovery_types".to_string(), json!(["pattern", "anomaly"]));
                filter
            },
            
            // Filter by both agent ID and discovery types
            {
                let mut filter = HashMap::new();
                filter.insert("agent_id".to_string(), json!("analyst_agent_2"));
                filter.insert("discovery_types".to_string(), json!(["solution", "insight"]));
                filter
            },
        ]
    }

    /// Generate malformed data for error testing
    pub fn generate_invalid_discoveries() -> Vec<(String, String, Value, &'static str)> {
        vec![
            // Invalid discovery type
            (
                "invalid_type".to_string(),
                "This should fail".to_string(),
                json!({}),
                "invalid_discovery_type"
            ),
            
            // Empty agent ID
            (
                "pattern".to_string(),
                "Valid content".to_string(),
                json!({}),
                "empty_agent_id"
            ),
            
            // Empty content
            (
                "pattern".to_string(),
                "".to_string(),
                json!({}),
                "empty_content"
            ),
            
            // Invalid metadata format
            (
                "pattern".to_string(),
                "Valid content".to_string(),
                json!("invalid_metadata"),
                "invalid_metadata"
            ),
        ]
    }

    /// Generate performance test data
    pub fn generate_performance_test_data(agent_count: usize, discoveries_per_agent: usize) -> Vec<(String, Vec<(String, String, Value)>)> {
        let agent_ids = Self::generate_agent_ids(agent_count);
        let mut test_data = Vec::new();

        for agent_id in agent_ids {
            let discoveries = Self::generate_discovery_batch(discoveries_per_agent, &agent_id);
            test_data.push((agent_id, discoveries));
        }

        test_data
    }

    /// Generate realistic system health check data
    pub fn generate_expected_health_response() -> Value {
        json!({
            "status": "healthy",
            "components": {
                "nats": "connected",
                "discovery_store": "operational",
                "handlers": "active"
            },
            "timestamp": "2024-01-01T00:00:00Z"
        })
    }

    /// Generate expected metrics response
    pub fn generate_expected_metrics_response() -> Value {
        json!({
            "agents": {
                "active": 0,
                "total": 0
            },
            "discoveries": {
                "total": 0,
                "types": {}
            },
            "performance": {
                "avg_response_time": 0.0,
                "memory_usage": 0.0
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_discovery() {
        let (content, metadata) = TestDataGenerator::generate_discovery("pattern", "test_agent", 1);
        assert!(content.contains("Pattern #1"));
        assert!(metadata.get("confidence").is_some());
    }

    #[test]
    fn test_generate_discovery_batch() {
        let discoveries = TestDataGenerator::generate_discovery_batch(6, "test_agent");
        assert_eq!(discoveries.len(), 6);
        assert!(discoveries.iter().any(|(type_str, _, _)| type_str == "pattern"));
        assert!(discoveries.iter().any(|(type_str, _, _)| type_str == "anomaly"));
    }

    #[test]
    fn test_generate_agent_ids() {
        let agent_ids = TestDataGenerator::generate_agent_ids(3);
        assert_eq!(agent_ids.len(), 3);
        assert!(agent_ids[0].contains("claude_agent_1"));
    }

    #[test]
    fn test_generate_subscription_filters() {
        let filters = TestDataGenerator::generate_subscription_filters();
        assert_eq!(filters.len(), 4);
        assert!(filters[0].is_empty()); // No filters
        assert!(filters[1].contains_key("agent_id"));
        assert!(filters[2].contains_key("discovery_types"));
        assert!(filters[3].contains_key("agent_id") && filters[3].contains_key("discovery_types"));
    }

    #[test]
    fn test_generate_invalid_discoveries() {
        let invalid_data = TestDataGenerator::generate_invalid_discoveries();
        assert_eq!(invalid_data.len(), 4);
        assert_eq!(invalid_data[0].3, "invalid_discovery_type");
        assert_eq!(invalid_data[1].3, "empty_agent_id");
    }

    #[test]
    fn test_generate_performance_test_data() {
        let test_data = TestDataGenerator::generate_performance_test_data(3, 5);
        assert_eq!(test_data.len(), 3);
        assert_eq!(test_data[0].1.len(), 5);
    }
}