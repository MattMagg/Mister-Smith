# AWS API Specifications for MisterSmith Migration

## Overview
This document contains exact AWS API calls and configurations for migrating MisterSmith from Railway to AWS infrastructure.

## 1. VPC Configuration

### 1.1 Create VPC
```json
{
  "Action": "CreateVpc",
  "Version": "2016-11-15",
  "Request": {
    "CidrBlock": "10.0.0.0/16",
    "TagSpecifications": [
      {
        "ResourceType": "vpc",
        "Tags": [
          {"Key": "Name", "Value": "mistersmith-vpc"},
          {"Key": "Environment", "Value": "production"},
          {"Key": "ManagedBy", "Value": "terraform"}
        ]
      }
    ],
    "AmazonProvidedIpv6CidrBlock": true,
    "EnableDnsHostnames": true,
    "EnableDnsSupport": true
  },
  "ExpectedResponse": {
    "Vpc": {
      "VpcId": "vpc-xxxxxxxxx",
      "State": "pending",
      "CidrBlock": "10.0.0.0/16",
      "Ipv6CidrBlockAssociationSet": [{
        "AssociationId": "vpc-cidr-assoc-xxxxxxxxx",
        "Ipv6CidrBlock": "2600:xxxx:xxxx:xxxx::/56",
        "Ipv6CidrBlockState": {"State": "associating"}
      }]
    }
  }
}
```

### 1.2 Create Subnets
```json
{
  "Action": "CreateSubnet",
  "Version": "2016-11-15",
  "PublicSubnets": [
    {
      "Request": {
        "VpcId": "${VPC_ID}",
        "CidrBlock": "10.0.1.0/24",
        "AvailabilityZone": "us-east-1a",
        "MapPublicIpOnLaunch": true,
        "TagSpecifications": [{
          "ResourceType": "subnet",
          "Tags": [
            {"Key": "Name", "Value": "mistersmith-public-subnet-1a"},
            {"Key": "Type", "Value": "public"}
          ]
        }]
      }
    },
    {
      "Request": {
        "VpcId": "${VPC_ID}",
        "CidrBlock": "10.0.2.0/24",
        "AvailabilityZone": "us-east-1b",
        "MapPublicIpOnLaunch": true,
        "TagSpecifications": [{
          "ResourceType": "subnet",
          "Tags": [
            {"Key": "Name", "Value": "mistersmith-public-subnet-1b"},
            {"Key": "Type", "Value": "public"}
          ]
        }]
      }
    }
  ],
  "PrivateSubnets": [
    {
      "Request": {
        "VpcId": "${VPC_ID}",
        "CidrBlock": "10.0.11.0/24",
        "AvailabilityZone": "us-east-1a",
        "TagSpecifications": [{
          "ResourceType": "subnet",
          "Tags": [
            {"Key": "Name", "Value": "mistersmith-private-subnet-1a"},
            {"Key": "Type", "Value": "private"}
          ]
        }]
      }
    },
    {
      "Request": {
        "VpcId": "${VPC_ID}",
        "CidrBlock": "10.0.12.0/24",
        "AvailabilityZone": "us-east-1b",
        "TagSpecifications": [{
          "ResourceType": "subnet",
          "Tags": [
            {"Key": "Name", "Value": "mistersmith-private-subnet-1b"},
            {"Key": "Type", "Value": "private"}
          ]
        }]
      }
    }
  ]
}
```

### 1.3 Create Security Groups
```json
{
  "Action": "CreateSecurityGroup",
  "Version": "2016-11-15",
  "Groups": [
    {
      "Name": "alb-security-group",
      "Request": {
        "GroupName": "mistersmith-alb-sg",
        "Description": "Security group for Application Load Balancer",
        "VpcId": "${VPC_ID}",
        "TagSpecifications": [{
          "ResourceType": "security-group",
          "Tags": [{"Key": "Name", "Value": "mistersmith-alb-sg"}]
        }]
      },
      "IngressRules": [
        {
          "IpProtocol": "tcp",
          "FromPort": 80,
          "ToPort": 80,
          "CidrIp": "0.0.0.0/0",
          "Description": "HTTP from anywhere"
        },
        {
          "IpProtocol": "tcp",
          "FromPort": 443,
          "ToPort": 443,
          "CidrIp": "0.0.0.0/0",
          "Description": "HTTPS from anywhere"
        }
      ]
    },
    {
      "Name": "ecs-service-security-group",
      "Request": {
        "GroupName": "mistersmith-ecs-service-sg",
        "Description": "Security group for ECS services",
        "VpcId": "${VPC_ID}"
      },
      "IngressRules": [
        {
          "IpProtocol": "tcp",
          "FromPort": 0,
          "ToPort": 65535,
          "SourceSecurityGroupId": "${ALB_SG_ID}",
          "Description": "All traffic from ALB"
        }
      ]
    },
    {
      "Name": "rds-security-group",
      "Request": {
        "GroupName": "mistersmith-rds-sg",
        "Description": "Security group for Aurora database",
        "VpcId": "${VPC_ID}"
      },
      "IngressRules": [
        {
          "IpProtocol": "tcp",
          "FromPort": 5432,
          "ToPort": 5432,
          "SourceSecurityGroupId": "${ECS_SERVICE_SG_ID}",
          "Description": "PostgreSQL from ECS services"
        }
      ]
    }
  ]
}
```

