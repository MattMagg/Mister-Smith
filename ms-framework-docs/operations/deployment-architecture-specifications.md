---
title: Deployment Architecture Specifications - Revised
type: note
permalink: revision-swarm/operations/deployment-architecture-specifications-revised
---

# Deployment Architecture Specifications - Revised
## Multi-Agent Platform Deployment Patterns

### Executive Summary

This document provides deployment architecture patterns for multi-agent platforms, focusing on containerization strategies, orchestration patterns, and scalable infrastructure designs without implementation-specific details.

### 1. Container Architecture Patterns

#### 1.1 Base Container Strategy Pattern
```pseudocode
PATTERN MultiStageContainer:
    STAGES:
        build_stage:
            setup_build_environment()
            copy_source_code()
            compile_application()
            run_tests()
        
        runtime_stage:
            create_minimal_image()
            copy_built_artifacts()
            configure_runtime()
            expose_service_ports()
            define_entry_point()
```

#### 1.2 Agent Container Types Pattern
```pseudocode
PATTERN AgentContainerTypes:
    CONTAINER_CATEGORIES:
        orchestrator_container:
            purpose: "coordination and management"
            resource_profile: "moderate"
            exposed_services: ["management_api", "coordination_rpc"]
        
        worker_container:
            purpose: "task execution"
            resource_profile: "variable"
            exposed_services: ["worker_api"]
        
        messaging_container:
            purpose: "inter-agent communication"
            resource_profile: "minimal"
            exposed_services: ["message_bus", "event_stream"]
```

#### 1.3 Sidecar Pattern Implementation
```pseudocode
PATTERN SidecarArchitecture:
    SIDECAR_TYPES:
        coordination_proxy:
            intercept_agent_communication()
            apply_routing_rules()
            enforce_policies()
        
        metrics_collector:
            gather_agent_metrics()
            aggregate_locally()
            export_to_backend()
        
        security_proxy:
            handle_authentication()
            enforce_authorization()
            audit_access()
```

### 2. Orchestration Patterns

#### 2.1 DAG vs FSM Orchestration Pattern
```pseudocode
PATTERN OrchestrationModelSelection:
    DAG_PATTERN:
        characteristics:
            - "explicit dependency graph"
            - "batch processing oriented"
            - "visual representation"
            - "static or dynamic construction"
        
        use_cases:
            - "data pipelines"
            - "batch workflows"
            - "ETL processes"
            - "parallel task execution"
        
        implementation:
            static_dag:
                define_all_tasks()
                explicit_dependencies()
                compile_time_validation()
            
            dynamic_dag:
                runtime_task_generation()
                inferred_dependencies()
                flexible_execution_paths()
    
    FSM_PATTERN:
        characteristics:
            - "state-based control flow"
            - "complex decision logic"
            - "compensation support"
            - "dynamic transitions"
        
        use_cases:
            - "order processing"
            - "approval workflows"
            - "multi-step transactions"
            - "error recovery flows"
        
        implementation:
            state_machine:
                define_states()
                transition_rules()
                compensation_stack()
                rollback_handlers()
```

#### 2.2 Workflow Retry Pattern
```pseudocode
PATTERN RetryStrategies:
    RETRY_POLICIES:
        exponential_backoff:
            initial_delay: "1s"
            max_delay: "300s"
            backoff_factor: 2
            jitter: "random_0_to_1"
            
        linear_backoff:
            fixed_delay: "30s"
            max_attempts: 3
            
        circuit_breaker:
            failure_threshold: 5
            recovery_timeout: "60s"
            half_open_attempts: 3
    
    RETRY_IMPLEMENTATION:
        task_level_retry:
            configure_per_task_policy()
            track_attempt_count()
            handle_final_failure()
            
        workflow_level_retry:
            checkpoint_state()
            resume_from_failure()
            compensate_completed_tasks()
            
        dead_letter_handling:
            capture_failed_tasks()
            manual_intervention_queue()
            retry_with_modifications()
```

#### 2.3 Namespace Organization Pattern
```pseudocode
PATTERN NamespaceArchitecture:
    NAMESPACE_CATEGORIES:
        system_namespace:
            purpose: "core platform components"
            isolation_level: "strict"
            components: ["orchestrators", "messaging", "configuration"]
        
        workload_namespace:
            purpose: "agent workloads"
            isolation_level: "moderate"
            components: ["agent_pools", "task_queues"]
        
        data_namespace:
            purpose: "persistence layer"
            isolation_level: "strict"
            components: ["databases", "caches", "storage"]
        
        monitoring_namespace:
            purpose: "observability stack"
            isolation_level: "moderate"
            components: ["metrics", "logs", "traces"]
```

