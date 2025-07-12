# Phase 3.1: Atomic Operations Tracker
## MisterSmith AWS Migration - Reversible Operation Specifications

**Generated**: 2025-01-11T13:18:00Z  
**Agent**: Migration Architect (MS-3 Swarm)  
**Status**: 🟢 ACTIVE DESIGN

---

## 1. INFRASTRUCTURE OPERATIONS

### 1.1 VPC Creation/Deletion

#### CREATE OPERATION
```bash
# Create VPC with exact specifications
aws ec2 create-vpc \
  --cidr-block 10.0.0.0/16 \
  --tag-specifications 'ResourceType=vpc,Tags=[{Key=Name,Value=MisterSmith-VPC},{Key=Environment,Value=Production}]' \
  --region us-east-1

# Create subnets
aws ec2 create-subnet \
  --vpc-id <vpc-id> \
  --cidr-block 10.0.1.0/24 \
  --availability-zone us-east-1a \
  --tag-specifications 'ResourceType=subnet,Tags=[{Key=Name,Value=MisterSmith-Private-1A}]'

aws ec2 create-subnet \
  --vpc-id <vpc-id> \
  --cidr-block 10.0.2.0/24 \
  --availability-zone us-east-1b \
  --tag-specifications 'ResourceType=subnet,Tags=[{Key=Name,Value=MisterSmith-Private-1B}]'

# Create Internet Gateway
aws ec2 create-internet-gateway \
  --tag-specifications 'ResourceType=internet-gateway,Tags=[{Key=Name,Value=MisterSmith-IGW}]'

# Attach IGW to VPC
aws ec2 attach-internet-gateway \
  --vpc-id <vpc-id> \
  --internet-gateway-id <igw-id>
```

#### ROLLBACK OPERATION
```bash
# Detach and delete IGW
aws ec2 detach-internet-gateway \
  --vpc-id <vpc-id> \
  --internet-gateway-id <igw-id>

aws ec2 delete-internet-gateway \
  --internet-gateway-id <igw-id>

# Delete subnets
aws ec2 delete-subnet --subnet-id <subnet-1a-id>
aws ec2 delete-subnet --subnet-id <subnet-1b-id>

# Delete VPC
aws ec2 delete-vpc --vpc-id <vpc-id>
```

#### VERIFICATION
```bash
# Verify VPC creation
aws ec2 describe-vpcs --vpc-ids <vpc-id>

# Verify subnets
aws ec2 describe-subnets --filters "Name=vpc-id,Values=<vpc-id>"

# Check IGW attachment
aws ec2 describe-internet-gateways --internet-gateway-ids <igw-id>
```

### 1.2 Aurora Serverless v2 Cluster

#### CREATE OPERATION
```bash
# Create DB subnet group
aws rds create-db-subnet-group \
  --db-subnet-group-name mistersmith-aurora-subnet-group \
  --db-subnet-group-description "MisterSmith Aurora subnet group" \
  --subnet-ids <subnet-1a-id> <subnet-1b-id> \
  --tags Key=Name,Value=MisterSmith-Aurora-SubnetGroup

# Create Aurora cluster
aws rds create-db-cluster \
  --db-cluster-identifier mistersmith-aurora-cluster \
  --engine aurora-postgresql \
  --engine-version 15.4 \
  --serverless-v2-scaling-configuration MinCapacity=0.5,MaxCapacity=4 \
  --master-username postgres \
  --master-user-password <secure-password> \
  --db-subnet-group-name mistersmith-aurora-subnet-group \
  --vpc-security-group-ids <security-group-id> \
  --enable-cloudwatch-logs-exports postgresql \
  --backup-retention-period 7 \
  --preferred-backup-window "03:00-04:00" \
  --preferred-maintenance-window "sun:04:00-sun:05:00"

# Create Aurora instance
aws rds create-db-instance \
  --db-instance-identifier mistersmith-aurora-instance-1 \
  --db-cluster-identifier mistersmith-aurora-cluster \
  --db-instance-class db.serverless \
  --engine aurora-postgresql
```

