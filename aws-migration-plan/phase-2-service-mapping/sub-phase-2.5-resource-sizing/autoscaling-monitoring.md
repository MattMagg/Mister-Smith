# Auto-Scaling and Monitoring Configuration

## Auto-Scaling Strategy

Based on MisterSmith's agent-based architecture, we design auto-scaling policies that respond to both infrastructure metrics and application-specific signals.

## ECS Service Auto-Scaling

### 1. Supervisor Service Scaling
```yaml
supervisor_scaling:
  target_type: ECSService
  service_name: mistersmith-supervisor
  
  scaling_policies:
    - name: cpu-scaling
      metric_type: ECSServiceAverageCPUUtilization
      target_value: 70
      scale_out_cooldown: 300
      scale_in_cooldown: 600
      
    - name: memory-scaling
      metric_type: ECSServiceAverageMemoryUtilization
      target_value: 80
      scale_out_cooldown: 300
      scale_in_cooldown: 600
      
    - name: active-agents
      metric_type: CustomMetric
      metric_name: ActiveAgentCount
      target_value: 50  # Scale when > 50 agents per supervisor
      scale_out_cooldown: 180
      scale_in_cooldown: 900
  
  capacity:
    min_capacity: 1
    max_capacity: 10
    desired_capacity: 2
    
  step_scaling:
    - adjustment_type: PercentChangeInCapacity
      metric_interval_lower_bound: 0
      metric_interval_upper_bound: 50
      scaling_adjustment: 25
      
    - adjustment_type: PercentChangeInCapacity
      metric_interval_lower_bound: 50
      scaling_adjustment: 50
```

### 2. Worker Service Scaling
```yaml
worker_scaling:
  target_type: ECSService
  service_name: mistersmith-worker
  
  scaling_policies:
    - name: pending-tasks
      metric_type: CustomMetric
      metric_name: PendingTaskCount
      target_value: 100  # Scale when > 100 pending tasks
      scale_out_cooldown: 120
      scale_in_cooldown: 300
      
    - name: agent-utilization
      metric_type: CustomMetric
      metric_name: AgentUtilization
      target_value: 70  # Scale when > 70% agent utilization
      scale_out_cooldown: 180
      scale_in_cooldown: 600
      
    - name: response-time
      metric_type: CustomMetric
      metric_name: AverageResponseTime
      target_value: 500  # Scale when > 500ms response time
      scale_out_cooldown: 120
      scale_in_cooldown: 300
  
  capacity:
    min_capacity: 2
    max_capacity: 100
    desired_capacity: 5
    
  predictive_scaling:
    enabled: true
    forecast_window: 3600  # 1 hour
    mode: ForecastAndScale
    
  scheduled_scaling:
    - name: business-hours-scale-up
      schedule: "0 8 * * MON-FRI"
      min_capacity: 10
      max_capacity: 100
      desired_capacity: 20
      
    - name: off-hours-scale-down
      schedule: "0 18 * * MON-FRI"
      min_capacity: 2
      max_capacity: 50
      desired_capacity: 5
```

### 3. Gateway Service Scaling
```yaml
gateway_scaling:
  target_type: ECSService
  service_name: mistersmith-gateway
  
  scaling_policies:
    - name: connection-count
      metric_type: CustomMetric
      metric_name: ActiveConnectionCount
      target_value: 800  # Scale when > 800 connections per instance
      scale_out_cooldown: 180
      scale_in_cooldown: 300
      
    - name: request-rate
      metric_type: ALBRequestCountPerTarget
      target_value: 1000  # Scale when > 1000 requests per minute
      scale_out_cooldown: 120
      scale_in_cooldown: 600
      
    - name: response-time
      metric_type: ALBTargetResponseTime
      target_value: 200  # Scale when > 200ms response time
      scale_out_cooldown: 60
      scale_in_cooldown: 300
  
  capacity:
    min_capacity: 2
    max_capacity: 20
    desired_capacity: 3
```

## Aurora Auto-Scaling

### Aurora Serverless v2 Configuration
```yaml
aurora_scaling:
  cluster_identifier: mistersmith-aurora
  
  scaling_configuration:
    min_capacity: 2
    max_capacity: 32
    auto_pause: false
    
    scaling_policy:
      target_cpu: 70
      target_connections: 80  # Percentage of max connections
      
    scale_up_triggers:
      - metric: CPUUtilization
        threshold: 70
        duration: 300
        
      - metric: DatabaseConnections
        threshold: 80
        duration: 180
        
      - metric: ReadLatency
        threshold: 0.2
        duration: 300
        
    scale_down_triggers:
      - metric: CPUUtilization
        threshold: 30
        duration: 900
        
      - metric: DatabaseConnections
        threshold: 40
        duration: 600
        
  read_replicas:
    auto_scaling:
      enabled: true
      min_replicas: 1
      max_replicas: 5
      
      scale_up_policy:
        metric: ReadReplicaLag
        threshold: 1000  # 1 second
        
      scale_down_policy:
        metric: ReadReplicaLag
        threshold: 100   # 100ms
```

