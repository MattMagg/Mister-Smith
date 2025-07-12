// Monitoring module for protocol translation layer

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_port: u16,
    pub enable_tracing: bool,
    pub log_level: String,
}