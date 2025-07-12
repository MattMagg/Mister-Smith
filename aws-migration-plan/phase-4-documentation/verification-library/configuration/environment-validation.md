# Environment Configuration Validation

## Overview
Commands for validating environment variables, application configurations, and service parameters.

## Prerequisites
- AWS CLI with Systems Manager permissions
- Access to Parameter Store and Secrets Manager
- jq for JSON parsing
- Application access for runtime validation

## Commands

### 1. Parameter Store Validation
```bash
# List all parameters for MisterSmith
aws ssm describe-parameters \
  --filters "Key=Name,Values=/mistersmith/" \
  --query 'Parameters[].[Name,Type,LastModifiedDate,Version]' \
  --output table

# Verify parameter values exist
function verify_parameters() {
    local env=$1  # production, staging, etc.
    local required_params=(
        "/mistersmith/$env/database/host"
        "/mistersmith/$env/database/port"
        "/mistersmith/$env/redis/endpoint"
        "/mistersmith/$env/sqs/queue-url"
        "/mistersmith/$env/s3/bucket"
        "/mistersmith/$env/api/key"
    )
    
    local missing=0
    for param in "${required_params[@]}"; do
        if ! aws ssm get-parameter --name "$param" &>/dev/null; then
            echo "❌ Missing: $param"
            ((missing++))
        else
            echo "✓ Found: $param"
        fi
    done
    
    echo "Total missing parameters: $missing"
    return $missing
}

# Check parameter encryption
function verify_parameter_encryption() {
    aws ssm describe-parameters \
      --filters "Key=Name,Values=/mistersmith/" \
      --query 'Parameters[?Type!=`SecureString`].[Name,Type]' \
      --output table
}

# Validate parameter format
function validate_parameter_format() {
    local param_name=$1
    local expected_format=$2  # regex pattern
    
    value=$(aws ssm get-parameter --name "$param_name" --query 'Parameter.Value' --output text)
    
    if [[ $value =~ $expected_format ]]; then
        echo "✓ $param_name format valid"
        return 0
    else
        echo "❌ $param_name format invalid: $value"
        return 1
    fi
}
```

### 2. Secrets Manager Validation
```bash
# List all secrets
aws secretsmanager list-secrets \
  --filters Key=name,Values=mistersmith \
  --query 'SecretList[].[Name,LastChangedDate,RotationEnabled]' \
  --output table

# Verify secret structure
function verify_secret_structure() {
    local secret_name=$1
    local required_keys=("${@:2}")
    
    secret_value=$(aws secretsmanager get-secret-value \
      --secret-id "$secret_name" \
      --query 'SecretString' \
      --output text)
    
    local missing=0
    for key in "${required_keys[@]}"; do
        if ! echo "$secret_value" | jq -e "has(\"$key\")" &>/dev/null; then
            echo "❌ Missing key in $secret_name: $key"
            ((missing++))
        fi
    done
    
    return $missing
}

# Check secret rotation
function verify_secret_rotation() {
    aws secretsmanager list-secrets \
      --filters Key=name,Values=mistersmith \
      --query 'SecretList[?RotationEnabled!=`true`].Name' \
      --output table
}

# Test secret accessibility
function test_secret_access() {
    local secret_name=$1
    local iam_role=$2
    
    # Assume role and try to access secret
    temp_creds=$(aws sts assume-role \
      --role-arn "arn:aws:iam::123456789012:role/$iam_role" \
      --role-session-name "secret-test" \
      --query 'Credentials.[AccessKeyId,SecretAccessKey,SessionToken]' \
      --output text)
    
    AWS_ACCESS_KEY_ID=$(echo $temp_creds | cut -d' ' -f1) \
    AWS_SECRET_ACCESS_KEY=$(echo $temp_creds | cut -d' ' -f2) \
    AWS_SESSION_TOKEN=$(echo $temp_creds | cut -d' ' -f3) \
    aws secretsmanager get-secret-value --secret-id "$secret_name" &>/dev/null
    
    if [ $? -eq 0 ]; then
        echo "✓ Role $iam_role can access $secret_name"
    else
        echo "❌ Role $iam_role cannot access $secret_name"
    fi
}
```

