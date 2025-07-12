# Configuration Migration Guide

## Overview
This guide helps developers migrate from hardcoded values to the new configuration management system.

## Quick Start

### 1. Environment Variables (Immediate Use)

Set these environment variables to override defaults without code changes:

```bash
# NATS Configuration
export NATS_URL="nats://your-nats-server:4222"

# HTTP Configuration  
export HTTP_PORT="8080"
export SSE_PORT="3000"

# Timeout Configuration
export CIRCUIT_BREAKER_TIMEOUT="100ms"
export PROCESS_TIMEOUT="10s"

# Runtime Configuration
export ENVIRONMENT="development"  # or "staging", "production"
export USE_AWS_CONFIG="false"     # Set to "true" for AWS integration
```

### 2. Configuration File (Optional)

Create `config/default.toml`:

```toml
[nats]
url = "nats://localhost:4222"
connect_timeout = "10s"
ping_interval = "30s"
max_reconnects = 10

[http]
port = 8080
sse_port = 3000
bind_address = "0.0.0.0"
cors_origins = ["*"]

[timeouts]
circuit_breaker_reset = "100ms"
process_execution = "10s"
agent_heartbeat = "30s"
task_completion = "5m"

[runtime]
environment = "development"
log_level = "info"
enable_telemetry = true
enable_circuit_breaker = true
```

## Code Migration Examples

### Before (Hardcoded):

```rust
// Old: Hardcoded NATS connection
let nats_client = async_nats::connect("nats://localhost:4222").await?;

// Old: Hardcoded HTTP port
let addr = "127.0.0.1:8080".parse()?;

// Old: Hardcoded timeout
let timeout = Duration::from_secs(10);
```

### After (Configuration-based):

```rust
use crate::config::{load_configuration, get_nats_url};

// New: Load configuration once at startup
let config = load_configuration().await?;

// New: Use configured NATS URL
let nats_client = async_nats::connect(&config.nats.url).await?;

// New: Use configured HTTP port
let addr = format!("{}:{}", config.http.bind_address, config.http.port).parse()?;

// New: Use configured timeout
let timeout = config.timeouts.process_execution;
```

## Migration Steps by Component

### 1. Main Application (`src/main.rs`)

```rust
// Add to main.rs
use crate::config::{AppConfig, load_configuration};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration at startup
    let config = load_configuration().await?;
    
    // Pass config to components
    let nats_transport = NatsTransport::new(&config.nats).await?;
    let http_server = HttpServer::new(&config.http)?;
    
    // Rest of application...
}
```

### 2. NATS Configuration (`src/mcp_server/nats_config.rs`)

```rust
// Update NatsConfig to use new configuration
impl From<&crate::config::NatsConfig> for NatsConfig {
    fn from(config: &crate::config::NatsConfig) -> Self {
        Self {
            server_url: config.url.clone(),
            connect_timeout: config.connect_timeout,
            ping_interval: config.ping_interval,
            max_reconnects: config.max_reconnects,
            // ... other fields
        }
    }
}
```

### 3. HTTP Server (`src/mcp_server/server.rs`)

```rust
// Update server initialization
pub async fn start_server(config: &HttpConfig) -> Result<()> {
    let addr = format!("{}:{}", config.bind_address, config.port);
    
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(
            config.cors_origins
                .iter()
                .map(|o| o.parse().unwrap())
                .collect::<Vec<_>>()
        );
    
    // Rest of server setup...
}
```

### 4. Test Files