#### 2.4 Deployment Topology Pattern
```pseudocode
PATTERN DeploymentTopology:
    ORCHESTRATOR_DEPLOYMENT:
        replica_strategy: "odd_number_for_consensus"
        distribution: "across_availability_zones"
        update_strategy: "rolling_with_zero_downtime"
        health_checks: ["liveness", "readiness", "startup"]
    
    WORKER_DEPLOYMENT:
        replica_strategy: "dynamic_based_on_load"
        distribution: "maximize_spread"
        update_strategy: "blue_green_or_canary"
        scaling_policy: "horizontal_and_vertical"
```

#### 2.5 Service Discovery Pattern
```pseudocode
PATTERN ServiceDiscovery:
    DISCOVERY_METHODS:
        dns_based:
            register_service_endpoints()
            resolve_by_service_name()
            handle_endpoint_changes()
        
        registry_based:
            register_with_metadata()
            query_by_attributes()
            watch_for_updates()
        
        mesh_based:
            automatic_sidecar_injection()
            intelligent_routing()
            circuit_breaking()
```

### 3. Scaling Architecture Patterns

#### 3.1 Horizontal Scaling Pattern
```pseudocode
PATTERN HorizontalScaling:
    SCALING_TRIGGERS:
        resource_based:
            monitor_cpu_usage()
            monitor_memory_usage()
            monitor_network_throughput()
        
        application_based:
            monitor_queue_depth()
            monitor_response_time()
            monitor_error_rate()
        
        custom_metrics:
            monitor_business_metrics()
            monitor_agent_specific_metrics()
    
    SCALING_BEHAVIOR:
        scale_up_policy:
            stabilization_window
            maximum_scale_rate
            scale_up_threshold
        
        scale_down_policy:
            cooldown_period
            gradual_reduction
            minimum_instances
```

#### 3.2 Cluster Autoscaling Pattern
```pseudocode
PATTERN ClusterAutoscaling:
    NODE_GROUPS:
        control_plane_nodes:
            purpose: "orchestration"
            scaling: "vertical_preferred"
            placement: "dedicated_hosts"
        
        worker_nodes:
            purpose: "agent_workloads"
            scaling: "horizontal_preferred"
            placement: "spot_instances_allowed"
        
        data_nodes:
            purpose: "persistence"
            scaling: "careful_horizontal"
            placement: "storage_optimized"
```

#### 3.3 Resource Allocation Pattern
```pseudocode
PATTERN ResourceAllocation:
    RESOURCE_TIERS:
        minimal_tier:
            cpu_allocation: "fractional"
            memory_allocation: "constrained"
            priority: "best_effort"
        
        standard_tier:
            cpu_allocation: "guaranteed_minimum"
            memory_allocation: "reserved"
            priority: "normal"
        
        premium_tier:
            cpu_allocation: "dedicated"
            memory_allocation: "exclusive"
            priority: "high"
```

#### 3.4 Orchestration Autoscale Pattern
```pseudocode
PATTERN OrchestrationAutoscale:
    SCALING_STRATEGIES:
        queue_based_scaling:
            monitor_queue_depth()
            calculate_processing_rate()
            scale_workers_proportionally()
            consider_message_age()
            
        event_driven_scaling:
            monitor_event_streams()
            predict_load_patterns()
            preemptive_scaling()
            custom_metric_triggers()
            
        schedule_based_scaling:
            define_time_patterns()
            pre_scale_for_known_loads()
            combine_with_reactive_scaling()
            handle_timezone_variations()
    
    ORCHESTRATOR_SCALING:
        task_distribution_model:
            worker_pool_sizing:
                min_workers_per_queue: 2
                max_workers_per_queue: 100
                scale_increment: "10%_or_1_worker"
                
            task_assignment:
                round_robin_distribution()
                load_based_assignment()
                affinity_based_routing()
                
        resource_based_autoscale:
            cpu_threshold: "70%_sustained_60s"
            memory_threshold: "80%_sustained_30s"
            queue_depth_threshold: "100_pending_tasks"
            
        failure_aware_scaling:
            monitor_task_failures()
            detect_poison_messages()
            isolate_failing_workers()
            scale_healthy_workers()
```

### 4. Package Management Patterns

#### 4.1 Helm Chart Structure Pattern
```pseudocode
PATTERN ChartOrganization:
    STRUCTURE:
        chart_metadata:
            define_chart_properties()
            specify_dependencies()
            set_version_constraints()
        
        templates:
            deployment_templates()
            service_templates()
            configuration_templates()
            security_templates()
        
        values:
            default_values()
            environment_overrides()
            secret_references()
```

