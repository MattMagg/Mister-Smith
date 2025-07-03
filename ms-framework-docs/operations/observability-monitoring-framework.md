---
title: Observability & Monitoring Framework - Revised
type: note
permalink: revision-swarm/operations/observability-monitoring-framework-revised
---

# Observability & Monitoring Framework - Revised
## Multi-Agent Systems Monitoring Patterns

### 1. Architecture Overview

#### 1.1 Core Observability Components Pattern
```pseudocode
PATTERN ObservabilityArchitecture:
    COMPONENTS:
        instrumentation_layer
        collection_pipeline
        storage_backend
        analysis_engine
        visualization_layer
        alerting_system
```

#### 1.2 Data Flow Pattern
```pseudocode
PATTERN ObservabilityPipeline:
    STAGES:
        agent_instrumentation
        local_buffering
        centralized_collection
        processing_enrichment
        persistent_storage
        real_time_analysis
        user_presentation
```

### 2. Instrumentation Patterns

#### 2.1 Agent Instrumentation Pattern
```pseudocode
PATTERN AgentInstrumentation:
    initialize_telemetry_context()
    configure_data_exporters()
    set_resource_attributes()
    enable_context_propagation()
    configure_sampling_strategy()
```

#### 2.2 Context Propagation Pattern
```pseudocode
PATTERN ContextPropagation:
    inject_trace_context(outgoing_message)
    extract_trace_context(incoming_message)
    maintain_correlation_chain()
    preserve_causality_information()
```

### 3. Distributed Tracing Patterns

#### 3.1 Trace Structure Pattern
```pseudocode
PATTERN TraceStructure:
    COMPONENTS:
        trace_identifier
        span_collection
        timing_information
        status_indicators
        attribute_metadata
        event_markers
        relationship_links
```

#### 3.2 Sampling Strategy Pattern
```pseudocode
PATTERN AdaptiveSampling:
    DECISION_FACTORS:
        operation_criticality
        system_load_level
        error_presence
        latency_threshold
        business_importance
    
    SAMPLING_LOGIC:
        IF critical_operation: ALWAYS_SAMPLE
        ELIF error_detected: ALWAYS_SAMPLE
        ELIF high_load: MINIMAL_SAMPLING
        ELSE: ADAPTIVE_RATE
```

#### 3.3 Cross-Agent Tracing Pattern
```pseudocode
PATTERN CrossAgentTracing:
    create_client_span(source_agent)
    inject_context_headers(message)
    transmit_to_target(message)
    extract_context_on_receive()
    create_server_span(target_agent)
    link_span_hierarchy()
```

#### 3.4 Trace Layers Pattern
```pseudocode
PATTERN TraceLayers:
    CHILD_PROCESS_TRACING:
        spawn_agent_with_piped_output()
        create_tracing_span(agent_id)
        capture_stdout_to_tracing()
        capture_stderr_to_tracing()
        correlate_process_lifecycle()

    CONTEXT_PROPAGATION:
        inject_trace_headers(outgoing_message)
        extract_trace_headers(incoming_message)
        maintain_span_relationships()
        preserve_baggage_items()
```

#### 3.5 Claude-CLI Parallel Agent Tracing Pattern
```pseudocode
PATTERN ClaudeCLIParallelTracing:
    PARALLEL_AGENT_INSTRUMENTATION:
        # Handle multiple internal agents sharing one PID
        create_root_span(claude_cli_process_id)

        FOR each_internal_agent IN parallel_agents:
            create_child_span(internal_agent_id)
            add_span_attribute("agent_internal_id", internal_agent_id)
            add_span_attribute("agent_type", "task_patch_agent")
            add_span_attribute("parallel_group", parallel_execution_id)

        # Parse and correlate output lines
        parse_output_line("Task(Patch Agent <n> ...)")
        correlate_to_span(agent_internal_id)

    LOG_DISAMBIGUATION:
        # Multiple internal agents share PID but need separate traces
        span_attributes:
            agent_internal_id: "unique_identifier_per_internal_agent"
            process_id: "shared_claude_cli_pid"
            parallel_execution_id: "batch_identifier"
            agent_role: "task_agent" | "patch_agent"

    TRACE_CORRELATION:
        # Link parallel agent spans to parent orchestration
        parent_span: "orchestrator_task_span"
        child_spans: ["agent_0_span", "agent_1_span", ..., "agent_n_span"]
        correlation_key: "parallel_execution_batch_id"
```

