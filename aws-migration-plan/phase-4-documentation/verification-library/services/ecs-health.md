# ECS Health Check Commands

## Overview
Comprehensive commands for verifying ECS cluster, services, tasks, and container health.

## Prerequisites
- AWS CLI with ECS permissions
- jq for JSON parsing
- Docker CLI (optional, for container inspection)
- Session Manager plugin for container access

## Commands

### 1. Cluster Health Verification
```bash
# Get cluster status
aws ecs describe-clusters \
  --clusters mistersmith-prod \
  --query 'clusters[0].[clusterName,status,registeredContainerInstancesCount,runningTasksCount,pendingTasksCount,activeServicesCount]' \
  --output table

# Check cluster capacity providers
aws ecs describe-clusters \
  --clusters mistersmith-prod \
  --include ATTACHMENTS \
  --query 'clusters[0].capacityProviders' \
  --output json

# Verify cluster metrics
function check_cluster_metrics() {
    local cluster_name=$1
    aws cloudwatch get-metric-statistics \
      --namespace AWS/ECS \
      --metric-name CPUUtilization \
      --dimensions Name=ClusterName,Value=$cluster_name \
      --start-time $(date -u -d '10 minutes ago' +%Y-%m-%dT%H:%M:%S) \
      --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
      --period 300 \
      --statistics Average,Maximum \
      --output json | jq '.Datapoints | sort_by(.Timestamp) | .[-1]'
}
```

### 2. Service Health Checks
```bash
# List all services with health status
aws ecs list-services \
  --cluster mistersmith-prod \
  --query 'serviceArns[]' \
  --output text | xargs -I {} aws ecs describe-services \
  --cluster mistersmith-prod \
  --services {} \
  --query 'services[].[serviceName,status,runningCount,desiredCount,pendingCount,deployments[0].rolloutState]' \
  --output table

# Detailed service health check
function verify_service_health() {
    local cluster=$1
    local service=$2
    
    echo "Checking service: $service"
    
    # Get service details
    aws ecs describe-services \
      --cluster $cluster \
      --services $service \
      --query 'services[0]' > /tmp/service-details.json
    
    # Check deployment status
    deployment_status=$(jq -r '.deployments[0].status' /tmp/service-details.json)
    echo "Deployment status: $deployment_status"
    
    # Check task health
    running_count=$(jq -r '.runningCount' /tmp/service-details.json)
    desired_count=$(jq -r '.desiredCount' /tmp/service-details.json)
    echo "Tasks: $running_count/$desired_count running"
    
    # Check load balancer health
    target_group_arn=$(jq -r '.loadBalancers[0].targetGroupArn // empty' /tmp/service-details.json)
    if [ ! -z "$target_group_arn" ]; then
        echo "Checking target group health..."
        aws elbv2 describe-target-health \
          --target-group-arn $target_group_arn \
          --query 'TargetHealthDescriptions[].[Target.Id,TargetHealth.State,TargetHealth.Reason]' \
          --output table
    fi
}

# Monitor service events
aws ecs describe-services \
  --cluster mistersmith-prod \
  --services agent-orchestrator \
  --query 'services[0].events[0:5].[createdAt,message]' \
  --output table
```

### 3. Task Health Verification
```bash
# List running tasks
aws ecs list-tasks \
  --cluster mistersmith-prod \
  --desired-status RUNNING \
  --query 'taskArns' \
  --output json | jq -r '.[]' | while read task_arn; do
    aws ecs describe-tasks \
      --cluster mistersmith-prod \
      --tasks $task_arn \
      --query 'tasks[0].[taskDefinitionArn,lastStatus,healthStatus,cpu,memory]' \
      --output table
done

# Check task health by service
function check_service_tasks() {
    local cluster=$1
    local service=$2
    
    # Get tasks for service
    task_arns=$(aws ecs list-tasks \
      --cluster $cluster \
      --service-name $service \
      --query 'taskArns' \
      --output json)
    
    # Check each task
    echo "$task_arns" | jq -r '.[]' | while read task_arn; do
        echo "Checking task: $task_arn"
        aws ecs describe-tasks \
          --cluster $cluster \
          --tasks $task_arn \
          --query 'tasks[0].[lastStatus,healthStatus,stoppedReason]' \
          --output json
    done
}

# Get task failure reasons
aws ecs describe-tasks \
  --cluster mistersmith-prod \
  --tasks $(aws ecs list-tasks --cluster mistersmith-prod --desired-status STOPPED --query 'taskArns[0:5]' --output json | jq -r '.[]') \
  --query 'tasks[].[taskArn,stoppedReason,stopCode]' \
  --output table
```