#### 4.2 Configuration Management Pattern
```pseudocode
PATTERN ConfigurationHierarchy:
    LEVELS:
        base_configuration:
            common_settings()
            default_behaviors()

        environment_configuration:
            tier_specific_settings()
            resource_adjustments()

        runtime_configuration:
            dynamic_overrides()
            feature_toggles()
```

#### 4.3 Claude-CLI Configuration Pattern
```pseudocode
PATTERN ClaudeCLIConfiguration:
    PARALLEL_EXECUTION_SETTINGS:
        environment_variables:
            CLAUDE_PARALLEL_DEFAULT: "default_fan_out_per_task_plan"
            # Controls default number of parallel agents spawned
            # when --parallel flag is not explicitly specified

        deployment_configuration:
            parallel_agent_limits:
                min_parallel_agents: 1
                max_parallel_agents: 50
                default_parallel_agents: 4

            resource_allocation:
                per_agent_cpu_limit: "0.5"
                per_agent_memory_limit: "512Mi"
                total_parallel_cpu_budget: "4.0"

        integration_settings:
            output_parsing_enabled: true
            nats_subject_mapping: true
            observability_integration: true
            span_tagging_per_agent: true
```

### 5. Network Architecture Patterns

#### 5.1 Network Segmentation Pattern
```pseudocode
PATTERN NetworkSegmentation:
    SEGMENTS:
        control_plane_network:
            purpose: "management_traffic"
            isolation: "strict"
            encryption: "required"
        
        data_plane_network:
            purpose: "agent_communication"
            isolation: "moderate"
            encryption: "optional"
        
        ingress_network:
            purpose: "external_access"
            isolation: "dmz_pattern"
            encryption: "tls_required"
```

#### 5.2 Service Mesh Pattern
```pseudocode
PATTERN ServiceMesh:
    CAPABILITIES:
        traffic_management:
            intelligent_routing()
            load_balancing()
            circuit_breaking()
            retry_logic:
                per_service_configuration()
                backoff_algorithms()
                retry_budgets()
                idempotency_headers()
        
        security:
            mutual_tls()
            authorization_policies()
            encryption_in_transit()
        
        observability:
            distributed_tracing()
            metrics_collection()
            traffic_visualization()
    
    ADVANCED_RETRY_CONFIGURATION:
        retry_conditions:
            retriable_status_codes: [502, 503, 504]
            retriable_headers: ["x-retry-after"]
            connect_failure: true
            reset_before_request: true
            
        retry_budgets:
            percentage_allowed: "20%_of_requests"
            min_retry_concurrency: 10
            ttl_seconds: 10
            
        backoff_configuration:
            base_interval: "25ms"
            max_interval: "250ms"
            backoff_algorithm: "exponential_with_jitter"
```

### 6. Deployment Pipeline Patterns

#### 6.1 GitOps Pattern
```pseudocode
PATTERN GitOpsDeployment:
    WORKFLOW:
        source_of_truth: "git_repository"
        
        change_process:
            propose_change_via_pr()
            automated_validation()
            approval_workflow()
            automated_deployment()
        
        reconciliation:
            continuous_monitoring()
            drift_detection()
            automatic_correction()
```

#### 6.2 Progressive Delivery Pattern
```pseudocode
PATTERN ProgressiveDelivery:
    STAGES:
        canary_release:
            deploy_to_small_subset()
            monitor_key_metrics()
            automated_analysis()
            promotion_decision()
        
        feature_flags:
            gradual_enablement()
            user_segmentation()
            instant_rollback()
        
        blue_green:
            parallel_environments()
            instant_switchover()
            rollback_capability()
```

### 7. Multi-Environment Strategy Pattern

#### 7.1 Environment Tiers Pattern
```pseudocode
PATTERN EnvironmentTiers:
    TIER_DEFINITIONS:
        tier_1_experimental:
            purpose: "development_testing"
            scale: "minimal"
            data: "synthetic"
            stability: "volatile"
        
        tier_2_validation:
            purpose: "integration_testing"
            scale: "moderate"
            data: "anonymized"
            stability: "stable"
        
        tier_3_operational:
            purpose: "live_operations"
            scale: "full"
            data: "production"
            stability: "highly_stable"
```

#### 7.2 Environment Promotion Pattern
```pseudocode
PATTERN EnvironmentPromotion:
    PROMOTION_FLOW:
        build_artifacts()
        deploy_to_tier_1()
        run_tier_1_validation()
        promote_to_tier_2()
        run_integration_tests()
        gate_checks()
        promote_to_tier_3()
        monitor_deployment()
```

### 8. High Availability Patterns