#### 3.6 Claude-CLI Hook System Tracing Pattern
```pseudocode
PATTERN HookSystemTracing:
    HOOK_EXECUTION_SPANS:
        # Create spans for each hook execution
        create_hook_span(hook_name, hook_type)
        add_span_attribute("hook_name", hook_name)
        add_span_attribute("hook_type", hook_type)  # startup, pre_task, post_task, on_error, on_file_change
        add_span_attribute("hook_outcome", outcome)  # success, failure, timeout
        add_span_attribute("hook_latency_ms", execution_time)
        add_span_attribute("hook_script_path", script_path)

    HOOK_LIFECYCLE_TRACING:
        # Trace complete hook lifecycle
        start_span("hook_execution")
        record_event("hook_started", {"hook_type": hook_type})

        # Execute hook and capture metrics
        execution_start = current_timestamp()
        hook_result = execute_hook_script(hook_payload)
        execution_end = current_timestamp()

        # Record outcome and timing
        add_span_attribute("execution_duration_ms", execution_end - execution_start)
        add_span_attribute("hook_exit_code", hook_result.exit_code)
        add_span_attribute("payload_size_bytes", payload_size)
        add_span_attribute("response_size_bytes", response_size)

        IF hook_result.success:
            record_event("hook_completed", {"mutations": hook_result.mutations})
        ELSE:
            record_event("hook_failed", {"error": hook_result.error})
            add_span_attribute("error_type", hook_result.error_type)

        end_span()

    HOOK_CORRELATION_PATTERN:
        # Link hook spans to triggering task/agent spans
        parent_span: "task_execution_span" | "agent_lifecycle_span"
        hook_span: "hook_execution_span"
        correlation_attributes:
            task_id: "triggering_task_identifier"
            agent_id: "executing_agent_identifier"
            hook_trigger: "pre_task" | "post_task" | "on_error" | "on_file_change"
```

#### 3.5 OTLP Integration Pattern
```pseudocode
PATTERN OTLPConfiguration:
    EXPORTER_SETUP:
        protocol: binary_protobuf | grpc | http_json
        endpoint: collector_address
        compression: gzip | none
        retry_policy: exponential_backoff
    
    COLLECTOR_PIPELINE:
        receivers: [otlp_grpc, otlp_http]
        processors: [batch, memory_limiter]
        exporters: [backend_storage]
    
    PERFORMANCE_CONSIDERATIONS:
        binary_encoding: 2-3x_cpu_efficiency
        batch_processing: reduce_network_calls
        async_export: prevent_blocking
```

### 4. Metrics Collection Patterns

#### 4.1 Metric Types Pattern
```pseudocode
PATTERN MetricTypes:
    CATEGORIES:
        counters: cumulative_values
        gauges: point_in_time_values
        histograms: distribution_values
        summaries: statistical_aggregates
```

#### 4.2 Agent-Specific Metrics Pattern
```pseudocode
PATTERN AgentMetrics:
    COMMON_METRICS:
        operation_count
        operation_duration
        error_rate
        resource_utilization
        queue_depth
    
    SPECIALIZED_METRICS:
        researcher_agent: [sources_searched, relevance_scores]
        coder_agent: [lines_generated, complexity_scores]
        coordinator_agent: [agents_managed, coordination_latency]
```

#### 4.3 System-Wide Metrics Pattern
```pseudocode
PATTERN SystemMetrics:
    CATEGORIES:
        swarm_coordination_metrics
        resource_utilization_metrics
        agent_lifecycle_metrics
        task_completion_metrics
        communication_overhead_metrics
```

#### 4.4 Messaging Backplane Metrics Pattern
```pseudocode
PATTERN BackplaneMetrics:
    CONSUMER_HEALTH:
        consumer_lag_ms
        pending_message_count
        ack_rate_per_second
        reconnection_count
    
    BROKER_PERFORMANCE:
        fsync_latency_p99
        disk_write_queue_depth
        memory_buffer_utilization
        active_connection_count
    
    MESSAGE_FLOW:
        publish_rate_per_topic
        fan_out_multiplier
        message_size_p95
        throughput_mb_per_second
    
    SYSTEM_HEALTH:
        broker_cpu_percentage
        jvm_gc_pause_ms
        network_retransmits
        partition_leader_changes
```

#### 4.5 Critical Threshold Monitoring Pattern
```pseudocode
PATTERN CriticalThresholds:
    MESSAGING_THRESHOLDS:
        jetstream_fsync_lag: 50M_msgs_per_day
        rabbitmq_queue_depth: 10000_messages
        redis_output_buffer: approaching_limit
        kafka_consumer_lag: 1000_msgs_or_30sec
    
    RESPONSE_ACTIONS:
        emit_threshold_alert()
        trigger_backpressure()
        scale_consumer_pool()
        initiate_flow_control()
```

