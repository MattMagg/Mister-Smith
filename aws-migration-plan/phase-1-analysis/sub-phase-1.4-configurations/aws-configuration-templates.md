# AWS Configuration Templates

## ECS Task Definition Environment Variables

```json
{
  "family": "mistersmith-service",
  "taskRoleArn": "arn:aws:iam::ACCOUNT_ID:role/MisterSmithTaskRole",
  "executionRoleArn": "arn:aws:iam::ACCOUNT_ID:role/MisterSmithExecutionRole",
  "networkMode": "awsvpc",
  "containerDefinitions": [
    {
      "name": "mistersmith",
      "image": "ACCOUNT_ID.dkr.ecr.REGION.amazonaws.com/mistersmith:latest",
      "memory": 2048,
      "cpu": 1024,
      "essential": true,
      "environment": [
        {
          "name": "SERVICE_NAME",
          "value": "mistersmith"
        },
        {
          "name": "SERVICE_PORT",
          "value": "8080"
        },
        {
          "name": "AWS_REGION",
          "value": "us-east-1"
        },
        {
          "name": "NATS_CLUSTER_ID",
          "value": "mistersmith-cluster"
        },
        {
          "name": "MAX_WORKERS",
          "value": "16"
        },
        {
          "name": "CONSENSUS_ALGORITHM",
          "value": "majority"
        },
        {
          "name": "MEMORY_SIZE_MB",
          "value": "200"
        },
        {
          "name": "ENABLE_AUTO_SCALE",
          "value": "true"
        },
        {
          "name": "ENABLE_ENCRYPTION",
          "value": "true"
        },
        {
          "name": "METRICS_PORT",
          "value": "9090"
        },
        {
          "name": "OTEL_SERVICE_NAME",
          "value": "mistersmith"
        },
        {
          "name": "OTEL_RESOURCE_ATTRIBUTES",
          "value": "deployment.environment=production,service.namespace=mistersmith"
        }
      ],
      "secrets": [
        {
          "name": "NATS_URL",
          "valueFrom": "arn:aws:secretsmanager:REGION:ACCOUNT_ID:secret:mistersmith/nats/url"
        },
        {
          "name": "DATABASE_URL",
          "valueFrom": "arn:aws:secretsmanager:REGION:ACCOUNT_ID:secret:mistersmith/database/url"
        }
      ],
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        },
        {
          "containerPort": 9090,
          "protocol": "tcp"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/mistersmith",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:8080/system/health || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 60
      }
    }
  ]
}
```

## AWS Systems Manager Parameter Store Setup

```bash
#!/bin/bash

# Common parameters
aws ssm put-parameter \
  --name "/mistersmith/common/service_name" \
  --value "mistersmith" \
  --type "String" \
  --description "Service name"

aws ssm put-parameter \
  --name "/mistersmith/common/version" \
  --value "0.1.0" \
  --type "String" \
  --description "Service version"

# Development environment
aws ssm put-parameter \
  --name "/mistersmith/dev/nats/url" \
  --value "nats://dev-nats.internal:4222" \
  --type "String" \
  --description "NATS URL for development"

aws ssm put-parameter \
  --name "/mistersmith/dev/agents/max_workers" \
  --value "4" \
  --type "String" \
  --description "Maximum workers in development"

aws ssm put-parameter \
  --name "/mistersmith/dev/features/enable_auto_scale" \
  --value "false" \
  --type "String" \
  --description "Auto-scaling disabled in dev"

# Production environment
aws ssm put-parameter \
  --name "/mistersmith/prod/nats/url" \
  --value "nats://prod-nats-nlb.internal:4222" \
  --type "SecureString" \
  --key-id "alias/mistersmith-kms" \
  --description "NATS URL for production"

aws ssm put-parameter \
  --name "/mistersmith/prod/agents/max_workers" \
  --value "16" \
  --type "String" \
  --description "Maximum workers in production"

aws ssm put-parameter \
  --name "/mistersmith/prod/features/enable_auto_scale" \
  --value "true" \
  --type "String" \
  --description "Auto-scaling enabled in production"
```

## AWS Secrets Manager Templates

```bash
#!/bin/bash

# Database credentials
aws secretsmanager create-secret \
  --name "mistersmith/database/credentials" \
  --description "MisterSmith database credentials" \
  --kms-key-id "alias/mistersmith-kms" \
  --secret-string '{
    "username": "mistersmith_app",
    "password": "GENERATED_PASSWORD",
    "engine": "postgres",
    "host": "mistersmith-db.cluster-xxxxx.us-east-1.rds.amazonaws.com",
    "port": 5432,
    "dbname": "mistersmith"
  }'

# NATS credentials
aws secretsmanager create-secret \
  --name "mistersmith/nats/credentials" \
  --description "NATS authentication credentials" \
  --kms-key-id "alias/mistersmith-kms" \
  --secret-string '{
    "username": "mistersmith_service",
    "password": "GENERATED_PASSWORD",
    "token": "GENERATED_TOKEN"
  }'

# API keys
aws secretsmanager create-secret \
  --name "mistersmith/api/keys" \
  --description "API keys and secrets" \
  --kms-key-id "alias/mistersmith-kms" \
  --secret-string '{
    "jwt_secret": "GENERATED_JWT_SECRET",
    "api_key": "GENERATED_API_KEY",
    "webhook_secret": "GENERATED_WEBHOOK_SECRET"
  }'
```