#### 8.1 Multi-Region Deployment Pattern
```pseudocode
PATTERN MultiRegionDeployment:
    TOPOLOGY:
        active_active:
            all_regions_serve_traffic()
            data_replication_async()
            conflict_resolution()
        
        active_passive:
            primary_region_active()
            standby_regions_ready()
            failover_automation()
```

#### 8.2 Disaster Recovery Pattern
```pseudocode
PATTERN DisasterRecovery:
    COMPONENTS:
        backup_strategy:
            continuous_backups()
            point_in_time_recovery()
            geographic_distribution()
        
        recovery_procedures:
            automated_failover()
            data_restoration()
            service_reconstruction()
            validation_testing()
```

### 9. Security Patterns

#### 9.1 Zero Trust Architecture Pattern
```pseudocode
PATTERN ZeroTrustSecurity:
    PRINCIPLES:
        never_trust_always_verify()
        least_privilege_access()
        assume_breach_mindset()
        continuous_verification()
    
    IMPLEMENTATION:
        identity_verification()
        device_validation()
        application_authentication()
        data_encryption()
        continuous_monitoring()
```

#### 9.2 Secret Management Pattern
```pseudocode
PATTERN SecretManagement:
    SECRET_LIFECYCLE:
        generation: "automated_strong_secrets"
        storage: "encrypted_vault"
        distribution: "secure_injection"
        rotation: "automated_periodic"
        auditing: "comprehensive_logging"
```

### 10. Monitoring Integration Patterns

#### 10.1 Observability Stack Pattern
```pseudocode
PATTERN ObservabilityIntegration:
    COMPONENTS:
        metrics_pipeline:
            collection_agents()
            aggregation_layer()
            storage_backend()
            query_interface()
        
        logging_pipeline:
            log_collectors()
            processing_layer()
            indexing_system()
            search_interface()
        
        tracing_pipeline:
            trace_collectors()
            sampling_layer()
            storage_system()
            analysis_tools()
```

### 11. Cost Optimization Patterns

#### 11.1 Resource Optimization Pattern
```pseudocode
PATTERN ResourceOptimization:
    STRATEGIES:
        right_sizing:
            analyze_usage_patterns()
            adjust_allocations()
            continuous_monitoring()
        
        spot_instance_usage:
            identify_suitable_workloads()
            implement_fallback_strategy()
            cost_benefit_analysis()
        
        auto_shutdown:
            identify_idle_resources()
            schedule_based_scaling()
            on_demand_activation()
```

### 12. Implementation Guidelines

#### 12.1 Deployment Orchestration Pattern
```pseudocode
PATTERN DeploymentOrchestration:
    PHASES:
        infrastructure_preparation:
            provision_compute_resources()
            configure_networking()
            setup_storage_systems()
        
        platform_deployment:
            deploy_core_services()
            configure_messaging()
            setup_orchestration()
        
        workload_deployment:
            deploy_agent_pools()
            configure_scaling()
            enable_monitoring()
        
        validation:
            health_checks()
            integration_tests()
            performance_validation()
```

### 13. Best Practices Summary

#### 13.1 Container Best Practices
- Use multi-stage builds for optimization
- Implement proper health checks
- Follow security scanning practices
- Maintain minimal base images

#### 13.2 Orchestration Best Practices
- Implement proper resource limits
- Use anti-affinity for high availability
- Enable pod disruption budgets
- Implement graceful shutdowns

#### 13.3 Scaling Best Practices
- Define clear scaling metrics
- Implement gradual scaling policies
- Monitor scaling effectiveness
- Plan for burst capacity

#### 13.4 Security Best Practices
- Implement network policies
- Use service accounts properly
- Enable audit logging
- Regular security updates

---

## CONCRETE DEPLOYMENT TEMPLATES

### 14. Dockerfile Templates

#### 14.1 Base Orchestrator Container Template
```dockerfile
# syntax=docker/dockerfile:1
# Multi-stage Dockerfile for Mister Smith Orchestrator Agent

ARG RUST_VERSION="1.75"
ARG ALPINE_VERSION="3.19"

# Build Stage
FROM rust:${RUST_VERSION}-alpine${ALPINE_VERSION} AS builder
LABEL stage=builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    && rm -rf /var/cache/apk/*

# Create app user for security
RUN addgroup -g 1001 -S appgroup && \
    adduser -u 1001 -S appuser -G appgroup

WORKDIR /app

# Copy dependency manifests first for better caching
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build with optimizations
RUN cargo build --release --locked && \
    strip target/release/mister-smith-orchestrator

# Runtime Stage
FROM alpine:${ALPINE_VERSION}@sha256:a8560b36e8b8210634f77d9f7f9efd7ffa463e380b75e2e74aff4511df3ef88c AS runtime

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    openssl \
    && rm -rf /var/cache/apk/*

# Create app user (matching builder stage)
RUN addgroup -g 1001 -S appgroup && \
    adduser -u 1001 -S appuser -G appgroup

# Copy binary from builder stage
COPY --from=builder --chown=appuser:appgroup /app/target/release/mister-smith-orchestrator /usr/local/bin/orchestrator

# Create necessary directories
RUN mkdir -p /app/logs /app/config && \
    chown -R appuser:appgroup /app

# Switch to non-root user
USER appuser

# Set working directory
WORKDIR /app

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Environment variables
ENV RUST_LOG=info
ENV AGENT_TYPE=orchestrator
ENV HTTP_PORT=8080
ENV GRPC_PORT=9090

# Expose ports
EXPOSE 8080 9090

# Start the orchestrator
ENTRYPOINT ["/usr/local/bin/orchestrator"]
```

