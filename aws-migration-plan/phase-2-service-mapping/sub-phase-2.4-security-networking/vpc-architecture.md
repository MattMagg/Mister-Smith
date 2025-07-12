# VPC Architecture Design for MisterSmith AWS Migration

## Overview

This document defines the Virtual Private Cloud (VPC) architecture for the MisterSmith platform on AWS, implementing security best practices and CDK Nag compliance.

## VPC Design Specifications

### Network Topology

```
Production VPC: 10.0.0.0/16 (65,536 IPs)
├── Public Subnets (Internet-facing)
│   ├── us-east-1a: 10.0.1.0/24 (256 IPs) - ALB
│   └── us-east-1b: 10.0.2.0/24 (256 IPs) - ALB
├── Private Subnets (Application Layer)
│   ├── us-east-1a: 10.0.10.0/23 (512 IPs) - ECS Tasks
│   └── us-east-1b: 10.0.12.0/23 (512 IPs) - ECS Tasks
├── Database Subnets (Isolated)
│   ├── us-east-1a: 10.0.20.0/24 (256 IPs) - Aurora
│   └── us-east-1b: 10.0.21.0/24 (256 IPs) - Aurora
└── Messaging Subnets (Isolated)
    ├── us-east-1a: 10.0.30.0/24 (256 IPs) - Amazon MQ
    └── us-east-1b: 10.0.31.0/24 (256 IPs) - Amazon MQ
```

### Multi-AZ Configuration

- **Availability Zones**: us-east-1a, us-east-1b (minimum 2 AZs)
- **Subnet Distribution**: Each tier spans multiple AZs for high availability
- **NAT Gateways**: One per AZ for redundancy
- **Load Balancer**: Multi-AZ deployment for fault tolerance

### Network Components

#### 1. Internet Gateway (IGW)
- Attached to VPC for public subnet internet access
- Used by ALB and NAT Gateways only

#### 2. NAT Gateways
- **Count**: 2 (one per AZ)
- **Purpose**: Outbound internet for private subnets
- **Placement**: Public subnets
- **High Availability**: Multi-AZ deployment

#### 3. VPC Endpoints (PrivateLink)
```yaml
Interface Endpoints:
  - com.amazonaws.region.ecs         # ECS API
  - com.amazonaws.region.ecs-agent   # ECS Agent
  - com.amazonaws.region.ecs-telemetry # ECS Telemetry
  - com.amazonaws.region.ecr.api     # ECR API
  - com.amazonaws.region.ecr.dkr     # ECR Docker Registry
  - com.amazonaws.region.logs        # CloudWatch Logs
  - com.amazonaws.region.secretsmanager # Secrets Manager
  - com.amazonaws.region.ssm         # Systems Manager
  - com.amazonaws.region.monitoring  # CloudWatch Metrics

Gateway Endpoints:
  - com.amazonaws.region.s3          # S3 Access
```

#### 4. Route Tables
```yaml
Public Route Table:
  - 0.0.0.0/0 → Internet Gateway
  - 10.0.0.0/16 → Local

Private Route Table (per AZ):
  - 0.0.0.0/0 → NAT Gateway (same AZ)
  - 10.0.0.0/16 → Local
  - S3 Prefix List → S3 Gateway Endpoint

Database Route Table:
  - 10.0.0.0/16 → Local
  - No internet access

Messaging Route Table:
  - 10.0.0.0/16 → Local
  - No internet access
```

### Network ACLs

#### Default Network ACL Rules
```yaml
Inbound:
  - Rule 100: Allow all traffic from VPC CIDR
  - Rule 200: Allow return traffic (1024-65535)
  - Rule *: Deny all

Outbound:
  - Rule 100: Allow all traffic to VPC CIDR
  - Rule 200: Allow HTTPS (443) to 0.0.0.0/0
  - Rule 300: Allow HTTP (80) to 0.0.0.0/0
  - Rule *: Deny all
```

#### Database Subnet ACL (Restrictive)
```yaml
Inbound:
  - Rule 100: Allow PostgreSQL (5432) from Private Subnets
  - Rule *: Deny all

Outbound:
  - Rule 100: Allow return traffic to Private Subnets
  - Rule *: Deny all
```

### VPC Flow Logs (CDK Nag: VPC7)

```yaml
Configuration:
  - Destination: CloudWatch Logs
  - Traffic Type: ALL (Accept, Reject, All)
  - Format: Custom format with additional fields
  - Retention: 30 days
  - Encryption: CloudWatch Logs default encryption

Custom Format Fields:
  - version
  - account-id
  - interface-id
  - srcaddr
  - dstaddr
  - srcport
  - dstport
  - protocol
  - packets
  - bytes
  - start
  - end
  - action
  - log-status
  - vpc-id
  - subnet-id
  - instance-id
  - tcp-flags
  - type
  - pkt-srcaddr
  - pkt-dstaddr
```

### DNS Configuration

```yaml
VPC Settings:
  - Enable DNS Resolution: true
  - Enable DNS Hostnames: true
  
Route 53 Private Hosted Zone:
  - Domain: mistersmith.internal
  - Associated VPCs: Production VPC
  
Service Discovery:
  - Namespace: mistersmith.local
  - Type: Private DNS namespace
  - Services: Auto-register ECS services
```

### Disaster Recovery Considerations

1. **Multi-Region Readiness**
   - VPC CIDR designed to not conflict with other regions
   - Reserved CIDR blocks for future regions:
     - us-west-2: 10.1.0.0/16
     - eu-west-1: 10.2.0.0/16

2. **Backup Connectivity**
   - AWS Direct Connect ready (Virtual Interface support)
   - Site-to-Site VPN as backup connection option

3. **Network Resilience**
   - Multi-AZ NAT Gateways
   - Cross-AZ subnet connectivity
   - No single points of failure

## CDK Implementation Notes

### CDK Constructs
```typescript
// Use L2 constructs for VPC
import { Vpc, SubnetType, InterfaceVpcEndpoint } from 'aws-cdk-lib/aws-ec2';

// Enable Flow Logs (VPC7 compliance)
new FlowLog(this, 'VpcFlowLog', {
  resourceType: FlowLogResourceType.fromVpc(vpc),
  trafficType: FlowLogTrafficType.ALL,
  destination: FlowLogDestination.toCloudWatchLogs()
});
```

### Tagging Strategy
```yaml
Mandatory Tags:
  - Environment: production
  - Project: mistersmith
  - ManagedBy: cdk
  - CostCenter: engineering
  - SecurityLevel: high
```

## Security Considerations

1. **Network Isolation**
   - Database and messaging tiers have no direct internet access
   - All outbound traffic from private subnets goes through NAT Gateway
   - VPC endpoints reduce internet exposure

2. **Traffic Monitoring**
   - VPC Flow Logs enabled for all traffic
   - Integration with CloudWatch for alerting
   - GuardDuty monitoring enabled

3. **Access Control**
   - Security groups provide instance-level firewall
   - Network ACLs provide subnet-level defense in depth
   - No default allow rules

## Next Steps

1. Implement security group designs (see security-groups.md)
2. Configure IAM roles and policies (see iam-design.md)
3. Set up secrets management (see secrets-management.md)
4. Deploy CDK stack with proper suppressions