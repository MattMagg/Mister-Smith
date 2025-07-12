# ECS Fargate Code Examples for MisterSmith

## 1. Rust Implementation Changes

### 1.1 Queue-Based Worker Pattern

```rust
// Before: Process-based supervision
pub struct WorkerSupervisor {
    children: Vec<tokio::process::Child>,
}

impl WorkerSupervisor {
    pub async fn spawn_worker(&mut self) -> Result<(), Error> {
        let child = tokio::process::Command::new("./worker")
            .spawn()?;
        self.children.push(child);
        Ok(())
    }
}

// After: Queue-based worker pattern for Fargate
use aws_sdk_sqs::{Client as SqsClient, types::Message};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkTask {
    pub id: String,
    pub task_type: String,
    pub payload: serde_json::Value,
}

pub struct FargateWorker {
    sqs_client: SqsClient,
    queue_url: String,
}

impl FargateWorker {
    pub async fn run(&self) -> Result<(), Error> {
        loop {
            // Poll for messages
            let messages = self.receive_messages().await?;
            
            for message in messages {
                // Process task
                match self.process_message(&message).await {
                    Ok(_) => self.delete_message(&message).await?,
                    Err(e) => {
                        error!("Failed to process message: {}", e);
                        // Message will be retried via visibility timeout
                    }
                }
            }
            
            // Short sleep if no messages
            if messages.is_empty() {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    async fn receive_messages(&self) -> Result<Vec<Message>, Error> {
        let resp = self.sqs_client
            .receive_message()
            .queue_url(&self.queue_url)
            .max_number_of_messages(10)
            .wait_time_seconds(20) // Long polling
            .send()
            .await?;
            
        Ok(resp.messages.unwrap_or_default())
    }
    
    async fn process_message(&self, message: &Message) -> Result<(), Error> {
        let body = message.body.as_ref().ok_or("Empty message")?;
        let task: WorkTask = serde_json::from_str(body)?;
        
        match task.task_type.as_str() {
            "compile" => self.handle_compile_task(task).await,
            "analyze" => self.handle_analyze_task(task).await,
            "execute" => self.handle_execute_task(task).await,
            _ => Err(Error::UnknownTaskType(task.task_type)),
        }
    }
}
```

### 1.2 Health Check Implementation

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::json;

#[derive(Clone)]
pub struct HealthState {
    pub service_ready: Arc<RwLock<bool>>,
    pub dependencies_healthy: Arc<RwLock<bool>>,
}

pub fn create_health_router(state: HealthState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .with_state(state)
}

async fn health_check() -> StatusCode {
    // Basic liveness check - container is running
    StatusCode::OK
}