## CloudFormation Configuration Resources

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: 'MisterSmith Configuration Resources'

Parameters:
  Environment:
    Type: String
    Default: dev
    AllowedValues: [dev, staging, prod]
    Description: Deployment environment

Resources:
  # KMS Key for secrets encryption
  MisterSmithKMSKey:
    Type: AWS::KMS::Key
    Properties:
      Description: KMS key for MisterSmith secrets
      KeyPolicy:
        Version: '2012-10-17'
        Statement:
          - Sid: Enable IAM User Permissions
            Effect: Allow
            Principal:
              AWS: !Sub 'arn:aws:iam::${AWS::AccountId}:root'
            Action: 'kms:*'
            Resource: '*'
          - Sid: Allow ECS Task Role
            Effect: Allow
            Principal:
              AWS: !GetAtt MisterSmithTaskRole.Arn
            Action:
              - 'kms:Decrypt'
              - 'kms:DescribeKey'
            Resource: '*'

  MisterSmithKMSAlias:
    Type: AWS::KMS::Alias
    Properties:
      AliasName: alias/mistersmith-kms
      TargetKeyId: !Ref MisterSmithKMSKey

  # IAM Role for ECS Tasks
  MisterSmithTaskRole:
    Type: AWS::IAM::Role
    Properties:
      RoleName: !Sub 'MisterSmithTaskRole-${Environment}'
      AssumeRolePolicyDocument:
        Version: '2012-10-17'
        Statement:
          - Effect: Allow
            Principal:
              Service: ecs-tasks.amazonaws.com
            Action: 'sts:AssumeRole'
      ManagedPolicyArns:
        - 'arn:aws:iam::aws:policy/CloudWatchLogsFullAccess'
      Policies:
        - PolicyName: MisterSmithTaskPolicy
          PolicyDocument:
            Version: '2012-10-17'
            Statement:
              - Effect: Allow
                Action:
                  - 'ssm:GetParameter'
                  - 'ssm:GetParameters'
                  - 'ssm:GetParametersByPath'
                Resource:
                  - !Sub 'arn:aws:ssm:${AWS::Region}:${AWS::AccountId}:parameter/mistersmith/*'
              - Effect: Allow
                Action:
                  - 'secretsmanager:GetSecretValue'
                Resource:
                  - !Sub 'arn:aws:secretsmanager:${AWS::Region}:${AWS::AccountId}:secret:mistersmith/*'
              - Effect: Allow
                Action:
                  - 'kms:Decrypt'
                  - 'kms:DescribeKey'
                Resource:
                  - !GetAtt MisterSmithKMSKey.Arn

  # Parameter Store Configuration
  ServiceNameParameter:
    Type: AWS::SSM::Parameter
    Properties:
      Name: !Sub '/mistersmith/${Environment}/service_name'
      Type: String
      Value: mistersmith
      Description: Service name

  MaxWorkersParameter:
    Type: AWS::SSM::Parameter
    Properties:
      Name: !Sub '/mistersmith/${Environment}/agents/max_workers'
      Type: String
      Value: !If [IsProduction, '16', '4']
      Description: Maximum number of worker agents

  AutoScaleParameter:
    Type: AWS::SSM::Parameter
    Properties:
      Name: !Sub '/mistersmith/${Environment}/features/enable_auto_scale'
      Type: String
      Value: !If [IsProduction, 'true', 'false']
      Description: Enable auto-scaling feature

Conditions:
  IsProduction: !Equals [!Ref Environment, prod]

Outputs:
  KMSKeyId:
    Description: KMS Key ID for secrets encryption
    Value: !Ref MisterSmithKMSKey
    Export:
      Name: !Sub '${AWS::StackName}-KMSKeyId'

  TaskRoleArn:
    Description: IAM Role ARN for ECS tasks
    Value: !GetAtt MisterSmithTaskRole.Arn
    Export:
      Name: !Sub '${AWS::StackName}-TaskRoleArn'
```

## Terraform Configuration Module

```hcl
# modules/mistersmith-config/main.tf

variable "environment" {
  description = "Deployment environment"
  type        = string
  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be dev, staging, or prod."
  }
}