### 5. Logging Patterns

#### 5.1 Structured Logging Pattern
```pseudocode
PATTERN StructuredLogging:
    LOG_ENTRY_STRUCTURE:
        timestamp
        severity_level
        message_content
        trace_correlation
        agent_context
        operation_context
        environment_metadata
```

#### 5.2 Log Level Classification Pattern
```pseudocode
PATTERN LogLevels:
    DEBUG: detailed_diagnostic_info
    INFO: general_operational_events
    WARNING: potential_issue_indicators
    ERROR: failure_notifications
    CRITICAL: system_critical_events
```

#### 5.3 Log Aggregation Pattern
```pseudocode
PATTERN LogAggregation:
    PIPELINE_STAGES:
        collection_from_sources
        parsing_and_structuring
        enrichment_with_context
        correlation_by_identifiers
        aggregation_by_patterns
        storage_and_indexing
```

#### 5.4 Multiline Log Handling Pattern
```pseudocode
PATTERN MultilineLogHandling:
    DETECTION_STRATEGIES:
        prefix_pattern_matching
        continuation_character_detection
        timeout_based_grouping
        structured_format_parsing
    
    PROCESSING_OPTIONS:
        buffer_until_complete()
        apply_max_wait_timeout()
        preserve_original_formatting()
        add_correlation_metadata()
    
    OUTPUT_FORMATS:
        single_log_entry: complete_text_block
        json_lines: structured_per_line
        otlp_format: single_log_record
```

### 6. Multi-Agent Observability Patterns

#### 6.1 Agent State Tracking Pattern
```pseudocode
PATTERN AgentStateObservability:
    STATES:
        initializing
        idle
        processing
        waiting
        error
        terminating
    
    STATE_TRANSITIONS:
        capture_state_change()
        measure_state_duration()
        detect_anomalous_transitions()
        maintain_state_history()
```

#### 6.2 Swarm Coordination Observability Pattern
```pseudocode
PATTERN SwarmObservability:
    track_agent_assignments()
    monitor_task_distribution()
    measure_coordination_latency()
    analyze_communication_patterns()
    detect_coordination_anomalies()
```

#### 6.3 Communication Pattern Analysis
```pseudocode
PATTERN CommunicationAnalysis:
    build_communication_graph()
    track_message_flow()
    measure_communication_latency()
    identify_bottlenecks()
    detect_anomalous_patterns()
```

### 7. Alerting and Anomaly Detection Patterns

#### 7.1 Alert Rule Pattern
```pseudocode
PATTERN AlertRules:
    RULE_COMPONENTS:
        condition_expression
        severity_level
        notification_channels
        remediation_actions
        suppression_windows
```

#### 7.2 Anomaly Detection Pattern
```pseudocode
PATTERN AnomalyDetection:
    DETECTION_METHODS:
        statistical_analysis
        pattern_recognition
        machine_learning_models
        threshold_monitoring
        behavioral_analysis
    
    RESPONSE_ACTIONS:
        emit_alert()
        trigger_investigation()
        initiate_auto_remediation()
        update_baselines()
```

#### 7.3 Prometheus Alert Pattern
```pseudocode
PATTERN PrometheusAlerts:
    ALERT_DEFINITION:
        name: descriptive_alert_name
        expression: promql_query
        duration: evaluation_period
        severity: critical | warning | info
        annotations: detailed_descriptions
    
    COMMON_ALERTS:
        agent_crash_loop:
            expr: rate(restarts_total[5m]) > 1/min
            severity: critical
        
        high_consumer_lag:
            expr: consumer_lag > 1000
            severity: warning
        
        resource_exhaustion:
            expr: memory_percent > 90
            severity: critical
    
    ALERT_ROUTING:
        group_by: [alertname, cluster, agent]
        group_wait: 30s
        group_interval: 5m
        repeat_interval: 12h
```

### 8. Visualization Patterns

#### 8.1 Dashboard Organization Pattern
```pseudocode
PATTERN DashboardStructure:
    DASHBOARD_TYPES:
        system_overview
        agent_performance
        swarm_coordination
        error_analysis
        resource_utilization
    
    VISUALIZATION_TYPES:
        time_series_graphs
        distribution_histograms
        relationship_matrices
        status_indicators
        flow_diagrams
```

#### 8.2 Real-Time Monitoring Pattern
```pseudocode
PATTERN RealTimeMonitoring:
    COMPONENTS:
        live_trace_visualization
        metric_sparklines
        log_streaming
        topology_mapping
        alert_notifications
```