#### 14.2 Worker Container Template
```dockerfile
# syntax=docker/dockerfile:1
# Multi-stage Dockerfile for Mister Smith Worker Agent

ARG RUST_VERSION="1.75"
ARG ALPINE_VERSION="3.19"

# Build Stage  
FROM rust:${RUST_VERSION}-alpine${ALPINE_VERSION} AS builder
LABEL stage=builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    && rm -rf /var/cache/apk/*

WORKDIR /app

# Copy and build application
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked && \
    strip target/release/mister-smith-worker

# Runtime Stage
FROM alpine:${ALPINE_VERSION}@sha256:a8560b36e8b8210634f77d9f7f9efd7ffa463e380b75e2e74aff4511df3ef88c AS runtime

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    openssl \
    curl \
    && rm -rf /var/cache/apk/*

# Create app user
RUN addgroup -g 1001 -S appgroup && \
    adduser -u 1001 -S appuser -G appgroup

# Copy binary
COPY --from=builder --chown=appuser:appgroup /app/target/release/mister-smith-worker /usr/local/bin/worker

# Create directories
RUN mkdir -p /app/workspace /app/logs && \
    chown -R appuser:appgroup /app

USER appuser
WORKDIR /app

# Health checks
HEALTHCHECK --interval=15s --timeout=5s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

# Environment configuration
ENV RUST_LOG=info
ENV AGENT_TYPE=worker
ENV HTTP_PORT=8081
ENV WORKER_CONCURRENCY=4

EXPOSE 8081

ENTRYPOINT ["/usr/local/bin/worker"]
```

#### 14.3 Messaging Container Template  
```dockerfile
# syntax=docker/dockerfile:1
# NATS-based Messaging Container for Mister Smith

FROM nats:2.10-alpine AS runtime

# Install additional tools
RUN apk add --no-cache curl

# Create nats user and directories
RUN addgroup -g 1001 -S natsgroup && \
    adduser -u 1001 -S natsuser -G natsgroup && \
    mkdir -p /etc/nats /var/lib/nats && \
    chown -R natsuser:natsgroup /etc/nats /var/lib/nats

# Copy NATS configuration
COPY --chown=natsuser:natsgroup nats.conf /etc/nats/nats.conf

USER natsuser

# Health check for NATS
HEALTHCHECK --interval=10s --timeout=5s --start-period=10s --retries=3 \
    CMD nats server ping || exit 1

# Environment variables
ENV NATS_CONFIG_FILE=/etc/nats/nats.conf

EXPOSE 4222 8222 6222

ENTRYPOINT ["/nats-server", "-c", "/etc/nats/nats.conf"]
```

### 15. Docker Compose Specifications