async fn readiness_check(State(state): State<HealthState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let service_ready = *state.service_ready.read().await;
    let deps_healthy = *state.dependencies_healthy.read().await;
    
    if service_ready && deps_healthy {
        Ok(Json(json!({
            "status": "ready",
            "service": service_ready,
            "dependencies": deps_healthy
        })))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}
```

### 1.3 Service Discovery Integration

```rust
use aws_sdk_servicediscovery::{Client as ServiceDiscoveryClient};
use std::collections::HashMap;

pub struct ServiceResolver {
    client: ServiceDiscoveryClient,
    namespace: String,
    cache: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl ServiceResolver {
    pub async fn resolve_service(&self, service_name: &str) -> Result<Vec<String>, Error> {
        // Check cache first
        if let Some(endpoints) = self.cache.read().await.get(service_name) {
            if !endpoints.is_empty() {
                return Ok(endpoints.clone());
            }
        }
        
        // Query service discovery
        let instances = self.client
            .discover_instances()
            .namespace_name(&self.namespace)
            .service_name(service_name)
            .send()
            .await?;
            
        let endpoints: Vec<String> = instances
            .instances
            .unwrap_or_default()
            .into_iter()
            .filter_map(|instance| {
                let attrs = instance.attributes?;
                let ip = attrs.get("AWS_INSTANCE_IPV4")?;
                let port = attrs.get("AWS_INSTANCE_PORT")?;
                Some(format!("{}:{}", ip, port))
            })
            .collect();
            
        // Update cache
        self.cache.write().await.insert(service_name.to_string(), endpoints.clone());
        
        Ok(endpoints)
    }
}
```

## 2. CDK Infrastructure Patterns

### 2.1 Complete Fargate Service Pattern

```typescript
import * as cdk from 'aws-cdk-lib';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as ecs_patterns from 'aws-cdk-lib/aws-ecs-patterns';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as cloudwatch from 'aws-cdk-lib/aws-cloudwatch';

export interface MisterSmithServiceProps {
  cluster: ecs.Cluster;
  serviceName: string;
  cpu: number;
  memory: number;
  desiredCount: number;
  maxCount: number;
  image: ecs.ContainerImage;
  environment?: { [key: string]: string };
  useSpot?: boolean;
}

export class MisterSmithFargateService extends Construct {
  public readonly service: ecs.FargateService;
  public readonly taskDefinition: ecs.FargateTaskDefinition;
  
  constructor(scope: Construct, id: string, props: MisterSmithServiceProps) {
    super(scope, id);
    
    // Task Definition
    this.taskDefinition = new ecs.FargateTaskDefinition(this, 'TaskDef', {
      cpu: props.cpu,
      memoryLimitMiB: props.memory,
      runtimePlatform: {
        operatingSystemFamily: ecs.OperatingSystemFamily.LINUX,
        cpuArchitecture: ecs.CpuArchitecture.X86_64
      }
    });
    
    // Add policies
    this.taskDefinition.addToTaskRolePolicy(new iam.PolicyStatement({
      effect: iam.Effect.ALLOW,
      actions: [
        'sqs:ReceiveMessage',
        'sqs:DeleteMessage',
        'sqs:GetQueueAttributes'
      ],
      resources: ['*']
    }));
    
    // Container
    const container = this.taskDefinition.addContainer('app', {
      image: props.image,
      logging: ecs.LogDrivers.awsLogs({
        streamPrefix: props.serviceName,
        logRetention: logs.RetentionDays.ONE_WEEK
      }),
      environment: {
        RUST_LOG: 'info',
        SERVICE_NAME: props.serviceName,
        ...props.environment
      },
      healthCheck: {
        command: ['CMD-SHELL', 'curl -f http://localhost:8080/health || exit 1'],
        interval: cdk.Duration.seconds(30),
        timeout: cdk.Duration.seconds(10),
        retries: 3,
        startPeriod: cdk.Duration.seconds(60)
      }
    });
    
    container.addPortMappings({
      containerPort: 8080,
      protocol: ecs.Protocol.TCP
    });
    
    // Service with mixed capacity
    const capacityProviderStrategies: ecs.CapacityProviderStrategy[] = props.useSpot ? [
      {
        capacityProvider: 'FARGATE_SPOT',
        weight: 2,
        base: 0
      },
      {
        capacityProvider: 'FARGATE',
        weight: 1,
        base: props.desiredCount
      }
    ] : [
      {
        capacityProvider: 'FARGATE',
        weight: 1,
        base: props.desiredCount
      }
    ];
    
    this.service = new ecs.FargateService(this, 'Service', {
      cluster: props.cluster,
      taskDefinition: this.taskDefinition,
      desiredCount: props.desiredCount,
      serviceName: props.serviceName,
      capacityProviderStrategies,
      enableLogging: true,
      circuitBreaker: {
        rollback: true
      }
    });
    
    // Auto-scaling
    const scaling = this.service.autoScaleTaskCount({
      minCapacity: props.desiredCount,
      maxCapacity: props.maxCount
    });
    
    scaling.scaleOnCpuUtilization('CpuScaling', {
      targetUtilizationPercent: 70
    });
    
    scaling.scaleOnMemoryUtilization('MemoryScaling', {
      targetUtilizationPercent: 80
    });
    
    // CloudWatch Dashboard
    new cloudwatch.Dashboard(this, 'Dashboard', {
      dashboardName: `${props.serviceName}-dashboard`,
      widgets: [
        [
          new cloudwatch.GraphWidget({
            title: 'CPU Utilization',
            left: [this.service.metricCpuUtilization()]
          }),
          new cloudwatch.GraphWidget({
            title: 'Memory Utilization',
            left: [this.service.metricMemoryUtilization()]
          })
        ],
        [
          new cloudwatch.GraphWidget({
            title: 'Task Count',
            left: [this.service.metricTaskCount()]
          })
        ]
      ]
    });
  }
}
```

### 2.2 Blue/Green Deployment Pattern

```typescript
export class BlueGreenFargateService extends Construct {
  constructor(scope: Construct, id: string, props: BlueGreenProps) {
    super(scope, id);
    
    // Application Load Balancer
    const alb = new elbv2.ApplicationLoadBalancer(this, 'ALB', {
      vpc: props.vpc,
      internetFacing: true
    });
    
    // Target Groups
    const blueTargetGroup = new elbv2.ApplicationTargetGroup(this, 'BlueTarget', {
      vpc: props.vpc,
      port: 80,
      targetType: elbv2.TargetType.IP,
      healthCheck: {
        path: '/health',
        interval: cdk.Duration.seconds(30)
      }
    });
    
    const greenTargetGroup = new elbv2.ApplicationTargetGroup(this, 'GreenTarget', {
      vpc: props.vpc,
      port: 80,
      targetType: elbv2.TargetType.IP,
      healthCheck: {
        path: '/health',
        interval: cdk.Duration.seconds(30)
      }
    });
    
    // Listener with rules
    const listener = alb.addListener('Listener', {
      port: 80,
      defaultTargetGroups: [blueTargetGroup]
    });
    
    // CodeDeploy application
    const application = new codedeploy.EcsApplication(this, 'CodeDeployApp', {
      applicationName: `${props.serviceName}-app`
    });
    
    // Deployment group
    new codedeploy.EcsDeploymentGroup(this, 'DeploymentGroup', {
      application,
      service: props.service,
      blueGreenDeploymentConfig: {
        blueTargetGroup,
        greenTargetGroup,
        listener,
        deploymentApprovalWaitTime: cdk.Duration.minutes(5),
        terminationWaitTime: cdk.Duration.minutes(5)
      },
      deploymentConfig: codedeploy.EcsDeploymentConfig.CANARY_10PERCENT_5MINUTES
    });
  }
}
```

### 2.3 Multi-Region Pattern

```typescript
export class MultiRegionMisterSmith extends cdk.Stack {
  constructor(scope: Construct, id: string, props: MultiRegionProps) {
    super(scope, id, props);
    
    // Global DynamoDB table for state
    const globalTable = new dynamodb.Table(this, 'GlobalState', {
      tableName: 'mistersmith-global-state',
      partitionKey: { name: 'id', type: dynamodb.AttributeType.STRING },
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      replicationRegions: ['us-west-2', 'eu-west-1'],
      stream: dynamodb.StreamViewType.NEW_AND_OLD_IMAGES
    });
    
    // Route 53 for global routing
    const hostedZone = new route53.HostedZone(this, 'GlobalZone', {
      zoneName: 'mistersmith.example.com'
    });
    
    // Health checks for each region
    const healthChecks = props.regions.map(region => {
      return new route53.HealthCheck(this, `HealthCheck-${region}`, {
        type: route53.HealthCheckType.HTTPS,
        resourcePath: '/health',
        domainName: `${region}.mistersmith.example.com`
      });
    });
    
    // Failover routing
    new route53.RecordSet(this, 'FailoverRecord', {
      zone: hostedZone,
      recordName: 'api',
      recordType: route53.RecordType.A,
      target: route53.RecordTarget.fromAlias(new targets.LoadBalancerTarget(props.primaryAlb)),
      setIdentifier: 'Primary',
      healthCheckId: healthChecks[0].healthCheckId,
      routingPolicy: route53.RoutingPolicy.failover({
        failoverConfig: {
          priority: route53.FailoverPriority.PRIMARY
        }
      })
    });
  }
}
```

## 3. Queue-Based Scaling Configuration

### 3.1 SQS Integration

```typescript
// Queue setup with DLQ
const dlq = new sqs.Queue(this, 'WorkerDLQ', {
  retentionPeriod: cdk.Duration.days(14)
});

const workerQueue = new sqs.Queue(this, 'WorkerQueue', {
  visibilityTimeout: cdk.Duration.seconds(300),
  deadLetterQueue: {
    maxReceiveCount: 3,
    queue: dlq
  },
  encryption: sqs.QueueEncryption.AWS_MANAGED
});

// Custom metric for queue depth
const queueDepthMetric = new cloudwatch.Metric({
  namespace: 'AWS/SQS',
  metricName: 'ApproximateNumberOfMessagesVisible',
  dimensionsMap: {
    QueueName: workerQueue.queueName
  },
  statistic: 'Average',
  period: cdk.Duration.minutes(1)
});

// Step scaling based on queue depth
const workerScaling = workerService.autoScaleTaskCount({
  minCapacity: 2,
  maxCapacity: 1000
});

workerScaling.scaleOnMetric('QueueScaling', {
  metric: queueDepthMetric,
  scalingSteps: [
    { upper: 0, change: -1 },
    { lower: 10, change: +1 },
    { lower: 100, change: +5 },
    { lower: 1000, change: +20 },
    { lower: 5000, change: +50 }
  ],
  adjustmentType: autoscaling.AdjustmentType.CHANGE_IN_CAPACITY,
  cooldown: cdk.Duration.seconds(60)
});
```

### 3.2 EventBridge Integration

```typescript
// EventBridge for async communication
const eventBus = new events.EventBus(this, 'MisterSmithBus', {
  eventBusName: 'mistersmith-events'
});

// Rule for worker events
new events.Rule(this, 'WorkerEventRule', {
  eventBus: eventBus,
  eventPattern: {
    source: ['mistersmith.orchestrator'],
    detailType: ['Worker Task']
  },
  targets: [
    new targets.SqsQueue(workerQueue, {
      messageGroupId: 'worker-tasks'
    })
  ]
});
```

## 4. Monitoring and Observability

### 4.1 X-Ray Tracing

```rust
use aws_sdk_xray::{Client as XRayClient};
use opentelemetry::trace::{Tracer, SpanKind};
use opentelemetry_aws::trace::XrayPropagator;

pub fn init_tracing() -> Result<(), Error> {
    // Configure X-Ray exporter
    opentelemetry::global::set_text_map_propagator(XrayPropagator::new());
    
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint("http://localhost:4318")
        )
        .install_batch(opentelemetry::runtime::Tokio)?;
    
    Ok(())
}

// Instrument functions
#[tracing::instrument]
pub async fn process_task(task: WorkTask) -> Result<(), Error> {
    let tracer = opentelemetry::global::tracer("mistersmith-worker");
    let mut span = tracer.start_with_context("process_task", &Context::current());
    span.set_attribute(KeyValue::new("task.id", task.id.clone()));
    span.set_attribute(KeyValue::new("task.type", task.task_type.clone()));
    
    // Process task...
    
    Ok(())
}
```

### 4.2 CloudWatch Insights Queries

```sql
-- Find slow worker tasks
fields @timestamp, @message
| filter @message like /task_duration/
| parse @message /"task_id":"(?<taskId>[^"]+)"/
| parse @message /"duration_ms":(?<duration>\d+)/
| filter duration > 10000
| sort @timestamp desc
| limit 100

