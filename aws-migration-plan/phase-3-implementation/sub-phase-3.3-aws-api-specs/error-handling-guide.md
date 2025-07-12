# AWS API Error Handling Guide for MisterSmith

## Common API Errors and Mitigation Strategies

### 1. VPC and Networking Errors

#### InvalidVpcID.NotFound
```json
{
  "Error": "InvalidVpcID.NotFound",
  "Message": "The vpc ID 'vpc-xxx' does not exist",
  "Mitigation": {
    "retry": false,
    "action": "Verify VPC creation completed",
    "code": "await ec2.describeVpcs({ VpcIds: [vpcId] }).promise()"
  }
}
```

#### InvalidSubnet.Conflict
```json
{
  "Error": "InvalidSubnet.Conflict",
  "Message": "The CIDR '10.0.1.0/24' conflicts with another subnet",
  "Mitigation": {
    "retry": false,
    "action": "Check existing subnets and adjust CIDR",
    "code": "await ec2.describeSubnets({ Filters: [{ Name: 'vpc-id', Values: [vpcId] }] }).promise()"
  }
}
```

### 2. ECS Service Errors

#### InvalidParameterException
```json
{
  "Error": "InvalidParameterException",
  "Common Causes": [
    "Task definition not found",
    "Invalid launch type",
    "Missing required parameters"
  ],
  "Mitigation": {
    "retry": false,
    "validation": {
      "taskDefinition": "Ensure format is family:revision or full ARN",
      "launchType": "Must be 'EC2' or 'FARGATE'",
      "networkConfiguration": "Required for FARGATE launch type"
    }
  }
}
```

#### ResourceNotFoundException
```json
{
  "Error": "ResourceNotFoundException",
  "Context": "ECS Cluster not found",
  "Mitigation": {
    "retry": true,
    "backoff": "exponential",
    "maxRetries": 3,
    "action": "Verify cluster exists and is ACTIVE"
  }
}
```

### 3. RDS/Aurora Errors

#### DBClusterAlreadyExistsFault
```json
{
  "Error": "DBClusterAlreadyExistsFault",
  "Mitigation": {
    "retry": false,
    "action": "Use unique identifier or check existing clusters",
    "code": "await rds.describeDBClusters({ DBClusterIdentifier: clusterId }).promise()"
  }
}
```

#### InsufficientDBClusterCapacityFault
```json
{
  "Error": "InsufficientDBClusterCapacityFault",
  "Mitigation": {
    "retry": true,
    "backoff": "linear",
    "delay": 300000,
    "maxRetries": 5,
    "fallback": "Try different availability zone or instance class"
  }
}
```

### 4. IAM Permission Errors

#### AccessDenied
```json
{
  "Error": "AccessDenied",
  "Common Scenarios": [
    {
      "service": "ECS",
      "required_permissions": [
        "ecs:CreateCluster",
        "ecs:RegisterTaskDefinition",
        "ecs:CreateService"
      ]
    },
    {
      "service": "RDS",
      "required_permissions": [
        "rds:CreateDBCluster",
        "rds:CreateDBInstance",
        "kms:CreateGrant"
      ]
    }
  ],
  "Mitigation": {
    "action": "Verify IAM policies and service-linked roles"
  }
}
```

### 5. Rate Limiting and Throttling

#### ThrottlingException
```json
{
  "Error": "ThrottlingException",
  "Services": ["ECS", "Lambda", "DynamoDB"],
  "Mitigation": {
    "retry": true,
    "strategy": "exponentialBackoff",
    "implementation": {
      "initialDelay": 100,
      "maxDelay": 20000,
      "multiplier": 2,
      "jitter": true
    }
  }
}
```

## Retry Strategy Implementation

### Exponential Backoff with Jitter
```javascript
class AWSRetryHandler {
  constructor(options = {}) {
    this.maxRetries = options.maxRetries || 3;
    this.initialDelay = options.initialDelay || 100;
    this.maxDelay = options.maxDelay || 20000;
    this.multiplier = options.multiplier || 2;
  }

  async executeWithRetry(operation, context) {
    let lastError;
    
    for (let attempt = 0; attempt <= this.maxRetries; attempt++) {
      try {
        return await operation();
      } catch (error) {
        lastError = error;
        
        if (!this.isRetryable(error) || attempt === this.maxRetries) {
          throw error;
        }
        
        const delay = this.calculateDelay(attempt);
        console.log(`Retry attempt ${attempt + 1} after ${delay}ms for ${context}`);
        await this.sleep(delay);
      }
    }
    
    throw lastError;
  }

  isRetryable(error) {
    const retryableErrors = [
      'ThrottlingException',
      'TooManyRequestsException',
      'RequestLimitExceeded',
      'ServiceUnavailable',
      'InternalServerError'
    ];
    
    return retryableErrors.includes(error.code) || 
           (error.statusCode >= 500 && error.statusCode < 600);
  }

  calculateDelay(attempt) {
    const delay = Math.min(
      this.initialDelay * Math.pow(this.multiplier, attempt),
      this.maxDelay
    );
    
    // Add jitter (±25%)
    const jitter = delay * 0.25 * (Math.random() * 2 - 1);
    return Math.floor(delay + jitter);
  }

  sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}
```