### 3. Environment Variable Validation
```bash
# Check ECS task definition environment variables
function verify_ecs_env_vars() {
    local cluster=$1
    local service=$2
    
    # Get task definition
    task_def=$(aws ecs describe-services \
      --cluster $cluster \
      --services $service \
      --query 'services[0].taskDefinition' \
      --output text)
    
    # Extract environment variables
    aws ecs describe-task-definition \
      --task-definition $task_def \
      --query 'taskDefinition.containerDefinitions[].environment[]' \
      --output json | jq -r '.[] | "\(.name)=\(.value)"' | sort
}

# Validate required environment variables
function validate_required_env() {
    local task_def=$1
    local required_vars=(
        "NODE_ENV"
        "AWS_REGION"
        "LOG_LEVEL"
        "SERVICE_NAME"
        "METRICS_ENABLED"
    )
    
    current_vars=$(aws ecs describe-task-definition \
      --task-definition $task_def \
      --query 'taskDefinition.containerDefinitions[0].environment[].name' \
      --output json | jq -r '.[]')
    
    for var in "${required_vars[@]}"; do
        if ! echo "$current_vars" | grep -q "^$var$"; then
            echo "❌ Missing required env var: $var"
        else
            echo "✓ Found: $var"
        fi
    done
}

# Check for sensitive values in environment
function check_sensitive_env_vars() {
    local task_def=$1
    local sensitive_patterns=(
        "password"
        "secret"
        "key"
        "token"
        "credential"
    )
    
    env_vars=$(aws ecs describe-task-definition \
      --task-definition $task_def \
      --query 'taskDefinition.containerDefinitions[].environment[]' \
      --output json)
    
    for pattern in "${sensitive_patterns[@]}"; do
        matches=$(echo "$env_vars" | jq -r --arg p "$pattern" '.[] | select(.name | test($p; "i")) | "\(.name)=\(.value)"')
        if [ ! -z "$matches" ]; then
            echo "⚠️  Potential sensitive value in environment:"
            echo "$matches" | sed 's/=.*/=***REDACTED***/'
        fi
    done
}
```

### 4. Database Configuration Validation
```bash
# Verify Aurora parameter groups
function verify_db_parameters() {
    local cluster_id=$1
    
    # Get parameter group
    param_group=$(aws rds describe-db-clusters \
      --db-cluster-identifier $cluster_id \
      --query 'DBClusters[0].DBClusterParameterGroup' \
      --output text)
    
    # Check critical parameters
    critical_params=(
        "shared_preload_libraries"
        "max_connections"
        "log_statement"
        "log_min_duration_statement"
        "ssl"
    )
    
    for param in "${critical_params[@]}"; do
        value=$(aws rds describe-db-cluster-parameters \
          --db-cluster-parameter-group-name $param_group \
          --query "Parameters[?ParameterName=='$param'].ParameterValue" \
          --output text)
        echo "$param = $value"
    done
}

# Test database connectivity with config
function test_db_connection() {
    local endpoint=$1
    local port=$2
    local database=$3
    local secret_name=$4
    
    # Get credentials from Secrets Manager
    creds=$(aws secretsmanager get-secret-value \
      --secret-id $secret_name \
      --query 'SecretString' \
      --output text)
    
    username=$(echo $creds | jq -r '.username')
    password=$(echo $creds | jq -r '.password')
    
    # Test connection
    PGPASSWORD=$password psql \
      -h $endpoint \
      -p $port \
      -U $username \
      -d $database \
      -c "SELECT version();" &>/dev/null
    
    if [ $? -eq 0 ]; then
        echo "✓ Database connection successful"
    else
        echo "❌ Database connection failed"
    fi
}
```