### 9. Data Management Patterns

#### 9.1 Retention Policy Pattern
```pseudocode
PATTERN DataRetention:
    RETENTION_TIERS:
        hot_storage: recent_full_fidelity
        warm_storage: medium_term_sampled
        cold_storage: long_term_compressed
        archive: historical_metadata
    
    TRANSITION_RULES:
        age_based_migration()
        importance_based_retention()
        compliance_driven_archival()
```

#### 9.2 Data Lifecycle Pattern
```pseudocode
PATTERN DataLifecycle:
    ingest_and_validate()
    process_and_enrich()
    store_with_indexing()
    age_and_compress()
    archive_or_delete()
```

### 10. Performance Optimization Patterns

#### 10.1 Overhead Management Pattern
```pseudocode
PATTERN OverheadOptimization:
    STRATEGIES:
        adaptive_sampling
        batch_processing
        asynchronous_export
        intelligent_filtering
        resource_pooling
```

#### 10.2 Scalability Pattern
```pseudocode
PATTERN ObservabilityScaling:
    HORIZONTAL_SCALING:
        distribute_collection_load()
        shard_storage_backend()
        parallelize_processing()
    
    VERTICAL_SCALING:
        optimize_resource_allocation()
        implement_tiered_storage()
        cache_frequent_queries()
```

### 11. Integration Patterns

#### 11.1 External System Integration Pattern
```pseudocode
PATTERN ExternalIntegration:
    INTEGRATION_POINTS:
        incident_management_systems
        analytics_platforms
        machine_learning_pipelines
        notification_services
        automation_tools
```

#### 11.2 Data Export Pattern
```pseudocode
PATTERN DataExport:
    configure_export_format()
    set_export_schedule()
    filter_export_data()
    transform_for_destination()
    verify_export_completion()
```

### 12. Security and Compliance Patterns

#### 12.1 Observability Security Pattern
```pseudocode
PATTERN ObservabilitySecurity:
    SECURITY_MEASURES:
        encrypt_data_transmission()
        authenticate_access()
        authorize_operations()
        audit_all_access()
        detect_sensitive_data()
        apply_data_masking()
```

#### 12.2 Compliance Pattern
```pseudocode
PATTERN ComplianceMonitoring:
    track_data_access()
    enforce_retention_policies()
    maintain_audit_trails()
    generate_compliance_reports()
    automate_policy_enforcement()
```

### 13. Implementation Considerations

#### 13.1 Framework Selection Criteria
- Language-agnostic instrumentation capabilities
- Support for distributed tracing standards
- Flexible metric collection options
- Structured logging support
- Extensible architecture

#### 13.2 Protocol Performance Considerations
```pseudocode
PATTERN ProtocolSelection:
    ENCODING_EFFICIENCY:
        binary_protobuf: 15_microsec_per_log
        json_encoding: 45_microsec_per_log
        size_ratio: 1:2_binary_vs_json
    
    TRANSPORT_OPTIONS:
        grpc_tonic: minimal_async_overhead
        http_binary: wide_compatibility
        direct_stdout: essentially_free
    
    OPTIMIZATION_STRATEGIES:
        use_non_blocking_appenders()
        batch_telemetry_exports()
        implement_circuit_breakers()
```

#### 13.3 Deployment Considerations
- Minimize instrumentation overhead
- Ensure high availability of monitoring
- Plan for data volume growth
- Implement proper access controls
- Enable disaster recovery
- Consider protocol efficiency for scale

### 14. Best Practices Summary

#### 14.1 Instrumentation Best Practices
- Instrument critical code paths
- Use consistent naming conventions
- Implement proper context propagation
- Apply appropriate sampling rates
- Minimize performance impact

#### 14.2 Data Management Best Practices
- Define clear retention policies
- Implement data compression strategies
- Use appropriate storage tiers
- Regular backup and archival
- Monitor storage growth trends

#### 14.3 Operational Best Practices
- Establish baseline metrics
- Define meaningful alerts
- Regular review of dashboards
- Continuous optimization
- Document monitoring patterns

---

## 15. Concrete Specifications

### 15.1 Prometheus Metrics Schema

