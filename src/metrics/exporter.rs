//! Prometheus metrics exporter

use crate::metrics::types::{MetricValue, MetricType, MetricDescriptor};
use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

/// Prometheus metrics exporter
pub struct PrometheusExporter {
    registry: Arc<RwLock<MetricsRegistry>>,
}

struct MetricsRegistry {
    collectors: Vec<Box<dyn MetricCollector>>,
}

impl PrometheusExporter {
    /// Create a new Prometheus exporter
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            registry: Arc::new(RwLock::new(MetricsRegistry {
                collectors: Vec::new(),
            })),
        })
    }
    
    /// Start the HTTP server for metrics export
    pub async fn start(self: Arc<Self>, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let addr = ([0, 0, 0, 0], port).into();
        
        let make_service = make_service_fn(move |_| {
            let exporter = self.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let exporter = exporter.clone();
                    async move { exporter.handle_request(req).await }
                }))
            }
        });
        
        let server = Server::bind(&addr).serve(make_service);
        info!("Prometheus metrics available at http://{}/metrics", addr);
        
        server.await?;
        Ok(())
    }
    
    /// Handle HTTP requests
    async fn handle_request(&self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        match req.uri().path() {
            "/metrics" => self.serve_metrics().await,
            "/health" => self.serve_health().await,
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("404 Not Found\n"))
                .unwrap()),
        }
    }
    
    /// Serve metrics in Prometheus format
    async fn serve_metrics(&self) -> Result<Response<Body>, hyper::Error> {
        // In a real implementation, this would collect metrics from all registered collectors
        // For now, we'll create a sample response
        let metrics = self.format_metrics().await;
        
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
            .body(Body::from(metrics))
            .unwrap())
    }
    
    /// Serve health check endpoint
    async fn serve_health(&self) -> Result<Response<Body>, hyper::Error> {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from("{\"status\":\"healthy\"}\n"))
            .unwrap())
    }
    
    /// Format metrics in Prometheus exposition format
    async fn format_metrics(&self) -> String {
        let mut output = String::new();
        
        // Add standard metadata metrics
        output.push_str(&self.format_metadata());
        
        // In a real implementation, this would iterate through collected metrics
        // For now, we'll add sample metrics based on the documentation
        output.push_str("\n# Agent Metrics\n");
        output.push_str("# HELP agent_operation_count Total number of operations performed by the agent\n");
        output.push_str("# TYPE agent_operation_count counter\n");
        output.push_str("agent_operation_count{agent_id=\"agent_1\",agent_type=\"researcher\"} 100\n");
        
        output.push_str("\n# HELP agent_operation_duration_ms Duration of agent operations in milliseconds\n");
        output.push_str("# TYPE agent_operation_duration_ms histogram\n");
        output.push_str("agent_operation_duration_ms_bucket{agent_id=\"agent_1\",agent_type=\"researcher\",le=\"10\"} 5\n");
        output.push_str("agent_operation_duration_ms_bucket{agent_id=\"agent_1\",agent_type=\"researcher\",le=\"50\"} 15\n");
        output.push_str("agent_operation_duration_ms_bucket{agent_id=\"agent_1\",agent_type=\"researcher\",le=\"100\"} 20\n");
        output.push_str("agent_operation_duration_ms_bucket{agent_id=\"agent_1\",agent_type=\"researcher\",le=\"+Inf\"} 25\n");
        output.push_str("agent_operation_duration_ms_sum{agent_id=\"agent_1\",agent_type=\"researcher\"} 1250\n");
        output.push_str("agent_operation_duration_ms_count{agent_id=\"agent_1\",agent_type=\"researcher\"} 25\n");
        
        output.push_str("\n# System Metrics\n");
        output.push_str("# HELP system_active_agents Number of currently active agents\n");
        output.push_str("# TYPE system_active_agents gauge\n");
        output.push_str("system_active_agents 5\n");
        
        output.push_str("\n# HELP system_throughput_per_second Tasks processed per second\n");
        output.push_str("# TYPE system_throughput_per_second gauge\n");
        output.push_str("system_throughput_per_second 45.5\n");
        
        output
    }
    
    /// Format standard metadata metrics
    fn format_metadata(&self) -> String {
        let mut output = String::new();
        
        // OpenTelemetry scope info
        output.push_str("# HELP otel_scope_info Instrumentation Scope metadata\n");
        output.push_str("# TYPE otel_scope_info gauge\n");
        output.push_str("otel_scope_info{otel_scope_name=\"mistersmith\",otel_scope_version=\"v0.1.0\"} 1\n");
        
        // Target info
        output.push_str("\n# HELP target_info Target metadata\n");
        output.push_str("# TYPE target_info gauge\n");
        output.push_str("target_info{service_name=\"mistersmith\",telemetry_sdk_language=\"rust\",telemetry_sdk_name=\"opentelemetry\",telemetry_sdk_version=\"latest\"} 1\n");
        
        output
    }
}

/// Format a metric value in Prometheus format
fn format_metric_line(metric: &MetricValue) -> String {
    let mut line = String::from(&metric.name);
    
    // Add labels
    if !metric.labels.is_empty() {
        line.push('{');
        let labels: Vec<String> = metric.labels
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, escape_label_value(v)))
            .collect();
        line.push_str(&labels.join(","));
        line.push('}');
    }
    
    // Add value
    line.push(' ');
    line.push_str(&format!("{}", metric.value));
    
    // Add timestamp (optional in Prometheus format)
    // line.push(' ');
    // line.push_str(&format!("{}", metric.timestamp.duration_since(UNIX_EPOCH).unwrap().as_millis()));
    
    line
}

/// Escape label values for Prometheus format
fn escape_label_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Trait for metric collectors (duplicate definition for exporter module)
#[async_trait::async_trait]
trait MetricCollector: Send + Sync {
    async fn collect(&self) -> Vec<MetricValue>;
    fn name(&self) -> &str;
    fn descriptors(&self) -> Vec<MetricDescriptor>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_escape_label_value() {
        assert_eq!(escape_label_value("simple"), "simple");
        assert_eq!(escape_label_value("with\\backslash"), "with\\\\backslash");
        assert_eq!(escape_label_value("with\"quote"), "with\\\"quote");
        assert_eq!(escape_label_value("with\nnewline"), "with\\nnewline");
    }
    
    #[test]
    fn test_format_metric_line() {
        let metric = MetricValue::new("test_metric", MetricType::Counter, 42.0)
            .with_label("env", "production")
            .with_label("service", "api");
        
        let line = format_metric_line(&metric);
        assert!(line.contains("test_metric{"));
        assert!(line.contains("env=\"production\""));
        assert!(line.contains("service=\"api\""));
        assert!(line.contains(" 42"));
    }
}