### 5. Application Configuration Validation
```bash
# Validate application config files in S3
function validate_app_config() {
    local bucket=$1
    local config_path=$2
    local env=$3
    
    # Download config file
    aws s3 cp "s3://$bucket/$config_path/$env/config.json" /tmp/config.json
    
    # Validate JSON structure
    if jq . /tmp/config.json &>/dev/null; then
        echo "✓ Config JSON is valid"
    else
        echo "❌ Config JSON is invalid"
        return 1
    fi
    
    # Check required fields
    required_fields=(
        ".api.version"
        ".api.endpoints"
        ".features"
        ".limits.rateLimit"
        ".monitoring.enabled"
    )
    
    for field in "${required_fields[@]}"; do
        if jq -e "$field" /tmp/config.json &>/dev/null; then
            echo "✓ Found required field: $field"
        else
            echo "❌ Missing required field: $field"
        fi
    done
}

# Verify feature flags
function verify_feature_flags() {
    local env=$1
    
    # Get feature flags from Parameter Store
    aws ssm get-parameters-by-path \
      --path "/mistersmith/$env/features/" \
      --recursive \
      --query 'Parameters[].[Name,Value]' \
      --output table
}
```

### 6. Service Discovery Configuration
```bash
# Verify Cloud Map services
function verify_service_discovery() {
    local namespace=$1
    
    # List services in namespace
    namespace_id=$(aws servicediscovery list-namespaces \
      --filters "Name=TYPE,Values=DNS_PRIVATE" \
      --query "Namespaces[?Name=='$namespace'].Id" \
      --output text)
    
    aws servicediscovery list-services \
      --filters "Name=NAMESPACE_ID,Values=$namespace_id" \
      --query 'Services[].[Name,InstanceCount,HealthCheckConfig.Type]' \
      --output table
}

# Test service discovery resolution
function test_service_discovery() {
    local service_name=$1
    local namespace=$2
    
    # Resolve service
    nslookup "$service_name.$namespace" &>/dev/null
    
    if [ $? -eq 0 ]; then
        echo "✓ Service $service_name resolves correctly"
        dig +short "$service_name.$namespace"
    else
        echo "❌ Service $service_name resolution failed"
    fi
}
```

### 7. API Gateway Configuration
```bash
# Verify API Gateway configuration
function verify_api_gateway() {
    local api_name=$1
    
    # Get API details
    api_id=$(aws apigateway get-rest-apis \
      --query "items[?name=='$api_name'].id" \
      --output text)
    
    # Check stages
    aws apigateway get-stages \
      --rest-api-id $api_id \
      --query 'item[].[stageName,deploymentId,cacheClusterEnabled,throttle.rateLimit]' \
      --output table
    
    # Verify custom domain
    aws apigateway get-domain-names \
      --query "items[?contains(domainName, 'mistersmith')].[domainName,certificateName,securityPolicy]" \
      --output table
}

# Test API endpoints
function test_api_endpoints() {
    local base_url=$1
    local api_key=$2
    
    endpoints=(
        "/health"
        "/v1/status"
        "/v1/agents"
        "/v1/config"
    )
    
    for endpoint in "${endpoints[@]}"; do
        response=$(curl -s -o /dev/null -w "%{http_code}" \
          -H "X-API-Key: $api_key" \
          "$base_url$endpoint")
        
        if [ "$response" -eq 200 ]; then
            echo "✓ $endpoint - $response"
        else
            echo "❌ $endpoint - $response"
        fi
    done
}
```