#### 15.1 Base Development Stack
```yaml
# docker-compose.yml - Base Mister Smith development stack
version: '3.8'

services:
  nats:
    image: nats:2.10-alpine
    container_name: mister-smith-nats
    ports:
      - "4222:4222"
      - "8222:8222"
      - "6222:6222"
    command: ["-js", "-m", "8222"]
    healthcheck:
      test: ["CMD", "wget", "--spider", "-q", "http://localhost:8222/healthz"]
      interval: 10s
      timeout: 5s
      retries: 3
    networks:
      - mister-smith-net

  redis:
    image: redis:7-alpine
    container_name: mister-smith-redis
    ports:
      - "6379:6379"
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 3
    networks:
      - mister-smith-net

  postgres:
    image: postgres:16-alpine
    container_name: mister-smith-postgres
    environment:
      POSTGRES_DB: mister_smith
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 10s
      timeout: 5s
      retries: 3
    networks:
      - mister-smith-net

  orchestrator:
    build:
      context: .
      dockerfile: Dockerfile.orchestrator
      target: runtime
    container_name: mister-smith-orchestrator
    ports:
      - "8080:8080"
      - "9090:9090"
    environment:
      - RUST_LOG=debug
      - NATS_URL=nats://nats:4222
      - REDIS_URL=redis://redis:6379
      - DATABASE_URL=postgresql://postgres:password@postgres:5432/mister_smith
      - AGENT_TYPE=orchestrator
      - AGENT_TIER=standard
    depends_on:
      nats:
        condition: service_healthy
      redis:
        condition: service_healthy
      postgres:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "wget", "--spider", "-q", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    networks:
      - mister-smith-net

  worker:
    build:
      context: .
      dockerfile: Dockerfile.worker
      target: runtime
    environment:
      - RUST_LOG=debug
      - NATS_URL=nats://nats:4222
      - REDIS_URL=redis://redis:6379
      - ORCHESTRATOR_URL=http://orchestrator:8080
      - AGENT_TYPE=worker
      - AGENT_TIER=standard
      - WORKER_CONCURRENCY=2
    depends_on:
      orchestrator:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/health"]
      interval: 15s
      timeout: 5s
      retries: 3
    networks:
      - mister-smith-net
    deploy:
      replicas: 2

volumes:
  redis_data:
  postgres_data:

networks:
  mister-smith-net:
    driver: bridge
```

#### 15.2 Development Override Configuration
```yaml
# docker-compose.override.yml - Development-specific overrides
version: '3.8'

services:
  orchestrator:
    volumes:
      - ./src:/app/src:ro
      - ./target/debug:/app/target/debug
    environment:
      - RUST_LOG=debug
      - RUST_BACKTRACE=1
    ports:
      - "5005:5005"  # Debug port

  worker:
    volumes:
      - ./src:/app/src:ro
      - ./workspace:/app/workspace
    environment:
      - RUST_LOG=debug
      - RUST_BACKTRACE=1
      - WORKER_CONCURRENCY=1
    ports:
      - "5006:5006"  # Debug port
```

#### 15.3 Monitoring Stack Extension
```yaml
# docker-compose.monitoring.yml - Observability stack
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:v2.48.0
    container_name: mister-smith-prometheus
    ports:
      - "9091:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'
    networks:
      - mister-smith-net

  grafana:
    image: grafana/grafana:10.2.0
    container_name: mister-smith-grafana
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana_data:/var/lib/grafana
      - ./monitoring/grafana/dashboards:/etc/grafana/provisioning/dashboards:ro
      - ./monitoring/grafana/datasources:/etc/grafana/provisioning/datasources:ro
    networks:
      - mister-smith-net

  jaeger:
    image: jaegertracing/all-in-one:1.51
    container_name: mister-smith-jaeger
    ports:
      - "16686:16686"
      - "14268:14268"
    environment:
      - COLLECTOR_OTLP_ENABLED=true
    networks:
      - mister-smith-net

volumes:
  prometheus_data:
  grafana_data:
```

### 16. Kubernetes Manifest Templates

#### 16.1 Namespace Definitions
```yaml
# namespaces.yaml - Namespace organization
apiVersion: v1
kind: Namespace
metadata:
  name: mister-smith-system
  labels:
    name: mister-smith-system
    tier: system
---
apiVersion: v1
kind: Namespace
metadata:
  name: mister-smith-workload
  labels:
    name: mister-smith-workload
    tier: workload
---
apiVersion: v1
kind: Namespace
metadata:
  name: mister-smith-data
  labels:
    name: mister-smith-data
    tier: data
---
apiVersion: v1
kind: Namespace
metadata:
  name: mister-smith-monitoring
  labels:
    name: mister-smith-monitoring
    tier: monitoring
```