## Service-Specific Error Handling

### ECS Task Definition Registration
```javascript
async function registerTaskDefinitionSafely(ecs, params) {
  try {
    return await ecs.registerTaskDefinition(params).promise();
  } catch (error) {
    if (error.code === 'ClientException' && error.message.includes('Role is not valid')) {
      // IAM role might not be propagated yet
      console.log('IAM role not ready, waiting 10 seconds...');
      await new Promise(resolve => setTimeout(resolve, 10000));
      return await ecs.registerTaskDefinition(params).promise();
    }
    throw error;
  }
}
```

### RDS Cluster Creation
```javascript
async function createAuroraClusterSafely(rds, params) {
  const retryHandler = new AWSRetryHandler({ maxRetries: 5 });
  
  try {
    return await retryHandler.executeWithRetry(
      () => rds.createDBCluster(params).promise(),
      'CreateDBCluster'
    );
  } catch (error) {
    if (error.code === 'InvalidDBClusterStateFault') {
      // Cluster might be in incompatible state
      const cluster = await rds.describeDBClusters({
        DBClusterIdentifier: params.DBClusterIdentifier
      }).promise();
      
      if (cluster.DBClusters[0].Status === 'creating') {
        // Wait for creation to complete
        return await waitForClusterAvailable(rds, params.DBClusterIdentifier);
      }
    }
    throw error;
  }
}
```

### DynamoDB Table Creation
```javascript
async function createDynamoTableSafely(dynamodb, params) {
  try {
    return await dynamodb.createTable(params).promise();
  } catch (error) {
    if (error.code === 'ResourceInUseException') {
      // Table already exists, check if it matches our requirements
      const existing = await dynamodb.describeTable({
        TableName: params.TableName
      }).promise();
      
      if (validateTableSchema(existing.Table, params)) {
        console.log(`Table ${params.TableName} already exists with correct schema`);
        return existing;
      } else {
        throw new Error(`Table ${params.TableName} exists but schema doesn't match`);
      }
    }
    throw error;
  }
}
```

## Deployment Rollback Procedures

### Automated Rollback on Failure
```javascript
class DeploymentManager {
  constructor() {
    this.deploymentStack = [];
  }

  async deploy(step) {
    try {
      const result = await step.action();
      this.deploymentStack.push({
        step: step.name,
        rollback: step.rollback,
        resource: result
      });
      return result;
    } catch (error) {
      console.error(`Deployment failed at step: ${step.name}`);
      await this.rollback();
      throw error;
    }
  }

  async rollback() {
    console.log('Starting rollback...');
    
    while (this.deploymentStack.length > 0) {
      const item = this.deploymentStack.pop();
      try {
        console.log(`Rolling back: ${item.step}`);
        await item.rollback(item.resource);
      } catch (rollbackError) {
        console.error(`Rollback failed for ${item.step}:`, rollbackError);
        // Continue with other rollbacks
      }
    }
  }
}
```

## Monitoring and Alerting for API Errors

### CloudWatch Metrics for API Calls
```javascript
const AWS = require('aws-sdk');
const cloudwatch = new AWS.CloudWatch();

async function trackAPIMetric(service, operation, error = null) {
  const params = {
    Namespace: 'MisterSmith/APIOperations',
    MetricData: [
      {
        MetricName: error ? 'APIErrors' : 'APISuccess',
        Value: 1,
        Unit: 'Count',
        Dimensions: [
          { Name: 'Service', Value: service },
          { Name: 'Operation', Value: operation }
        ],
        Timestamp: new Date()
      }
    ]
  };

  if (error) {
    params.MetricData[0].Dimensions.push({
      Name: 'ErrorCode',
      Value: error.code || 'Unknown'
    });
  }

  await cloudwatch.putMetricData(params).promise();
}
```

## Best Practices Summary

1. **Always implement retry logic** for transient errors
2. **Use exponential backoff** with jitter for rate limiting
3. **Validate resources exist** before dependent operations
4. **Implement circuit breakers** for cascading failures
5. **Log all errors** with context for debugging
6. **Monitor API error rates** and set up alerts
7. **Test rollback procedures** regularly
8. **Use service quotas** to prevent hitting limits
9. **Implement graceful degradation** where possible
10. **Document error scenarios** and recovery procedures