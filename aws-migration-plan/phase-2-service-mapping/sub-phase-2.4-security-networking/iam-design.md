# IAM Roles and Policies Design for MisterSmith AWS Migration

## Overview

This document defines the IAM roles and policies for MisterSmith services, following AWS best practices and avoiding CDK Nag rule violations (particularly AwsSolutions-IAM4).

## IAM Design Principles

1. **Least Privilege**: Grant only necessary permissions
2. **Customer Managed Policies**: Avoid AWS managed policies (IAM4 compliance)
3. **Resource Scoping**: Restrict to specific resources where possible
4. **Separation of Duties**: Different roles for different functions
5. **Temporary Credentials**: Use roles, not users

## Role Definitions

### 1. ECS Task Execution Role
**Name**: `MisterSmithEcsTaskExecutionRole`
**Purpose**: Used by ECS to pull images and write logs

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "ecr:GetAuthorizationToken",
        "ecr:BatchCheckLayerAvailability",
        "ecr:GetDownloadUrlForLayer",
        "ecr:BatchGetImage"
      ],
      "Resource": [
        "arn:aws:ecr:us-east-1:123456789012:repository/mistersmith/*"
      ]
    },
    {
      "Effect": "Allow",
      "Action": [
        "logs:CreateLogStream",
        "logs:PutLogEvents"
      ],
      "Resource": [
        "arn:aws:logs:us-east-1:123456789012:log-group:/ecs/mistersmith/*:*"
      ]
    },
    {
      "Effect": "Allow",
      "Action": [
        "secretsmanager:GetSecretValue"
      ],
      "Resource": [
        "arn:aws:secretsmanager:us-east-1:123456789012:secret:mistersmith/*"
      ]
    }
  ]
}
```

### 2. ECS Task Role (Application Permissions)
**Name**: `MisterSmithEcsTaskRole`
**Purpose**: Permissions for the application running in ECS

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "S3Access",
      "Effect": "Allow",
      "Action": [
        "s3:GetObject",
        "s3:PutObject",
        "s3:DeleteObject"
      ],
      "Resource": [
        "arn:aws:s3:::mistersmith-data-bucket/*"
      ]
    },
    {
      "Sid": "S3ListBucket",
      "Effect": "Allow",
      "Action": [
        "s3:ListBucket"
      ],
      "Resource": [
        "arn:aws:s3:::mistersmith-data-bucket"
      ]
    },
    {
      "Sid": "SecretAccess",
      "Effect": "Allow",
      "Action": [
        "secretsmanager:GetSecretValue"
      ],
      "Resource": [
        "arn:aws:secretsmanager:us-east-1:123456789012:secret:mistersmith/app/*"
      ]
    },
    {
      "Sid": "ParameterStoreAccess",
      "Effect": "Allow",
      "Action": [
        "ssm:GetParameter",
        "ssm:GetParameters",
        "ssm:GetParametersByPath"
      ],
      "Resource": [
        "arn:aws:ssm:us-east-1:123456789012:parameter/mistersmith/*"
      ]
    },
    {
      "Sid": "CloudWatchMetrics",
      "Effect": "Allow",
      "Action": [
        "cloudwatch:PutMetricData"
      ],
      "Resource": "*",
      "Condition": {
        "StringEquals": {
          "cloudwatch:namespace": "MisterSmith"
        }
      }
    },
    {
      "Sid": "XRayTracing",
      "Effect": "Allow",
      "Action": [
        "xray:PutTraceSegments",
        "xray:PutTelemetryRecords"
      ],
      "Resource": "*"
    },
    {
      "Sid": "ServiceDiscovery",
      "Effect": "Allow",
      "Action": [
        "servicediscovery:DiscoverInstances"
      ],
      "Resource": "*"
    }
  ]
}
```

### 3. Aurora Database Access Role
**Name**: `MisterSmithAuroraRole`
**Purpose**: IAM database authentication for Aurora

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "rds-db:connect"
      ],
      "Resource": [
        "arn:aws:rds-db:us-east-1:123456789012:dbuser:mistersmith-aurora-cluster/mistersmith_app"
      ]
    }
  ]
}
```

### 4. GitHub Actions Deployment Role
**Name**: `MisterSmithGitHubActionsRole`
**Purpose**: CI/CD pipeline permissions

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "ECRAccess",
      "Effect": "Allow",
      "Action": [
        "ecr:GetAuthorizationToken",
        "ecr:BatchCheckLayerAvailability",
        "ecr:GetDownloadUrlForLayer",
        "ecr:BatchGetImage",
        "ecr:PutImage",
        "ecr:InitiateLayerUpload",
        "ecr:UploadLayerPart",
        "ecr:CompleteLayerUpload"
      ],
      "Resource": [
        "arn:aws:ecr:us-east-1:123456789012:repository/mistersmith/*"
      ]
    },
    {
      "Sid": "ECSDeployment",
      "Effect": "Allow",
      "Action": [
        "ecs:UpdateService",
        "ecs:DescribeServices",
        "ecs:DescribeTaskDefinition",
        "ecs:RegisterTaskDefinition",
        "ecs:ListTasks",
        "ecs:DescribeTasks"
      ],
      "Resource": [
        "arn:aws:ecs:us-east-1:123456789012:service/mistersmith-cluster/*",
        "arn:aws:ecs:us-east-1:123456789012:task-definition/mistersmith-*:*",
        "arn:aws:ecs:us-east-1:123456789012:task/mistersmith-cluster/*"
      ]
    },
    {
      "Sid": "PassRole",
      "Effect": "Allow",
      "Action": [
        "iam:PassRole"
      ],
      "Resource": [
        "arn:aws:iam::123456789012:role/MisterSmithEcsTaskExecutionRole",
        "arn:aws:iam::123456789012:role/MisterSmithEcsTaskRole"
      ]
    }
  ]
}
```