#### 15.1.1 Agent Lifecycle Metrics
```prometheus
# HELP agent_spawns_total Number of agent instances spawned
# TYPE agent_spawns_total counter
agent_spawns_total{agent_type="task_agent",spawn_method="subprocess"} 125

# HELP agent_active_count Current number of active agents
# TYPE agent_active_count gauge
agent_active_count{agent_type="task_agent",state="processing"} 8

# HELP agent_duration_seconds Time agents spend in each state
# TYPE agent_duration_seconds histogram
agent_duration_seconds_bucket{agent_type="patch_agent",state="processing",le="1.0"} 45
agent_duration_seconds_bucket{agent_type="patch_agent",state="processing",le="5.0"} 89
agent_duration_seconds_bucket{agent_type="patch_agent",state="processing",le="10.0"} 112
agent_duration_seconds_bucket{agent_type="patch_agent",state="processing",le="+Inf"} 125
agent_duration_seconds_sum{agent_type="patch_agent",state="processing"} 387.5
agent_duration_seconds_count{agent_type="patch_agent",state="processing"} 125

# HELP agent_memory_bytes Memory usage per agent
# TYPE agent_memory_bytes gauge
agent_memory_bytes{agent_id="agent-42",agent_type="analysis_agent"} 524288000

# HELP agent_cpu_percent CPU utilization per agent
# TYPE agent_cpu_percent gauge
agent_cpu_percent{agent_id="agent-42",agent_type="analysis_agent"} 23.5
```

#### 15.1.2 Task Execution Metrics
```prometheus
# HELP task_completions_total Number of completed tasks
# TYPE task_completions_total counter
task_completions_total{task_type="analysis",status="success",agent_type="analysis_agent"} 1247

# HELP task_errors_total Number of failed tasks
# TYPE task_errors_total counter
task_errors_total{task_type="code_generation",error_type="timeout",agent_type="coder_agent"} 23

# HELP task_duration_seconds Task execution duration distribution
# TYPE task_duration_seconds histogram
task_duration_seconds_bucket{task_type="analysis",le="5.0"} 234
task_duration_seconds_bucket{task_type="analysis",le="10.0"} 567
task_duration_seconds_bucket{task_type="analysis",le="30.0"} 890
task_duration_seconds_bucket{task_type="analysis",le="+Inf"} 923
task_duration_seconds_sum{task_type="analysis"} 8934.2
task_duration_seconds_count{task_type="analysis"} 923

# HELP task_queue_depth Current number of pending tasks
# TYPE task_queue_depth gauge
task_queue_depth{queue_type="priority",agent_type="coordinator_agent"} 15
```

#### 15.1.3 Communication Metrics
```prometheus
# HELP messages_sent_total Number of messages sent between agents
# TYPE messages_sent_total counter
messages_sent_total{from_agent_type="coordinator",to_agent_type="worker",message_type="task_assignment"} 1542

# HELP message_latency_seconds Message delivery latency distribution
# TYPE message_latency_seconds histogram
message_latency_seconds_bucket{message_type="task_result",transport="nats",le="0.001"} 1234
message_latency_seconds_bucket{message_type="task_result",transport="nats",le="0.005"} 2345
message_latency_seconds_bucket{message_type="task_result",transport="nats",le="0.01"} 2456
message_latency_seconds_bucket{message_type="task_result",transport="nats",le="+Inf"} 2467
message_latency_seconds_sum{message_type="task_result",transport="nats"} 12.34
message_latency_seconds_count{message_type="task_result",transport="nats"} 2467

# HELP message_queue_depth Number of messages in transport queues
# TYPE message_queue_depth gauge
message_queue_depth{queue_name="task_assignments",broker="nats"} 8

# HELP message_delivery_failures_total Failed message deliveries
# TYPE message_delivery_failures_total counter
message_delivery_failures_total{failure_reason="timeout",transport="nats"} 12
```

#### 15.1.4 Claude-CLI Integration Metrics
```prometheus
# HELP claude_cli_processes_total Number of Claude CLI processes spawned
# TYPE claude_cli_processes_total counter
claude_cli_processes_total{command_type="parallel_agents",batch_size="5"} 45

# HELP claude_cli_parallel_agents_active Current number of parallel internal agents
# TYPE claude_cli_parallel_agents_active gauge
claude_cli_parallel_agents_active{batch_id="batch-123",process_id="12345"} 5

# HELP claude_cli_hook_executions_total Number of hook executions
# TYPE claude_cli_hook_executions_total counter
claude_cli_hook_executions_total{hook_type="post_task",hook_name="validation",status="success"} 234

# HELP claude_cli_hook_duration_seconds Hook execution duration
# TYPE claude_cli_hook_duration_seconds histogram
claude_cli_hook_duration_seconds_bucket{hook_type="pre_task",hook_name="setup",le="0.1"} 45
claude_cli_hook_duration_seconds_bucket{hook_type="pre_task",hook_name="setup",le="0.5"} 67
claude_cli_hook_duration_seconds_bucket{hook_type="pre_task",hook_name="setup",le="1.0"} 78
claude_cli_hook_duration_seconds_bucket{hook_type="pre_task",hook_name="setup",le="+Inf"} 80
claude_cli_hook_duration_seconds_sum{hook_type="pre_task",hook_name="setup"} 23.45
claude_cli_hook_duration_seconds_count{hook_type="pre_task",hook_name="setup"} 80
```

