# Aurora PostgreSQL Migration Tracker

## Sub-Phase 2.2: Aurora Serverless v2 PostgreSQL Validation

### Current Status: ✅ VALIDATED

### 1. Database Requirements Analysis

#### MisterSmith Database Profile
- **Database Type**: PostgreSQL (supervision database)
- **Schema Complexity**: High
  - 10 core supervision tables
  - Custom PostgreSQL types (ENUMs)
  - Complex JSONB queries
  - Partitioned tables for time-series data
  - Triggers and stored functions
  
#### Connection Requirements
- **Max Connections**: 30 (for agent swarms)
- **Connection Pooling**: Required
- **Persistent Connections**: Yes
- **Transaction Isolation**: READ COMMITTED

#### Data Characteristics
- **Initial Size**: 10-50GB
- **Growth Rate**: 100GB+ expected
- **Write Pattern**: High volume for events/metrics
- **Read Pattern**: Complex aggregations, real-time queries

### 2. Aurora PostgreSQL Compatibility

#### ✅ Fully Compatible Features
- PostgreSQL custom types (ENUMs) - SUPPORTED
- JSONB data type and operators - SUPPORTED
- Table partitioning - SUPPORTED
- Foreign key constraints - SUPPORTED
- Triggers and functions - SUPPORTED
- gen_random_uuid() - SUPPORTED
- ACID transactions - SUPPORTED

#### 🔧 Features Requiring Configuration
1. **Connection Pooling**
   - Solution: Use RDS Proxy
   - Benefits: Connection reuse, automatic failover
   
2. **Max Connections**
   - Solution: Configure via Aurora capacity range
   - Recommendation: Start with 2-16 ACUs
   
3. **Performance Tuning**
   - Solution: Use Aurora Query Plan Management
   - Monitor with Performance Insights

### 3. Aurora Serverless v2 Configuration

#### Recommended Configuration
```yaml
Aurora Serverless v2:
  Engine: aurora-postgresql15
  Capacity Range:
    Min: 2 ACUs (4 GB RAM)
    Max: 16 ACUs (32 GB RAM)
  Features:
    - Auto-pause: Disabled (24/7 operation)
    - Multi-AZ: Enabled
    - Backup Retention: 7 days
    - Performance Insights: Enabled
```

#### RDS Proxy Configuration
```yaml
RDS Proxy:
  Connection Pool:
    Max Connections: 30
    Max Idle Connections: 10
    Connection Borrow Timeout: 120s
  Authentication:
    - IAM Authentication: Enabled
    - Secrets Manager: Enabled
```

### 4. Migration Strategy

#### Phase 1: Schema Migration
```sql
-- Export schema from existing PostgreSQL
pg_dump --schema-only supervision_db > schema.sql

-- Create Aurora cluster
aws rds create-db-cluster \
  --engine aurora-postgresql \
  --engine-version 15.4 \
  --serverless-v2-scaling-configuration MinCapacity=2,MaxCapacity=16

-- Import schema
psql -h aurora-endpoint -d supervision_db -f schema.sql
```

#### Phase 2: Data Migration
```bash
# Option 1: Direct pg_dump/restore (for smaller databases)
pg_dump supervision_db | psql -h aurora-endpoint -d supervision_db

# Option 2: AWS DMS for minimal downtime
# - Create DMS replication instance
# - Set up continuous replication
# - Cutover when lag is minimal
```

#### Phase 3: Connection String Update
```typescript
// Before:
const dbUrl = 'postgresql://user:pass@localhost:5432/supervision_db?pool_max=30'

// After (with RDS Proxy):
const dbUrl = 'postgresql://user:pass@proxy-endpoint.proxy-abc123.us-east-1.rds.amazonaws.com:5432/supervision_db'
```

### 5. CDK Infrastructure Code