#### ROLLBACK OPERATION
```bash
# Delete Aurora instance
aws rds delete-db-instance \
  --db-instance-identifier mistersmith-aurora-instance-1 \
  --skip-final-snapshot

# Delete Aurora cluster
aws rds delete-db-cluster \
  --db-cluster-identifier mistersmith-aurora-cluster \
  --skip-final-snapshot

# Delete DB subnet group
aws rds delete-db-subnet-group \
  --db-subnet-group-name mistersmith-aurora-subnet-group
```

#### VERIFICATION
```bash
# Check cluster status
aws rds describe-db-clusters \
  --db-cluster-identifier mistersmith-aurora-cluster

# Check instance status
aws rds describe-db-instances \
  --db-instance-identifier mistersmith-aurora-instance-1

# Test connection
psql -h <cluster-endpoint> -U postgres -d postgres -c "SELECT version();"
```

### 1.3 ECS Cluster Setup

#### CREATE OPERATION
```bash
# Create ECS cluster
aws ecs create-cluster \
  --cluster-name mistersmith-cluster \
  --capacity-providers FARGATE FARGATE_SPOT \
  --default-capacity-provider-strategy capacityProvider=FARGATE,weight=1,base=1 \
  --settings name=containerInsights,value=enabled

# Create CloudWatch log group
aws logs create-log-group \
  --log-group-name /ecs/mistersmith

# Create ECR repositories
aws ecr create-repository \
  --repository-name mistersmith/orchestrator \
  --image-scanning-configuration scanOnPush=true

aws ecr create-repository \
  --repository-name mistersmith/agent \
  --image-scanning-configuration scanOnPush=true

aws ecr create-repository \
  --repository-name mistersmith/discovery-engine \
  --image-scanning-configuration scanOnPush=true
```

#### ROLLBACK OPERATION
```bash
# Delete ECR repositories
aws ecr delete-repository \
  --repository-name mistersmith/orchestrator \
  --force

aws ecr delete-repository \
  --repository-name mistersmith/agent \
  --force

aws ecr delete-repository \
  --repository-name mistersmith/discovery-engine \
  --force

# Delete log group
aws logs delete-log-group \
  --log-group-name /ecs/mistersmith

# Delete ECS cluster
aws ecs delete-cluster \
  --cluster mistersmith-cluster
```

#### VERIFICATION
```bash
# Check cluster status
aws ecs describe-clusters --clusters mistersmith-cluster

# List ECR repositories
aws ecr describe-repositories \
  --repository-names mistersmith/orchestrator mistersmith/agent mistersmith/discovery-engine

# Check log group
aws logs describe-log-groups \
  --log-group-name-prefix /ecs/mistersmith
```

### 1.4 Security Groups

#### CREATE OPERATION
```bash
# Create Aurora security group
aws ec2 create-security-group \
  --group-name mistersmith-aurora-sg \
  --description "MisterSmith Aurora security group" \
  --vpc-id <vpc-id>

aws ec2 authorize-security-group-ingress \
  --group-id <aurora-sg-id> \
  --protocol tcp \
  --port 5432 \
  --source-group <ecs-sg-id>

# Create ECS security group
aws ec2 create-security-group \
  --group-name mistersmith-ecs-sg \
  --description "MisterSmith ECS security group" \
  --vpc-id <vpc-id>

# Allow NATS communication
aws ec2 authorize-security-group-ingress \
  --group-id <ecs-sg-id> \
  --protocol tcp \
  --port 4222 \
  --source-group <ecs-sg-id>

# Allow WebSocket for discovery
aws ec2 authorize-security-group-ingress \
  --group-id <ecs-sg-id> \
  --protocol tcp \
  --port 8080 \
  --source-group <ecs-sg-id>
```

#### ROLLBACK OPERATION
```bash
# Revoke all rules and delete security groups
aws ec2 revoke-security-group-ingress \
  --group-id <aurora-sg-id> \
  --ip-permissions '[{"IpProtocol": "tcp", "FromPort": 5432, "ToPort": 5432, "UserIdGroupPairs": [{"GroupId": "<ecs-sg-id>"}]}]'

aws ec2 revoke-security-group-ingress \
  --group-id <ecs-sg-id> \
  --ip-permissions '[{"IpProtocol": "tcp", "FromPort": 4222, "ToPort": 4222, "UserIdGroupPairs": [{"GroupId": "<ecs-sg-id>"}]}]'

aws ec2 delete-security-group --group-id <aurora-sg-id>
aws ec2 delete-security-group --group-id <ecs-sg-id>
```