variable "service_config" {
  description = "Service configuration"
  type = object({
    max_workers         = number
    memory_size_mb      = number
    enable_auto_scale   = bool
    enable_encryption   = bool
    consensus_algorithm = string
  })
  default = {
    max_workers         = 8
    memory_size_mb      = 100
    enable_auto_scale   = true
    enable_encryption   = false
    consensus_algorithm = "majority"
  }
}

# KMS Key for secrets
resource "aws_kms_key" "mistersmith" {
  description             = "KMS key for MisterSmith secrets"
  deletion_window_in_days = 10
  enable_key_rotation     = true

  tags = {
    Name        = "mistersmith-kms-${var.environment}"
    Environment = var.environment
    Service     = "mistersmith"
  }
}

resource "aws_kms_alias" "mistersmith" {
  name          = "alias/mistersmith-${var.environment}"
  target_key_id = aws_kms_key.mistersmith.key_id
}

# Systems Manager Parameters
resource "aws_ssm_parameter" "service_name" {
  name        = "/mistersmith/${var.environment}/service_name"
  description = "Service name"
  type        = "String"
  value       = "mistersmith"

  tags = {
    Environment = var.environment
    Service     = "mistersmith"
  }
}

resource "aws_ssm_parameter" "max_workers" {
  name        = "/mistersmith/${var.environment}/agents/max_workers"
  description = "Maximum number of worker agents"
  type        = "String"
  value       = tostring(var.service_config.max_workers)

  tags = {
    Environment = var.environment
    Service     = "mistersmith"
  }
}

resource "aws_ssm_parameter" "memory_size" {
  name        = "/mistersmith/${var.environment}/agents/memory_size_mb"
  description = "Agent memory size in MB"
  type        = "String"
  value       = tostring(var.service_config.memory_size_mb)

  tags = {
    Environment = var.environment
    Service     = "mistersmith"
  }
}

resource "aws_ssm_parameter" "enable_auto_scale" {
  name        = "/mistersmith/${var.environment}/features/enable_auto_scale"
  description = "Enable auto-scaling"
  type        = "String"
  value       = tostring(var.service_config.enable_auto_scale)

  tags = {
    Environment = var.environment
    Service     = "mistersmith"
  }
}

resource "aws_ssm_parameter" "enable_encryption" {
  name        = "/mistersmith/${var.environment}/features/enable_encryption"
  description = "Enable encryption"
  type        = "String"
  value       = tostring(var.service_config.enable_encryption)

  tags = {
    Environment = var.environment
    Service     = "mistersmith"
  }
}

resource "aws_ssm_parameter" "consensus_algorithm" {
  name        = "/mistersmith/${var.environment}/agents/consensus_algorithm"
  description = "Consensus algorithm"
  type        = "String"
  value       = var.service_config.consensus_algorithm

  tags = {
    Environment = var.environment
    Service     = "mistersmith"
  }
}

# Outputs
output "kms_key_id" {
  description = "KMS key ID"
  value       = aws_kms_key.mistersmith.id
}

output "kms_key_arn" {
  description = "KMS key ARN"
  value       = aws_kms_key.mistersmith.arn
}

output "parameter_prefix" {
  description = "SSM parameter prefix"
  value       = "/mistersmith/${var.environment}"
}
```

## Environment-Specific Configuration Files

### Development (.env.development)
```bash
# Development Configuration
SERVICE_NAME=mistersmith-dev
SERVICE_PORT=8080
AWS_REGION=us-east-1
AWS_PROFILE=mistersmith-dev

# NATS Configuration
NATS_URL=nats://localhost:4222
NATS_CLUSTER_ID=mistersmith-dev
NATS_CLIENT_ID=mistersmith-dev-local

# Agent Configuration
MAX_WORKERS=4
CONSENSUS_ALGORITHM=majority
MEMORY_SIZE_MB=50
ENABLE_AUTO_SCALE=false
ENABLE_ENCRYPTION=false

# Monitoring
METRICS_PORT=9090
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=mistersmith-dev
OTEL_RESOURCE_ATTRIBUTES=deployment.environment=development

# Logging
RUST_LOG=debug,mistersmith=trace
LOG_FORMAT=pretty
```

### Production (.env.production)
```bash
# Production Configuration
SERVICE_NAME=mistersmith
SERVICE_PORT=8080
AWS_REGION=us-east-1

# These will be loaded from Secrets Manager
# NATS_URL=
# DATABASE_URL=

# Agent Configuration (from Parameter Store)
# MAX_WORKERS=
# CONSENSUS_ALGORITHM=
# MEMORY_SIZE_MB=
# ENABLE_AUTO_SCALE=
# ENABLE_ENCRYPTION=

# Monitoring
METRICS_PORT=9090
OTEL_SERVICE_NAME=mistersmith
OTEL_RESOURCE_ATTRIBUTES=deployment.environment=production

# Logging
RUST_LOG=info,mistersmith=debug
LOG_FORMAT=json
```