### 4. Container Health Checks
```bash
# Check container health within tasks
function verify_container_health() {
    local cluster=$1
    local task_arn=$2
    
    # Get container details
    aws ecs describe-tasks \
      --cluster $cluster \
      --tasks $task_arn \
      --query 'tasks[0].containers[].[name,lastStatus,healthStatus,exitCode,reason]' \
      --output table
}

# Execute command in container for health check
function exec_health_check() {
    local cluster=$1
    local task_arn=$2
    local container_name=$3
    
    aws ecs execute-command \
      --cluster $cluster \
      --task $task_arn \
      --container $container_name \
      --interactive \
      --command "/health-check.sh"
}

# Check container logs
function check_container_logs() {
    local log_group=$1
    local log_stream=$2
    
    aws logs get-log-events \
      --log-group-name $log_group \
      --log-stream-name $log_stream \
      --start-time $(date -d '5 minutes ago' +%s)000 \
      --limit 50 \
      --query 'events[].[timestamp,message]' \
      --output text | tail -20
}
```

### 5. Load Balancer Target Health
```bash
# Check ALB target health for ECS services
function check_alb_targets() {
    local target_group_arn=$1
    
    # Get target health
    aws elbv2 describe-target-health \
      --target-group-arn $target_group_arn \
      --query 'TargetHealthDescriptions[]' \
      --output json | jq -r '.[] | "\(.Target.Id) - \(.TargetHealth.State) - \(.TargetHealth.Reason // "N/A")"'
    
    # Get unhealthy target details
    aws elbv2 describe-target-health \
      --target-group-arn $target_group_arn \
      --query 'TargetHealthDescriptions[?TargetHealth.State!=`healthy`]' \
      --output json
}

# Check health check configuration
aws elbv2 describe-target-groups \
  --target-group-arns arn:aws:elasticloadbalancing:region:account:targetgroup/name/id \
  --query 'TargetGroups[0].[HealthCheckPath,HealthCheckIntervalSeconds,HealthyThresholdCount,UnhealthyThresholdCount,HealthCheckTimeoutSeconds]' \
  --output table
```

### 6. Auto Scaling Health
```bash
# Check service auto scaling
function verify_auto_scaling() {
    local cluster=$1
    local service=$2
    local resource_id="service/$cluster/$service"
    
    # Get scaling policies
    aws application-autoscaling describe-scaling-policies \
      --service-namespace ecs \
      --resource-id $resource_id \
      --query 'ScalingPolicies[].[PolicyName,PolicyType,TargetTrackingScalingPolicyConfiguration.TargetValue]' \
      --output table
    
    # Get scaling activities
    aws application-autoscaling describe-scaling-activities \
      --service-namespace ecs \
      --resource-id $resource_id \
      --max-results 10 \
      --query 'ScalingActivities[].[StartTime,StatusCode,StatusMessage]' \
      --output table
}

# Check current scaling metrics
function check_scaling_metrics() {
    local cluster=$1
    local service=$2
    
    # CPU utilization
    aws cloudwatch get-metric-statistics \
      --namespace AWS/ECS \
      --metric-name CPUUtilization \
      --dimensions Name=ServiceName,Value=$service Name=ClusterName,Value=$cluster \
      --start-time $(date -u -d '30 minutes ago' +%Y-%m-%dT%H:%M:%S) \
      --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
      --period 300 \
      --statistics Average \
      --output json | jq '.Datapoints | sort_by(.Timestamp) | .[-6:]'
}
```

### 7. Container Insights Verification
```bash
# Check if Container Insights is enabled
aws ecs describe-clusters \
  --clusters mistersmith-prod \
  --query 'clusters[0].settings[?name==`containerInsights`].value' \
  --output text

# Query Container Insights metrics
function query_container_insights() {
    local cluster=$1
    local metric=$2  # e.g., TaskCount, ServiceCount, CPUUtilization
    
    aws cloudwatch get-metric-statistics \
      --namespace ECS/ContainerInsights \
      --metric-name $metric \
      --dimensions Name=ClusterName,Value=$cluster \
      --start-time $(date -u -d '1 hour ago' +%Y-%m-%dT%H:%M:%S) \
      --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
      --period 300 \
      --statistics Average,Maximum,Minimum \
      --output json
}
```