### 1.5 IAM Roles and Policies

#### CREATE OPERATION
```bash
# Create ECS task execution role
aws iam create-role \
  --role-name MisterSmithECSTaskExecutionRole \
  --assume-role-policy-document '{
    "Version": "2012-10-17",
    "Statement": [{
      "Effect": "Allow",
      "Principal": {"Service": "ecs-tasks.amazonaws.com"},
      "Action": "sts:AssumeRole"
    }]
  }'

# Attach managed policy
aws iam attach-role-policy \
  --role-name MisterSmithECSTaskExecutionRole \
  --policy-arn arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy

# Create task role
aws iam create-role \
  --role-name MisterSmithECSTaskRole \
  --assume-role-policy-document '{
    "Version": "2012-10-17",
    "Statement": [{
      "Effect": "Allow",
      "Principal": {"Service": "ecs-tasks.amazonaws.com"},
      "Action": "sts:AssumeRole"
    }]
  }'

# Create custom policy for task role
aws iam create-policy \
  --policy-name MisterSmithTaskPolicy \
  --policy-document '{
    "Version": "2012-10-17",
    "Statement": [
      {
        "Effect": "Allow",
        "Action": [
          "s3:GetObject",
          "s3:PutObject"
        ],
        "Resource": "arn:aws:s3:::mistersmith-artifacts/*"
      },
      {
        "Effect": "Allow",
        "Action": [
          "secretsmanager:GetSecretValue"
        ],
        "Resource": "arn:aws:secretsmanager:*:*:secret:mistersmith/*"
      },
      {
        "Effect": "Allow",
        "Action": [
          "xray:PutTraceSegments",
          "xray:PutTelemetryRecords"
        ],
        "Resource": "*"
      }
    ]
  }'

# Attach custom policy
aws iam attach-role-policy \
  --role-name MisterSmithECSTaskRole \
  --policy-arn <policy-arn>
```

#### ROLLBACK OPERATION
```bash
# Detach and delete policies
aws iam detach-role-policy \
  --role-name MisterSmithECSTaskExecutionRole \
  --policy-arn arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy

aws iam detach-role-policy \
  --role-name MisterSmithECSTaskRole \
  --policy-arn <policy-arn>

aws iam delete-policy --policy-arn <policy-arn>

# Delete roles
aws iam delete-role --role-name MisterSmithECSTaskExecutionRole
aws iam delete-role --role-name MisterSmithECSTaskRole
```

---

## 2. APPLICATION OPERATIONS

### 2.1 Container Image Build/Push

#### CREATE OPERATION
```bash
# Get ECR login token
aws ecr get-login-password --region us-east-1 | \
  docker login --username AWS --password-stdin <account-id>.dkr.ecr.us-east-1.amazonaws.com

# Build images
docker build -t mistersmith/orchestrator:latest -f Dockerfile.orchestrator .
docker build -t mistersmith/agent:latest -f Dockerfile.agent .
docker build -t mistersmith/discovery-engine:latest -f Dockerfile.discovery .

# Tag for ECR
docker tag mistersmith/orchestrator:latest \
  <account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/orchestrator:latest

docker tag mistersmith/agent:latest \
  <account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/agent:latest

docker tag mistersmith/discovery-engine:latest \
  <account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/discovery-engine:latest

# Push to ECR
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/orchestrator:latest
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/agent:latest
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/discovery-engine:latest
```

#### ROLLBACK OPERATION
```bash
# Delete image from ECR
aws ecr batch-delete-image \
  --repository-name mistersmith/orchestrator \
  --image-ids imageTag=latest

aws ecr batch-delete-image \
  --repository-name mistersmith/agent \
  --image-ids imageTag=latest

aws ecr batch-delete-image \
  --repository-name mistersmith/discovery-engine \
  --image-ids imageTag=latest

# Remove local images
docker rmi <account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/orchestrator:latest
docker rmi <account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/agent:latest
docker rmi <account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/discovery-engine:latest
```

### 2.2 Task Definition Registration