### 1.4 Create VPC Endpoints
```json
{
  "Action": "CreateVpcEndpoint",
  "Version": "2016-11-15",
  "Endpoints": [
    {
      "Request": {
        "VpcEndpointType": "Interface",
        "VpcId": "${VPC_ID}",
        "ServiceName": "com.amazonaws.us-east-1.ecr.dkr",
        "SubnetIds": ["${PRIVATE_SUBNET_1A}", "${PRIVATE_SUBNET_1B}"],
        "SecurityGroupIds": ["${ECS_SERVICE_SG_ID}"],
        "PrivateDnsEnabled": true,
        "TagSpecifications": [{
          "ResourceType": "vpc-endpoint",
          "Tags": [{"Key": "Name", "Value": "mistersmith-ecr-dkr-endpoint"}]
        }]
      }
    },
    {
      "Request": {
        "VpcEndpointType": "Interface",
        "VpcId": "${VPC_ID}",
        "ServiceName": "com.amazonaws.us-east-1.ecr.api",
        "SubnetIds": ["${PRIVATE_SUBNET_1A}", "${PRIVATE_SUBNET_1B}"],
        "SecurityGroupIds": ["${ECS_SERVICE_SG_ID}"],
        "PrivateDnsEnabled": true
      }
    },
    {
      "Request": {
        "VpcEndpointType": "Gateway",
        "VpcId": "${VPC_ID}",
        "ServiceName": "com.amazonaws.us-east-1.s3",
        "RouteTableIds": ["${PRIVATE_ROUTE_TABLE_ID}"]
      }
    }
  ]
}
```

## 2. ECS Configuration

### 2.1 Create ECS Cluster
```json
{
  "Action": "CreateCluster",
  "Version": "2014-11-13",
  "Request": {
    "clusterName": "mistersmith-cluster",
    "capacityProviders": ["FARGATE", "FARGATE_SPOT"],
    "defaultCapacityProviderStrategy": [
      {
        "capacityProvider": "FARGATE_SPOT",
        "weight": 2,
        "base": 0
      },
      {
        "capacityProvider": "FARGATE",
        "weight": 1,
        "base": 1
      }
    ],
    "settings": [
      {
        "name": "containerInsights",
        "value": "enabled"
      }
    ],
    "tags": [
      {"key": "Name", "value": "mistersmith-cluster"},
      {"key": "Environment", "value": "production"}
    ]
  },
  "ExpectedResponse": {
    "cluster": {
      "clusterArn": "arn:aws:ecs:us-east-1:xxxx:cluster/mistersmith-cluster",
      "clusterName": "mistersmith-cluster",
      "status": "ACTIVE",
      "capacityProviders": ["FARGATE", "FARGATE_SPOT"]
    }
  }
}
```

