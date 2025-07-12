// NATS to AWS Protocol Translation Layer
// Core implementation for MisterSmith AWS migration

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use aws_sdk_eventbridge::types::PutEventsRequestEntry;
use aws_sdk_sqs::types::MessageAttributeValue;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use regex::Regex;
use thiserror::Error;
use serde_json::json;

pub mod connection;
pub mod transformation;
pub mod routing;
pub mod performance;
pub mod monitoring;

#[derive(Error, Debug)]
pub enum TranslationError {
    #[error("Invalid subject format: {0}")]
    InvalidSubjectFormat(String),
    
    #[error("No matching pattern for subject: {0}")]
    NoMatchingPattern(String),
    
    #[error("AWS service error: {0}")]
    AwsError(#[from] aws_sdk_eventbridge::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("No function mapping for subject: {0}")]
    NoFunctionMapping(String),
    
    #[error("No Lambda response received")]
    NoLambdaResponse,
    
    #[error("Invalid DynamoDB item")]
    InvalidDynamoDbItem,
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Timeout error: {0}")]
    TimeoutError(String),
}

// Core message envelope structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsMessageEnvelope {
    pub id: String,
    pub subject: String,
    pub timestamp: i64,
    pub sender_id: String,
    pub correlation_id: Option<String>,
    pub payload: serde_json::Value,
    pub headers: HashMap<String, String>,
}

// AWS target enumeration
#[derive(Debug, Clone)]
pub enum AwsTarget {
    EventBridge(EventBridgePattern),
    Sqs(SqsMessage),
    Lambda(LambdaInvocation),
    DynamoDb(DynamoDbOperation),
}

#[derive(Debug, Clone, Serialize)]
pub struct EventBridgePattern {
    pub source: Vec<String>,
    pub detail_type: Vec<String>,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct SqsMessage {
    pub queue_url: String,
    pub message_body: serde_json::Value,
    pub message_attributes: HashMap<String, MessageAttributeValue>,
}

#[derive(Debug, Clone)]
pub struct LambdaInvocation {
    pub function_name: String,
    pub payload: Vec<u8>,
    pub invocation_type: String,
}

#[derive(Debug, Clone)]
pub struct DynamoDbOperation {
    pub table_name: String,
    pub operation_type: DbOperationType,
    pub item: HashMap<String, aws_sdk_dynamodb::types::AttributeValue>,
}

#[derive(Debug, Clone)]
pub enum DbOperationType {
    Put,
    Get,
    Update,
    Delete,
    Query,
    Scan,
}

// Main translator struct
pub struct ProtocolTranslator {
    subject_mapper: Arc<SubjectMapper>,
    message_transformer: Arc<MessageTransformer>,
    connection_manager: Arc<connection::ConnectionManager>,
    performance_monitor: Arc<performance::PerformanceMonitor>,
    wildcard_translator: Arc<WildcardTranslator>,
}

impl ProtocolTranslator {
    pub async fn new(config: TranslatorConfig) -> Result<Self, TranslationError> {
        let connection_manager = Arc::new(
            connection::ConnectionManager::new(config.connection_config).await?
        );
        
        Ok(Self {
            subject_mapper: Arc::new(SubjectMapper::new()),
            message_transformer: Arc::new(MessageTransformer::new()),
            connection_manager,
            performance_monitor: Arc::new(performance::PerformanceMonitor::new()),
            wildcard_translator: Arc::new(WildcardTranslator::new()),
        })
    }
    
    pub async fn translate_message(
        &self,
        envelope: NatsMessageEnvelope,
    ) -> Result<AwsTarget, TranslationError> {
        let start = std::time::Instant::now();
        
        // First try direct mapping
        let target = match self.subject_mapper.map_subject(&envelope.subject).await {
            Ok(target) => target,
            Err(_) => {
                // Fall back to wildcard matching
                self.wildcard_translator.translate(&envelope.subject)?
            }
        };
        
        // Transform message content
        let transformed_target = self.message_transformer
            .transform(envelope, target)
            .await?;
        
        // Record performance metrics
        self.performance_monitor.record_translation(
            &envelope.subject,
            &transformed_target,
            start.elapsed(),
        ).await;
        
        Ok(transformed_target)
    }
    