#### CREATE OPERATION
```bash
# Register orchestrator task definition
aws ecs register-task-definition \
  --family mistersmith-orchestrator \
  --network-mode awsvpc \
  --requires-compatibilities FARGATE \
  --cpu "512" \
  --memory "1024" \
  --execution-role-arn arn:aws:iam::<account-id>:role/MisterSmithECSTaskExecutionRole \
  --task-role-arn arn:aws:iam::<account-id>:role/MisterSmithECSTaskRole \
  --container-definitions '[
    {
      "name": "orchestrator",
      "image": "<account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/orchestrator:latest",
      "portMappings": [
        {"containerPort": 4222, "protocol": "tcp"},
        {"containerPort": 8080, "protocol": "tcp"}
      ],
      "environment": [
        {"name": "NATS_URL", "value": "nats://localhost:4222"},
        {"name": "AWS_REGION", "value": "us-east-1"}
      ],
      "secrets": [
        {
          "name": "DB_PASSWORD",
          "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:mistersmith/db-password"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/mistersmith",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "orchestrator"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:8080/health || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 60
      }
    }
  ]'

# Register agent task definition
aws ecs register-task-definition \
  --family mistersmith-agent \
  --network-mode awsvpc \
  --requires-compatibilities FARGATE \
  --cpu "256" \
  --memory "512" \
  --execution-role-arn arn:aws:iam::<account-id>:role/MisterSmithECSTaskExecutionRole \
  --task-role-arn arn:aws:iam::<account-id>:role/MisterSmithECSTaskRole \
  --container-definitions '[
    {
      "name": "agent",
      "image": "<account-id>.dkr.ecr.us-east-1.amazonaws.com/mistersmith/agent:latest",
      "environment": [
        {"name": "NATS_URL", "value": "nats://orchestrator.local:4222"},
        {"name": "AGENT_TYPE", "value": "worker"}
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/mistersmith",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "agent"
        }
      }
    }
  ]'
```

#### ROLLBACK OPERATION
```bash
# Deregister task definitions
aws ecs deregister-task-definition \
  --task-definition mistersmith-orchestrator:1

aws ecs deregister-task-definition \
  --task-definition mistersmith-agent:1
```

### 2.3 Service Deployment

#### CREATE OPERATION
```bash
# Create orchestrator service
aws ecs create-service \
  --cluster mistersmith-cluster \
  --service-name mistersmith-orchestrator \
  --task-definition mistersmith-orchestrator:1 \
  --desired-count 1 \
  --launch-type FARGATE \
  --network-configuration '{
    "awsvpcConfiguration": {
      "subnets": ["<subnet-1a-id>", "<subnet-1b-id>"],
      "securityGroups": ["<ecs-sg-id>"],
      "assignPublicIp": "DISABLED"
    }
  }' \
  --service-registries '[{
    "registryArn": "<service-discovery-arn>",
    "containerName": "orchestrator",
    "containerPort": 4222
  }]' \
  --health-check-grace-period-seconds 60

# Create agent service
aws ecs create-service \
  --cluster mistersmith-cluster \
  --service-name mistersmith-agent \
  --task-definition mistersmith-agent:1 \
  --desired-count 3 \
  --launch-type FARGATE \
  --network-configuration '{
    "awsvpcConfiguration": {
      "subnets": ["<subnet-1a-id>", "<subnet-1b-id>"],
      "securityGroups": ["<ecs-sg-id>"],
      "assignPublicIp": "DISABLED"
    }
  }'
```

#### ROLLBACK OPERATION
```bash
# Update services to 0 desired count
aws ecs update-service \
  --cluster mistersmith-cluster \
  --service mistersmith-orchestrator \
  --desired-count 0

aws ecs update-service \
  --cluster mistersmith-cluster \
  --service mistersmith-agent \
  --desired-count 0

# Wait for tasks to stop
aws ecs wait services-stable \
  --cluster mistersmith-cluster \
  --services mistersmith-orchestrator mistersmith-agent

# Delete services
aws ecs delete-service \
  --cluster mistersmith-cluster \
  --service mistersmith-orchestrator \
  --force

aws ecs delete-service \
  --cluster mistersmith-cluster \
  --service mistersmith-agent \
  --force
```

### 2.4 Environment Variables