### 2.2 Register Task Definitions
```json
{
  "Action": "RegisterTaskDefinition",
  "Version": "2014-11-13",
  "TaskDefinitions": {
    "api-service": {
      "family": "mistersmith-api",
      "networkMode": "awsvpc",
      "requiresCompatibilities": ["FARGATE"],
      "cpu": "512",
      "memory": "1024",
      "executionRoleArn": "${ECS_EXECUTION_ROLE_ARN}",
      "taskRoleArn": "${ECS_TASK_ROLE_ARN}",
      "containerDefinitions": [
        {
          "name": "api",
          "image": "${ECR_REPOSITORY_URI}:latest",
          "essential": true,
          "portMappings": [
            {
              "containerPort": 3000,
              "protocol": "tcp"
            }
          ],
          "environment": [
            {"name": "NODE_ENV", "value": "production"},
            {"name": "PORT", "value": "3000"},
            {"name": "AWS_REGION", "value": "us-east-1"}
          ],
          "secrets": [
            {
              "name": "DATABASE_URL",
              "valueFrom": "${DATABASE_URL_SECRET_ARN}"
            },
            {
              "name": "JWT_SECRET",
              "valueFrom": "${JWT_SECRET_ARN}"
            }
          ],
          "logConfiguration": {
            "logDriver": "awslogs",
            "options": {
              "awslogs-group": "/ecs/mistersmith-api",
              "awslogs-region": "us-east-1",
              "awslogs-stream-prefix": "ecs"
            }
          },
          "healthCheck": {
            "command": ["CMD-SHELL", "curl -f http://localhost:3000/health || exit 1"],
            "interval": 30,
            "timeout": 5,
            "retries": 3,
            "startPeriod": 60
          }
        }
      ]
    },
    "mcp-server": {
      "family": "mistersmith-mcp",
      "networkMode": "awsvpc",
      "requiresCompatibilities": ["FARGATE"],
      "cpu": "256",
      "memory": "512",
      "containerDefinitions": [
        {
          "name": "mcp-server",
          "image": "${ECR_REPOSITORY_URI_MCP}:latest",
          "essential": true,
          "portMappings": [
            {
              "containerPort": 3001,
              "protocol": "tcp"
            }
          ],
          "environment": [
            {"name": "NODE_ENV", "value": "production"},
            {"name": "MCP_TRANSPORT", "value": "sse"}
          ]
        }
      ]
    }
  }
}
```

### 2.3 Create ECS Services
```json
{
  "Action": "CreateService",
  "Version": "2014-11-13",
  "Services": {
    "api-service": {
      "cluster": "mistersmith-cluster",
      "serviceName": "mistersmith-api-service",
      "taskDefinition": "mistersmith-api:1",
      "desiredCount": 2,
      "launchType": "FARGATE",
      "platformVersion": "LATEST",
      "networkConfiguration": {
        "awsvpcConfiguration": {
          "subnets": ["${PRIVATE_SUBNET_1A}", "${PRIVATE_SUBNET_1B}"],
          "securityGroups": ["${ECS_SERVICE_SG_ID}"],
          "assignPublicIp": "DISABLED"
        }
      },
      "loadBalancers": [
        {
          "targetGroupArn": "${TARGET_GROUP_ARN}",
          "containerName": "api",
          "containerPort": 3000
        }
      ],
      "healthCheckGracePeriodSeconds": 60,
      "deploymentConfiguration": {
        "deploymentCircuitBreaker": {
          "enable": true,
          "rollback": true
        },
        "maximumPercent": 200,
        "minimumHealthyPercent": 100
      },
      "enableExecuteCommand": true,
      "propagateTags": "SERVICE",
      "tags": [
        {"key": "Name", "value": "mistersmith-api-service"},
        {"key": "Service", "value": "api"}
      ]
    }
  }
}
```

### 2.4 Service Auto Scaling
```json
{
  "Action": "RegisterScalableTarget",
  "Version": "2016-02-06",
  "Request": {
    "ServiceNamespace": "ecs",
    "ResourceId": "service/mistersmith-cluster/mistersmith-api-service",
    "ScalableDimension": "ecs:service:DesiredCount",
    "MinCapacity": 2,
    "MaxCapacity": 10,
    "RoleARN": "${AUTO_SCALING_ROLE_ARN}"
  }
}
```

```json
{
  "Action": "PutScalingPolicy",
  "Version": "2016-02-06",
  "Policies": [
    {
      "PolicyName": "mistersmith-api-cpu-scaling",
      "ServiceNamespace": "ecs",
      "ResourceId": "service/mistersmith-cluster/mistersmith-api-service",
      "ScalableDimension": "ecs:service:DesiredCount",
      "PolicyType": "TargetTrackingScaling",
      "TargetTrackingScalingPolicyConfiguration": {
        "TargetValue": 70.0,
        "PredefinedMetricSpecification": {
          "PredefinedMetricType": "ECSServiceAverageCPUUtilization"
        },
        "ScaleInCooldown": 300,
        "ScaleOutCooldown": 60
      }
    },
    {
      "PolicyName": "mistersmith-api-memory-scaling",
      "TargetTrackingScalingPolicyConfiguration": {
        "TargetValue": 80.0,
        "PredefinedMetricSpecification": {
          "PredefinedMetricType": "ECSServiceAverageMemoryUtilization"
        }
      }
    }
  ]
}
```

