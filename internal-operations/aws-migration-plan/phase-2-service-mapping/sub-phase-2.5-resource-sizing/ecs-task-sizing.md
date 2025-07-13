# ECS Task Resource Sizing Calculations

## Sizing Methodology

Based on MisterSmith's architecture and performance requirements, we calculate ECS task resources using:
- Agent concurrency requirements
- Message processing throughput
- Memory usage patterns
- Connection handling capacity

## Detailed Task Definitions

### 1. Supervisor Service (Control Plane)

#### Resource Calculation
```
Base Requirements:
- Coordination overhead: 256 MB
- Agent registry (per 100 agents): 100 MB
- Message routing table: 128 MB
- Metrics collection: 128 MB
- Buffer/Cache: 256 MB

CPU Requirements:
- Base coordination: 0.25 vCPU
- Per 50 agents managed: +0.5 vCPU
- Message routing: 0.25 vCPU
```

#### Task Definition Sizes
```yaml
# Development (1-10 agents)
supervisor-dev:
  cpu: 512     # 0.5 vCPU
  memory: 1024 # 1 GB
  environment:
    MAX_AGENTS: 10
    ENABLE_DEBUG: true

# Staging (10-50 agents)  
supervisor-staging:
  cpu: 1024    # 1 vCPU
  memory: 2048 # 2 GB
  environment:
    MAX_AGENTS: 50
    ENABLE_METRICS: true

# Production Small (50-100 agents)
supervisor-prod-small:
  cpu: 2048    # 2 vCPU
  memory: 4096 # 4 GB
  environment:
    MAX_AGENTS: 100
    ENABLE_HA: true

# Production Large (100-500 agents)
supervisor-prod-large:
  cpu: 4096    # 4 vCPU
  memory: 8192 # 8 GB
  environment:
    MAX_AGENTS: 500
    ENABLE_CLUSTERING: true
```

### 2. Worker Service (Agent Execution)

#### Resource Calculation
```
Per Agent Requirements:
- Base runtime: 128 MB
- Working memory: 256 MB
- Model cache: 128 MB
- Total per agent: 512 MB

CPU per Agent:
- Idle: 0.1 vCPU
- Active: 0.25-0.5 vCPU
- Peak: 1.0 vCPU
```

#### Container Packing Strategy
```yaml
# Small Worker (1-4 agents)
worker-small:
  cpu: 1024    # 1 vCPU
  memory: 2048 # 2 GB
  agents_per_container: 4
  
# Medium Worker (5-10 agents)
worker-medium:
  cpu: 2048    # 2 vCPU
  memory: 5120 # 5 GB  
  agents_per_container: 10

# Large Worker (11-20 agents)
worker-large:
  cpu: 4096     # 4 vCPU
  memory: 10240 # 10 GB
  agents_per_container: 20

# XLarge Worker (21-50 agents)
worker-xlarge:
  cpu: 8192     # 8 vCPU
  memory: 25600 # 25 GB
  agents_per_container: 50
```

#### Scaling Formula
```python
# Calculate required worker containers
def calculate_worker_containers(total_agents, container_size):
    containers = math.ceil(total_agents / container_size)
    return {
        'containers': containers,
        'total_cpu': containers * cpu_per_container[container_size],
        'total_memory': containers * memory_per_container[container_size],
        'efficiency': (total_agents / (containers * container_size)) * 100
    }

# Example: 1000 agents with large containers
# Result: 50 containers, 200 vCPU, 500 GB memory, 100% efficiency
```

### 3. Gateway Service (API/WebSocket/gRPC)

#### Resource Calculation
```
Connection Handling:
- Per 100 SSE connections: 100 MB
- Per 100 WebSocket connections: 150 MB
- Per 100 gRPC streams: 200 MB
- Connection state tracking: 256 MB base

CPU Requirements:
- Base API handling: 0.5 vCPU
- Per 1000 connections: +1.5 vCPU
- TLS termination: +0.5 vCPU
```