#### CREATE OPERATION
```bash
# Store secrets in AWS Secrets Manager
aws secretsmanager create-secret \
  --name mistersmith/db-password \
  --secret-string '<secure-password>'

aws secretsmanager create-secret \
  --name mistersmith/nats-credentials \
  --secret-string '{
    "user": "mistersmith",
    "password": "<secure-password>"
  }'

# Create parameter store entries
aws ssm put-parameter \
  --name /mistersmith/config/nats-url \
  --value "nats://orchestrator.local:4222" \
  --type String

aws ssm put-parameter \
  --name /mistersmith/config/db-host \
  --value "<aurora-endpoint>" \
  --type String

aws ssm put-parameter \
  --name /mistersmith/config/discovery-rate \
  --value "150000" \
  --type String
```

#### ROLLBACK OPERATION
```bash
# Delete secrets
aws secretsmanager delete-secret \
  --secret-id mistersmith/db-password \
  --force-delete-without-recovery

aws secretsmanager delete-secret \
  --secret-id mistersmith/nats-credentials \
  --force-delete-without-recovery

# Delete parameters
aws ssm delete-parameter --name /mistersmith/config/nats-url
aws ssm delete-parameter --name /mistersmith/config/db-host
aws ssm delete-parameter --name /mistersmith/config/discovery-rate
```

---

## 3. DATA OPERATIONS

### 3.1 PostgreSQL Schema Migration

#### CREATE OPERATION
```bash
# Connect to Aurora and create schema
psql -h <aurora-endpoint> -U postgres -d postgres << EOF
-- Create database
CREATE DATABASE mistersmith;

-- Connect to mistersmith database
\c mistersmith

-- Create schema
CREATE SCHEMA IF NOT EXISTS core;

-- Create tables
CREATE TABLE core.agents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL,
    status VARCHAR(50) DEFAULT 'inactive',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE core.discoveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID REFERENCES core.agents(id),
    type VARCHAR(50) NOT NULL,
    content TEXT NOT NULL,
    confidence DECIMAL(3,2) CHECK (confidence >= 0 AND confidence <= 1),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE core.coordination_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    swarm_id VARCHAR(255) NOT NULL,
    state JSONB NOT NULL,
    version INTEGER DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes
CREATE INDEX idx_agents_type ON core.agents(type);
CREATE INDEX idx_agents_status ON core.agents(status);
CREATE INDEX idx_discoveries_agent_id ON core.discoveries(agent_id);
CREATE INDEX idx_discoveries_type ON core.discoveries(type);
CREATE INDEX idx_discoveries_created_at ON core.discoveries(created_at DESC);
CREATE INDEX idx_coordination_swarm_id ON core.coordination_state(swarm_id);

-- Create update trigger
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_agents_updated_at
    BEFORE UPDATE ON core.agents
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER update_coordination_updated_at
    BEFORE UPDATE ON core.coordination_state
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
EOF
```

#### ROLLBACK OPERATION
```bash
# Drop schema and database
psql -h <aurora-endpoint> -U postgres -d postgres << EOF
-- Drop database (cascades all objects)
DROP DATABASE IF EXISTS mistersmith;
EOF
```

#### VERIFICATION
```bash
# Verify schema creation
psql -h <aurora-endpoint> -U postgres -d mistersmith << EOF
-- List tables
\dt core.*

-- Check table structures
\d core.agents
\d core.discoveries
\d core.coordination_state

-- Verify indexes
\di core.*
EOF
```

### 3.2 JetStream to DynamoDB Migration