## 3. Aurora Serverless v2 Configuration

### 3.1 Create DB Subnet Group
```json
{
  "Action": "CreateDBSubnetGroup",
  "Version": "2014-10-31",
  "Request": {
    "DBSubnetGroupName": "mistersmith-db-subnet-group",
    "DBSubnetGroupDescription": "Subnet group for MisterSmith Aurora cluster",
    "SubnetIds": ["${PRIVATE_SUBNET_1A}", "${PRIVATE_SUBNET_1B}"],
    "Tags": [
      {"Key": "Name", "Value": "mistersmith-db-subnet-group"}
    ]
  }
}
```

### 3.2 Create Aurora Cluster
```json
{
  "Action": "CreateDBCluster",
  "Version": "2014-10-31",
  "Request": {
    "DBClusterIdentifier": "mistersmith-aurora-cluster",
    "Engine": "aurora-postgresql",
    "EngineVersion": "15.4",
    "EngineMode": "provisioned",
    "ServerlessV2ScalingConfiguration": {
      "MinCapacity": 0.5,
      "MaxCapacity": 4
    },
    "MasterUsername": "mistersmith_admin",
    "MasterUserPassword": "${MASTER_PASSWORD}",
    "DatabaseName": "mistersmith",
    "DBSubnetGroupName": "mistersmith-db-subnet-group",
    "VpcSecurityGroupIds": ["${RDS_SG_ID}"],
    "BackupRetentionPeriod": 7,
    "PreferredBackupWindow": "03:00-04:00",
    "PreferredMaintenanceWindow": "sun:04:00-sun:05:00",
    "EnableCloudwatchLogsExports": ["postgresql"],
    "DeletionProtection": true,
    "StorageEncrypted": true,
    "KmsKeyId": "${KMS_KEY_ID}",
    "EnableHttpEndpoint": true,
    "Tags": [
      {"Key": "Name", "Value": "mistersmith-aurora-cluster"},
      {"Key": "Environment", "Value": "production"}
    ]
  }
}
```

### 3.3 Create DB Instances
```json
{
  "Action": "CreateDBInstance",
  "Version": "2014-10-31",
  "Instances": [
    {
      "DBInstanceIdentifier": "mistersmith-aurora-instance-1",
      "DBInstanceClass": "db.serverless",
      "Engine": "aurora-postgresql",
      "DBClusterIdentifier": "mistersmith-aurora-cluster",
      "PubliclyAccessible": false,
      "AvailabilityZone": "us-east-1a",
      "PerformanceInsightsEnabled": true,
      "PerformanceInsightsRetentionPeriod": 7,
      "MonitoringInterval": 60,
      "MonitoringRoleArn": "${RDS_MONITORING_ROLE_ARN}",
      "Tags": [
        {"Key": "Name", "Value": "mistersmith-aurora-instance-1"}
      ]
    },
    {
      "DBInstanceIdentifier": "mistersmith-aurora-instance-2",
      "DBInstanceClass": "db.serverless",
      "Engine": "aurora-postgresql",
      "DBClusterIdentifier": "mistersmith-aurora-cluster",
      "PubliclyAccessible": false,
      "AvailabilityZone": "us-east-1b"
    }
  ]
}
```

### 3.4 Create RDS Proxy
```json
{
  "Action": "CreateDBProxy",
  "Version": "2014-10-31",
  "Request": {
    "DBProxyName": "mistersmith-rds-proxy",
    "EngineFamily": "POSTGRESQL",
    "Auth": [
      {
        "AuthScheme": "SECRETS",
        "SecretArn": "${DATABASE_SECRET_ARN}",
        "IAMAuth": "DISABLED"
      }
    ],
    "RoleArn": "${RDS_PROXY_ROLE_ARN}",
    "VpcSubnetIds": ["${PRIVATE_SUBNET_1A}", "${PRIVATE_SUBNET_1B}"],
    "VpcSecurityGroupIds": ["${RDS_SG_ID}"],
    "RequireTLS": true,
    "IdleClientTimeout": 1800,
    "MaxConnectionsPercent": 100,
    "MaxIdleConnectionsPercent": 50,
    "ConnectionBorrowTimeout": 120,
    "Tags": [
      {"Key": "Name", "Value": "mistersmith-rds-proxy"}
    ]
  }
}
```

## 4. Messaging Services Configuration

