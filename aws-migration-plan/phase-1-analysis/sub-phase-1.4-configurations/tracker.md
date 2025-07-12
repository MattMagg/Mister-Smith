# Sub-Phase 1.4: Configuration Analysis

## Status: In Progress
Started: 2025-07-11T09:52:57.516Z

## Overview
Comprehensive extraction and analysis of all configuration patterns in the MisterSmith codebase, including environment variables, configuration files, secrets management, and runtime configurations.

## Configuration Files Found

### 1. Core Configuration Files

#### Cargo.toml (Root)
- **Location**: `/Cargo.toml`
- **Type**: Rust project manifest
- **Key Settings**:
  - Project metadata (name, version, authors, license)
  - Dependencies with specific versions and features
  - Binary and benchmark configurations
- **AWS Consideration**: Dependencies need to be reviewed for AWS SDK integration

#### Hive Mind Configuration
- **Location**: `/.hive-mind/config.json`
- **Type**: JSON configuration
- **Key Settings**:
  ```json
  {
    "version": "2.0.0",
    "defaults": {
      "queenType": "strategic",
      "maxWorkers": 8,
      "consensusAlgorithm": "majority",
      "memorySize": 100,
      "autoScale": true,
      "encryption": false
    },
    "mcpTools": {
      "enabled": true,
      "parallel": true,
      "timeout": 60000
    }
  }
  ```
- **AWS Consideration**: Auto-scaling and worker configuration should map to ECS/Fargate task definitions

### 2. API Configuration

#### OpenAPI Specification
- **Location**: `/mistersmith-api.json`
- **Type**: OpenAPI 3.0.3 spec
- **Key Settings**:
  - Server URL: `http://localhost:8080`
  - API endpoints for agents, tasks, system health, discoveries
  - No authentication/authorization defined
- **AWS Consideration**: 
  - Need to add AWS API Gateway integration
  - Add AWS Cognito/IAM authentication schemes
  - Update server URLs for AWS endpoints

### 3. Monitoring Configuration

#### OpenTelemetry Collector
- **Location**: `/mistersmith-monitoring-ui/otel-collector-config.yaml`
- **Type**: YAML configuration
- **Key Settings**:
  - OTLP receivers on ports 4317 (gRPC) and 4318 (HTTP)
  - CORS configuration for multiple origins
  - Exporters: Jaeger, Prometheus, Debug, File
  - Memory limits: 256 MiB with 64 MiB spike
- **AWS Consideration**: 
  - Replace with AWS X-Ray exporter
  - Use CloudWatch Logs exporter
  - Configure AWS-specific resource attributes

#### Prometheus Configuration
- **Location**: `/mistersmith-monitoring-ui/prometheus.yml`
- **Type**: YAML configuration
- **Key Settings**:
  - Scrape interval: 15s
  - Multiple job configurations
  - Environment label: 'development'
- **AWS Consideration**: 
  - Replace with Amazon Managed Prometheus
  - Configure remote write to AMP
  - Update service discovery for ECS/EKS

### 4. Hard-coded Configuration in Code

#### Connection Configuration (Rust)
- **Location**: `/src/transport/connection.rs`
- **Type**: Rust struct with defaults
- **Key Settings**:
  ```rust
  ConnectionConfig {
      ping_interval: Duration::from_secs(30),
      max_reconnects: Some(10),
      connect_timeout: Duration::from_secs(10),
      health_check_interval: Duration::from_secs(60),
  }
  ```
- **AWS Consideration**: 
  - Move to AWS Systems Manager Parameter Store
  - Use environment variables for different environments
  - Configure based on AWS network characteristics

## Environment Variables Analysis

### Current State
- **No .env files found** in the repository
- **Minimal environment variable usage** in code (only in tests)
- **Hard-coded values** throughout the codebase

### Required Environment Variables for AWS

```bash
# AWS Configuration
AWS_REGION=us-east-1
AWS_ACCOUNT_ID=123456789012

# NATS Configuration
NATS_URL=nats://nats-nlb.internal:4222
NATS_CLUSTER_ID=mistersmith-cluster
NATS_CLIENT_ID=mistersmith-${INSTANCE_ID}

# Service Configuration
SERVICE_PORT=8080
METRICS_PORT=9090
HEALTH_CHECK_PATH=/system/health

# Database Configuration
DATABASE_URL=postgres://username:password@rds-endpoint:5432/mistersmith
DATABASE_POOL_SIZE=20

# Monitoring Configuration
OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
OTEL_SERVICE_NAME=mistersmith
OTEL_RESOURCE_ATTRIBUTES=deployment.environment=production

# Feature Flags
ENABLE_ENCRYPTION=true
ENABLE_AUTO_SCALING=true
MAX_WORKERS=16
MEMORY_SIZE_MB=200
```