### 15.2 OpenTelemetry Tracing Schema

#### 15.2.1 Agent Execution Trace Structure
```json
{
  "traceID": "4bf92f3577b34da6a3ce929d0e0e4736",
  "spanID": "00f067aa0ba902b7",
  "parentSpanID": "00f067aa0ba90000",
  "operationName": "agent.task_execution",
  "startTime": 1609459200000000,
  "duration": 5000000,
  "tags": {
    "agent.id": "agent-42",
    "agent.type": "analysis_agent",
    "agent.version": "1.0.0",
    "task.id": "task-12345",
    "task.type": "code_analysis",
    "task.priority": "high",
    "component": "mister-smith-agent",
    "span.kind": "internal"
  },
  "process": {
    "serviceName": "mister-smith-framework",
    "tags": {
      "hostname": "agent-node-01",
      "process.pid": "12345",
      "jaeger.version": "1.29.0"
    }
  },
  "logs": [
    {
      "timestamp": 1609459201000000,
      "fields": [
        {"key": "event", "value": "task_started"},
        {"key": "agent.state", "value": "processing"}
      ]
    },
    {
      "timestamp": 1609459205000000,
      "fields": [
        {"key": "event", "value": "task_completed"},
        {"key": "result.status", "value": "success"},
        {"key": "result.items_processed", "value": 47}
      ]
    }
  ]
}
```

#### 15.2.2 Cross-Agent Correlation Pattern
```json
{
  "orchestrator_span": {
    "traceID": "4bf92f3577b34da6a3ce929d0e0e4736",
    "spanID": "00f067aa0ba90000",
    "operationName": "orchestrator.task_distribution",
    "tags": {
      "component": "task_orchestrator",
      "task.batch_id": "batch-123",
      "parallel.agent_count": 5
    }
  },
  "child_spans": [
    {
      "traceID": "4bf92f3577b34da6a3ce929d0e0e4736",
      "spanID": "00f067aa0ba902b7",
      "parentSpanID": "00f067aa0ba90000",
      "operationName": "agent.parallel_execution",
      "tags": {
        "agent.internal_id": "agent-0",
        "parallel.execution_id": "batch-123",
        "agent.role": "task_agent"
      }
    }
  ]
}
```

#### 15.2.3 Hook System Tracing Pattern
```json
{
  "traceID": "4bf92f3577b34da6a3ce929d0e0e4736",
  "spanID": "00f067aa0ba90abc",
  "parentSpanID": "00f067aa0ba902b7",
  "operationName": "hook.execution",
  "tags": {
    "hook.name": "validation_hook",
    "hook.type": "post_task",
    "hook.script_path": "/hooks/validate.sh",
    "hook.trigger": "task_completion"
  },
  "events": [
    {
      "timestamp": 1609459205100000,
      "name": "hook_started",
      "attributes": {
        "payload.size_bytes": 1024
      }
    },
    {
      "timestamp": 1609459205300000,
      "name": "hook_completed",
      "attributes": {
        "hook.exit_code": 0,
        "response.size_bytes": 256,
        "mutations.count": 3
      }
    }
  ]
}
```

### 15.3 Structured Logging Schema

#### 15.3.1 Agent Lifecycle Log Format
```json
{
  "timestamp": "2024-01-01T12:00:00.000Z",
  "level": "INFO",
  "msg": "Agent spawned successfully",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "resource": {
    "service.name": "mister-smith-agent",
    "service.version": "1.0.0",
    "service.instance.id": "agent-node-01"
  },
  "attributes": {
    "agent.id": "agent-42",
    "agent.type": "analysis_agent",
    "agent.state": "initializing",
    "spawn.method": "subprocess",
    "parent.agent_id": "coordinator-01",
    "memory.allocated_mb": 256,
    "cpu.cores_allocated": 2
  }
}
```