## Custom Metrics for Auto-Scaling

### 1. Agent-Specific Metrics
```python
# Custom metric publishing
import boto3

cloudwatch = boto3.client('cloudwatch')

def publish_agent_metrics(agent_count, pending_tasks, utilization):
    cloudwatch.put_metric_data(
        Namespace='MisterSmith/Agents',
        MetricData=[
            {
                'MetricName': 'ActiveAgentCount',
                'Value': agent_count,
                'Unit': 'Count',
                'Dimensions': [
                    {
                        'Name': 'Environment',
                        'Value': 'production'
                    }
                ]
            },
            {
                'MetricName': 'PendingTaskCount',
                'Value': pending_tasks,
                'Unit': 'Count'
            },
            {
                'MetricName': 'AgentUtilization',
                'Value': utilization,
                'Unit': 'Percent'
            }
        ]
    )
```

### 2. Application Performance Metrics
```yaml
custom_metrics:
  - name: TaskProcessingLatency
    namespace: MisterSmith/Performance
    dimensions:
      - TaskType
      - AgentId
    unit: Milliseconds
    
  - name: DiscoveryEventRate
    namespace: MisterSmith/Discovery
    dimensions:
      - EventType
      - Confidence
    unit: Count/Second
    
  - name: ConsensusLatency
    namespace: MisterSmith/Coordination
    dimensions:
      - Algorithm
      - ParticipantCount
    unit: Milliseconds
    
  - name: MessageQueueDepth
    namespace: MisterSmith/Messaging
    dimensions:
      - QueueName
      - MessageType
    unit: Count
```

## CloudWatch Dashboards

### 1. System Overview Dashboard
```json
{
  "widgets": [
    {
      "type": "metric",
      "properties": {
        "metrics": [
          ["AWS/ECS", "CPUUtilization", "ServiceName", "mistersmith-supervisor"],
          ["AWS/ECS", "MemoryUtilization", "ServiceName", "mistersmith-supervisor"],
          ["AWS/ECS", "CPUUtilization", "ServiceName", "mistersmith-worker"],
          ["AWS/ECS", "MemoryUtilization", "ServiceName", "mistersmith-worker"]
        ],
        "period": 300,
        "stat": "Average",
        "region": "us-west-2",
        "title": "ECS Service Resource Utilization"
      }
    },
    {
      "type": "metric",
      "properties": {
        "metrics": [
          ["MisterSmith/Agents", "ActiveAgentCount"],
          ["MisterSmith/Agents", "PendingTaskCount"],
          ["MisterSmith/Agents", "AgentUtilization"]
        ],
        "period": 300,
        "stat": "Average",
        "region": "us-west-2",
        "title": "Agent Metrics"
      }
    }
  ]
}
```

### 2. Performance Dashboard
```json
{
  "widgets": [
    {
      "type": "metric",
      "properties": {
        "metrics": [
          ["MisterSmith/Performance", "TaskProcessingLatency"],
          ["MisterSmith/Coordination", "ConsensusLatency"],
          ["AWS/ApplicationELB", "TargetResponseTime"]
        ],
        "period": 300,
        "stat": "Average",
        "region": "us-west-2",
        "title": "Response Time Metrics"
      }
    },
    {
      "type": "metric",
      "properties": {
        "metrics": [
          ["MisterSmith/Discovery", "DiscoveryEventRate"],
          ["MisterSmith/Messaging", "MessageQueueDepth"]
        ],
        "period": 300,
        "stat": "Sum",
        "region": "us-west-2",
        "title": "Throughput Metrics"
      }
    }
  ]
}
```

### 3. Database Dashboard
```json
{
  "widgets": [
    {
      "type": "metric",
      "properties": {
        "metrics": [
          ["AWS/RDS", "CPUUtilization", "DBClusterIdentifier", "mistersmith-aurora"],
          ["AWS/RDS", "ServerlessDatabaseCapacity", "DBClusterIdentifier", "mistersmith-aurora"],
          ["AWS/RDS", "DatabaseConnections", "DBClusterIdentifier", "mistersmith-aurora"]
        ],
        "period": 300,
        "stat": "Average",
        "region": "us-west-2",
        "title": "Aurora Metrics"
      }
    }
  ]
}
```

## Alerting Configuration

### 1. Critical Alerts
```yaml
critical_alerts:
  - name: HighCPUUtilization
    metric: AWS/ECS/CPUUtilization
    threshold: 90
    comparison: GreaterThanThreshold
    period: 300
    evaluation_periods: 2
    actions:
      - arn:aws:sns:us-west-2:123456789012:critical-alerts
      
  - name: HighMemoryUtilization
    metric: AWS/ECS/MemoryUtilization
    threshold: 95
    comparison: GreaterThanThreshold
    period: 300
    evaluation_periods: 2
    actions:
      - arn:aws:sns:us-west-2:123456789012:critical-alerts
      
  - name: DatabaseConnectionsExhausted
    metric: AWS/RDS/DatabaseConnections
    threshold: 900  # 90% of max connections
    comparison: GreaterThanThreshold
    period: 300
    evaluation_periods: 1
    actions:
      - arn:aws:sns:us-west-2:123456789012:critical-alerts
      
  - name: TaskProcessingLatencyHigh
    metric: MisterSmith/Performance/TaskProcessingLatency
    threshold: 5000  # 5 seconds
    comparison: GreaterThanThreshold
    period: 300
    evaluation_periods: 3
    actions:
      - arn:aws:sns:us-west-2:123456789012:critical-alerts
```