-- Analyze error patterns
fields @timestamp, @message, @logStream
| filter @message like /ERROR/
| parse @message /"error_type":"(?<errorType>[^"]+)"/
| stats count() by errorType
| sort count desc

-- Track scaling events
fields @timestamp, @message
| filter @message like /scaling_event/
| parse @message /"desired_count":(?<desired>\d+)/
| parse @message /"running_count":(?<running>\d+)/
| sort @timestamp desc
```

## 5. Security Best Practices

### 5.1 Task Role Permissions

```typescript
// Least privilege task role
const taskRole = new iam.Role(this, 'TaskRole', {
  assumedBy: new iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
  inlinePolicies: {
    WorkerPolicy: new iam.PolicyDocument({
      statements: [
        new iam.PolicyStatement({
          effect: iam.Effect.ALLOW,
          actions: [
            'sqs:ReceiveMessage',
            'sqs:DeleteMessage',
            'sqs:GetQueueAttributes'
          ],
          resources: [workerQueue.queueArn]
        }),
        new iam.PolicyStatement({
          effect: iam.Effect.ALLOW,
          actions: [
            's3:GetObject',
            's3:PutObject'
          ],
          resources: [`${workBucket.bucketArn}/*`]
        }),
        new iam.PolicyStatement({
          effect: iam.Effect.ALLOW,
          actions: [
            'kms:Decrypt',
            'kms:GenerateDataKey'
          ],
          resources: [kmsKey.keyArn]
        })
      ]
    })
  }
});
```

### 5.2 Network Security

```typescript
// Security group with minimal access
const serviceSecurityGroup = new ec2.SecurityGroup(this, 'ServiceSG', {
  vpc: props.vpc,
  description: 'Security group for MisterSmith service',
  allowAllOutbound: false
});

