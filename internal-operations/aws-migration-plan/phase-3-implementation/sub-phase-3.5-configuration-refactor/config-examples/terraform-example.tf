# Terraform configuration for AWS Parameter Store and Secrets Manager
# This creates the configuration resources needed for MisterSmith

variable "environment" {
  description = "Environment name (dev, staging, production)"
  type        = string
}

# Parameter Store entries for non-sensitive configuration
resource "aws_ssm_parameter" "nats_url" {
  name  = "/mistersmith/${var.environment}/nats/url"
  type  = "String"
  value = var.environment == "production" ? "nats://prod-nats.internal:4222" : "nats://staging-nats.internal:4222"
  
  tags = {
    Environment = var.environment
    Service     = "mistersmith"
    Type        = "configuration"
  }
}

resource "aws_ssm_parameter" "http_port" {
  name  = "/mistersmith/${var.environment}/http/port"
  type  = "String"
  value = "8080"
  
  tags = {
    Environment = var.environment
    Service     = "mistersmith"
    Type        = "configuration"
  }
}

resource "aws_ssm_parameter" "sse_port" {
  name  = "/mistersmith/${var.environment}/http/sse_port"
  type  = "String"
  value = "3000"
  
  tags = {
    Environment = var.environment
    Service     = "mistersmith"
    Type        = "configuration"
  }
}

resource "aws_ssm_parameter" "circuit_breaker_timeout" {
  name  = "/mistersmith/${var.environment}/timeouts/circuit_breaker"
  type  = "String"
  value = var.environment == "production" ? "1s" : "100ms"
  
  tags = {
    Environment = var.environment
    Service     = "mistersmith"
    Type        = "configuration"
  }
}

resource "aws_ssm_parameter" "process_timeout" {
  name  = "/mistersmith/${var.environment}/timeouts/process_execution"
  type  = "String"
  value = var.environment == "production" ? "30s" : "10s"
  
  tags = {
    Environment = var.environment
    Service     = "mistersmith"
    Type        = "configuration"
  }
}

# Secrets Manager for sensitive data
resource "aws_secretsmanager_secret" "mistersmith_credentials" {
  name = "mistersmith/${var.environment}/credentials"
  
  tags = {
    Environment = var.environment
    Service     = "mistersmith"
    Type        = "secrets"
  }
}

resource "aws_secretsmanager_secret_version" "mistersmith_credentials" {
  secret_id = aws_secretsmanager_secret.mistersmith_credentials.id
  
  secret_string = jsonencode({
    nats_username = var.nats_username
    nats_password = var.nats_password
    api_key       = var.api_key
  })
}

# IAM policy for ECS task to access configuration
resource "aws_iam_policy" "mistersmith_config_access" {
  name        = "mistersmith-${var.environment}-config-access"
  description = "Allow MisterSmith to access configuration in Parameter Store and Secrets Manager"
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ssm:GetParameter",
          "ssm:GetParameters",
          "ssm:GetParametersByPath"
        ]
        Resource = "arn:aws:ssm:*:*:parameter/mistersmith/${var.environment}/*"
      },
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = aws_secretsmanager_secret.mistersmith_credentials.arn
      }
    ]
  })
}

# Attach policy to ECS task role
resource "aws_iam_role_policy_attachment" "mistersmith_config_access" {
  role       = var.ecs_task_role_name
  policy_arn = aws_iam_policy.mistersmith_config_access.arn
}

# Outputs for reference
output "parameter_store_prefix" {
  value = "/mistersmith/${var.environment}/"
}

output "secrets_arn" {
  value     = aws_secretsmanager_secret.mistersmith_credentials.arn
  sensitive = true
}