## Secrets Management

### Current State
- **No secrets management** implemented
- **No encryption** for sensitive data (encryption: false in config)
- **Database credentials** would be hard-coded

### AWS Secrets Manager Integration

```yaml
# Required secrets in AWS Secrets Manager
secrets:
  - name: mistersmith/database/credentials
    content:
      username: postgres
      password: <generated>
      host: <rds-endpoint>
      port: 5432
      
  - name: mistersmith/nats/credentials
    content:
      username: nats_user
      password: <generated>
      
  - name: mistersmith/api/keys
    content:
      jwt_secret: <generated>
      api_key: <generated>
```

## Configuration Validation

### Missing Validation
1. No schema validation for JSON configurations
2. No environment variable validation
3. No configuration health checks

### Recommended Validation Commands

```bash
# Configuration validation script
#!/bin/bash

# Validate required environment variables
required_vars=(
    "AWS_REGION"
    "NATS_URL"
    "DATABASE_URL"
    "SERVICE_PORT"
)

for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        echo "ERROR: Required environment variable $var is not set"
        exit 1
    fi
done

# Validate configuration files
jsonschema -i config.json config-schema.json
yamllint otel-collector-config.yaml
```

## AWS-Compatible Configuration Recommendations

### 1. Use AWS Systems Manager Parameter Store
```rust
// Add AWS SDK dependency
// aws-config = "1.0"
// aws-sdk-ssm = "1.0"

async fn load_config_from_parameter_store() -> Result<Config> {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_ssm::Client::new(&config);
    
    // Load parameters by path
    let params = client
        .get_parameters_by_path()
        .path("/mistersmith/prod/")
        .with_decryption(true)
        .send()
        .await?;
    
    // Convert to Config struct
    Config::from_parameters(params)
}
```

### 2. Implement Configuration Layers
```rust
// Priority order:
// 1. Environment variables (highest)
// 2. AWS Parameter Store
// 3. Configuration files
// 4. Default values (lowest)

pub struct ConfigLoader {
    sources: Vec<Box<dyn ConfigSource>>,
}

impl ConfigLoader {
    pub async fn load() -> Result<Config> {
        let mut config = Config::default();
        
        // Load from each source in order
        for source in &self.sources {
            config.merge(source.load().await?);
        }
        
        config.validate()?;
        Ok(config)
    }
}
```

### 3. Add Configuration Hot-Reloading
```rust
// Watch for configuration changes
pub struct ConfigWatcher {
    parameter_store_client: aws_sdk_ssm::Client,
    reload_interval: Duration,
}

impl ConfigWatcher {
    pub async fn watch(&self, callback: impl Fn(Config)) {
        let mut interval = tokio::time::interval(self.reload_interval);
        
        loop {
            interval.tick().await;
            
            if let Ok(new_config) = self.load_latest().await {
                callback(new_config);
            }
        }
    }
}
```

## Configuration Migration Checklist

- [ ] Create configuration schema definitions
- [ ] Implement environment variable loading
- [ ] Add AWS SDK dependencies
- [ ] Create Parameter Store integration
- [ ] Implement Secrets Manager client
- [ ] Add configuration validation
- [ ] Create configuration hot-reloading
- [ ] Document all configuration options
- [ ] Create environment-specific configs
- [ ] Add configuration health checks

## Next Steps

1. **Create configuration module** in Rust code
2. **Define configuration schemas** for validation
3. **Implement AWS Parameter Store client**
4. **Add Secrets Manager integration**
5. **Create configuration documentation**
6. **Set up configuration testing**

## Dependencies

- AWS SDK for Rust (Parameter Store, Secrets Manager)
- Configuration validation libraries
- Schema definition tools

## Risks

1. **Hard-coded values** throughout codebase need systematic replacement
2. **No current secrets management** poses security risk
3. **Missing validation** could lead to runtime failures
4. **No configuration versioning** makes rollbacks difficult

## Completed Tasks

- [x] Identified all configuration files
- [x] Extracted hard-coded configurations
- [x] Analyzed environment variable usage
- [x] Documented AWS-compatible patterns
- [x] Created migration recommendations