### 4.1 EventBridge Configuration
```json
{
  "Action": "CreateEventBus",
  "Version": "2015-10-07",
  "Request": {
    "Name": "mistersmith-event-bus",
    "Description": "Central event bus for MisterSmith orchestration",
    "Tags": [
      {"Key": "Name", "Value": "mistersmith-event-bus"}
    ]
  }
}
```

### 4.2 Event Rules
```json
{
  "Action": "PutRule",
  "Version": "2015-10-07",
  "Rules": [
    {
      "Name": "agent-task-events",
      "EventBusName": "mistersmith-event-bus",
      "EventPattern": {
        "source": ["mistersmith.agents"],
        "detail-type": ["Agent Task Started", "Agent Task Completed", "Agent Task Failed"]
      },
      "State": "ENABLED",
      "Description": "Routes agent task events to appropriate handlers"
    },
    {
      "Name": "discovery-events",
      "EventBusName": "mistersmith-event-bus",
      "EventPattern": {
        "source": ["mistersmith.discovery"],
        "detail-type": ["Discovery Shared", "Discovery Updated"]
      },
      "State": "ENABLED"
    }
  ]
}
```

### 4.3 SQS Queues
```json
{
  "Action": "CreateQueue",
  "Version": "2012-11-05",
  "Queues": [
    {
      "QueueName": "mistersmith-task-queue.fifo",
      "Attributes": {
        "FifoQueue": "true",
        "ContentBasedDeduplication": "true",
        "MessageRetentionPeriod": "1209600",
        "VisibilityTimeout": "300",
        "ReceiveMessageWaitTimeSeconds": "20",
        "RedrivePolicy": {
          "deadLetterTargetArn": "${DLQ_ARN}",
          "maxReceiveCount": 3
        },
        "KmsMasterKeyId": "${KMS_KEY_ID}"
      },
      "Tags": {
        "Name": "mistersmith-task-queue",
        "Type": "fifo"
      }
    },
    {
      "QueueName": "mistersmith-discovery-queue",
      "Attributes": {
        "MessageRetentionPeriod": "345600",
        "VisibilityTimeout": "60",
        "DelaySeconds": "0"
      }
    }
  ]
}
```

### 4.4 DynamoDB Tables
```json
{
  "Action": "CreateTable",
  "Version": "2012-08-10",
  "Tables": [
    {
      "TableName": "mistersmith-discoveries",
      "KeySchema": [
        {"AttributeName": "discovery_id", "KeyType": "HASH"},
        {"AttributeName": "timestamp", "KeyType": "RANGE"}
      ],
      "AttributeDefinitions": [
        {"AttributeName": "discovery_id", "AttributeType": "S"},
        {"AttributeName": "timestamp", "AttributeType": "N"},
        {"AttributeName": "agent_id", "AttributeType": "S"},
        {"AttributeName": "discovery_type", "AttributeType": "S"}
      ],
      "BillingMode": "PAY_PER_REQUEST",
      "StreamSpecification": {
        "StreamEnabled": true,
        "StreamViewType": "NEW_AND_OLD_IMAGES"
      },
      "GlobalSecondaryIndexes": [
        {
          "IndexName": "agent-index",
          "Keys": [
            {"AttributeName": "agent_id", "KeyType": "HASH"},
            {"AttributeName": "timestamp", "KeyType": "RANGE"}
          ],
          "Projection": {"ProjectionType": "ALL"}
        },
        {
          "IndexName": "type-index",
          "Keys": [
            {"AttributeName": "discovery_type", "KeyType": "HASH"},
            {"AttributeName": "timestamp", "KeyType": "RANGE"}
          ],
          "Projection": {"ProjectionType": "ALL"}
        }
      ],
      "PointInTimeRecoverySpecification": {
        "PointInTimeRecoveryEnabled": true
      },
      "Tags": [
        {"Key": "Name", "Value": "mistersmith-discoveries"}
      ]
    }
  ]
}
```

## 5. IAM Policies and Roles

### 5.1 ECS Execution Role
```json
{
  "Action": "CreateRole",
  "Version": "2010-05-08",
  "Request": {
    "RoleName": "mistersmith-ecs-execution-role",
    "AssumeRolePolicyDocument": {
      "Version": "2012-10-17",
      "Statement": [
        {
          "Effect": "Allow",
          "Principal": {
            "Service": "ecs-tasks.amazonaws.com"
          },
          "Action": "sts:AssumeRole"
        }
      ]
    },
    "ManagedPolicyArns": [
      "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
    ],
    "Tags": [
      {"Key": "Name", "Value": "mistersmith-ecs-execution-role"}
    ]
  }
}
```