#### 16.2 Orchestrator Deployment Template
```yaml
# orchestrator-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mister-smith-orchestrator
  namespace: mister-smith-system
  labels:
    app: mister-smith-orchestrator
    component: orchestrator
    tier: system
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1
      maxSurge: 1
  selector:
    matchLabels:
      app: mister-smith-orchestrator
  template:
    metadata:
      labels:
        app: mister-smith-orchestrator
        component: orchestrator
        tier: system
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: mister-smith-orchestrator
      securityContext:
        runAsNonRoot: true
        runAsUser: 1001
        runAsGroup: 1001
        fsGroup: 1001
      containers:
      - name: orchestrator
        image: mister-smith/orchestrator:latest
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
          name: http
          protocol: TCP
        - containerPort: 9090
          name: grpc
          protocol: TCP
        env:
        - name: RUST_LOG
          value: "info"
        - name: AGENT_TYPE
          value: "orchestrator"
        - name: AGENT_TIER
          value: "standard"
        - name: NATS_URL
          value: "nats://nats:4222"
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: POD_NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 60
          periodSeconds: 30
          timeoutSeconds: 10
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        startupProbe:
          httpGet:
            path: /startup
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 30
        volumeMounts:
        - name: config
          mountPath: /app/config
          readOnly: true
        - name: logs
          mountPath: /app/logs
      - name: metrics-sidecar
        image: prom/node-exporter:v1.7.0
        args:
        - '--path.procfs=/host/proc'
        - '--path.sysfs=/host/sys'
        - '--collector.filesystem.mount-points-exclude=^/(sys|proc|dev|host|etc)($$|/)'
        ports:
        - containerPort: 9100
          name: metrics
        resources:
          requests:
            memory: "64Mi"
            cpu: "50m"
          limits:
            memory: "128Mi"
            cpu: "100m"
        volumeMounts:
        - name: proc
          mountPath: /host/proc
          readOnly: true
        - name: sys
          mountPath: /host/sys
          readOnly: true
      volumes:
      - name: config
        configMap:
          name: mister-smith-orchestrator-config
      - name: logs
        emptyDir: {}
      - name: proc
        hostPath:
          path: /proc
      - name: sys
        hostPath:
          path: /sys
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - mister-smith-orchestrator
              topologyKey: kubernetes.io/hostname
      tolerations:
      - key: "node.kubernetes.io/not-ready"
        operator: "Exists"
        effect: "NoExecute"
        tolerationSeconds: 300
      - key: "node.kubernetes.io/unreachable"
        operator: "Exists"
        effect: "NoExecute"
        tolerationSeconds: 300
```

#### 16.3 Worker Deployment with HPA
```yaml
# worker-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mister-smith-worker
  namespace: mister-smith-workload
  labels:
    app: mister-smith-worker
    component: worker
    tier: workload
spec:
  replicas: 2
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 0
      maxSurge: 2
  selector:
    matchLabels:
      app: mister-smith-worker
  template:
    metadata:
      labels:
        app: mister-smith-worker
        component: worker
        tier: workload
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8081"
    spec:
      serviceAccountName: mister-smith-worker
      securityContext:
        runAsNonRoot: true
        runAsUser: 1001
        runAsGroup: 1001
        fsGroup: 1001
      containers:
      - name: worker
        image: mister-smith/worker:latest
        imagePullPolicy: Always
        ports:
        - containerPort: 8081
          name: http
        env:
        - name: RUST_LOG
          value: "info"
        - name: AGENT_TYPE
          value: "worker"
        - name: AGENT_TIER
          valueFrom:
            configMapKeyRef:
              name: mister-smith-worker-config
              key: agent.tier
        - name: NATS_URL
          value: "nats://nats.mister-smith-system:4222"
        - name: WORKER_CONCURRENCY
          valueFrom:
            configMapKeyRef:
              name: mister-smith-worker-config
              key: worker.concurrency
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8081
          initialDelaySeconds: 30
          periodSeconds: 15
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /ready
            port: 8081
          initialDelaySeconds: 15
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
        startupProbe:
          httpGet:
            path: /startup
            port: 8081
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 20
        volumeMounts:
        - name: workspace
          mountPath: /app/workspace
        - name: temp
          mountPath: /tmp
      volumes:
      - name: workspace
        emptyDir:
          sizeLimit: 1Gi
      - name: temp
        emptyDir:
          sizeLimit: 512Mi
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: mister-smith-worker-hpa
  namespace: mister-smith-workload
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: mister-smith-worker
  minReplicas: 2
  maxReplicas: 50
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  - type: Pods
    pods:
      metric:
        name: pending_tasks
      target:
        type: AverageValue
        averageValue: "100"
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 100
        periodSeconds: 15
      - type: Pods
        value: 4
        periodSeconds: 15
      selectPolicy: Max
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
      selectPolicy: Min
```

### 17. Environment Variable Naming Conventions