```typescript
import * as cdk from 'aws-cdk-lib';
import * as rds from 'aws-cdk-lib/aws-rds';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';

export class AuroraStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // VPC
    const vpc = new ec2.Vpc(this, 'MisterSmithVPC', {
      maxAzs: 2,
    });

    // Database credentials
    const dbSecret = new secretsmanager.Secret(this, 'DBSecret', {
      generateSecretString: {
        secretStringTemplate: JSON.stringify({ username: 'postgres' }),
        generateStringKey: 'password',
        excludeCharacters: ' %+~`#$&*()|[]{}:;<>?!\'/@"\\',
      },
    });

    // Aurora Serverless v2 cluster
    const cluster = new rds.DatabaseCluster(this, 'SupervisionDB', {
      engine: rds.DatabaseClusterEngine.auroraPostgres({
        version: rds.AuroraPostgresEngineVersion.VER_15_4,
      }),
      credentials: rds.Credentials.fromSecret(dbSecret),
      serverlessV2MinCapacity: 2,
      serverlessV2MaxCapacity: 16,
      vpc,
      defaultDatabaseName: 'supervision_db',
      enableDataApi: true,
      backup: {
        retention: cdk.Duration.days(7),
      },
      writer: rds.ClusterInstance.serverlessV2('writer'),
      readers: [
        rds.ClusterInstance.serverlessV2('reader', {
          scaleWithWriter: true,
        }),
      ],
    });

    // RDS Proxy
    const proxy = new rds.DatabaseProxy(this, 'DBProxy', {
      proxyTarget: rds.ProxyTarget.fromCluster(cluster),
      secrets: [dbSecret],
      vpc,
      maxConnectionsPercent: 100,
      maxIdleConnectionsPercent: 50,
      borrowTimeout: cdk.Duration.seconds(120),
      requireTLS: true,
    });

    // Outputs
    new cdk.CfnOutput(this, 'ClusterEndpoint', {
      value: cluster.clusterEndpoint.socketAddress,
    });

    new cdk.CfnOutput(this, 'ProxyEndpoint', {
      value: proxy.endpoint,
    });
  }
}
```

### 6. Rollback Plan

1. **Pre-Migration Backup**
   ```bash
   pg_dump supervision_db > backup_$(date +%Y%m%d_%H%M%S).sql
   ```

2. **Connection String Revert**
   - Keep original connection strings in environment variables
   - Switch back if issues arise

3. **Aurora Snapshot**
   - Take snapshot before going live
   - Can restore to new cluster if needed

### 7. Performance Optimization

#### Query Optimization
- Use Aurora Query Plan Management
- Enable Performance Insights
- Monitor slow query logs

#### Connection Optimization
- Use RDS Proxy for connection pooling
- Implement connection retry logic
- Use read replicas for read-heavy workloads

### 8. Monitoring & Alerts

#### CloudWatch Metrics
- DatabaseConnections
- ServerlessDatabaseCapacity
- CPUUtilization
- ReadLatency/WriteLatency

#### Alarms
- High connection count (>25)
- Capacity scaling events
- Replication lag (if using replicas)

### 9. Cost Estimation

| Component | Configuration | Monthly Cost (Est.) |
|-----------|--------------|-------------------|
| Aurora Serverless v2 | 2-16 ACUs | $150-$1,200 |
| RDS Proxy | 30 connections | $45 |
| Backup Storage | 100GB | $10 |
| **Total** | | **$205-$1,255** |

### 10. Next Steps

1. ✅ Validate Aurora PostgreSQL compatibility
2. ⏳ Create proof-of-concept with sample data
3. ⏳ Performance testing with production workload
4. ⏳ Plan migration window
5. ⏳ Execute migration with rollback ready

### Resources

- [Aurora Serverless v2 Documentation](https://docs.aws.amazon.com/AmazonRDS/latest/AuroraUserGuide/aurora-serverless-v2.html)
- [RDS Proxy Documentation](https://docs.aws.amazon.com/AmazonRDS/latest/AuroraUserGuide/rds-proxy.html)
- [PostgreSQL to Aurora Migration Guide](https://docs.aws.amazon.com/AmazonRDS/latest/AuroraUserGuide/AuroraPostgreSQL.Migrating.html)