    pub async fn batch_translate(
        &self,
        envelopes: Vec<NatsMessageEnvelope>,
    ) -> Vec<Result<AwsTarget, TranslationError>> {
        use futures::future::join_all;
        
        let futures = envelopes
            .into_iter()
            .map(|env| self.translate_message(env));
        
        join_all(futures).await
    }
}

// Subject mapping logic
pub struct SubjectMapper {
    static_mappings: HashMap<String, fn(&str) -> Result<AwsTarget, TranslationError>>,
}

impl SubjectMapper {
    pub fn new() -> Self {
        let mut mappings = HashMap::new();
        
        // Register static mappings
        mappings.insert("agent.*.heartbeat".to_string(), 
            Self::map_agent_heartbeat as fn(&str) -> Result<AwsTarget, TranslationError>);
        mappings.insert("work.*.*.* ".to_string(), 
            Self::map_work_queue as fn(&str) -> Result<AwsTarget, TranslationError>);
        mappings.insert("coord.*.complete".to_string(), 
            Self::map_coord_complete as fn(&str) -> Result<AwsTarget, TranslationError>);
        
        Self { static_mappings: mappings }
    }
    
    pub async fn map_subject(&self, subject: &str) -> Result<AwsTarget, TranslationError> {
        // Check static mappings first
        for (pattern, mapper) in &self.static_mappings {
            if Self::matches_pattern(pattern, subject) {
                return mapper(subject);
            }
        }
        
        Err(TranslationError::NoMatchingPattern(subject.to_string()))
    }
    
    fn matches_pattern(pattern: &str, subject: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split('.').collect();
        let subject_parts: Vec<&str> = subject.split('.').collect();
        
        if pattern_parts.len() != subject_parts.len() {
            return false;
        }
        
        pattern_parts
            .iter()
            .zip(subject_parts.iter())
            .all(|(p, s)| p == &"*" || p == s)
    }
    
    fn map_agent_heartbeat(subject: &str) -> Result<AwsTarget, TranslationError> {
        let parts: Vec<&str> = subject.split('.').collect();
        if parts.len() >= 3 && parts[0] == "agent" && parts[2] == "heartbeat" {
            let agent_id = parts[1];
            Ok(AwsTarget::EventBridge(EventBridgePattern {
                source: vec!["mistersmith.agent".to_string()],
                detail_type: vec!["Agent Heartbeat".to_string()],
                detail: json!({
                    "agentId": agent_id,
                    "type": "heartbeat"
                })
            }))
        } else {
            Err(TranslationError::InvalidSubjectFormat(subject.to_string()))
        }
    }
    
    fn map_work_queue(subject: &str) -> Result<AwsTarget, TranslationError> {
        let parts: Vec<&str> = subject.split('.').collect();
        if parts.len() >= 4 && parts[0] == "work" {
            let queue_url = format!(
                "https://sqs.{}.amazonaws.com/{}/mistersmith-work-{}-{}",
                std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
                std::env::var("AWS_ACCOUNT_ID").unwrap_or_else(|_| "123456789012".to_string()),
                parts[1],
                parts[2]
            );
            
            Ok(AwsTarget::Sqs(SqsMessage {
                queue_url,
                message_body: json!({
                    "domain": parts[1],
                    "priority": parts[2],
                    "taskType": parts[3]
                }),
                message_attributes: HashMap::new(),
            }))
        } else {
            Err(TranslationError::InvalidSubjectFormat(subject.to_string()))
        }
    }
    
    fn map_coord_complete(subject: &str) -> Result<AwsTarget, TranslationError> {
        let parts: Vec<&str> = subject.split('.').collect();
        if parts.len() >= 3 && parts[0] == "coord" && parts[2] == "complete" {
            let coord_id = parts[1];
            
            let mut item = HashMap::new();
            item.insert(
                "coordinationId".to_string(),
                aws_sdk_dynamodb::types::AttributeValue::S(coord_id.to_string())
            );
            item.insert(
                "status".to_string(),
                aws_sdk_dynamodb::types::AttributeValue::S("COMPLETE".to_string())
            );
            item.insert(
                "completedAt".to_string(),
                aws_sdk_dynamodb::types::AttributeValue::N(Utc::now().timestamp().to_string())
            );
            
            Ok(AwsTarget::DynamoDb(DynamoDbOperation {
                table_name: "CoordinationState".to_string(),
                operation_type: DbOperationType::Update,
                item,
            }))
        } else {
            Err(TranslationError::InvalidSubjectFormat(subject.to_string()))
        }
    }
}

// Wildcard pattern translator
pub struct WildcardTranslator {
    patterns: Vec<(Regex, Box<dyn Fn(&str) -> Result<AwsTarget, TranslationError> + Send + Sync>)>,
}

impl WildcardTranslator {
    pub fn new() -> Self {
        let mut translator = WildcardTranslator {
            patterns: vec![],
        };
        
        // Agent wildcard patterns
        translator.add_pattern(
            r"^agent\.([^.]+)\.(.+)$",
            Box::new(|subject| {
                let caps = Regex::new(r"^agent\.([^.]+)\.(.+)$")
                    .unwrap()
                    .captures(subject)
                    .unwrap();
                let agent_id = &caps[1];
                let action = &caps[2];
                
                Ok(AwsTarget::EventBridge(EventBridgePattern {
                    source: vec!["mistersmith.agent".to_string()],
                    detail_type: vec![format!("Agent {}", action)],
                    detail: json!({
                        "agentId": agent_id,
                        "action": action
                    })
                }))
            })
        );
        
        // Work queue wildcard patterns
        translator.add_pattern(
            r"^work\.([^.]+)\.([^.]+)\.(.+)$",
            Box::new(|subject| {
                let caps = Regex::new(r"^work\.([^.]+)\.([^.]+)\.(.+)$")
                    .unwrap()
                    .captures(subject)
                    .unwrap();
                let domain = &caps[1];
                let priority = &caps[2];
                let task_type = &caps[3];
                
                let queue_url = format!(
                    "https://sqs.{}.amazonaws.com/{}/mistersmith-work-{}-{}",
                    std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
                    std::env::var("AWS_ACCOUNT_ID").unwrap_or_else(|_| "123456789012".to_string()),
                    domain,
                    priority
                );
                
                Ok(AwsTarget::Sqs(SqsMessage {
                    queue_url,
                    message_body: json!({
                        "domain": domain,
                        "priority": priority,
                        "taskType": task_type
                    }),
                    message_attributes: HashMap::new(),
                }))
            })
        );
        
        translator
    }
    