### 5.2 ECS Task Role
```json
{
  "Action": "CreateRole",
  "Version": "2010-05-08",
  "Request": {
    "RoleName": "mistersmith-ecs-task-role",
    "AssumeRolePolicyDocument": {
      "Version": "2012-10-17",
      "Statement": [
        {
          "Effect": "Allow",
          "Principal": {
            "Service": "ecs-tasks.amazonaws.com"
          },
          "Action": "sts:AssumeRole"
        }
      ]
    }
  }
}
```

### 5.3 Task Role Policy
```json
{
  "Action": "CreatePolicy",
  "Version": "2010-05-08",
  "Request": {
    "PolicyName": "mistersmith-ecs-task-policy",
    "PolicyDocument": {
      "Version": "2012-10-17",
      "Statement": [
        {
          "Effect": "Allow",
          "Action": [
            "ssmmessages:CreateControlChannel",
            "ssmmessages:CreateDataChannel",
            "ssmmessages:OpenControlChannel",
            "ssmmessages:OpenDataChannel"
          ],
          "Resource": "*"
        },
        {
          "Effect": "Allow",
          "Action": [
            "logs:CreateLogGroup",
            "logs:CreateLogStream",
            "logs:PutLogEvents",
            "logs:DescribeLogStreams"
          ],
          "Resource": "arn:aws:logs:us-east-1:*:log-group:/ecs/mistersmith-*"
        },
        {
          "Effect": "Allow",
          "Action": [
            "secretsmanager:GetSecretValue"
          ],
          "Resource": [
            "${DATABASE_URL_SECRET_ARN}",
            "${JWT_SECRET_ARN}",
            "${API_KEYS_SECRET_ARN}"
          ]
        },
        {
          "Effect": "Allow",
          "Action": [
            "dynamodb:GetItem",
            "dynamodb:PutItem",
            "dynamodb:UpdateItem",
            "dynamodb:Query",
            "dynamodb:Scan",
            "dynamodb:BatchGetItem",
            "dynamodb:BatchWriteItem"
          ],
          "Resource": [
            "arn:aws:dynamodb:us-east-1:*:table/mistersmith-*",
            "arn:aws:dynamodb:us-east-1:*:table/mistersmith-*/index/*"
          ]
        },
        {
          "Effect": "Allow",
          "Action": [
            "sqs:SendMessage",
            "sqs:ReceiveMessage",
            "sqs:DeleteMessage",
            "sqs:GetQueueAttributes"
          ],
          "Resource": "arn:aws:sqs:us-east-1:*:mistersmith-*"
        },
        {
          "Effect": "Allow",
          "Action": [
            "events:PutEvents"
          ],
          "Resource": "arn:aws:events:us-east-1:*:event-bus/mistersmith-*"
        },
        {
          "Effect": "Allow",
          "Action": [
            "s3:GetObject",
            "s3:PutObject",
            "s3:DeleteObject"
          ],
          "Resource": "arn:aws:s3:::mistersmith-*/*"
        }
      ]
    }
  }
}
```

### 5.4 RDS Proxy Role
```json
{
  "Action": "CreateRole",
  "Version": "2010-05-08",
  "Request": {
    "RoleName": "mistersmith-rds-proxy-role",
    "AssumeRolePolicyDocument": {
      "Version": "2012-10-17",
      "Statement": [
        {
          "Effect": "Allow",
          "Principal": {
            "Service": "rds.amazonaws.com"
          },
          "Action": "sts:AssumeRole"
        }
      ]
    }
  }
}
```

```json
{
  "Action": "AttachRolePolicy",
  "Version": "2010-05-08",
  "Request": {
    "RoleName": "mistersmith-rds-proxy-role",
    "PolicyArn": "arn:aws:iam::aws:policy/SecretsManagerReadWrite"
  }
}
```

## 6. Application Load Balancer Configuration

### 6.1 Create ALB
```json
{
  "Action": "CreateLoadBalancer",
  "Version": "2015-12-01",
  "Request": {
    "Name": "mistersmith-alb",
    "Type": "application",
    "Scheme": "internet-facing",
    "IpAddressType": "dualstack",
    "Subnets": ["${PUBLIC_SUBNET_1A}", "${PUBLIC_SUBNET_1B}"],
    "SecurityGroups": ["${ALB_SG_ID}"],
    "Tags": [
      {"Key": "Name", "Value": "mistersmith-alb"}
    ]
  }
}
```

