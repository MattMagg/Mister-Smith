//! Performance baseline benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use mistersmith::agent::{Agent, AgentConfig, AgentType};
use mistersmith::message::{Message, MessageType};
use mistersmith::metrics::init_metrics;
use std::time::Duration;
use tokio::runtime::Runtime;

fn agent_spawn_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("agent_spawn", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = AgentConfig {
                    id: "bench_agent".to_string(),
                    agent_type: AgentType::Worker,
                    ..Default::default()
                };
                
                let agent = Agent::spawn(config).await.unwrap();
                black_box(agent);
            })
        })
    });
}

fn message_throughput_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("message_throughput");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let (tx, mut rx) = tokio::sync::mpsc::channel(1000);
                    
                    // Send messages
                    for i in 0..size {
                        let msg = Message {
                            id: format!("msg_{}", i),
                            message_type: MessageType::Task,
                            source: "bench".to_string(),
                            target: Some("target".to_string()),
                            payload: serde_json::json!({"index": i}),
                            timestamp: std::time::SystemTime::now(),
                            correlation_id: None,
                        };
                        
                        tx.send(msg).await.unwrap();
                    }
                    
                    // Receive messages
                    for _ in 0..size {
                        let msg = rx.recv().await.unwrap();
                        black_box(msg);
                    }
                })
            })
        });
    }
    
    group.finish();
}

fn metrics_collection_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("metrics_collection", |b| {
        b.iter(|| {
            rt.block_on(async {
                let metrics = init_metrics().await.unwrap();
                let all_metrics = metrics.collect_all().await.unwrap();
                black_box(all_metrics);
            })
        })
    });
}

fn task_completion_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("task_completion", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Simulate task execution
                tokio::time::sleep(Duration::from_micros(100)).await;
                black_box(true);
            })
        })
    });
}

criterion_group!(
    benches,
    agent_spawn_benchmark,
    message_throughput_benchmark,
    metrics_collection_benchmark,
    task_completion_benchmark
);

criterion_main!(benches);