### 5. CloudWatch Events Role
**Name**: `MisterSmithEventsRole`
**Purpose**: Scheduled tasks and event-driven actions

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "ecs:RunTask"
      ],
      "Resource": [
        "arn:aws:ecs:us-east-1:123456789012:task-definition/mistersmith-scheduled-*:*"
      ],
      "Condition": {
        "ArnLike": {
          "ecs:cluster": "arn:aws:ecs:us-east-1:123456789012:cluster/mistersmith-cluster"
        }
      }
    },
    {
      "Effect": "Allow",
      "Action": [
        "iam:PassRole"
      ],
      "Resource": [
        "arn:aws:iam::123456789012:role/MisterSmithEcsTaskExecutionRole",
        "arn:aws:iam::123456789012:role/MisterSmithEcsTaskRole"
      ]
    }
  ]
}
```

## Trust Relationships

### ECS Task Roles Trust Policy
```json
{
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
```

### GitHub Actions OIDC Trust Policy
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Federated": "arn:aws:iam::123456789012:oidc-provider/token.actions.githubusercontent.com"
      },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "token.actions.githubusercontent.com:aud": "sts.amazonaws.com",
          "token.actions.githubusercontent.com:sub": "repo:mistersmith/mistersmith:ref:refs/heads/main"
        }
      }
    }
  ]
}
```

## Service Control Policies (SCPs)

### Production Protection SCP
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Deny",
      "Action": [
        "ec2:TerminateInstances",
        "rds:DeleteDBCluster",
        "rds:DeleteDBInstance",
        "s3:DeleteBucket"
      ],
      "Resource": "*",
      "Condition": {
        "StringEquals": {
          "aws:RequestedRegion": "us-east-1",
          "aws:ResourceTag/Environment": "production"
        }
      }
    }
  ]
}
```

## CDK Implementation

```typescript
// Custom managed policy - NOT AWS managed (IAM4 compliance)
const ecsTaskPolicy = new ManagedPolicy(this, 'EcsTaskPolicy', {
  managedPolicyName: 'MisterSmithEcsTaskPolicy',
  statements: [
    new PolicyStatement({
      effect: Effect.ALLOW,
      actions: ['s3:GetObject', 's3:PutObject'],
      resources: [`${dataBucket.bucketArn}/*`],
    }),
    // Add other statements
  ],
});

// Task execution role with custom policy
const taskExecutionRole = new Role(this, 'TaskExecutionRole', {
  assumedBy: new ServicePrincipal('ecs-tasks.amazonaws.com'),
  roleName: 'MisterSmithEcsTaskExecutionRole',
  inlinePolicies: {
    EcsTaskExecution: new PolicyDocument({
      statements: [
        // Custom statements instead of AWS managed policy
      ],
    }),
  },
});

// CDK Nag suppressions if absolutely necessary
NagSuppressions.addResourceSuppressions(xrayStatement, [
  {
    id: 'AwsSolutions-IAM5',
    reason: 'X-Ray requires * resource for PutTraceSegments',
  },
]);
```

## Security Best Practices

1. **No AWS Managed Policies** (IAM4 Compliance)
   - All policies are customer-managed
   - Scoped to specific resources
   - Regular review and updates

2. **Resource Constraints**
   - ARNs include account ID and region
   - Wildcards only where necessary
   - Condition keys for additional security

3. **Temporary Credentials**
   - No IAM users created
   - All access through roles
   - Short session durations

4. **Audit and Compliance**
   - CloudTrail logging enabled
   - Access Analyzer findings reviewed
   - Regular permission audits

## Monitoring and Alerts

### CloudWatch Alarms
1. **Unauthorized API Calls**: Alert on AccessDenied events
2. **Role Assumption**: Monitor unusual role assumptions
3. **Policy Changes**: Alert on IAM policy modifications

### Access Analyzer
- Continuous monitoring of resource policies
- External access findings
- Unused access findings

## Regular Review Process

1. **Monthly**: Review Access Analyzer findings
2. **Quarterly**: Audit all custom policies
3. **Annually**: Complete least-privilege review