### 6.2 Create Target Groups
```json
{
  "Action": "CreateTargetGroup",
  "Version": "2015-12-01",
  "TargetGroups": [
    {
      "Name": "mistersmith-api-tg",
      "Protocol": "HTTP",
      "Port": 3000,
      "VpcId": "${VPC_ID}",
      "TargetType": "ip",
      "HealthCheckProtocol": "HTTP",
      "HealthCheckPath": "/health",
      "HealthCheckIntervalSeconds": 30,
      "HealthCheckTimeoutSeconds": 5,
      "HealthyThresholdCount": 2,
      "UnhealthyThresholdCount": 3,
      "Matcher": {
        "HttpCode": "200"
      },
      "TargetGroupAttributes": [
        {"Key": "deregistration_delay.timeout_seconds", "Value": "30"},
        {"Key": "stickiness.enabled", "Value": "true"},
        {"Key": "stickiness.type", "Value": "lb_cookie"},
        {"Key": "stickiness.lb_cookie.duration_seconds", "Value": "86400"}
      ]
    }
  ]
}
```

### 6.3 Create Listeners
```json
{
  "Action": "CreateListener",
  "Version": "2015-12-01",
  "Listeners": [
    {
      "LoadBalancerArn": "${ALB_ARN}",
      "Protocol": "HTTPS",
      "Port": 443,
      "Certificates": [
        {
          "CertificateArn": "${ACM_CERTIFICATE_ARN}"
        }
      ],
      "DefaultActions": [
        {
          "Type": "forward",
          "TargetGroupArn": "${API_TARGET_GROUP_ARN}"
        }
      ],
      "SslPolicy": "ELBSecurityPolicy-TLS13-1-2-2021-06"
    },
    {
      "LoadBalancerArn": "${ALB_ARN}",
      "Protocol": "HTTP",
      "Port": 80,
      "DefaultActions": [
        {
          "Type": "redirect",
          "RedirectConfig": {
            "Protocol": "HTTPS",
            "Port": "443",
            "StatusCode": "HTTP_301"
          }
        }
      ]
    }
  ]
}
```

## 7. CloudWatch and Monitoring

### 7.1 Create Log Groups
```json
{
  "Action": "CreateLogGroup",
  "Version": "2014-03-28",
  "LogGroups": [
    {
      "logGroupName": "/ecs/mistersmith-api",
      "retentionInDays": 30,
      "kmsKeyId": "${KMS_KEY_ID}",
      "tags": {
        "Application": "mistersmith",
        "Service": "api"
      }
    },
    {
      "logGroupName": "/ecs/mistersmith-mcp",
      "retentionInDays": 30
    },
    {
      "logGroupName": "/aws/rds/cluster/mistersmith-aurora-cluster/postgresql",
      "retentionInDays": 7
    }
  ]
}
```

### 7.2 Create CloudWatch Alarms
```json
{
  "Action": "PutMetricAlarm",
  "Version": "2010-08-01",
  "Alarms": [
    {
      "AlarmName": "mistersmith-api-high-cpu",
      "ComparisonOperator": "GreaterThanThreshold",
      "EvaluationPeriods": 2,
      "MetricName": "CPUUtilization",
      "Namespace": "AWS/ECS",
      "Period": 300,
      "Statistic": "Average",
      "Threshold": 80.0,
      "ActionsEnabled": true,
      "AlarmActions": ["${SNS_TOPIC_ARN}"],
      "AlarmDescription": "Triggers when API service CPU exceeds 80%",
      "Dimensions": [
        {"Name": "ServiceName", "Value": "mistersmith-api-service"},
        {"Name": "ClusterName", "Value": "mistersmith-cluster"}
      ]
    },
    {
      "AlarmName": "mistersmith-aurora-high-connections",
      "MetricName": "DatabaseConnections",
      "Namespace": "AWS/RDS",
      "Threshold": 80.0,
      "ComparisonOperator": "GreaterThanThreshold",
      "Dimensions": [
        {"Name": "DBClusterIdentifier", "Value": "mistersmith-aurora-cluster"}
      ]
    }
  ]
}
```

## 8. Secrets Manager Configuration

