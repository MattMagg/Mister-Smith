# Configuration Extraction Report

## Hard-coded Values Found

### 1. Network Addresses
| Location | Value | Purpose | AWS Replacement |
|----------|-------|---------|-----------------|
| `/src/collaboration/*.rs` | `nats://localhost:4222` | NATS connection | Environment variable `NATS_URL` |
| `/mistersmith-api.json` | `http://localhost:8080` | API server | API Gateway URL |
| `/otel-collector-config.yaml` | `0.0.0.0:4317` | OTLP gRPC | ECS task definition |
| `/otel-collector-config.yaml` | `0.0.0.0:4318` | OTLP HTTP | ECS task definition |

### 2. Port Numbers
| Port | Service | Configuration Location | AWS Service |
|------|---------|----------------------|-------------|
| 4222 | NATS | Hard-coded in source | NLB + ECS Service |
| 8080 | API Server | OpenAPI spec | ALB + ECS Service |
| 9090 | Prometheus | prometheus.yml | Amazon Managed Prometheus |
| 5173 | UI Dev Server | CORS config | CloudFront + S3 |
| 4317 | OTLP gRPC | otel-collector | AWS Distro for OpenTelemetry |
| 8888 | Collector Metrics | otel-collector | CloudWatch Metrics |

### 3. Configuration Defaults
```rust
// Connection timeouts
ping_interval: Duration::from_secs(30)
max_reconnects: Some(10)
connect_timeout: Duration::from_secs(10)
health_check_interval: Duration::from_secs(60)

// Hive mind defaults
maxWorkers: 8
consensusAlgorithm: "majority"
memorySize: 100
autoScale: true
encryption: false
```

## Environment Variable Schema

```yaml
# Required Environment Variables
environment:
  # Core Service Configuration
  SERVICE_NAME: 
    type: string
    default: "mistersmith"
    description: "Service identifier for logging and metrics"
  
  SERVICE_PORT:
    type: integer
    default: 8080
    description: "HTTP API server port"
  
  # NATS Configuration
  NATS_URL:
    type: string
    required: true
    description: "NATS server connection URL"
    example: "nats://nats.service.local:4222"
  
  NATS_CLUSTER_ID:
    type: string
    default: "mistersmith-cluster"
    description: "NATS cluster identifier"
  
  NATS_CLIENT_ID:
    type: string
    default: "${SERVICE_NAME}-${INSTANCE_ID}"
    description: "Unique NATS client identifier"
  
  # Connection Settings
  NATS_PING_INTERVAL_SECS:
    type: integer
    default: 30
    description: "NATS ping interval in seconds"
  
  NATS_MAX_RECONNECTS:
    type: integer
    default: 10
    description: "Maximum NATS reconnection attempts"
  
  NATS_CONNECT_TIMEOUT_SECS:
    type: integer
    default: 10
    description: "NATS connection timeout in seconds"
  
  # Agent Configuration
  MAX_WORKERS:
    type: integer
    default: 8
    description: "Maximum number of worker agents"
  
  CONSENSUS_ALGORITHM:
    type: string
    default: "majority"
    enum: ["majority", "unanimous", "quorum"]
    description: "Consensus algorithm for agent decisions"
  
  MEMORY_SIZE_MB:
    type: integer
    default: 100
    description: "Agent memory size in megabytes"
  
  # Feature Flags
  ENABLE_AUTO_SCALE:
    type: boolean
    default: true
    description: "Enable automatic agent scaling"
  
  ENABLE_ENCRYPTION:
    type: boolean
    default: false
    description: "Enable data encryption"
  
  # Monitoring Configuration
  METRICS_PORT:
    type: integer
    default: 9090
    description: "Prometheus metrics port"
  
  OTEL_EXPORTER_OTLP_ENDPOINT:
    type: string
    default: "http://localhost:4317"
    description: "OpenTelemetry collector endpoint"
  
  OTEL_SERVICE_NAME:
    type: string
    default: "${SERVICE_NAME}"
    description: "Service name for telemetry"
  
  OTEL_RESOURCE_ATTRIBUTES:
    type: string
    default: "deployment.environment=development"
    description: "Additional resource attributes"
  
  # AWS Specific
  AWS_REGION:
    type: string
    required: true
    description: "AWS region for deployment"
  
  AWS_ACCOUNT_ID:
    type: string
    required: true
    description: "AWS account identifier"
```

## Configuration Loading Strategy