For tests, use a test configuration:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    fn test_config() -> AppConfig {
        // Use defaults or load test-specific config
        AppConfig {
            nats: NatsConfig {
                url: "nats://localhost:4222".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    
    #[tokio::test]
    async fn test_with_config() {
        let config = test_config();
        // Use config in tests...
    }
}
```

## AWS Integration

### 1. Enable AWS Configuration

```bash
export USE_AWS_CONFIG="true"
export AWS_REGION="us-east-1"
```

### 2. Parameter Store Structure

```
/mistersmith/production/
├── nats/
│   └── url                    # nats://prod-nats.internal:4222
├── http/
│   ├── port                   # 8080
│   └── sse_port              # 3000
└── timeouts/
    ├── circuit_breaker       # 100ms
    └── process_execution     # 30s
```

### 3. Secrets Manager Structure

```json
{
  "secret_id": "mistersmith/production/credentials",
  "content": {
    "nats_username": "mistersmith",
    "nats_password": "secure-password",
    "api_key": "api-key-value"
  }
}
```

## Docker/ECS Configuration

### Dockerfile Updates

```dockerfile
# Add configuration directory
COPY config /app/config

# Set default environment
ENV ENVIRONMENT=production
ENV USE_AWS_CONFIG=true
```

### ECS Task Definition

```json
{
  "containerDefinitions": [{
    "environment": [
      {"name": "ENVIRONMENT", "value": "production"},
      {"name": "USE_AWS_CONFIG", "value": "true"},
      {"name": "AWS_REGION", "value": "us-east-1"}
    ],
    "secrets": [
      {
        "name": "NATS_URL",
        "valueFrom": "/mistersmith/production/nats/url"
      }
    ]
  }]
}
```

## Testing Configuration

### 1. Unit Tests

```rust
#[test]
fn test_config_loading() {
    // Set test environment
    env::set_var("ENVIRONMENT", "test");
    env::set_var("NATS_URL", "nats://test:4222");
    
    let config = load_configuration().await.unwrap();
    assert_eq!(config.nats.url, "nats://test:4222");
    
    // Clean up
    env::remove_var("ENVIRONMENT");
    env::remove_var("NATS_URL");
}
```

### 2. Integration Tests with LocalStack

```rust
#[tokio::test]
async fn test_aws_parameter_store() {
    // Requires LocalStack running
    env::set_var("AWS_ENDPOINT_URL", "http://localhost:4566");
    env::set_var("USE_AWS_CONFIG", "true");
    
    let config = load_configuration().await.unwrap();
    // Test AWS-loaded configuration...
}
```

## Rollback Strategy

If issues occur, disable new configuration system:

```bash
# Disable AWS integration
export USE_AWS_CONFIG="false"

# Or use emergency override
export MISTERSMITH_CONFIG_OVERRIDE="hardcoded"
```

In code:

```rust
// Emergency fallback
let nats_url = if env::var("MISTERSMITH_CONFIG_OVERRIDE").is_ok() {
    "nats://localhost:4222".to_string()
} else {
    config.nats.url.clone()
};
```

## Common Issues & Solutions

### Issue 1: Configuration Not Loading

**Symptom**: Application uses default values despite environment variables

**Solution**: Check variable names and prefixes
```bash
# Correct
export NATS_URL="nats://custom:4222"

# Also correct (with prefix)
export MISTERSMITH__NATS__URL="nats://custom:4222"
```

### Issue 2: AWS Permissions

**Symptom**: Failed to load parameters from Parameter Store

**Solution**: Ensure IAM role has permissions
```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Action": [
      "ssm:GetParameter",
      "ssm:GetParameters",
      "ssm:GetParametersByPath"
    ],
    "Resource": "arn:aws:ssm:*:*:parameter/mistersmith/*"
  }]
}
```

### Issue 3: Timeout Parsing

**Symptom**: Invalid timeout format

**Solution**: Use valid duration formats
```bash
# Valid formats
export CIRCUIT_BREAKER_TIMEOUT="100ms"    # milliseconds
export PROCESS_TIMEOUT="10s"              # seconds
export TASK_TIMEOUT="5m"                  # minutes
export LONG_TIMEOUT="1h30m"               # hours and minutes
```

## Monitoring Configuration

Add logging to track configuration sources:

```rust
// In load_configuration()
info!("Configuration sources:");
info!("  - Defaults: Applied");
info!("  - Config file: {}", if has_file { "Loaded" } else { "Not found" });
info!("  - Environment: {} variables", env_count);
info!("  - AWS Parameter Store: {}", if aws_enabled { "Enabled" } else { "Disabled" });
info!("  - Final NATS URL: {}", config.nats.url);
```

## Next Steps

1. **Phase 1**: Update local development to use environment variables
2. **Phase 2**: Add configuration files for different environments
3. **Phase 3**: Enable AWS integration in staging
4. **Phase 4**: Roll out to production with full AWS integration

---

*For questions or issues, contact the platform team or create an issue in the repository.*