// Only allow health checks from ALB
serviceSecurityGroup.addIngressRule(
  ec2.Peer.securityGroupId(albSecurityGroup.securityGroupId),
  ec2.Port.tcp(8080),
  'Allow health checks from ALB'
);

// Egress only to required services
serviceSecurityGroup.addEgressRule(
  ec2.Peer.ipv4('10.0.0.0/16'),
  ec2.Port.tcp(443),
  'Allow HTTPS to VPC endpoints'
);
```

## 6. Cost Optimization

### 6.1 Compute Savings Plans

```typescript
// Tag resources for cost allocation
cdk.Tags.of(this).add('Project', 'MisterSmith');
cdk.Tags.of(this).add('Environment', props.environment);
cdk.Tags.of(this).add('CostCenter', 'Engineering');
cdk.Tags.of(this).add('ManagedBy', 'CDK');

// Use Fargate Spot for non-critical workloads
const spotStrategy: ecs.CapacityProviderStrategy = [
  {
    capacityProvider: 'FARGATE_SPOT',
    weight: 4,
    base: 0
  },
  {
    capacityProvider: 'FARGATE',
    weight: 1,
    base: 2 // Minimum on-demand tasks
  }
];
```

### 6.2 Resource Right-Sizing

```typescript
// CloudWatch dashboard for right-sizing analysis
const sizingDashboard = new cloudwatch.Dashboard(this, 'SizingDashboard', {
  widgets: [
    [
      new cloudwatch.GraphWidget({
        title: 'CPU Utilization vs Reservation',
        left: [service.metricCpuUtilization()],
        leftAnnotations: [
          { value: 70, color: cloudwatch.Color.RED, label: 'Target' }
        ]
      })
    ],
    [
      new cloudwatch.GraphWidget({
        title: 'Memory Utilization vs Reservation',
        left: [service.metricMemoryUtilization()],
        leftAnnotations: [
          { value: 80, color: cloudwatch.Color.RED, label: 'Target' }
        ]
      })
    ]
  ]
});
```

This completes the comprehensive ECS Fargate compatibility analysis and implementation guide for MisterSmith.