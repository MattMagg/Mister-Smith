# Sub-Phase 1.1: Network Foundation - Living Tracker

## Sub-Phase Metadata
- **Parent Phase**: 1 - Infrastructure Blueprint
- **Sub-Phase Number**: 1.1
- **Sub-Phase Name**: Network Foundation
- **Lead Agent**: [ASSIGN: Network Specialist]
- **Status**: NOT_STARTED
- **Progress**: 0%

## Execution Context
**From Phase 0**: 
- Current network: Single machine localhost
- Target scale: 1-1000 concurrent agents
- Availability requirement: Multi-AZ
- Security requirement: Zero-trust networking

## Task Execution Details

### Task 1.1.1: VPC CIDR Planning
**Assigned Agent**: [AWAITING]
**Status**: NOT_STARTED

**Instructions**:
```python
# Calculate CIDR requirements
agent_count_max = 1000
container_overhead = 2  # supervisor + gateway
services_count = 10  # auxiliary services
ip_buffer = 2.0  # 100% headroom

total_ips_needed = (agent_count_max + container_overhead + services_count) * ip_buffer
cidr_block = calculate_cidr(total_ips_needed)  # Aim for /20 or larger
```

**Decision Points**:
1. Primary CIDR: 10.0.0.0/16 or 172.31.0.0/16?
2. Reserve space for future VPC peering?
3. Subnet sizing strategy (equal vs weighted)

**Deliverable**: 
- CIDR allocation plan
- IP address inventory
- Growth projections

**Memory Write**: `aws-migration/phase-1/network/vpc-design/cidr-plan`

### Task 1.1.2: Multi-AZ Subnet Design
**Assigned Agent**: [AWAITING]
**Status**: NOT_STARTED

**Instructions**:
1. Read Phase 0 availability requirements
2. Design subnet layout:
   ```
   Per AZ:
   - Public subnet: /24 (ALB, NAT Gateway)
   - Private subnet 1: /22 (ECS tasks)
   - Private subnet 2: /24 (RDS, ElastiCache)
   - Private subnet 3: /24 (Amazon MQ)
   ```
3. Create subnet allocation matrix

**AWS MCP Commands**:
```bash
# Get VPC best practices
mcp__awslabs-aws-documentation-mcp-server__search_documentation \
  search_phrase="VPC subnet design best practices multi-az"

# Get CDK construct
mcp__awslabs-cdk-mcp-server__GetAwsSolutionsConstructPattern \
  pattern_name="aws-vpc"
```

**Memory Write**: `aws-migration/phase-1/network/subnet-allocation/design`

### Task 1.1.3: NAT Gateway Configuration
**Assigned Agent**: [AWAITING]
**Status**: NOT_STARTED

**Cost Decision Required**:
- Option A: Single NAT Gateway (lower cost, SPOF)
- Option B: NAT Gateway per AZ (high availability)

**Analysis Required**:
1. Calculate egress data estimates
2. Cost implications of each option
3. Failover time requirements

**Memory Write**: `aws-migration/phase-1/network/routing-tables/nat-config`

### Task 1.1.4: Security Group Design
**Assigned Agent**: [AWAITING]
**Status**: NOT_STARTED

**Security Groups to Design**:
1. **alb-sg**: Internet → ALB (443 only)
2. **ecs-supervisor-sg**: ALB → Supervisor
3. **ecs-worker-sg**: Supervisor → Workers
4. **rds-sg**: ECS → Database (5432)
5. **mq-sg**: ECS → MQ (61614 STOMP)
6. **vpc-endpoints-sg**: ECS → AWS APIs

**Principle**: Least privilege, explicit deny default

**Memory Write**: `aws-migration/phase-1/network/security-groups/design`

### Task 1.1.5: VPC Endpoints
**Assigned Agent**: [AWAITING]
**Status**: NOT_STARTED

**Required Endpoints** (to avoid NAT costs):
- S3 Gateway endpoint
- ECR API & DKR endpoints
- CloudWatch Logs endpoint
- X-Ray endpoint
- Secrets Manager endpoint

**Cost Analysis**: Compare endpoint cost vs NAT Gateway data transfer

**Memory Write**: `aws-migration/phase-1/network/endpoints/design`

## Inter-Agent Coordination

### Information Flow:
```
Task 1.1.1 (CIDR) → Task 1.1.2 (Subnets)
                  ↓
            Task 1.1.3 (NAT)
                  ↓
            Task 1.1.4 (Security Groups)
                  ↓
            Task 1.1.5 (Endpoints)
```

### Synchronization Points:
- After 1.1.1: CIDR decision locks subnet sizing
- After 1.1.2: Subnet IDs needed for routes
- After 1.1.3: Route tables finalized
- After 1.1.4: Security posture defined

## Progress Matrix

| Task | Agent | Start Time | End Time | Duration | Output Memory Ref | Status |
|------|-------|------------|----------|----------|-------------------|--------|
| 1.1.1 | - | - | - | - | - | NOT_STARTED |
| 1.1.2 | - | - | - | - | - | NOT_STARTED |
| 1.1.3 | - | - | - | - | - | NOT_STARTED |
| 1.1.4 | - | - | - | - | - | NOT_STARTED |
| 1.1.5 | - | - | - | - | - | NOT_STARTED |

## Decision Log
[Agents record all decisions with rationale]

## Issue Tracking
[Agents report issues immediately]

## Sub-Phase Completion Criteria
- [ ] All tasks 100% complete
- [ ] CDK code generated and tested
- [ ] Network diagram generated
- [ ] Security review completed
- [ ] Cost analysis documented
- [ ] Parent phase updated

## Handoff Package Contents
1. VPC CDK stack: `aws-migration/phase-1/iac/cdk/stacks/vpc-stack.ts`
2. Subnet ID mapping: `aws-migration/phase-1/network/subnet-allocation/ids.json`
3. Security group matrix: `aws-migration/phase-1/network/security-groups/matrix.json`
4. Network diagram: `aws-migration/phase-1/network/diagrams/network.png`
5. Cost breakdown: `aws-migration/phase-1/network/costs/analysis.md`

---
*Sub-Phase Initialized*: [timestamp]
*Last Update*: [agent/timestamp]
*Health*: [GREEN|YELLOW|RED]
*Blocks Parent*: [YES|NO]