    fn add_pattern(
        &mut self,
        pattern: &str,
        translator: Box<dyn Fn(&str) -> Result<AwsTarget, TranslationError> + Send + Sync>,
    ) {
        if let Ok(regex) = Regex::new(pattern) {
            self.patterns.push((regex, translator));
        }
    }
    
    pub fn translate(&self, subject: &str) -> Result<AwsTarget, TranslationError> {
        for (pattern, translator) in &self.patterns {
            if pattern.is_match(subject) {
                return translator(subject);
            }
        }
        Err(TranslationError::NoMatchingPattern(subject.to_string()))
    }
}

// Message transformer
pub struct MessageTransformer {
    transformations: Arc<RwLock<HashMap<String, Box<dyn MessageTransformation + Send + Sync>>>>,
}

#[async_trait::async_trait]
pub trait MessageTransformation {
    async fn transform(
        &self,
        envelope: NatsMessageEnvelope,
        target: AwsTarget,
    ) -> Result<AwsTarget, TranslationError>;
}

impl MessageTransformer {
    pub fn new() -> Self {
        Self {
            transformations: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn transform(
        &self,
        envelope: NatsMessageEnvelope,
        target: AwsTarget,
    ) -> Result<AwsTarget, TranslationError> {
        // Apply any registered transformations
        let transformations = self.transformations.read().await;
        
        let mut result = target;
        for (_, transformation) in transformations.iter() {
            result = transformation.transform(envelope.clone(), result).await?;
        }
        
        Ok(result)
    }
}

// Configuration
#[derive(Debug, Clone, Deserialize)]
pub struct TranslatorConfig {
    pub connection_config: connection::ConnectionConfig,
    pub performance_config: performance::PerformanceConfig,
    pub monitoring_config: monitoring::MonitoringConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_heartbeat_mapping() {
        let subject = "agent.agent-123.heartbeat";
        let result = SubjectMapper::map_agent_heartbeat(subject).unwrap();
        
        match result {
            AwsTarget::EventBridge(pattern) => {
                assert_eq!(pattern.source, vec!["mistersmith.agent"]);
                assert_eq!(pattern.detail_type, vec!["Agent Heartbeat"]);
                assert_eq!(pattern.detail["agentId"], "agent-123");
            }
            _ => panic!("Expected EventBridge target"),
        }
    }
    
    #[test]
    fn test_wildcard_pattern_matching() {
        let translator = WildcardTranslator::new();
        let subject = "agent.test-agent.status";
        let result = translator.translate(subject).unwrap();
        
        match result {
            AwsTarget::EventBridge(pattern) => {
                assert_eq!(pattern.source, vec!["mistersmith.agent"]);
                assert!(pattern.detail_type[0].contains("status"));
            }
            _ => panic!("Expected EventBridge target"),
        }
    }
    
    #[tokio::test]
    async fn test_message_envelope_transformation() {
        let envelope = NatsMessageEnvelope {
            id: "msg-123".to_string(),
            subject: "agent.test.heartbeat".to_string(),
            timestamp: Utc::now().timestamp(),
            sender_id: "sender-1".to_string(),
            correlation_id: None,
            payload: json!({"status": "healthy"}),
            headers: HashMap::new(),
        };
        
        let mapper = SubjectMapper::new();
        let target = mapper.map_subject(&envelope.subject).await.unwrap();
        
        assert!(matches!(target, AwsTarget::EventBridge(_)));
    }
}