### 8. Comprehensive ECS Health Report
```bash
#!/bin/bash
# ecs-health-report.sh - Generate comprehensive ECS health report

function generate_ecs_health_report() {
    local cluster=$1
    local report_file="ecs-health-$(date +%Y%m%d-%H%M%S).json"
    
    echo "Generating ECS health report for cluster: $cluster"
    
    # Initialize report structure
    cat > $report_file << EOF
{
  "cluster": "$cluster",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "summary": {},
  "services": [],
  "issues": []
}
EOF
    
    # Get cluster summary
    cluster_info=$(aws ecs describe-clusters --clusters $cluster --query 'clusters[0]')
    echo "$cluster_info" | jq '{
      status: .status,
      registeredContainerInstances: .registeredContainerInstancesCount,
      runningTasks: .runningTasksCount,
      pendingTasks: .pendingTasksCount,
      activeServices: .activeServicesCount
    }' > /tmp/cluster-summary.json
    
    jq --slurpfile summary /tmp/cluster-summary.json '.summary = $summary[0]' $report_file > /tmp/report.json && mv /tmp/report.json $report_file
    
    # Check each service
    aws ecs list-services --cluster $cluster --query 'serviceArns[]' --output text | while read service_arn; do
        service_name=$(basename $service_arn)
        echo "Checking service: $service_name"
        
        service_info=$(aws ecs describe-services \
          --cluster $cluster \
          --services $service_arn \
          --query 'services[0]')
        
        service_health=$(echo "$service_info" | jq '{
          name: .serviceName,
          status: .status,
          desired: .desiredCount,
          running: .runningCount,
          pending: .pendingCount,
          deploymentStatus: .deployments[0].status,
          healthCheckGracePeriod: .healthCheckGracePeriodSeconds
        }')
        
        # Add to report
        jq --argjson service "$service_health" '.services += [$service]' $report_file > /tmp/report.json && mv /tmp/report.json $report_file
        
        # Check for issues
        if [ $(echo "$service_info" | jq '.runningCount < .desiredCount') = "true" ]; then
            issue="{\"type\": \"service_degraded\", \"service\": \"$service_name\", \"message\": \"Running count below desired count\"}"
            jq --argjson issue "$issue" '.issues += [$issue]' $report_file > /tmp/report.json && mv /tmp/report.json $report_file
        fi
    done
    
    # Generate summary
    echo "Health report generated: $report_file"
    
    # Print summary
    echo ""
    echo "=== ECS HEALTH SUMMARY ==="
    jq -r '
      "Cluster: \(.cluster)",
      "Status: \(.summary.status)",
      "Services: \(.summary.activeServices)",
      "Running Tasks: \(.summary.runningTasks)",
      "Issues Found: \(.issues | length)",
      "",
      "Service Status:",
      (.services[] | "  - \(.name): \(.running)/\(.desired) tasks (\(.status))")
    ' $report_file
}

# Run the report
generate_ecs_health_report "mistersmith-prod"
```

### 9. Real-time Health Monitoring
```bash
#!/bin/bash
# ecs-monitor.sh - Real-time ECS health monitoring

function monitor_ecs_health() {
    local cluster=$1
    local interval=${2:-30}  # Default 30 seconds
    
    while true; do
        clear
        echo "=== ECS Cluster Monitor: $cluster ==="
        echo "Time: $(date)"
        echo ""
        
        # Cluster status
        aws ecs describe-clusters \
          --clusters $cluster \
          --query 'clusters[0].[status,runningTasksCount,pendingTasksCount,activeServicesCount]' \
          --output text | awk '{print "Cluster Status:", $1, "| Tasks:", $2, "running,", $3, "pending | Services:", $4}'
        
        echo ""
        echo "Service Status:"
        aws ecs list-services --cluster $cluster --query 'serviceArns[]' --output text | while read service_arn; do
            service_name=$(basename $service_arn)
            aws ecs describe-services \
              --cluster $cluster \
              --services $service_arn \
              --query 'services[0].[runningCount,desiredCount,pendingCount]' \
              --output text | awk -v service=$service_name '{printf "  %-30s %d/%d running, %d pending\n", service, $1, $2, $3}'
        done
        
        echo ""
        echo "Recent Events:"
        aws ecs describe-services \
          --cluster $cluster \
          --services $(aws ecs list-services --cluster $cluster --query 'serviceArns[0]' --output text) \
          --query 'services[0].events[0:3].[createdAt,message]' \
          --output text | while read timestamp message; do
            echo "  $(date -d "$timestamp" '+%H:%M:%S') - $message"
        done
        
        echo ""
        echo "Press Ctrl+C to exit. Refreshing in $interval seconds..."
        sleep $interval
    done
}

# Start monitoring
monitor_ecs_health "mistersmith-prod" 30
```

## Troubleshooting Guide

### Common Issues

1. **Tasks Failing to Start**
   ```bash
   # Check task stopped reasons
   aws ecs describe-tasks \
     --cluster mistersmith-prod \
     --tasks $(aws ecs list-tasks --cluster mistersmith-prod --desired-status STOPPED --query 'taskArns[0:10]' --output json | jq -r '.[]') \
     --query 'tasks[].[taskArn,stoppedReason,containers[0].reason]' \
     --output table
   ```

2. **Health Check Failures**
   ```bash
   # Verify health check endpoint
   task_ip=$(aws ecs describe-tasks --cluster cluster-name --tasks task-arn --query 'tasks[0].containers[0].networkInterfaces[0].privateIpv4Address' --output text)
   curl -f http://$task_ip:8080/health || echo "Health check failed"
   ```

3. **Memory/CPU Issues**
   ```bash
   # Check resource utilization
   aws ecs describe-tasks \
     --cluster mistersmith-prod \
     --tasks task-arn \
     --query 'tasks[0].[cpu,memory,containers[0].[name,cpu,memory]]' \
     --output json
   ```