#### 17.1 Core Environment Variables
```bash
# Core Agent Configuration
AGENT_TYPE=orchestrator|worker|messaging
AGENT_TIER=minimal|standard|premium
AGENT_ID=${POD_NAME}  # Auto-populated in Kubernetes

# Claude CLI Integration
CLAUDE_PARALLEL_DEFAULT=4
CLAUDE_PARALLEL_MAX_AGENTS=50
CLAUDE_PARALLEL_CPU_BUDGET="4.0"
CLAUDE_PARALLEL_MEMORY_BUDGET="4Gi"

# Messaging Configuration
NATS_URL=nats://nats.mister-smith-system:4222
NATS_CLUSTER_ID=mister-smith-cluster
NATS_CLIENT_ID=${AGENT_TYPE}-${POD_NAME}
NATS_SUBJECT_PREFIX=mister-smith

# Persistence Configuration
REDIS_URL=redis://redis.mister-smith-data:6379
DATABASE_URL=postgresql://postgres:password@postgres.mister-smith-data:5432/mister_smith

# Observability Configuration
METRICS_ENABLED=true
METRICS_PORT=9090
TRACING_ENABLED=true
JAEGER_ENDPOINT=http://jaeger.mister-smith-monitoring:14268/api/traces

# Logging Configuration
RUST_LOG=info
LOG_FORMAT=json
LOG_LEVEL=info

# Security Configuration
TLS_ENABLED=true
MTLS_ENABLED=true
CERT_PATH=/etc/tls/certs
KEY_PATH=/etc/tls/private

# Resource Management
WORKER_CONCURRENCY=4
WORKER_TIMEOUT_SECONDS=300
ORCHESTRATOR_HEARTBEAT_INTERVAL=30
HEALTH_CHECK_INTERVAL=15

# Development Configuration (override files only)
RUST_BACKTRACE=1
DEBUG_MODE=true
HOT_RELOAD_ENABLED=true
```

### 18. Resource Allocation Specifications

#### 18.1 Resource Tier Definitions
```yaml
# resource-tiers.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: mister-smith-resource-tiers
  namespace: mister-smith-system
data:
  minimal.yaml: |
    resources:
      requests:
        memory: "128Mi"
        cpu: "100m"
      limits:
        memory: "256Mi"
        cpu: "250m"
    priorityClass: "best-effort"
    
  standard.yaml: |
    resources:
      requests:
        memory: "512Mi"
        cpu: "500m"
      limits:
        memory: "1Gi"
        cpu: "1000m"
    priorityClass: "normal"
    
  premium.yaml: |
    resources:
      requests:
        memory: "2Gi"
        cpu: "2000m"
      limits:
        memory: "4Gi"
        cpu: "4000m"
    priorityClass: "high"
---
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: best-effort
value: 100
globalDefault: false
description: "Best effort priority class for minimal tier"
---
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: normal
value: 500
globalDefault: true
description: "Normal priority class for standard tier"
---
apiVersion: scheduling.k8s.io/v1
kind: PriorityClass
metadata:
  name: high
value: 1000
globalDefault: false
description: "High priority class for premium tier"
```

### 19. Health Check Implementation Templates

#### 19.1 Application Health Check Server (Rust)
```rust
// health.rs - Health check implementation
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::time::{Duration, Instant};

#[derive(Clone)]
pub struct HealthState {
    pub start_time: Instant,
    pub nats_connected: Arc<tokio::sync::RwLock<bool>>,
    pub redis_connected: Arc<tokio::sync::RwLock<bool>>,
    pub db_connected: Arc<tokio::sync::RwLock<bool>>,
}

impl HealthState {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            nats_connected: Arc::new(tokio::sync::RwLock::new(false)),
            redis_connected: Arc::new(tokio::sync::RwLock::new(false)),
            db_connected: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }
}

pub fn health_router() -> Router<HealthState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/startup", get(startup_check))
        .route("/metrics", get(metrics_endpoint))
}

async fn health_check(State(state): State<HealthState>) -> Result<Json<Value>, StatusCode> {
    let uptime = state.start_time.elapsed().as_secs();
    
    Ok(Json(json!({
        "status": "healthy",
        "uptime_seconds": uptime,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "agent_type": std::env::var("AGENT_TYPE").unwrap_or_default()
    })))
}

async fn readiness_check(State(state): State<HealthState>) -> Result<Json<Value>, StatusCode> {
    let nats_ok = *state.nats_connected.read().await;
    let redis_ok = *state.redis_connected.read().await;
    let db_ok = *state.db_connected.read().await;
    
    let ready = nats_ok && redis_ok && db_ok;
    
    let response = json!({
        "ready": ready,
        "checks": {
            "nats": nats_ok,
            "redis": redis_ok,
            "database": db_ok
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    if ready {
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

async fn startup_check(State(state): State<HealthState>) -> Result<Json<Value>, StatusCode> {
    let uptime = state.start_time.elapsed();
    let startup_complete = uptime > Duration::from_secs(10);
    
    let response = json!({
        "started": startup_complete,
        "uptime_seconds": uptime.as_secs(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    if startup_complete {
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

async fn metrics_endpoint() -> Json<Value> {
    // Prometheus metrics would go here
    Json(json!({
        "metrics_endpoint": "/metrics",
        "format": "prometheus"
    }))
}
```

---

This deployment architecture specification provides comprehensive patterns and concrete implementation templates for deploying multi-agent systems at scale, including Docker best practices, Kubernetes manifests, and environment management suitable for production deployment of the Mister Smith AI Agent Framework.