### 2. Warning Alerts
```yaml
warning_alerts:
  - name: ModerateResourceUtilization
    metric: AWS/ECS/CPUUtilization
    threshold: 80
    comparison: GreaterThanThreshold
    period: 900
    evaluation_periods: 2
    actions:
      - arn:aws:sns:us-west-2:123456789012:warning-alerts
      
  - name: QueueDepthIncreasing
    metric: MisterSmith/Messaging/MessageQueueDepth
    threshold: 500
    comparison: GreaterThanThreshold
    period: 300
    evaluation_periods: 3
    actions:
      - arn:aws:sns:us-west-2:123456789012:warning-alerts
      
  - name: AgentUtilizationHigh
    metric: MisterSmith/Agents/AgentUtilization
    threshold: 85
    comparison: GreaterThanThreshold
    period: 600
    evaluation_periods: 2
    actions:
      - arn:aws:sns:us-west-2:123456789012:warning-alerts
```

## Automated Remediation

### 1. Lambda-Based Auto-Remediation
```python
import boto3
import json

def lambda_handler(event, context):
    """
    Auto-remediation for common issues
    """
    
    # Parse CloudWatch alarm
    message = json.loads(event['Records'][0]['Sns']['Message'])
    alarm_name = message['AlarmName']
    
    if alarm_name == 'HighCPUUtilization':
        # Trigger ECS service scaling
        scale_ecs_service()
    elif alarm_name == 'DatabaseConnectionsExhausted':
        # Restart application connections
        restart_connection_pools()
    elif alarm_name == 'QueueDepthIncreasing':
        # Scale worker services
        scale_worker_services()
    
    return {'statusCode': 200}

def scale_ecs_service():
    ecs = boto3.client('ecs')
    
    # Get current service configuration
    response = ecs.describe_services(
        cluster='mistersmith-cluster',
        services=['mistersmith-worker']
    )
    
    current_count = response['services'][0]['desiredCount']
    new_count = min(current_count + 2, 50)  # Scale up by 2, max 50
    
    # Update service
    ecs.update_service(
        cluster='mistersmith-cluster',
        service='mistersmith-worker',
        desiredCount=new_count
    )
```

### 2. Systems Manager Automation
```yaml
automation_documents:
  - name: RestartECSService
    description: Restart ECS service when health checks fail
    document_type: Automation
    
    steps:
      - name: GetServiceStatus
        action: aws:executeAwsApi
        inputs:
          Service: ecs
          Api: DescribeServices
          cluster: mistersmith-cluster
          services: ["{{ ServiceName }}"]
          
      - name: RestartService
        action: aws:executeAwsApi
        inputs:
          Service: ecs
          Api: UpdateService
          cluster: mistersmith-cluster
          service: "{{ ServiceName }}"
          forceNewDeployment: true
```

## Monitoring Best Practices

### 1. Metric Collection Strategy
```yaml
metric_collection:
  infrastructure:
    - CPU utilization
    - Memory utilization
    - Network I/O
    - Disk I/O
    
  application:
    - Response time
    - Error rate
    - Throughput
    - Agent performance
    
  business:
    - Task completion rate
    - Discovery accuracy
    - User satisfaction
    - System efficiency
```

### 2. Alerting Fatigue Prevention
```yaml
alert_optimization:
  grouping:
    - Group related alerts
    - Set escalation paths
    - Use composite alarms
    
  tuning:
    - Adjust thresholds based on historical data
    - Use anomaly detection
    - Implement progressive alerting
    
  response:
    - Automate common remediation
    - Provide runbooks
    - Track resolution time
```

## Cost Optimization

### 1. Auto-Scaling Cost Controls
```yaml
cost_controls:
  budget_alerts:
    - monthly_budget: $5000
      threshold: 80
      actions:
        - Notify team
        - Reduce max capacity
        
  resource_scheduling:
    - Scale down non-production during off-hours
    - Use spot instances for batch workloads
    - Implement reservation strategies
    
  optimization_triggers:
    - Right-size instances monthly
    - Review scaling policies quarterly
    - Analyze cost per agent metric
```

### 2. Performance vs Cost Balance
```yaml
balancing_strategy:
  performance_targets:
    - 99.9% availability
    - < 500ms response time
    - < 5% error rate
    
  cost_targets:
    - < $0.10 per agent per hour
    - < 50% waste in provisioned capacity
    - > 80% resource utilization
    
  optimization_actions:
    - Implement predictive scaling
    - Use mixed instance types
    - Optimize data transfer costs
```