#### 15.3.2 Task Execution Log Format
```json
{
  "timestamp": "2024-01-01T12:01:30.500Z",
  "level": "ERROR",
  "msg": "Task execution failed due to timeout",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "resource": {
    "service.name": "mister-smith-agent",
    "service.version": "1.0.0"
  },
  "attributes": {
    "agent.id": "agent-42",
    "task.id": "task-12345",
    "task.type": "code_analysis",
    "task.priority": "high",
    "task.timeout_seconds": 30,
    "task.duration_seconds": 30.1,
    "error.type": "timeout",
    "error.category": "execution",
    "retry.attempt": 2,
    "retry.max_attempts": 3
  },
  "exception": {
    "type": "TaskTimeoutException",
    "message": "Task execution exceeded 30 second timeout",
    "stacktrace": "TaskTimeoutException: Task execution exceeded...\n  at execute_task:142\n  at agent_loop:89"
  }
}
```

#### 15.3.3 Communication Log Format
```json
{
  "timestamp": "2024-01-01T12:02:15.250Z",
  "level": "WARN",
  "msg": "Message delivery retry required",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba90xyz",
  "resource": {
    "service.name": "mister-smith-messaging",
    "service.version": "1.0.0"
  },
  "attributes": {
    "message.id": "msg-67890",
    "message.type": "task_assignment",
    "message.size_bytes": 1024,
    "sender.agent_id": "coordinator-01",
    "receiver.agent_id": "worker-05",
    "transport.broker": "nats",
    "transport.subject": "agents.tasks.assign",
    "delivery.attempt": 2,
    "delivery.latency_ms": 150,
    "failure.reason": "connection_timeout"
  }
}
```

### 15.4 Health Check Endpoints

#### 15.4.1 Basic Health Check
```http
GET /health
Content-Type: application/json

{
  "status": "healthy",
  "timestamp": "2024-01-01T12:00:00.000Z",
  "uptime_seconds": 3600,
  "version": "1.0.0",
  "checks": {
    "messaging": "healthy",
    "database": "healthy",
    "agents": "healthy"
  }
}
```

#### 15.4.2 Agent-Specific Health Check
```http
GET /health/agent/agent-42
Content-Type: application/json

{
  "agent_id": "agent-42",
  "status": "healthy",
  "state": "processing",
  "uptime_seconds": 1800,
  "current_task": {
    "task_id": "task-12345",
    "task_type": "analysis",
    "started_at": "2024-01-01T11:45:00.000Z",
    "duration_seconds": 900
  },
  "resources": {
    "memory_usage_mb": 245,
    "memory_limit_mb": 512,
    "cpu_usage_percent": 23.5
  },
  "last_heartbeat": "2024-01-01T12:00:00.000Z"
}
```

#### 15.4.3 Readiness Check
```http
GET /health/ready
Content-Type: application/json

{
  "status": "ready",
  "timestamp": "2024-01-01T12:00:00.000Z",
  "ready_for_tasks": true,
  "available_agents": 8,
  "queue_depth": 5,
  "resource_availability": {
    "memory_available_mb": 2048,
    "cpu_available_percent": 65
  }
}
```

### 15.5 Alert Rule Definitions

#### 15.5.1 Critical Alerts (Prometheus AlertManager)
```yaml
groups:
- name: agent_critical_alerts
  rules:
  - alert: AgentCrashLoop
    expr: rate(agent_spawns_total[5m]) > 1
    for: 2m
    labels:
      severity: critical
    annotations:
      summary: "Agent {{ $labels.agent_type }} crash loop detected"
      description: "Agent {{ $labels.agent_type }} is restarting more than once per 5 minutes"

  - alert: HighTaskErrorRate
    expr: rate(task_errors_total[5m]) / rate(task_completions_total[5m]) > 0.05
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High task error rate for {{ $labels.task_type }}"
      description: "Task error rate is {{ $value | humanizePercentage }} over the last 5 minutes"

  - alert: AgentMemoryExhaustion
    expr: agent_memory_bytes / (1024*1024*1024) > 1.5
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "Agent {{ $labels.agent_id }} memory exhaustion"
      description: "Agent {{ $labels.agent_id }} is using {{ $value }}GB memory"

  - alert: MessageQueueBackup
    expr: message_queue_depth > 1000
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "Message queue {{ $labels.queue_name }} backup"
      description: "Queue depth is {{ $value }} messages"
```

#### 15.5.2 Warning Alerts
```yaml
- name: agent_warning_alerts
  rules:
  - alert: HighTaskLatency
    expr: histogram_quantile(0.95, task_duration_seconds) > 30
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "High task latency for {{ $labels.task_type }}"
      description: "95th percentile latency is {{ $value }}s"

  - alert: AgentSaturation
    expr: agent_cpu_percent > 80
    for: 15m
    labels:
      severity: warning
    annotations:
      summary: "Agent {{ $labels.agent_id }} high CPU usage"
      description: "CPU usage is {{ $value }}%"

  - alert: CommunicationDelay
    expr: histogram_quantile(0.99, message_latency_seconds) > 5
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "High message latency for {{ $labels.message_type }}"
      description: "99th percentile message latency is {{ $value }}s"
```