#### CREATE OPERATION
```bash
# Create DynamoDB table for stream data
aws dynamodb create-table \
  --table-name MisterSmithStreams \
  --attribute-definitions \
    AttributeName=StreamName,AttributeType=S \
    AttributeName=SequenceNumber,AttributeType=N \
  --key-schema \
    AttributeName=StreamName,KeyType=HASH \
    AttributeName=SequenceNumber,KeyType=RANGE \
  --provisioned-throughput ReadCapacityUnits=5,WriteCapacityUnits=5 \
  --stream-specification StreamEnabled=true,StreamViewType=NEW_AND_OLD_IMAGES

# Create table for discovery cache
aws dynamodb create-table \
  --table-name MisterSmithDiscoveryCache \
  --attribute-definitions \
    AttributeName=DiscoveryId,AttributeType=S \
    AttributeName=Timestamp,AttributeType=N \
  --key-schema \
    AttributeName=DiscoveryId,KeyType=HASH \
    AttributeName=Timestamp,KeyType=RANGE \
  --provisioned-throughput ReadCapacityUnits=10,WriteCapacityUnits=10 \
  --global-secondary-indexes '[{
    "IndexName": "TypeIndex",
    "Keys": [
      {"AttributeName": "Type", "KeyType": "HASH"},
      {"AttributeName": "Timestamp", "KeyType": "RANGE"}
    ],
    "Projection": {"ProjectionType": "ALL"},
    "ProvisionedThroughput": {
      "ReadCapacityUnits": 5,
      "WriteCapacityUnits": 5
    }
  }]'

# Wait for tables to be active
aws dynamodb wait table-exists --table-name MisterSmithStreams
aws dynamodb wait table-exists --table-name MisterSmithDiscoveryCache
```

#### ROLLBACK OPERATION
```bash
# Delete DynamoDB tables
aws dynamodb delete-table --table-name MisterSmithStreams
aws dynamodb delete-table --table-name MisterSmithDiscoveryCache
```

### 3.3 Connection String Updates

#### CREATE OPERATION
```bash
# Update task definitions with new connection strings
aws ecs register-task-definition \
  --family mistersmith-orchestrator \
  --revision 2 \
  --container-definitions '[{
    ...existing config...,
    "environment": [
      {"name": "DATABASE_URL", "value": "postgresql://postgres:<password>@<aurora-endpoint>:5432/mistersmith"},
      {"name": "DYNAMODB_STREAMS_TABLE", "value": "MisterSmithStreams"},
      {"name": "DYNAMODB_CACHE_TABLE", "value": "MisterSmithDiscoveryCache"}
    ]
  }]'

# Update services to use new task definition
aws ecs update-service \
  --cluster mistersmith-cluster \
  --service mistersmith-orchestrator \
  --task-definition mistersmith-orchestrator:2
```

#### ROLLBACK OPERATION
```bash
# Revert to previous task definition
aws ecs update-service \
  --cluster mistersmith-cluster \
  --service mistersmith-orchestrator \
  --task-definition mistersmith-orchestrator:1
```

### 3.4 State Synchronization

#### CREATE OPERATION
```bash
# Export current state from local system
docker exec mistersmith-orchestrator \
  /app/bin/export-state --format json > state-backup.json

# Import state to Aurora
psql -h <aurora-endpoint> -U postgres -d mistersmith << EOF
\copy core.coordination_state (swarm_id, state, version) 
FROM 'state-backup.json' 
WITH (FORMAT csv, DELIMITER E'\t', QUOTE E'\b');
EOF

# Sync JetStream to DynamoDB
docker run --rm \
  -e NATS_URL=nats://localhost:4222 \
  -e DYNAMODB_TABLE=MisterSmithStreams \
  -e AWS_REGION=us-east-1 \
  mistersmith/jetstream-migrator:latest
```

#### ROLLBACK OPERATION
```bash
# Clear imported state
psql -h <aurora-endpoint> -U postgres -d mistersmith << EOF
DELETE FROM core.coordination_state 
WHERE created_at >= NOW() - INTERVAL '1 hour';
EOF

# Clear DynamoDB data
aws dynamodb scan --table-name MisterSmithStreams \
  --projection-expression "StreamName,SequenceNumber" \
  | jq -r '.Items[] | @base64' \
  | while read item; do
      aws dynamodb delete-item \
        --table-name MisterSmithStreams \
        --key "$(echo $item | base64 -d)"
    done
```

---

## 4. ROLLBACK MECHANISMS

### 4.1 State Checkpoints

```bash
# Create checkpoint before major changes
aws backup create-backup-plan \
  --backup-plan '{
    "BackupPlanName": "MisterSmithMigrationCheckpoint",
    "Rules": [{
      "RuleName": "DailyBackups",
      "TargetBackupVaultName": "Default",
      "ScheduleExpression": "cron(0 5 ? * * *)",
      "StartWindowMinutes": 60,
      "CompletionWindowMinutes": 120,
      "Lifecycle": {
        "DeleteAfterDays": 7
      }
    }]
  }'

# Create on-demand backup
aws backup start-backup-job \
  --backup-vault-name Default \
  --resource-arn arn:aws:rds:us-east-1:<account-id>:cluster:mistersmith-aurora-cluster \
  --iam-role-arn arn:aws:iam::<account-id>:role/service-role/AWSBackupDefaultServiceRole
```