```rust
use std::env;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_name: String,
    pub service_port: u16,
    pub nats: NatsConfig,
    pub agents: AgentConfig,
    pub monitoring: MonitoringConfig,
    pub aws: AwsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    pub url: String,
    pub cluster_id: String,
    pub client_id: String,
    pub ping_interval_secs: u64,
    pub max_reconnects: Option<usize>,
    pub connect_timeout_secs: u64,
}

impl ServiceConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            service_name: env::var("SERVICE_NAME")
                .unwrap_or_else(|_| "mistersmith".to_string()),
            service_port: env::var("SERVICE_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            nats: NatsConfig {
                url: env::var("NATS_URL")
                    .expect("NATS_URL is required"),
                cluster_id: env::var("NATS_CLUSTER_ID")
                    .unwrap_or_else(|_| "mistersmith-cluster".to_string()),
                client_id: env::var("NATS_CLIENT_ID")
                    .unwrap_or_else(|_| {
                        format!("{}-{}", 
                            env::var("SERVICE_NAME").unwrap_or_else(|_| "mistersmith".to_string()),
                            env::var("INSTANCE_ID").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string())
                        )
                    }),
                ping_interval_secs: env::var("NATS_PING_INTERVAL_SECS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()?,
                max_reconnects: env::var("NATS_MAX_RECONNECTS")
                    .ok()
                    .and_then(|s| s.parse().ok()),
                connect_timeout_secs: env::var("NATS_CONNECT_TIMEOUT_SECS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()?,
            },
            agents: AgentConfig::from_env()?,
            monitoring: MonitoringConfig::from_env()?,
            aws: AwsConfig::from_env()?,
        })
    }
}
```

## AWS Parameter Store Structure

```
/mistersmith/
├── common/
│   ├── service_name
│   ├── environment
│   └── version
├── dev/
│   ├── nats/
│   │   ├── url
│   │   ├── cluster_id
│   │   └── client_id_prefix
│   ├── agents/
│   │   ├── max_workers
│   │   ├── consensus_algorithm
│   │   └── memory_size_mb
│   └── features/
│       ├── enable_auto_scale
│       └── enable_encryption
└── prod/
    ├── nats/
    │   ├── url
    │   ├── cluster_id
    │   └── client_id_prefix
    ├── agents/
    │   ├── max_workers
    │   ├── consensus_algorithm
    │   └── memory_size_mb
    ├── features/
    │   ├── enable_auto_scale
    │   └── enable_encryption
    └── monitoring/
        ├── metrics_port
        ├── otel_endpoint
        └── otel_attributes
```

## Configuration Validation Rules

```rust
use validator::{Validate, ValidationError};

#[derive(Debug, Validate)]
pub struct ConfigValidator {
    #[validate(range(min = 1024, max = 65535))]
    pub service_port: u16,
    
    #[validate(url)]
    pub nats_url: String,
    
    #[validate(range(min = 1, max = 1000))]
    pub max_workers: u32,
    
    #[validate(range(min = 10, max = 10000))]
    pub memory_size_mb: u32,
    
    #[validate(custom = "validate_consensus_algorithm")]
    pub consensus_algorithm: String,
}

fn validate_consensus_algorithm(algorithm: &str) -> Result<(), ValidationError> {
    match algorithm {
        "majority" | "unanimous" | "quorum" => Ok(()),
        _ => Err(ValidationError::new("invalid_consensus_algorithm")),
    }
}
```

## Migration Steps

1. **Phase 1: Add Environment Variable Support**
   - Create config module
   - Add env var loading
   - Keep defaults as fallback

2. **Phase 2: Add AWS Parameter Store**
   - Add AWS SDK dependency
   - Create parameter store client
   - Implement caching layer

3. **Phase 3: Add Secrets Manager**
   - Integrate for sensitive values
   - Implement rotation support
   - Add encryption at rest

4. **Phase 4: Configuration Hot-Reload**
   - Watch for parameter changes
   - Implement graceful updates
   - Add change notifications

## Security Considerations

1. **Secrets Rotation**
   - Database passwords: 30 days
   - API keys: 90 days
   - JWT secrets: 180 days

2. **Access Control**
   - Use IAM roles for parameter access
   - Separate read/write permissions
   - Audit configuration access

3. **Encryption**
   - Enable encryption for Parameter Store
   - Use KMS for sensitive values
   - Encrypt configuration in transit