### 8. Comprehensive Configuration Report
```bash
#!/bin/bash
# config-validation-report.sh - Generate configuration validation report

function generate_config_report() {
    local env=$1
    local report_file="config-validation-$(date +%Y%m%d-%H%M%S).json"
    
    echo "Generating configuration validation report for environment: $env"
    
    # Initialize report
    cat > $report_file << EOF
{
  "environment": "$env",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "validations": {}
}
EOF
    
    # Validate Parameter Store
    echo "Validating Parameter Store..."
    param_count=$(aws ssm describe-parameters \
      --filters "Key=Name,Values=/mistersmith/$env/" \
      --query 'length(Parameters)' --output text)
    
    jq --arg count "$param_count" '.validations.parameterStore = {
      "totalParameters": ($count | tonumber),
      "status": "validated"
    }' $report_file > tmp.json && mv tmp.json $report_file
    
    # Validate Secrets Manager
    echo "Validating Secrets Manager..."
    secret_count=$(aws secretsmanager list-secrets \
      --filters Key=name,Values="mistersmith/$env" \
      --query 'length(SecretList)' --output text)
    
    jq --arg count "$secret_count" '.validations.secretsManager = {
      "totalSecrets": ($count | tonumber),
      "status": "validated"
    }' $report_file > tmp.json && mv tmp.json $report_file
    
    # Validate ECS configurations
    echo "Validating ECS configurations..."
    service_count=$(aws ecs list-services \
      --cluster "mistersmith-$env" \
      --query 'length(serviceArns)' --output text)
    
    jq --arg count "$service_count" '.validations.ecsServices = {
      "totalServices": ($count | tonumber),
      "status": "validated"
    }' $report_file > tmp.json && mv tmp.json $report_file
    
    # Generate summary
    cat > "config-summary-$env.md" << EOF
# Configuration Validation Report
Environment: $env
Date: $(date)

## Summary
- Parameter Store entries: $param_count
- Secrets Manager secrets: $secret_count
- ECS services configured: $service_count

## Validation Results
$(jq -r '.validations | to_entries[] | "- \(.key): \(.value.status)"' $report_file)

## Recommendations
1. Ensure all sensitive values are in Secrets Manager
2. Enable rotation for database credentials
3. Review and update parameter versions
4. Validate all service endpoints

Full report: $report_file
EOF
    
    echo "Configuration validation complete. Report: $report_file"
    cat "config-summary-$env.md"
}

# Run validation
generate_config_report "${1:-production}"
```

### 9. Configuration Drift Detection
```bash
#!/bin/bash
# config-drift-detection.sh - Detect configuration drift

function detect_config_drift() {
    local env=$1
    local baseline_file="${2:-config-baseline.json}"
    
    # Capture current state
    current_state_file="config-current-$(date +%Y%m%d-%H%M%S).json"
    
    # Get current parameters
    aws ssm get-parameters-by-path \
      --path "/mistersmith/$env/" \
      --recursive \
      --query 'Parameters[].[Name,Value,Version]' \
      --output json > $current_state_file
    
    # Compare with baseline
    if [ -f "$baseline_file" ]; then
        echo "Comparing with baseline..."
        
        # Find differences
        diff -u <(jq -S . $baseline_file) <(jq -S . $current_state_file) > config-drift.diff
        
        if [ -s config-drift.diff ]; then
            echo "⚠️  Configuration drift detected!"
            echo "Changes found:"
            grep "^[+-]" config-drift.diff | grep -v "^[+-][+-][+-]" | head -20
        else
            echo "✓ No configuration drift detected"
        fi
    else
        echo "No baseline found. Creating baseline..."
        cp $current_state_file $baseline_file
    fi
}

# Monitor configuration changes
function monitor_config_changes() {
    local env=$1
    local interval=${2:-300}  # 5 minutes
    
    while true; do
        clear
        echo "=== Configuration Monitor - $env ==="
        echo "Time: $(date)"
        
        # Check recent parameter changes
        echo -e "\nRecent Parameter Store changes:"
        aws ssm describe-parameters \
          --filters "Key=Name,Values=/mistersmith/$env/" \
          --query 'Parameters[] | sort_by(@, &LastModifiedDate) | [-5:].[Name,LastModifiedDate]' \
          --output table
        
        # Check secret rotations
        echo -e "\nUpcoming secret rotations:"
        aws secretsmanager list-secrets \
          --filters Key=name,Values="mistersmith/$env" \
          --query 'SecretList[?NextRotationDate!=`null`].[Name,NextRotationDate]' \
          --output table
        
        sleep $interval
    done
}
```

## Troubleshooting

### Common Issues
1. **Parameter Not Found**: Check parameter path and permissions
2. **Secret Access Denied**: Verify IAM role policies
3. **Connection Failed**: Check network security groups and endpoints
4. **Invalid Format**: Review parameter value requirements

### Debug Commands
```bash
# Enable debug logging
export AWS_DEBUG=1

# Test with specific role
aws sts get-caller-identity

# Check effective permissions
aws iam simulate-principal-policy \
  --policy-source-arn arn:aws:iam::123456789012:role/MyRole \
  --action-names ssm:GetParameter \
  --resource-arns arn:aws:ssm:region:123456789012:parameter/mistersmith/*
```