### 15.6 Dashboard Configurations

#### 15.6.1 System Overview Dashboard (Grafana JSON)
```json
{
  "dashboard": {
    "title": "Mister Smith - System Overview",
    "panels": [
      {
        "title": "Active Agents",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(agent_active_count)",
            "legendFormat": "Total Active Agents"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "color": {"mode": "thresholds"},
            "thresholds": {
              "steps": [
                {"color": "green", "value": 0},
                {"color": "yellow", "value": 50},
                {"color": "red", "value": 100}
              ]
            }
          }
        }
      },
      {
        "title": "Task Throughput",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(task_completions_total[5m]) * 60",
            "legendFormat": "Tasks/minute - {{ task_type }}"
          }
        ]
      },
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(task_errors_total[5m]) / rate(task_completions_total[5m]) * 100",
            "legendFormat": "Error % - {{ task_type }}"
          }
        ]
      },
      {
        "title": "Resource Utilization",
        "type": "heatmap",
        "targets": [
          {
            "expr": "agent_cpu_percent",
            "legendFormat": "{{ agent_id }}"
          }
        ]
      }
    ]
  }
}
```

#### 15.6.2 Agent Performance Dashboard Configuration
```json
{
  "dashboard": {
    "title": "Mister Smith - Agent Performance",
    "panels": [
      {
        "title": "Agent Lifecycle Timeline",
        "type": "timeline",
        "targets": [
          {
            "expr": "agent_duration_seconds",
            "legendFormat": "{{ agent_id }} - {{ state }}"
          }
        ]
      },
      {
        "title": "Task Duration Distribution",
        "type": "histogram",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, task_duration_seconds)",
            "legendFormat": "50th percentile"
          },
          {
            "expr": "histogram_quantile(0.95, task_duration_seconds)",
            "legendFormat": "95th percentile"
          },
          {
            "expr": "histogram_quantile(0.99, task_duration_seconds)",
            "legendFormat": "99th percentile"
          }
        ]
      },
      {
        "title": "Agent State Distribution",
        "type": "piechart",
        "targets": [
          {
            "expr": "sum by (state) (agent_active_count)",
            "legendFormat": "{{ state }}"
          }
        ]
      }
    ]
  }
}
```

### 15.7 Performance Baselines

#### 15.7.1 Agent Performance Baselines
```yaml
agent_performance_baselines:
  task_completion:
    p95_duration_seconds: 10
    p99_duration_seconds: 30
    success_rate_percent: 99
  
  agent_spawn:
    p95_duration_seconds: 2
    p99_duration_seconds: 5
    success_rate_percent: 99.5
  
  resource_usage:
    memory_baseline_mb: 256
    memory_max_mb: 512
    cpu_baseline_percent: 15
    cpu_max_percent: 80

communication_baselines:
  message_delivery:
    p95_latency_ms: 50
    p99_latency_ms: 100
    delivery_success_rate: 99.9
  
  queue_processing:
    processing_time_ms: 25
    max_queue_depth: 100
  
  context_propagation:
    overhead_ms: 1
    success_rate: 99.99

system_baselines:
  throughput:
    tasks_per_minute: 100
    burst_capacity: 500
  
  utilization:
    optimal_range_percent: [70, 85]
    max_sustainable_percent: 90
  
  error_rates:
    baseline_error_rate: 0.01
    alert_threshold: 0.05
    critical_threshold: 0.10
```

### 15.8 OTLP Configuration

#### 15.8.1 OpenTelemetry Collector Configuration
```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    send_batch_size: 1024
    timeout: 1s
  
  memory_limiter:
    limit_mib: 512
  
  resource:
    attributes:
      - key: service.name
        value: mister-smith-framework
        action: upsert

exporters:
  prometheus:
    endpoint: "0.0.0.0:8889"
    namespace: mister_smith
  
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true
  
  logging:
    loglevel: debug

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [jaeger, logging]
    
    metrics:
      receivers: [otlp]
      processors: [memory_limiter, batch, resource]
      exporters: [prometheus, logging]
    
    logs:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [logging]
```

---

This framework provides comprehensive observability patterns for multi-agent systems without specifying particular technologies or implementations, suitable for specialized research agents to implement with their chosen technology stacks.