### 4.2 Automated Rollback Triggers

```bash
# CloudWatch alarm for high error rate
aws cloudwatch put-metric-alarm \
  --alarm-name MisterSmith-HighErrorRate \
  --alarm-description "Trigger rollback on high error rate" \
  --actions-enabled \
  --alarm-actions arn:aws:sns:us-east-1:<account-id>:mistersmith-alerts \
  --metric-name 4XXError \
  --namespace AWS/ECS \
  --statistic Sum \
  --dimensions Name=ServiceName,Value=mistersmith-orchestrator \
  --period 300 \
  --evaluation-periods 2 \
  --threshold 100 \
  --comparison-operator GreaterThanThreshold

# Lambda function for automated rollback
aws lambda create-function \
  --function-name MisterSmithRollbackHandler \
  --runtime python3.9 \
  --role arn:aws:iam::<account-id>:role/lambda-rollback-role \
  --handler rollback.handler \
  --code S3Bucket=mistersmith-artifacts,S3Key=rollback-handler.zip \
  --environment Variables={CLUSTER_NAME=mistersmith-cluster}
```

---

## 5. VERIFICATION REQUIREMENTS

### 5.1 Health Check Endpoints

```bash
# Orchestrator health check
curl -f http://<alb-endpoint>:8080/health

# Expected response:
{
  "status": "healthy",
  "version": "1.0.0",
  "components": {
    "nats": "connected",
    "database": "connected",
    "discovery_engine": "active"
  },
  "metrics": {
    "agents_active": 3,
    "discoveries_per_second": 150000,
    "memory_usage_mb": 256
  }
}
```

### 5.2 Database Connectivity

```bash
# Test Aurora connection
aws rds describe-db-cluster-endpoints \
  --db-cluster-identifier mistersmith-aurora-cluster

# Test query execution
psql -h <aurora-endpoint> -U postgres -d mistersmith \
  -c "SELECT COUNT(*) FROM core.agents;"
```

### 5.3 Message Flow Verification

```bash
# Check NATS connectivity
docker run --rm -it nats-io/nats-box \
  nats sub -s nats://<orchestrator-endpoint>:4222 "discovery.*"

# Publish test message
docker run --rm -it nats-io/nats-box \
  nats pub -s nats://<orchestrator-endpoint>:4222 \
  "discovery.test" '{"type":"test","content":"verification"}'
```

### 5.4 ECS Task Status

```bash
# Check running tasks
aws ecs list-tasks \
  --cluster mistersmith-cluster \
  --service-name mistersmith-orchestrator \
  --desired-status RUNNING

# Get task details
aws ecs describe-tasks \
  --cluster mistersmith-cluster \
  --tasks <task-arn>
```

---

## Operation Execution Order

### FORWARD SEQUENCE
1. **Infrastructure Setup**
   - VPC and networking
   - Security groups
   - IAM roles
   - Aurora cluster
   - ECS cluster

2. **Application Deployment**
   - ECR repositories
   - Container builds
   - Task definitions
   - Service creation

3. **Data Migration**
   - Schema creation
   - State export
   - Connection updates
   - State import

4. **Verification**
   - Health checks
   - Connectivity tests
   - Performance validation

### ROLLBACK SEQUENCE
1. **Stop Services**
   - Scale to 0
   - Wait for termination
   - Delete services

2. **Clean Data**
   - Export current state
   - Clear migrated data
   - Drop schemas

3. **Remove Application**
   - Delete task definitions
   - Remove container images
   - Delete ECR repositories

4. **Teardown Infrastructure**
   - Delete ECS cluster
   - Remove Aurora cluster
   - Clean up networking
   - Delete IAM resources

---

**Last Updated**: 2025-01-11T13:18:00Z  
**Next Review**: After each operation execution  
**Contact**: MS-3 Migration Architect via Claude Flow