### 8.1 Create Secrets
```json
{
  "Action": "CreateSecret",
  "Version": "2017-10-17",
  "Secrets": [
    {
      "Name": "mistersmith/database/credentials",
      "Description": "Database connection credentials for MisterSmith",
      "SecretString": {
        "username": "mistersmith_app",
        "password": "${GENERATED_PASSWORD}",
        "engine": "postgres",
        "host": "${RDS_PROXY_ENDPOINT}",
        "port": 5432,
        "dbname": "mistersmith"
      },
      "KmsKeyId": "${KMS_KEY_ID}",
      "Tags": [
        {"Key": "Application", "Value": "mistersmith"}
      ]
    },
    {
      "Name": "mistersmith/api/jwt-secret",
      "Description": "JWT secret for API authentication",
      "SecretString": "${JWT_SECRET}",
      "KmsKeyId": "${KMS_KEY_ID}"
    },
    {
      "Name": "mistersmith/api/keys",
      "Description": "External API keys",
      "SecretString": {
        "openai_api_key": "${OPENAI_API_KEY}",
        "github_token": "${GITHUB_TOKEN}",
        "slack_webhook": "${SLACK_WEBHOOK}"
      }
    }
  ]
}
```

## 9. Route 53 Configuration

### 9.1 Create Hosted Zone
```json
{
  "Action": "CreateHostedZone",
  "Version": "2013-04-01",
  "Request": {
    "Name": "mistersmith.ai",
    "CallerReference": "mistersmith-${TIMESTAMP}",
    "HostedZoneConfig": {
      "Comment": "Managed by Terraform",
      "PrivateZone": false
    }
  }
}
```

### 9.2 Create DNS Records
```json
{
  "Action": "ChangeResourceRecordSets",
  "Version": "2013-04-01",
  "Request": {
    "HostedZoneId": "${HOSTED_ZONE_ID}",
    "ChangeBatch": {
      "Changes": [
        {
          "Action": "CREATE",
          "ResourceRecordSet": {
            "Name": "mistersmith.ai",
            "Type": "A",
            "AliasTarget": {
              "HostedZoneId": "${ALB_HOSTED_ZONE_ID}",
              "DNSName": "${ALB_DNS_NAME}",
              "EvaluateTargetHealth": true
            }
          }
        },
        {
          "Action": "CREATE",
          "ResourceRecordSet": {
            "Name": "www.mistersmith.ai",
            "Type": "CNAME",
            "TTL": 300,
            "ResourceRecords": [
              {"Value": "mistersmith.ai"}
            ]
          }
        }
      ]
    }
  }
}
```

## 10. ECR Repository Configuration

### 10.1 Create ECR Repositories
```json
{
  "Action": "CreateRepository",
  "Version": "2015-09-21",
  "Repositories": [
    {
      "repositoryName": "mistersmith/api",
      "imageScanningConfiguration": {
        "scanOnPush": true
      },
      "imageTagMutability": "MUTABLE",
      "encryptionConfiguration": {
        "encryptionType": "KMS",
        "kmsKey": "${KMS_KEY_ID}"
      },
      "tags": [
        {"Key": "Application", "Value": "mistersmith"},
        {"Key": "Component", "Value": "api"}
      ]
    },
    {
      "repositoryName": "mistersmith/mcp-server",
      "imageScanningConfiguration": {
        "scanOnPush": true
      }
    }
  ]
}
```

### 10.2 Set Lifecycle Policy
```json
{
  "Action": "PutLifecyclePolicy",
  "Version": "2015-09-21",
  "Request": {
    "repositoryName": "mistersmith/api",
    "lifecyclePolicyText": {
      "rules": [
        {
          "rulePriority": 1,
          "description": "Keep last 10 images",
          "selection": {
            "tagStatus": "any",
            "countType": "imageCountMoreThan",
            "countNumber": 10
          },
          "action": {
            "type": "expire"
          }
        }
      ]
    }
  }
}
```

## Summary

This document provides exact AWS API calls for:
- Complete VPC setup with public/private subnets
- ECS Fargate cluster with auto-scaling
- Aurora Serverless v2 with RDS Proxy
- EventBridge, SQS, and DynamoDB for messaging
- Comprehensive IAM roles and policies
- Application Load Balancer with HTTPS
- CloudWatch monitoring and alarms
- Secrets Manager for credentials
- Route 53 DNS configuration
- ECR repositories with lifecycle policies

Each API call includes:
- Exact action and version
- Complete request parameters
- Expected response format
- All required configurations

These specifications can be directly implemented in Terraform, CloudFormation, or AWS SDK calls for the MisterSmith migration.