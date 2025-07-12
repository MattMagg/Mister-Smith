// Example: Basic NATS to AWS protocol translation

use nats_aws_translator::{
    ProtocolTranslator, TranslatorConfig, NatsMessageEnvelope,
    connection::ConnectionConfig,
    performance::PerformanceConfig,
    monitoring::MonitoringConfig,
};
use serde_json::json;
use std::collections::HashMap;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Configure the translator
    let config = TranslatorConfig {
        connection_config: ConnectionConfig {
            nats_clusters: {
                let mut clusters = HashMap::new();
                clusters.insert("primary".to_string(), NatsClusterConfig {
                    urls: vec!["nats://localhost:4222".to_string()],
                    auth_token: None,
                    tls_config: None,
                    pool_size: 10,
                    min_idle: 2,
                });
                clusters
            },
            aws_region: "us-east-1".to_string(),
            aws_retry_attempts: 3,
            aws_retry_initial_backoff_ms: 100,
            health_check_interval_secs: 30,
            health_check_timeout_secs: 5,
        },
        performance_config: PerformanceConfig {
            batch_size: 100,
            batch_timeout_ms: 100,
            cache_ttl_secs: 300,
            circuit_breaker_threshold: 5,
            circuit_breaker_recovery_secs: 60,
        },
        monitoring_config: MonitoringConfig {
            metrics_port: 9090,
            enable_tracing: true,
            log_level: "info".to_string(),
        },
    };
    
    // Create the translator
    let translator = ProtocolTranslator::new(config).await?;
    
    // Example 1: Agent Heartbeat Translation
    println!("Example 1: Agent Heartbeat");
    let heartbeat_msg = NatsMessageEnvelope {
        id: "msg-001".to_string(),
        subject: "agent.agent-123.heartbeat".to_string(),
        timestamp: Utc::now().timestamp(),
        sender_id: "agent-123".to_string(),
        correlation_id: None,
        payload: json!({
            "status": "healthy",
            "cpu_usage": 45.2,
            "memory_usage": 62.8,
            "active_tasks": 3
        }),
        headers: HashMap::new(),
    };
    
    match translator.translate_message(heartbeat_msg).await {
        Ok(target) => println!("Translated to: {:?}", target),
        Err(e) => eprintln!("Translation error: {}", e),
    }
    
    // Example 2: Work Queue Translation
    println!("\nExample 2: Work Queue");
    let work_msg = NatsMessageEnvelope {
        id: "msg-002".to_string(),
        subject: "work.nlp.high.text-analysis".to_string(),
        timestamp: Utc::now().timestamp(),
        sender_id: "coordinator-001".to_string(),
        correlation_id: Some("req-12345".to_string()),
        payload: json!({
            "task_id": "task-789",
            "text": "Analyze this complex document for sentiment and entities",
            "options": {
                "language": "en",
                "extract_entities": true,
                "sentiment_analysis": true
            }
        }),
        headers: {
            let mut headers = HashMap::new();
            headers.insert("priority".to_string(), "high".to_string());
            headers.insert("timeout".to_string(), "300".to_string());
            headers
        },
    };
    
    match translator.translate_message(work_msg).await {
        Ok(target) => println!("Translated to: {:?}", target),
        Err(e) => eprintln!("Translation error: {}", e),
    }
    
    // Example 3: Coordination Complete Translation
    println!("\nExample 3: Coordination Complete");
    let coord_msg = NatsMessageEnvelope {
        id: "msg-003".to_string(),
        subject: "coord.coord-456.complete".to_string(),
        timestamp: Utc::now().timestamp(),
        sender_id: "agent-789".to_string(),
        correlation_id: Some("coord-456".to_string()),
        payload: json!({
            "result": "success",
            "duration_ms": 1523,
            "output": {
                "processed_items": 42,
                "errors": 0,
                "warnings": 2
            }
        }),
        headers: HashMap::new(),
    };
    
    match translator.translate_message(coord_msg).await {
        Ok(target) => println!("Translated to: {:?}", target),
        Err(e) => eprintln!("Translation error: {}", e),
    }
    
    // Example 4: Batch Translation
    println!("\nExample 4: Batch Translation");
    let batch_messages = vec![
        NatsMessageEnvelope {
            id: "batch-001".to_string(),
            subject: "agent.agent-001.status".to_string(),
            timestamp: Utc::now().timestamp(),
            sender_id: "agent-001".to_string(),
            correlation_id: None,
            payload: json!({"status": "active"}),
            headers: HashMap::new(),
        },
        NatsMessageEnvelope {
            id: "batch-002".to_string(),
            subject: "agent.agent-002.status".to_string(),
            timestamp: Utc::now().timestamp(),
            sender_id: "agent-002".to_string(),
            correlation_id: None,
            payload: json!({"status": "idle"}),
            headers: HashMap::new(),
        },
        NatsMessageEnvelope {
            id: "batch-003".to_string(),
            subject: "work.vision.medium.image-classification".to_string(),
            timestamp: Utc::now().timestamp(),
            sender_id: "coordinator-002".to_string(),
            correlation_id: None,
            payload: json!({
                "image_url": "s3://bucket/image.jpg",
                "model": "resnet50"
            }),
            headers: HashMap::new(),
        },
    ];
    
    let results = translator.batch_translate(batch_messages).await;
    for (idx, result) in results.iter().enumerate() {
        match result {
            Ok(target) => println!("Message {} translated to: {:?}", idx + 1, target),
            Err(e) => eprintln!("Message {} translation error: {}", idx + 1, e),
        }
    }
    
    // Example 5: Ordered Message Processing
    println!("\nExample 5: Ordered Message Processing");
    let ordering_preserver = translator.performance_monitor().ordering_preserver();
    
    // Simulate out-of-order messages
    let ordered_messages = vec![
        (3, "msg-3", "Third message"),
        (1, "msg-1", "First message"),
        (2, "msg-2", "Second message"),
        (4, "msg-4", "Fourth message"),
    ];
    
    for (seq, id, content) in ordered_messages {
        let msg = NatsMessageEnvelope {
            id: id.to_string(),
            subject: "ordered.partition-1.data".to_string(),
            timestamp: Utc::now().timestamp(),
            sender_id: "producer-001".to_string(),
            correlation_id: None,
            payload: json!({"sequence": seq, "content": content}),
            headers: HashMap::new(),
        };
        
        // In production, you would translate and then process with ordering
        match translator.translate_message(msg.clone()).await {
            Ok(target) => {
                ordering_preserver.process_ordered_message(
                    "partition-1",
                    seq,
                    msg,
                    target,
                    |env, tgt| async move {
                        println!("Processing ordered message: {} - {}", env.id, env.payload["content"]);
                        Ok(())
                    }
                ).await?;
            }
            Err(e) => eprintln!("Translation error: {}", e),
        }
    }
    
    // Example 6: Latency Compensation with Caching
    println!("\nExample 6: Latency Compensation");
    let compensator = translator.performance_monitor().latency_compensator();
    
    // First request - will hit the actual service
    let result1 = compensator.compensate_request(
        "get_agent_status",
        "agent:agent-123:status",
        || async {
            // Simulate slow API call
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            Ok("Active".to_string())
        }
    ).await?;
    println!("First request result: {}", result1);
    
    // Second request - should hit cache
    let result2 = compensator.compensate_request(
        "get_agent_status",
        "agent:agent-123:status",
        || async {
            // This won't be called due to cache hit
            Ok("Active".to_string())
        }
    ).await?;
    println!("Second request result (from cache): {}", result2);
    
    println!("\nProtocol translation examples completed!");
    
    Ok(())
}