#### Task Definitions
```yaml
# Low Traffic (< 100 connections)
gateway-low:
  cpu: 512     # 0.5 vCPU
  memory: 1024 # 1 GB
  environment:
    MAX_CONNECTIONS: 100
    ENABLE_RATE_LIMIT: true

# Medium Traffic (100-1000 connections)
gateway-medium:
  cpu: 2048    # 2 vCPU
  memory: 4096 # 4 GB
  environment:
    MAX_CONNECTIONS: 1000
    CONNECTION_TIMEOUT: 300

# High Traffic (1000-5000 connections)
gateway-high:
  cpu: 4096    # 4 vCPU
  memory: 8192 # 8 GB
  environment:
    MAX_CONNECTIONS: 5000
    ENABLE_CONNECTION_POOLING: true

# Ultra High Traffic (5000+ connections)
gateway-ultra:
  cpu: 8192     # 8 vCPU
  memory: 16384 # 16 GB
  environment:
    MAX_CONNECTIONS: 10000
    ENABLE_HORIZONTAL_SCALING: true
```

### 4. Monitoring Sidecar

```yaml
# OTEL Collector Sidecar
otel-sidecar:
  cpu: 256      # 0.25 vCPU
  memory: 512   # 512 MB
  essential: false
  environment:
    OTEL_EXPORTER_OTLP_ENDPOINT: "http://localhost:4317"
    METRICS_INTERVAL: 60
```

## Resource Utilization Targets

### CPU Utilization
- Target: 70% average
- Scale-up threshold: 80%
- Scale-down threshold: 50%
- Burst capacity: 30%

### Memory Utilization
- Target: 80% average
- Scale-up threshold: 90%
- Scale-down threshold: 60%
- OOM buffer: 10%

## Fargate vs EC2 Comparison

### Fargate (Recommended for most services)
```yaml
Advantages:
- No cluster management
- Per-second billing
- Automatic scaling
- Built-in security

Best for:
- Supervisor Service
- Gateway Service
- Batch processing
```

### EC2 (Consider for worker service)
```yaml
Advantages:
- Better cost at scale
- GPU access available
- Custom AMIs
- Spot instance support

Best for:
- Worker Service (large scale)
- ML model serving
- Cost optimization
```

## Cost Optimization

### 1. Right-Sizing Strategy
```python
def optimize_container_size(avg_cpu, avg_memory, peak_factor=1.5):
    # Size for average load with peak headroom
    optimal_cpu = math.ceil(avg_cpu * peak_factor / 256) * 256
    optimal_memory = math.ceil(avg_memory * peak_factor / 512) * 512
    return optimal_cpu, optimal_memory
```

### 2. Savings Plans
- Compute Savings Plan: 20-30% discount
- Fargate Compute: Best for variable workloads
- EC2 Instance: Best for stable workloads

### 3. Spot Instances (Workers only)
```yaml
spot_config:
  allocation_strategy: "capacity-optimized"
  instance_types:
    - m5.large
    - m5a.large
    - m6i.large
  max_spot_price: "on-demand * 0.7"
  interruption_behavior: "terminate"
```

## Performance Testing Scenarios

### 1. Load Test Configuration
```yaml
scenarios:
  - name: "Base Load"
    agents: 10
    connections: 100
    duration: 1h
    
  - name: "Standard Load"
    agents: 100
    connections: 1000
    duration: 4h
    
  - name: "Peak Load"
    agents: 500
    connections: 5000
    duration: 1h
    
  - name: "Stress Test"
    agents: 1000
    connections: 10000
    duration: 30m
```

### 2. Resource Monitoring
```bash
# CloudWatch metrics to monitor
aws cloudwatch get-metric-statistics \
  --namespace AWS/ECS \
  --metric-name CPUUtilization \
  --dimensions Name=ServiceName,Value=mistersmith-supervisor \
  --statistics Average,Maximum \
  --start-time 2024-01-01T00:00:00Z \
  --end-time 2024-01-01T01:00:00Z \
  --period 300
```